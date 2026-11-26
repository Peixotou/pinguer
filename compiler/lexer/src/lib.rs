//! Lexical analysis for Pinguer.
//!
//! [`tokenize`] converts UTF-8 source text into a sequence of [`Token`]s,
//! reporting malformed input as diagnostics and recovering so that the rest of
//! the file is still tokenized. The normative description of the lexical
//! structure lives in `specifications/LEXICAL_STRUCTURE.md`; this crate
//! implements it.
//!
//! # Summary of the lexical rules
//!
//! * Whitespace is space, horizontal tab and line breaks (`\n` or `\r\n`). A
//!   carriage return that is not followed by a line feed is an error.
//! * Comments are `// ...` to the end of the line and `/* ... */`, which nest.
//! * Identifiers are ASCII: `[A-Za-z_][A-Za-z0-9_]*`. A lone `_` is the
//!   [`TokenKind::Underscore`] token. Keywords, including those reserved for
//!   future use, cannot be used as identifiers.
//! * Integer literals are decimal (`1_000`), hexadecimal (`0xFF`), octal
//!   (`0o17`) or binary (`0b1010`). Decimal literals cannot have leading
//!   zeros. Floating-point literals require digits on both sides of the `.`
//!   and may have an exponent (`6.02e23`, `1e-9`). `_` may only appear between
//!   two digits. Literal suffixes are not supported.
//! * String literals are delimited by `"` and cannot contain raw line breaks.
//!   Character literals are delimited by `'` and contain exactly one Unicode
//!   scalar value. Both support the escapes `\n \r \t \\ \0 \" \'` and
//!   `\u{X}` with one to six hexadecimal digits.
//! * Unicode bidirectional formatting characters are rejected everywhere,
//!   including inside comments and literals, to prevent "Trojan Source"
//!   attacks (CVE-2021-42574). So are control characters other than tab and
//!   line breaks, and U+2028 and U+2029, which other tools may show as line
//!   breaks or interpret (ADR-0024). Escapes can still produce all of them.
//!
//! # Line breaks and statement termination
//!
//! A run of line breaks (together with any comments and whitespace between
//! them) produces a single [`TokenKind::Newline`] token, but only when:
//!
//! * the innermost open delimiter is `{`, or there is no open delimiter; line
//!   breaks inside `(...)` and `[...]` are insignificant; and
//! * the line break is preceded by a token other than [`TokenKind::Newline`]
//!   (no `Newline` token is produced at the start of the file).
//!
//! A block comment that contains a line break counts as a line break. The
//! parser decides where a `Newline` token terminates a statement and where it
//! is ignored, for example after a binary operator.
//!
//! # Greedy operators
//!
//! Operators are read greedily, so `>>` is a single [`TokenKind::Shr`] token.
//! Where a shorter token is required, the parser splits the greedy one:
//! `>>`, `>=` and `>>=` when closing a generic argument list, and `??` in a
//! type such as `T??` (an optional optional). See
//! `specifications/OPERATORS.md`.

mod token;

pub use token::{Keyword, Token, TokenKind};

use pinguer_diagnostics::{Diagnostic, Diagnostics, Severity, codes};
use pinguer_source::{FileId, MAX_FILE_SIZE, Span};

/// Tokenizes `text`, which is the contents of `file`.
///
/// The returned vector always ends with exactly one [`TokenKind::Eof`] token
/// whose span is empty and located at the end of the text. Problems are
/// reported to `diagnostics` in source order; for every problem the lexer
/// still produces tokens covering the offending input, so later stages can
/// continue.
///
/// If `diagnostics` has a limit, the lexer reports the diagnostics that come
/// first in the source, up to the remaining capacity, and only counts the
/// others. It never holds more than twice that many diagnostics in memory.
///
/// # Panics
///
/// Panics if `text` is longer than [`MAX_FILE_SIZE`]. Text obtained from a
/// [`pinguer_source::SourceMap`] never is.
#[must_use]
pub fn tokenize(file: FileId, text: &str, diagnostics: &mut Diagnostics) -> Vec<Token> {
    assert!(
        text.len() <= MAX_FILE_SIZE,
        "source text exceeds the maximum file size"
    );
    let mut lexer = Lexer {
        file,
        text,
        pos: 0,
        tokens: Vec::new(),
        diagnostics: Vec::new(),
        capacity: diagnostics.remaining_capacity().unwrap_or(usize::MAX),
        discarded: 0,
        delimiters: Vec::new(),
        pending_newline: None,
    };
    lexer.check_text_flow();
    lexer.run();

    lexer.keep_earliest_diagnostics();
    for diagnostic in lexer.diagnostics {
        diagnostics.push(diagnostic);
    }
    diagnostics.record_suppressed(Severity::Error, lexer.discarded);
    lexer.tokens
}

/// Characters that change the visual order of text and are therefore
/// rejected anywhere in source code.
fn is_text_flow_control(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// Control characters other than tab and the line break characters, and the
/// Unicode line and paragraph separators. They are rejected anywhere in
/// source code because other tools show some of them as line breaks and
/// interpret others (ADR-0024).
fn is_forbidden_control(c: char) -> bool {
    (c.is_control() && !matches!(c, '\t' | '\n' | '\r')) || matches!(c, '\u{2028}' | '\u{2029}')
}

/// The name used in diagnostics for a character accepted by
/// [`is_forbidden_control`], and whether other tools may treat it as a line
/// break.
fn forbidden_control_name(c: char) -> (&'static str, bool) {
    match c {
        '\u{B}' => ("vertical tab", true),
        '\u{C}' => ("form feed", true),
        '\u{85}' => ("next line character", true),
        '\u{2028}' => ("line separator", true),
        '\u{2029}' => ("paragraph separator", true),
        _ => ("control character", false),
    }
}

/// The note explaining the form of a Unicode escape.
const UNICODE_ESCAPE_FORM: &str =
    "a Unicode escape is `\\u{` followed by one to six hexadecimal digits and `}`";

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

/// Non-ASCII characters that are treated as part of an (invalid) identifier
/// during error recovery, so that a word such as `café` produces one token
/// and one diagnostic. Combining marks are approximated by the Unicode
/// combining-mark blocks, because the standard library does not expose
/// general categories; this only affects recovery, never valid programs.
fn is_non_ascii_word_char(c: char) -> bool {
    !c.is_ascii()
        && (c.is_alphanumeric()
            || matches!(
                c,
                '\u{0300}'..='\u{036F}'
                    | '\u{1AB0}'..='\u{1AFF}'
                    | '\u{1DC0}'..='\u{1DFF}'
                    | '\u{20D0}'..='\u{20FF}'
                    | '\u{FE20}'..='\u{FE2F}'
            ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Delimiter {
    Paren,
    Bracket,
    Brace,
}

struct Lexer<'a> {
    file: FileId,
    text: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    /// Diagnostics found so far, in the order they were found. Reduced to
    /// the `capacity` earliest in the source whenever it reaches twice that.
    diagnostics: Vec<Diagnostic>,
    /// How many diagnostics `tokenize` may hand over.
    capacity: usize,
    /// Errors dropped because `capacity` earlier ones were already found.
    discarded: usize,
    /// Currently open delimiters. Used only to decide whether line breaks are
    /// significant; mismatched delimiters are reported by the parser.
    delimiters: Vec<Delimiter>,
    /// Span of the first line break seen since the last token, if that line
    /// break was significant.
    pending_newline: Option<Span>,
}

impl Lexer<'_> {
    fn span(&self, start: usize, end: usize) -> Span {
        // `tokenize` guarantees that every offset fits in a `u32`.
        Span::new(self.file, start as u32, end as u32)
    }

    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    /// Records an error. The lexer reports nothing but errors, which is what
    /// `discarded` counts.
    fn report(&mut self, diagnostic: Diagnostic) {
        debug_assert_eq!(diagnostic.severity, Severity::Error);
        self.diagnostics.push(diagnostic);
        if self.diagnostics.len() >= self.capacity.saturating_mul(2) {
            self.keep_earliest_diagnostics();
        }
    }

    /// Sorts the diagnostics by source position and drops all but the first
    /// `capacity`. The sort is stable, so diagnostics at the same position
    /// keep the order in which they were found; repeated reduction therefore
    /// keeps exactly the diagnostics that sorting everything at the end
    /// would keep.
    fn keep_earliest_diagnostics(&mut self) {
        self.diagnostics
            .sort_by_key(|diagnostic| diagnostic.main_label().map(|label| label.span.start));
        if self.diagnostics.len() > self.capacity {
            self.discarded += self.diagnostics.len() - self.capacity;
            self.diagnostics.truncate(self.capacity);
        }
    }

    /// Returns the end of the current line's content: the current position,
    /// moved back over a `\r` that belongs to a `\r\n` pair.
    fn line_content_end(&self) -> usize {
        if self.text[..self.pos].ends_with('\r') && self.peek() == Some('\n') {
            self.pos - 1
        } else {
            self.pos
        }
    }

    fn at_line_end(&self) -> bool {
        match self.peek() {
            None | Some('\n') => true,
            Some('\r') => self.peek_nth(1) == Some('\n'),
            Some(_) => false,
        }
    }

    fn note_line_break(&mut self, start: usize, end: usize) {
        let significant = matches!(self.delimiters.last(), None | Some(Delimiter::Brace));
        if significant && self.pending_newline.is_none() {
            self.pending_newline = Some(self.span(start, end));
        }
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        if let Some(span) = self.pending_newline.take() {
            let follows_token = self
                .tokens
                .last()
                .is_some_and(|token| token.kind != TokenKind::Newline);
            if follows_token {
                self.tokens.push(Token {
                    kind: TokenKind::Newline,
                    span,
                });
            }
        }
        let span = self.span(start, end);
        self.tokens.push(Token { kind, span });
        self.track_delimiter(kind);
    }

    fn track_delimiter(&mut self, kind: TokenKind) {
        let closing = match kind {
            TokenKind::LParen => return self.delimiters.push(Delimiter::Paren),
            TokenKind::LBracket => return self.delimiters.push(Delimiter::Bracket),
            TokenKind::LBrace => return self.delimiters.push(Delimiter::Brace),
            TokenKind::RParen => Delimiter::Paren,
            TokenKind::RBracket => Delimiter::Bracket,
            TokenKind::RBrace => Delimiter::Brace,
            _ => return,
        };
        // Close the innermost matching delimiter, discarding unclosed ones
        // nested inside it. A closer without any matching opener is ignored.
        if let Some(index) = self.delimiters.iter().rposition(|&open| open == closing) {
            self.delimiters.truncate(index);
        }
    }

    /// Reports every bidirectional formatting character, forbidden control
    /// character and lone carriage return in the file, wherever it appears.
    fn check_text_flow(&mut self) {
        let bytes = self.text.as_bytes();
        for (index, c) in self.text.char_indices() {
            if is_text_flow_control(c) {
                let span = self.span(index, index + c.len_utf8());
                let code_point = u32::from(c);
                self.report(
                    Diagnostic::error(format!(
                        "bidirectional formatting character U+{code_point:04X} in source"
                    ))
                    .with_code(codes::BIDI_CONTROL_CHARACTER)
                    .with_primary(span, "this character can make the code appear different from what the compiler reads")
                    .with_note("bidirectional formatting characters can hide malicious code (CVE-2021-42574, \"Trojan Source\")")
                    .with_help(format!(
                        "inside a string or character literal, write the character as the escape `\\u{{{code_point:X}}}`"
                    )),
                );
            } else if is_forbidden_control(c) {
                let span = self.span(index, index + c.len_utf8());
                let code_point = u32::from(c);
                let (name, line_break_look_alike) = forbidden_control_name(c);
                let (label, note) = if line_break_look_alike {
                    (
                        "other tools may display this character as a line break; the compiler does not",
                        "only a line feed, or a carriage return followed by a line feed, ends a line",
                    )
                } else {
                    (
                        "this invisible character can change how a terminal displays the code",
                        "control characters other than tab and line breaks are not allowed in source text",
                    )
                };
                self.report(
                    Diagnostic::error(format!("{name} U+{code_point:04X} in source"))
                        .with_code(codes::FORBIDDEN_CONTROL_CHARACTER)
                        .with_primary(span, label)
                        .with_note(note)
                        .with_help(format!(
                            "inside a string or character literal, write the character as the escape `\\u{{{code_point:X}}}`"
                        )),
                );
            } else if c == '\r' && bytes.get(index + 1) != Some(&b'\n') {
                let span = self.span(index, index + 1);
                self.report(
                    Diagnostic::error("carriage return not followed by a line feed")
                        .with_code(codes::LONE_CARRIAGE_RETURN)
                        .with_primary(span, "lone carriage return")
                        .with_help("use `\\n` or `\\r\\n` line endings"),
                );
            }
        }
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                ' ' | '\t' => {
                    self.bump();
                }
                '\n' => {
                    self.bump();
                    self.note_line_break(start, self.pos);
                }
                '\r' => {
                    self.bump();
                    if self.eat('\n') {
                        self.note_line_break(start, self.pos);
                    }
                    // A lone carriage return was reported by `check_text_flow`
                    // and is otherwise treated as whitespace.
                }
                '/' if self.peek_nth(1) == Some('/') => self.line_comment(),
                '/' if self.peek_nth(1) == Some('*') => self.block_comment(),
                '"' => self.string(),
                '\'' => self.char_literal(),
                '0'..='9' => self.number(),
                c if c == '_' || c.is_ascii_alphabetic() => self.identifier(),
                c if !c.is_ascii() && c.is_alphanumeric() => self.identifier(),
                c if is_text_flow_control(c) => {
                    // Reported by `check_text_flow`.
                    self.bump();
                }
                c if is_forbidden_control(c) => {
                    // Reported by `check_text_flow`; an `Unknown` token keeps
                    // the place of the rejected character.
                    self.bump();
                    self.push(TokenKind::Unknown, start, self.pos);
                }
                _ => self.punctuation_or_unknown(),
            }
        }
        let end = self.text.len();
        self.push(TokenKind::Eof, end, end);
    }

    fn line_comment(&mut self) {
        while !self.at_line_end() {
            self.bump();
        }
    }

    fn block_comment(&mut self) {
        let start = self.pos;
        self.pos += 2;
        let mut depth = 1usize;
        let mut contains_line_break = false;
        loop {
            match self.peek() {
                None => {
                    let span = self.span(start, start + 2);
                    self.report(
                        Diagnostic::error("unterminated block comment")
                            .with_code(codes::UNTERMINATED_BLOCK_COMMENT)
                            .with_primary(span, "comment starts here")
                            .with_note("block comments nest: every `/*` needs a matching `*/`")
                            .with_help("close the comment with `*/`"),
                    );
                    break;
                }
                Some('/') if self.peek_nth(1) == Some('*') => {
                    self.pos += 2;
                    depth += 1;
                }
                Some('*') if self.peek_nth(1) == Some('/') => {
                    self.pos += 2;
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Some(c) => {
                    contains_line_break |= c == '\n';
                    self.bump();
                }
            }
        }
        if contains_line_break {
            self.note_line_break(start, self.pos);
        }
    }

    fn identifier(&mut self) {
        let start = self.pos;
        let mut non_ascii = false;
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                self.bump();
            } else if is_non_ascii_word_char(c) {
                non_ascii = true;
                self.bump();
            } else {
                break;
            }
        }
        let text = &self.text[start..self.pos];
        let kind = if non_ascii {
            let span = self.span(start, self.pos);
            self.report(
                Diagnostic::error("identifiers must contain only ASCII letters, digits and underscores")
                    .with_code(codes::NON_ASCII_IDENTIFIER)
                    .with_primary(span, "contains non-ASCII characters")
                    .with_note("Unicode identifiers are a future proposal; see specifications/LEXICAL_STRUCTURE.md"),
            );
            TokenKind::Ident
        } else if text == "_" {
            TokenKind::Underscore
        } else if let Some(keyword) = Keyword::lookup(text) {
            TokenKind::Keyword(keyword)
        } else {
            TokenKind::Ident
        };
        self.push(kind, start, self.pos);
    }

    fn punctuation_or_unknown(&mut self) {
        let start = self.pos;
        // Maximal munch: the longest operator spelling that matches.
        let longest = TokenKind::PUNCTUATION
            .iter()
            .filter(|(_, spelling)| self.rest().starts_with(spelling))
            .max_by_key(|(_, spelling)| spelling.len());
        if let Some(&(kind, spelling)) = longest {
            self.pos += spelling.len();
            self.push(kind, start, self.pos);
            return;
        }

        let c = self.bump().expect("caller checked that input remains");
        let span = self.span(start, self.pos);
        let mut diagnostic = Diagnostic::error(format!(
            "unknown character `{}` (U+{:04X})",
            c.escape_debug(),
            u32::from(c)
        ))
        .with_code(codes::UNKNOWN_CHARACTER)
        .with_primary(span, "not valid here");
        if c.is_whitespace() {
            diagnostic =
                diagnostic.with_help("only spaces, tabs and line breaks are whitespace in Pinguer");
        } else if matches!(c, '\u{201C}' | '\u{201D}') {
            diagnostic = diagnostic
                .with_help("string literals are delimited by the ASCII quotation mark `\"`");
        } else if matches!(c, '\u{2018}' | '\u{2019}') {
            diagnostic = diagnostic
                .with_help("character literals are delimited by the ASCII apostrophe `'`");
        }
        self.report(diagnostic);
        self.push(TokenKind::Unknown, start, self.pos);
    }

    fn string(&mut self) {
        let start = self.pos;
        self.bump();
        loop {
            if self.at_line_end() {
                let end = self.line_content_end();
                let span = self.span(start, end);
                self.report(
                    Diagnostic::error("unterminated string literal")
                        .with_code(codes::UNTERMINATED_STRING)
                        .with_primary(span, "missing closing `\"`")
                        .with_note("string literals cannot contain line breaks; use `\\n` instead"),
                );
                self.push(TokenKind::String, start, end);
                return;
            }
            match self.peek() {
                Some('"') => {
                    self.bump();
                    break;
                }
                Some('\\') => self.escape(),
                _ => {
                    self.bump();
                }
            }
        }
        self.push(TokenKind::String, start, self.pos);
    }

    fn char_literal(&mut self) {
        let start = self.pos;
        self.bump();
        let mut characters = 0usize;
        loop {
            if self.at_line_end() {
                let end = self.line_content_end();
                let span = self.span(start, end);
                self.report(
                    Diagnostic::error("unterminated character literal")
                        .with_code(codes::UNTERMINATED_CHAR)
                        .with_primary(span, "missing closing `'`"),
                );
                self.push(TokenKind::Char, start, end);
                return;
            }
            match self.peek() {
                Some('\'') => {
                    self.bump();
                    break;
                }
                Some('\\') => {
                    self.escape();
                    characters += 1;
                }
                _ => {
                    self.bump();
                    characters += 1;
                }
            }
        }
        if characters != 1 {
            let span = self.span(start, self.pos);
            let (message, label) = if characters == 0 {
                (
                    "empty character literal",
                    "contains no character".to_owned(),
                )
            } else {
                (
                    "character literal must contain exactly one character",
                    format!("contains {characters} characters"),
                )
            };
            let mut diagnostic = Diagnostic::error(message)
                .with_code(codes::INVALID_CHAR_LITERAL)
                .with_primary(span, label);
            if characters > 1 {
                diagnostic = diagnostic
                    .with_note("a character literal holds one Unicode scalar value")
                    .with_help("string literals are delimited by `\"`");
            }
            self.report(diagnostic);
        }
        self.push(TokenKind::Char, start, self.pos);
    }

    /// Reads an escape sequence starting at a `\`. Leaves a line break after
    /// the backslash unconsumed so that the caller reports the unterminated
    /// literal.
    fn escape(&mut self) {
        let start = self.pos;
        self.bump();
        match self.peek() {
            Some('n' | 'r' | 't' | '\\' | '0' | '"' | '\'') => {
                self.bump();
            }
            Some('u') => {
                self.bump();
                self.unicode_escape(start);
            }
            _ if self.at_line_end() => {}
            // A lone carriage return or a forbidden control character is
            // reported by `check_text_flow`; it is not an escape sequence and
            // is left for the caller to consume.
            Some(c) if c == '\r' || is_forbidden_control(c) => {}
            Some(c) => {
                self.bump();
                let span = self.span(start, self.pos);
                self.report(
                    Diagnostic::error(format!("unknown escape sequence `\\{}`", c.escape_debug()))
                        .with_code(codes::INVALID_ESCAPE)
                        .with_primary(span, "unknown escape")
                        .with_note(
                            "the supported escapes are `\\n`, `\\r`, `\\t`, `\\\\`, `\\0`, `\\\"`, `\\'` and `\\u{...}`",
                        ),
                );
            }
            None => unreachable!("end of input is a line end"),
        }
    }

    /// Reads the part of a `\u{...}` escape after the `u`. `start` is the
    /// offset of the backslash.
    fn unicode_escape(&mut self, start: usize) {
        let invalid = |lexer: &mut Self, message: String, help: &str| {
            let span = lexer.span(start, lexer.pos);
            let mut diagnostic = Diagnostic::error(message)
                .with_code(codes::INVALID_ESCAPE)
                .with_primary(span, "invalid Unicode escape");
            if !help.is_empty() {
                diagnostic = diagnostic.with_help(help.to_owned());
            }
            lexer.report(diagnostic);
        };

        if !self.eat('{') {
            invalid(
                self,
                "expected `{` after `\\u`".to_owned(),
                "write Unicode escapes with braces, for example `\\u{1F427}`",
            );
            return;
        }
        let digits_start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
            self.bump();
        }
        let digits_end = self.pos;
        if !self.eat('}') {
            match self.escape_close() {
                // Closed later: the escape contains a character that is not a
                // hexadecimal digit. Point at it and resume after the `}`.
                Some(close) => {
                    let c = self.peek().expect("a closing brace follows");
                    let span = self.span(self.pos, self.pos + c.len_utf8());
                    self.report(
                        Diagnostic::error(format!(
                            "invalid character `{}` in Unicode escape",
                            c.escape_debug()
                        ))
                        .with_code(codes::INVALID_ESCAPE)
                        .with_primary(span, "not a hexadecimal digit")
                        .with_note(UNICODE_ESCAPE_FORM),
                    );
                    self.pos = close + 1;
                }
                None => invalid(
                    self,
                    "unterminated Unicode escape".to_owned(),
                    "close the escape with `}`",
                ),
            }
            return;
        }
        let digits = &self.text[digits_start..digits_end];
        if digits.is_empty() {
            invalid(
                self,
                "empty Unicode escape".to_owned(),
                "write between one and six hexadecimal digits inside the braces",
            );
        } else if digits.len() > 6 {
            invalid(
                self,
                "Unicode escape has more than six hexadecimal digits".to_owned(),
                "",
            );
        } else {
            let value = u32::from_str_radix(digits, 16).expect("at most six hex digits");
            if char::from_u32(value).is_none() {
                let message = if value <= 0xDFFF {
                    format!("`{value:X}` is a surrogate code point, not a Unicode scalar value")
                } else {
                    format!("`{value:X}` is above the largest Unicode scalar value, `10FFFF`")
                };
                let span = self.span(start, self.pos);
                self.report(
                    Diagnostic::error(message)
                        .with_code(codes::INVALID_ESCAPE)
                        .with_primary(span, "invalid Unicode escape")
                        .with_note("Unicode scalar values are 0 to D7FF and E000 to 10FFFF"),
                );
            }
        }
    }

    /// Returns the offset of the `}` that closes the Unicode escape being
    /// read, if one follows before the literal, the line or another escape
    /// ends. Scanning stops at the first `"`, `'` or `\`, so every character
    /// of a literal is scanned a bounded number of times.
    fn escape_close(&self) -> Option<usize> {
        for (offset, c) in self.rest().char_indices() {
            match c {
                '}' => return Some(self.pos + offset),
                '"' | '\'' | '\\' | '\n' | '\r' => return None,
                _ => {}
            }
        }
        None
    }

    fn number(&mut self) {
        let start = self.pos;
        let first = self.bump().expect("caller checked for a digit");
        let mut kind = TokenKind::Int;
        let mut exponent_read = false;

        let base = match (first, self.peek()) {
            ('0', Some('x')) => Some((16, "hexadecimal")),
            ('0', Some('o')) => Some((8, "octal")),
            ('0', Some('b')) => Some((2, "binary")),
            _ => None,
        };

        if let Some((radix, name)) = base {
            self.bump();
            let digits_start = self.pos;
            while self.peek().is_some_and(is_ident_continue) {
                self.bump();
            }
            self.check_prefixed_digits(start, digits_start, radix, name);
        } else {
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                self.bump();
            }
            self.check_separators(start, self.pos);
            let integer_part = &self.text[start..self.pos];
            let digit_count = integer_part.bytes().filter(u8::is_ascii_digit).count();
            if digit_count > 1 && integer_part.starts_with('0') {
                let span = self.span(start, self.pos);
                self.report(
                    Diagnostic::error("decimal literals cannot have leading zeros")
                        .with_code(codes::LEADING_ZERO)
                        .with_primary(span, "leading zero")
                        .with_note(
                            "octal literals are written with the `0o` prefix, for example `0o17`",
                        ),
                );
            }

            if self.peek() == Some('.') && self.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) {
                kind = TokenKind::Float;
                self.bump();
                let fraction_start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                    self.bump();
                }
                self.check_separators(fraction_start, self.pos);
            }

            if matches!(self.peek(), Some('e' | 'E')) {
                let has_exponent = match self.peek_nth(1) {
                    Some(c) if c.is_ascii_digit() => true,
                    Some('+' | '-') => self.peek_nth(2).is_some_and(|c| c.is_ascii_digit()),
                    _ => false,
                };
                if has_exponent {
                    exponent_read = true;
                    kind = TokenKind::Float;
                    self.bump();
                    if matches!(self.peek(), Some('+' | '-')) {
                        self.bump();
                    }
                    let exponent_start = self.pos;
                    while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                        self.bump();
                    }
                    self.check_separators(exponent_start, self.pos);
                }
            }
        }

        let suffix_start = self.pos;
        while self
            .peek()
            .is_some_and(|c| is_ident_continue(c) || is_non_ascii_word_char(c))
        {
            self.bump();
        }
        if self.pos > suffix_start {
            let suffix = &self.text[suffix_start..self.pos];
            let span = self.span(suffix_start, self.pos);
            let diagnostic = if base.is_none() && !exponent_read && suffix.starts_with(['e', 'E']) {
                Diagnostic::error("missing digits in exponent")
                    .with_code(codes::INVALID_NUMBER)
                    .with_primary(
                        span,
                        "expected at least one digit after the exponent marker",
                    )
            } else {
                Diagnostic::error(format!("invalid suffix `{suffix}` on numeric literal"))
                    .with_code(codes::INVALID_NUMBER)
                    .with_primary(span, "invalid suffix")
                    .with_note("numeric literal suffixes are not supported in Pinguer v0.1")
            };
            self.report(diagnostic);
        }
        self.push(kind, start, self.pos);
    }

    /// Validates the digits of a `0x`, `0o` or `0b` literal.
    fn check_prefixed_digits(&mut self, start: usize, digits_start: usize, radix: u32, name: &str) {
        let digits = &self.text[digits_start..self.pos];
        if let Some((offset, c)) = digits
            .char_indices()
            .find(|&(_, c)| c != '_' && !c.is_digit(radix))
        {
            let at = digits_start + offset;
            let span = self.span(at, at + c.len_utf8());
            self.report(
                Diagnostic::error(format!("invalid digit `{c}` in {name} literal"))
                    .with_code(codes::INVALID_NUMBER)
                    .with_primary(span, format!("not a {name} digit")),
            );
            return;
        }
        if !digits.chars().any(|c| c.is_digit(radix)) {
            let span = self.span(start, self.pos);
            let prefix = &self.text[start..digits_start];
            self.report(
                Diagnostic::error(format!("missing digits after `{prefix}`"))
                    .with_code(codes::INVALID_NUMBER)
                    .with_primary(span, format!("expected at least one {name} digit")),
            );
            return;
        }
        self.check_separators_with(digits_start, self.pos, |c| c.is_digit(radix));
    }

    /// Validates `_` separators in a run of decimal digits.
    fn check_separators(&mut self, start: usize, end: usize) {
        self.check_separators_with(start, end, |c| c.is_ascii_digit());
    }

    /// Reports the first `_` in `start..end` that is not between two digits.
    fn check_separators_with(&mut self, start: usize, end: usize, is_digit: impl Fn(char) -> bool) {
        let bytes = &self.text.as_bytes()[start..end];
        let misplaced = bytes.iter().enumerate().position(|(index, &byte)| {
            byte == b'_'
                && !(index > 0
                    && is_digit(char::from(bytes[index - 1]))
                    && bytes
                        .get(index + 1)
                        .is_some_and(|&next| is_digit(char::from(next))))
        });
        if let Some(index) = misplaced {
            let at = start + index;
            let span = self.span(at, at + 1);
            self.report(
                Diagnostic::error("misplaced digit separator `_`")
                    .with_code(codes::INVALID_NUMBER)
                    .with_primary(span, "`_` must be between two digits"),
            );
        }
    }
}
