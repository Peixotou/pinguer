//! Source management for the Pinguer compiler.
//!
//! This crate owns the text of every file taking part in a compilation and
//! provides [`Span`]s: compact, copyable references to byte ranges within a
//! file. Every later compiler stage reports locations through spans, and the
//! diagnostics renderer converts them back to human-readable lines and columns.
//!
//! # Conventions
//!
//! * Offsets are byte offsets into the file's text *after* an optional UTF-8
//!   byte order mark has been removed.
//! * Lines and columns are 1-based. Columns count Unicode scalar values, not
//!   bytes, display cells or UTF-16 code units.
//! * Only `\n` starts a new line. A `\r\n` pair therefore ends a line, while a
//!   lone `\r` does not (the lexer rejects lone carriage returns).

use std::fmt;
use std::io;
use std::ops::Range;
use std::path::Path;

/// The largest source file, in bytes, that the compiler accepts.
///
/// Offsets are stored as `u32` to keep [`Span`] small, which bounds the size
/// of a single file to 4 GiB minus one byte.
pub const MAX_FILE_SIZE: usize = u32::MAX as usize;

const BYTE_ORDER_MARK: char = '\u{FEFF}';

/// Identifies a file registered in a [`SourceMap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(u32);

impl FileId {
    /// Returns the zero-based index of this file within its [`SourceMap`].
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A half-open byte range `start..end` within a single source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    /// The file this span belongs to.
    pub file: FileId,
    /// Byte offset of the first byte in the span.
    pub start: u32,
    /// Byte offset one past the last byte in the span.
    pub end: u32,
}

impl Span {
    /// Creates a span covering `start..end` in `file`.
    ///
    /// # Panics
    ///
    /// Panics if `start > end`.
    #[must_use]
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        assert!(start <= end, "span start {start} is after span end {end}");
        Self { file, start, end }
    }

    /// Returns the length of the span in bytes.
    #[must_use]
    pub fn len(self) -> u32 {
        self.end - self.start
    }

    /// Returns `true` if the span covers no bytes.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns the smallest span that covers both `self` and `other`.
    ///
    /// # Panics
    ///
    /// Panics if the spans belong to different files.
    #[must_use]
    pub fn to(self, other: Span) -> Span {
        assert_eq!(
            self.file, other.file,
            "cannot merge spans from different files"
        );
        Span::new(
            self.file,
            self.start.min(other.start),
            self.end.max(other.end),
        )
    }

    /// Returns the span as a `usize` range suitable for slicing source text.
    #[must_use]
    pub fn range(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }
}

/// A 1-based line and column position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineCol {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column, counted in Unicode scalar values.
    pub column: u32,
}

impl fmt::Display for LineCol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A single source file and its precomputed line index.
#[derive(Debug)]
pub struct SourceFile {
    name: String,
    text: String,
    /// Byte offset at which each line starts. Always contains at least `0`.
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(name: String, text: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(index, _)| {
                // `text.len() <= MAX_FILE_SIZE` is checked by the caller, so
                // `index + 1 <= u32::MAX` and the conversion cannot truncate.
                (index + 1) as u32
            }))
            .collect();
        Self {
            name,
            text,
            line_starts,
        }
    }

    /// Returns the display name of the file, usually its path.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the full text of the file.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the number of lines in the file.
    ///
    /// An empty file has one (empty) line, and a trailing `\n` starts a new,
    /// empty final line.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Converts a byte offset to a 1-based line and column.
    ///
    /// Offsets that fall inside a multi-byte character are attributed to that
    /// character's column.
    ///
    /// # Panics
    ///
    /// Panics if `offset` is greater than the length of the file.
    #[must_use]
    pub fn line_col(&self, offset: u32) -> LineCol {
        assert!(
            offset as usize <= self.text.len(),
            "offset {offset} is outside of file `{}` ({} bytes)",
            self.name,
            self.text.len()
        );
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(insertion) => insertion - 1,
        };
        let line_start = self.line_starts[line_index] as usize;
        let prefix = &self.text.as_bytes()[line_start..offset as usize];
        // Count the characters that *start* before `offset`. Continuation
        // bytes (0b10xx_xxxx) do not start a character.
        let preceding_chars = prefix.iter().filter(|&&b| b & 0xC0 != 0x80).count();
        LineCol {
            line: line_index as u32 + 1,
            column: preceding_chars as u32 + 1,
        }
    }

    /// Returns the byte range of a 1-based line, excluding its line terminator
    /// (`\n` or `\r\n`).
    ///
    /// # Panics
    ///
    /// Panics if `line` is `0` or greater than [`SourceFile::line_count`].
    #[must_use]
    pub fn line_range(&self, line: usize) -> Range<usize> {
        assert!(
            line >= 1 && line <= self.line_count(),
            "line {line} is outside of file `{}`",
            self.name
        );
        let start = self.line_starts[line - 1] as usize;
        let mut end = self
            .line_starts
            .get(line)
            .map_or(self.text.len(), |&next| next as usize - 1);
        if end > start && self.text.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        start..end
    }

    /// Returns the text of a 1-based line, excluding its line terminator.
    ///
    /// # Panics
    ///
    /// Panics if `line` is `0` or greater than [`SourceFile::line_count`].
    #[must_use]
    pub fn line_text(&self, line: usize) -> &str {
        &self.text[self.line_range(line)]
    }
}

/// An error produced while registering or loading a source file.
#[derive(Debug)]
pub enum SourceError {
    /// The file could not be read.
    Io(io::Error),
    /// The file is not valid UTF-8.
    InvalidUtf8 {
        /// Byte offset of the first invalid byte sequence in the file.
        offset: usize,
    },
    /// The file exceeds [`MAX_FILE_SIZE`].
    TooLarge {
        /// Size of the file in bytes.
        size: usize,
    },
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::Io(error) => write!(f, "{error}"),
            SourceError::InvalidUtf8 { offset } => {
                write!(
                    f,
                    "file is not valid UTF-8 (invalid byte at offset {offset})"
                )
            }
            SourceError::TooLarge { size } => write!(
                f,
                "file is {size} bytes, which exceeds the maximum of {MAX_FILE_SIZE} bytes"
            ),
        }
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SourceError::Io(error) => Some(error),
            SourceError::InvalidUtf8 { .. } | SourceError::TooLarge { .. } => None,
        }
    }
}

/// Owns every source file taking part in a compilation.
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    /// Creates an empty source map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers in-memory source text under `name`.
    ///
    /// A leading UTF-8 byte order mark is removed.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::TooLarge`] if the text exceeds [`MAX_FILE_SIZE`].
    pub fn add(
        &mut self,
        name: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<FileId, SourceError> {
        let mut text = text.into();
        if text.starts_with(BYTE_ORDER_MARK) {
            text.drain(..BYTE_ORDER_MARK.len_utf8());
        }
        if text.len() > MAX_FILE_SIZE {
            return Err(SourceError::TooLarge { size: text.len() });
        }
        let id = u32::try_from(self.files.len()).expect("more than u32::MAX source files");
        self.files.push(SourceFile::new(name.into(), text));
        Ok(FileId(id))
    }

    /// Reads a file from disk, validates that it is UTF-8 and registers it.
    ///
    /// The file's display name is the path exactly as given.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is not valid UTF-8 or is
    /// larger than [`MAX_FILE_SIZE`].
    pub fn load(&mut self, path: &Path) -> Result<FileId, SourceError> {
        let bytes = std::fs::read(path).map_err(SourceError::Io)?;
        if bytes.len() > MAX_FILE_SIZE {
            return Err(SourceError::TooLarge { size: bytes.len() });
        }
        let text = String::from_utf8(bytes).map_err(|error| SourceError::InvalidUtf8 {
            offset: error.utf8_error().valid_up_to(),
        })?;
        self.add(path.display().to_string(), text)
    }

    /// Returns the file identified by `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not produced by this source map.
    #[must_use]
    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.index()]
    }

    /// Returns the source text covered by `span`.
    ///
    /// # Panics
    ///
    /// Panics if the span does not belong to this source map or does not fall
    /// on character boundaries.
    #[must_use]
    pub fn snippet(&self, span: Span) -> &str {
        &self.file(span.file).text()[span.range()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single(text: &str) -> (SourceMap, FileId) {
        let mut map = SourceMap::new();
        let id = map.add("test.pgr", text).unwrap();
        (map, id)
    }

    fn lc(line: u32, column: u32) -> LineCol {
        LineCol { line, column }
    }

    #[test]
    fn empty_file_has_one_line() {
        let (map, id) = single("");
        let file = map.file(id);
        assert_eq!(file.line_count(), 1);
        assert_eq!(file.line_text(1), "");
        assert_eq!(file.line_col(0), lc(1, 1));
    }

    #[test]
    fn line_col_handles_lf_and_crlf() {
        let (map, id) = single("ab\ncd\r\nef");
        let file = map.file(id);
        assert_eq!(file.line_count(), 3);
        assert_eq!(file.line_col(0), lc(1, 1));
        assert_eq!(file.line_col(2), lc(1, 3)); // the '\n' itself
        assert_eq!(file.line_col(3), lc(2, 1));
        assert_eq!(file.line_col(5), lc(2, 3)); // the '\r'
        assert_eq!(file.line_col(7), lc(3, 1));
        assert_eq!(file.line_col(9), lc(3, 3)); // end of file
        assert_eq!(file.line_text(1), "ab");
        assert_eq!(file.line_text(2), "cd");
        assert_eq!(file.line_text(3), "ef");
    }

    #[test]
    fn trailing_newline_starts_an_empty_line() {
        let (map, id) = single("x\n");
        let file = map.file(id);
        assert_eq!(file.line_count(), 2);
        assert_eq!(file.line_text(2), "");
        assert_eq!(file.line_col(2), lc(2, 1));
    }

    #[test]
    fn columns_count_unicode_scalar_values() {
        // 'é' is 2 bytes, '🐧' is 4 bytes.
        let (map, id) = single("é🐧x");
        let file = map.file(id);
        assert_eq!(file.line_col(0), lc(1, 1));
        assert_eq!(file.line_col(2), lc(1, 2));
        assert_eq!(file.line_col(6), lc(1, 3));
        assert_eq!(file.line_col(7), lc(1, 4));
        // An offset inside the penguin is attributed to the penguin.
        assert_eq!(file.line_col(4), lc(1, 2 + 1));
    }

    #[test]
    fn lone_carriage_return_is_not_a_line_break() {
        let (map, id) = single("a\rb");
        let file = map.file(id);
        assert_eq!(file.line_count(), 1);
        assert_eq!(file.line_col(2), lc(1, 3));
    }

    #[test]
    fn byte_order_mark_is_removed() {
        let (map, id) = single("\u{FEFF}func");
        assert_eq!(map.file(id).text(), "func");
    }

    #[test]
    fn snippet_returns_span_text() {
        let (map, id) = single("let x = 1");
        assert_eq!(map.snippet(Span::new(id, 4, 5)), "x");
    }

    #[test]
    fn span_merge_covers_both_spans() {
        let (_, id) = single("abcdef");
        let merged = Span::new(id, 4, 5).to(Span::new(id, 1, 2));
        assert_eq!(merged, Span::new(id, 1, 5));
        assert_eq!(merged.len(), 4);
        assert!(!merged.is_empty());
    }

    #[test]
    #[should_panic(expected = "span start")]
    fn inverted_span_panics() {
        let (_, id) = single("abc");
        let _ = Span::new(id, 2, 1);
    }

    #[test]
    fn load_rejects_invalid_utf8() {
        let dir = std::env::temp_dir().join(format!("pinguer-source-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("invalid.pgr");
        std::fs::write(&path, b"ok\xFFrest").unwrap();
        let mut map = SourceMap::new();
        let error = map.load(&path).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(matches!(error, SourceError::InvalidUtf8 { offset: 2 }));
    }

    #[test]
    fn load_reports_missing_files() {
        let mut map = SourceMap::new();
        let error = map
            .load(Path::new("this/file/does/not/exist.pgr"))
            .unwrap_err();
        assert!(matches!(error, SourceError::Io(_)));
    }
}
