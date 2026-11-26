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
use std::fs::{self, File};
use std::io::{self, Read};
use std::ops::Range;
use std::path::Path;

/// The largest source file, in bytes, that the compiler accepts.
///
/// Offsets are stored as `u32` to keep [`Span`] small, which bounds the size
/// of a single file to 4 GiB minus one byte.
pub const MAX_FILE_SIZE: usize = u32::MAX as usize;

const BYTE_ORDER_MARK: char = '\u{FEFF}';
const BYTE_ORDER_MARK_UTF8: &[u8] = b"\xEF\xBB\xBF";

/// Distance in bytes between two entries of [`SourceFile::char_checkpoints`].
///
/// Converting an offset to a column scans at most this many bytes twice, and
/// the checkpoints of a non-ASCII file cost 4 bytes per `CHECKPOINT_STRIDE`
/// bytes of text (about 1.6% of the file size).
const CHECKPOINT_STRIDE: usize = 256;

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
    /// Entry `i` is the number of characters that start before byte offset
    /// `i * CHECKPOINT_STRIDE`. Empty when the text is ASCII, because byte
    /// and character offsets then coincide.
    char_checkpoints: Vec<u32>,
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
        let char_checkpoints = if text.is_ascii() {
            Vec::new()
        } else {
            let mut checkpoints = Vec::with_capacity(text.len() / CHECKPOINT_STRIDE + 1);
            let mut chars = 0;
            checkpoints.push(chars);
            for chunk in text.as_bytes().as_chunks::<CHECKPOINT_STRIDE>().0 {
                // At most `text.len()` characters, so the sum fits in a `u32`.
                chars += count_char_starts(chunk) as u32;
                checkpoints.push(chars);
            }
            checkpoints
        };
        Self {
            name,
            text,
            line_starts,
            char_checkpoints,
        }
    }

    /// Returns the number of characters that start before `offset`, which
    /// must be a character boundary.
    fn chars_before(&self, offset: usize) -> usize {
        if self.char_checkpoints.is_empty() {
            return offset;
        }
        let checkpoint = offset / CHECKPOINT_STRIDE;
        let scanned = &self.text.as_bytes()[checkpoint * CHECKPOINT_STRIDE..offset];
        self.char_checkpoints[checkpoint] as usize + count_char_starts(scanned)
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
    /// The cost does not depend on the length of the line: a binary search
    /// over the line starts, plus a scan of at most two blocks of 256 bytes
    /// in files that contain non-ASCII text.
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
        let mut offset = offset as usize;
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        let preceding_chars = self.chars_before(offset) - self.chars_before(line_start);
        // Both values are at most `MAX_FILE_SIZE`, so they fit in a `u32`.
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
        let Some(&next) = self.line_starts.get(line) else {
            // The last line has no terminator; a carriage return at its end
            // is a lone carriage return and part of the line's content.
            return start..self.text.len();
        };
        let mut end = next as usize - 1;
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

/// Counts the bytes that start a UTF-8 character, that is, every byte except
/// continuation bytes (`0b10xx_xxxx`).
fn count_char_starts(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&byte| byte & 0xC0 != 0x80).count()
}

/// Reads all of `reader` if it holds at most `limit` bytes of text after an
/// optional byte order mark, without ever reading more than one byte past
/// that. `size_hint` is the size the file system reports for a regular file:
/// it sizes the buffer and rejects a file that is already known to be too
/// large, but the limit holds even when the hint is wrong.
fn read_bounded(
    reader: impl Read,
    size_hint: Option<u64>,
    limit: usize,
) -> Result<Vec<u8>, SourceError> {
    let max_read = limit as u64 + BYTE_ORDER_MARK_UTF8.len() as u64;
    if let Some(size) = size_hint
        && size > max_read
    {
        return Err(SourceError::TooLarge { size: Some(size) });
    }
    let capacity = size_hint
        .and_then(|size| usize::try_from(size).ok())
        .unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .take(max_read + 1)
        .read_to_end(&mut bytes)
        .map_err(SourceError::Io)?;
    if bytes.len() as u64 > max_read {
        return Err(SourceError::TooLarge { size: None });
    }
    Ok(bytes)
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
        /// Size of the file in bytes, if known. It is unknown when the input
        /// is not a regular file, for example a pipe, and reading stopped at
        /// the limit.
        size: Option<u64>,
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
            SourceError::TooLarge { size: Some(size) } => write!(
                f,
                "file is {size} bytes, which exceeds the maximum of {MAX_FILE_SIZE} bytes"
            ),
            SourceError::TooLarge { size: None } => {
                write!(f, "file exceeds the maximum of {MAX_FILE_SIZE} bytes")
            }
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
        self.insert(name.into(), text)
    }

    /// Reads a file from disk, validates that it is UTF-8 and registers it.
    ///
    /// The file's display name is the path exactly as given. A leading UTF-8
    /// byte order mark is removed before validation, so the offset reported
    /// for invalid UTF-8 is relative to the text after it, like every other
    /// offset.
    ///
    /// The file is never read past the size limit, whatever kind of file it
    /// is: a pipe or device that produces endless data, or a file that grows
    /// while it is read, stops being read one byte after the limit.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is not valid UTF-8 or is
    /// larger than [`MAX_FILE_SIZE`].
    pub fn load(&mut self, path: &Path) -> Result<FileId, SourceError> {
        self.load_with_limit(path, MAX_FILE_SIZE)
    }

    /// [`SourceMap::load`] with a configurable size limit, so that tests can
    /// reach the limit without gigabytes of input.
    fn load_with_limit(&mut self, path: &Path, limit: usize) -> Result<FileId, SourceError> {
        let file = File::open(path).map_err(SourceError::Io)?;
        // The size recorded by the file system is a hint only: it is
        // meaningless for pipes and devices and can change during the read.
        let size_hint = file
            .metadata()
            .ok()
            .filter(fs::Metadata::is_file)
            .map(|metadata| metadata.len());
        let mut bytes = read_bounded(file, size_hint, limit)?;
        let size = bytes.len() as u64;
        if bytes.starts_with(BYTE_ORDER_MARK_UTF8) {
            bytes.drain(..BYTE_ORDER_MARK_UTF8.len());
        }
        if bytes.len() > limit {
            return Err(SourceError::TooLarge { size: Some(size) });
        }
        let text = String::from_utf8(bytes).map_err(|error| SourceError::InvalidUtf8 {
            offset: error.utf8_error().valid_up_to(),
        })?;
        self.insert(path.display().to_string(), text)
    }

    /// Registers text whose byte order mark, if any, was already removed.
    fn insert(&mut self, name: String, text: String) -> Result<FileId, SourceError> {
        if text.len() > MAX_FILE_SIZE {
            return Err(SourceError::TooLarge {
                size: Some(text.len() as u64),
            });
        }
        let id = u32::try_from(self.files.len()).expect("more than u32::MAX source files");
        self.files.push(SourceFile::new(name, text));
        Ok(FileId(id))
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
        // Offsets inside the penguin are attributed to the penguin.
        for inside in 3..=5 {
            assert_eq!(file.line_col(inside), lc(1, 2));
        }
    }

    /// The column definition, computed directly from the line start.
    fn naive_line_col(text: &str, offset: usize) -> LineCol {
        let mut offset = offset;
        while !text.is_char_boundary(offset) {
            offset -= 1;
        }
        let before = &text[..offset];
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        LineCol {
            line: before.matches('\n').count() as u32 + 1,
            column: before[line_start..].chars().count() as u32 + 1,
        }
    }

    #[test]
    fn line_col_matches_the_definition_for_every_offset() {
        // Fragments of 1 to 4 bytes so that characters straddle checkpoint
        // boundaries at every alignment; xorshift with fixed seeds.
        const FRAGMENTS: &[&str] = &["a", "\n", "\r\n", "\t", "é", "€", "🐧", "\u{301}", "xyz"];
        for seed in 1..=40u64 {
            let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
            let mut text = String::new();
            let length = 200 + (seed as usize * 97) % 1_500;
            while text.len() < length {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                text.push_str(FRAGMENTS[(state % FRAGMENTS.len() as u64) as usize]);
            }
            let (map, id) = single(&text);
            let file = map.file(id);
            for offset in 0..=text.len() {
                assert_eq!(
                    file.line_col(offset as u32),
                    naive_line_col(&text, offset),
                    "seed {seed}, offset {offset}"
                );
            }
        }
    }

    #[test]
    fn columns_are_exact_at_the_end_of_a_long_non_ascii_line() {
        let mut text = String::from("é");
        text.push_str(&"a".repeat(100_000));
        text.push('🐧');
        text.push_str("\nb");
        let (map, id) = single(&text);
        let file = map.file(id);
        let penguin = text.find('🐧').unwrap() as u32;
        assert_eq!(file.line_col(penguin), lc(1, 100_002));
        assert_eq!(file.line_col(penguin + 4), lc(1, 100_003));
        assert_eq!(file.line_col(text.len() as u32), lc(2, 2));
    }

    #[test]
    fn ascii_files_need_no_character_index() {
        let (map, id) = single(&"let x = 1\n".repeat(1_000));
        assert!(map.file(id).char_checkpoints.is_empty());
        let (map, id) = single(&"é".repeat(1_000));
        // One checkpoint per full 256-byte block, plus the initial one.
        assert_eq!(map.file(id).char_checkpoints.len(), 2_000 / 256 + 1);
    }

    #[test]
    fn lone_carriage_return_is_not_a_line_break() {
        let (map, id) = single("a\rb");
        let file = map.file(id);
        assert_eq!(file.line_count(), 1);
        assert_eq!(file.line_col(2), lc(1, 3));
    }

    #[test]
    fn only_a_line_terminator_is_removed_from_line_text() {
        let (map, id) = single("ab\r\ncd\r\r\nef\r");
        let file = map.file(id);
        assert_eq!(file.line_text(1), "ab");
        // A lone carriage return before a CRLF terminator is content.
        assert_eq!(file.line_text(2), "cd\r");
        // So is a lone carriage return at the end of the file.
        assert_eq!(file.line_text(3), "ef\r");
        assert_eq!(file.line_range(3), 9..12);
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

    /// A file in a temporary directory unique to one test, removed when
    /// dropped.
    struct TempFile {
        dir: std::path::PathBuf,
        path: std::path::PathBuf,
    }

    impl TempFile {
        fn new(test: &str, contents: &[u8]) -> Self {
            let dir =
                std::env::temp_dir().join(format!("pinguer-source-{test}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("file.pgr");
            std::fs::write(&path, contents).unwrap();
            Self { dir, path }
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn invalid_utf8_offset_is_relative_to_the_text_after_the_byte_order_mark() {
        let file = TempFile::new("bom-invalid", b"\xEF\xBB\xBFok\xFFrest");
        let error = SourceMap::new().load(&file.path).unwrap_err();
        // The offset in the file is 5; in the text after the mark it is 2.
        assert!(
            matches!(error, SourceError::InvalidUtf8 { offset: 2 }),
            "{error:?}"
        );
    }

    #[test]
    fn load_removes_one_byte_order_mark_like_add() {
        let file = TempFile::new("bom-twice", "\u{FEFF}\u{FEFF}x".as_bytes());
        let mut map = SourceMap::new();
        let loaded = map.load(&file.path).unwrap();
        let added = map.add("added.pgr", "\u{FEFF}\u{FEFF}x").unwrap();
        assert_eq!(map.file(loaded).text(), "\u{FEFF}x");
        assert_eq!(map.file(loaded).text(), map.file(added).text());
    }

    #[test]
    fn endless_sources_are_read_only_up_to_the_limit() {
        // Like a pipe or a device that never ends. Without the bound this
        // test would not terminate.
        let error = read_bounded(io::repeat(b'x'), None, 1_000).unwrap_err();
        assert!(
            matches!(error, SourceError::TooLarge { size: None }),
            "{error:?}"
        );
    }

    #[test]
    fn a_wrong_size_hint_does_not_lift_the_limit() {
        // A file that grew after its size was read.
        let error = read_bounded(&[b'x'; 20][..], Some(5), 10).unwrap_err();
        assert!(matches!(error, SourceError::TooLarge { size: None }));
        // A hint above the limit rejects the file without reading it.
        let error = read_bounded(io::repeat(b'x'), Some(1 << 40), 10).unwrap_err();
        assert!(matches!(
            error,
            SourceError::TooLarge {
                size: Some(1_099_511_627_776)
            }
        ));
    }

    #[test]
    fn files_up_to_the_limit_load_and_larger_ones_are_rejected() {
        let mut map = SourceMap::new();
        let exact = TempFile::new("limit-exact", b"0123456789");
        assert!(map.load_with_limit(&exact.path, 10).is_ok());
        // The byte order mark does not count toward the limit.
        let marked = TempFile::new("limit-bom", b"\xEF\xBB\xBF0123456789");
        let id = map.load_with_limit(&marked.path, 10).unwrap();
        assert_eq!(map.file(id).text(), "0123456789");
        for (test, contents) in [
            ("limit-over", &b"0123456789a"[..]),
            ("limit-far-over", b"0123456789abcdef"),
        ] {
            let file = TempFile::new(test, contents);
            let error = map.load_with_limit(&file.path, 10).unwrap_err();
            let size = contents.len() as u64;
            assert!(
                matches!(error, SourceError::TooLarge { size: Some(s) } if s == size),
                "{test}: {error:?}"
            );
            assert_eq!(
                error.to_string(),
                format!("file is {size} bytes, which exceeds the maximum of {MAX_FILE_SIZE} bytes")
            );
        }
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
