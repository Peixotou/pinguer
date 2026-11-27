//! Parsing for Pinguer.
//!
//! [`parse`] turns the tokens of one source file into its syntax tree
//! ([`pinguer_ast`]), following `specifications/GRAMMAR.ebnf`. Items,
//! statements and types are parsed by recursive descent; diagnostics go to
//! the caller's [`Diagnostics`], next to the lexer's. The design is recorded
//! in ADR-0027 (architecture), ADR-0028 (tree), ADR-0029 (error recovery)
//! and ADR-0030 (resource limits).

mod cursor;
mod expr;
mod ids;
mod item;
mod literal;
mod newline;
mod stmt;
mod ty;

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, Diagnostics, codes};
use pinguer_lexer::{Token, TokenKind};
use pinguer_source::{FileId, Span};

use cursor::Cursor;

/// Parses `tokens`, the result of [`pinguer_lexer::tokenize`] for `text`,
/// which is the contents of `file`.
///
/// Problems are reported to `diagnostics`; the returned tree is complete
/// even then, with error nodes where input could not be parsed.
///
/// # Panics
///
/// Panics if `text` is longer than [`pinguer_source::MAX_FILE_SIZE`].
#[must_use]
pub fn parse(
    file: FileId,
    text: &str,
    mut tokens: Vec<Token>,
    diagnostics: &mut Diagnostics,
) -> ast::File {
    newline::remove_ignored_newlines(&mut tokens);
    let text_len = u32::try_from(text.len()).expect("source text exceeds the maximum file size");
    let mut parser = Parser {
        cursor: Cursor::new(tokens, file, text_len),
        text,
        file,
        diagnostics,
    };
    let mut tree = parser.file(Span::new(file, 0, text_len));
    ids::number(&mut tree);
    tree
}

/// Records that a diagnostic has been reported for the construct being
/// parsed. Parse functions return it so that their callers stop and
/// recover instead of reporting the same problem again.
struct Reported;

type PResult<T> = Result<T, Reported>;

struct Parser<'a> {
    cursor: Cursor,
    text: &'a str,
    file: FileId,
    diagnostics: &'a mut Diagnostics,
}

impl Parser<'_> {
    /// Returns the span from `start` to the end of the last consumed token,
    /// or an empty span at `start` if nothing was consumed since.
    fn span_from(&self, start: u32) -> Span {
        Span::new(self.file, start, self.cursor.prev_end().max(start))
    }

    /// Returns the start offset of the current token.
    fn start(&self) -> u32 {
        self.cursor.current().span.start
    }

    /// `IDENT`
    fn ident(&mut self) -> PResult<ast::Ident> {
        if self.cursor.at(TokenKind::Ident) {
            let token = self.cursor.bump();
            Ok(ast::Ident {
                name: self.text[token.span.range()].into(),
                span: token.span,
            })
        } else {
            let found = self.cursor.current();
            Err(self.error(
                Diagnostic::error(format!(
                    "expected identifier, found {}",
                    self.describe(found)
                ))
                .with_code(codes::EXPECTED_IDENTIFIER)
                .with_primary(found.span, "expected an identifier"),
            ))
        }
    }

    /// Consumes a token of the given kind, or reports that it is missing.
    fn expect(&mut self, kind: TokenKind) -> PResult<Token> {
        if self.cursor.at(kind) {
            return Ok(self.cursor.bump());
        }
        let found = self.cursor.current();
        Err(self.error(
            Diagnostic::error(format!("expected {kind}, found {}", self.describe(found)))
                .with_code(codes::EXPECTED_TOKEN)
                .with_primary(found.span, format!("expected {kind}")),
        ))
    }

    /// Consumes the delimiter that closes `open`, or reports that `open` is
    /// not closed, pointing at both.
    fn expect_closing(&mut self, open: Token, close: TokenKind) -> PResult<Token> {
        if self.cursor.at(close) {
            return Ok(self.cursor.bump());
        }
        let found = self.cursor.current();
        Err(self.error(
            Diagnostic::error(format!(
                "expected {close} to close {}, found {}",
                open.kind,
                self.describe(found)
            ))
            .with_code(codes::UNCLOSED_DELIMITER)
            .with_primary(found.span, format!("expected {close}"))
            .with_secondary(open.span, "unclosed delimiter"),
        ))
    }

    /// Returns `true` at a `Separator = NEWLINE | ";"`.
    fn at_separator(&self) -> bool {
        matches!(
            self.cursor.kind(),
            TokenKind::Newline | TokenKind::Semicolon
        )
    }

    fn skip_separators(&mut self) {
        while self.at_separator() {
            self.cursor.bump();
        }
    }

    /// Reports that the construct just parsed is not followed by a
    /// separator, the end of its block or the end of the file.
    fn missing_separator(&mut self, what: &str) -> Reported {
        let found = self.cursor.current();
        if is_closing_delimiter(found.kind) {
            return self.unexpected_closing_delimiter();
        }
        let mut diagnostic = Diagnostic::error(format!(
            "expected a line break or `;` after the {what}, found {}",
            self.describe(found)
        ))
        .with_code(codes::MISSING_SEPARATOR)
        .with_primary(found.span, "expected a line break or `;` before this")
        .with_note("each item and statement ends at a line break or `;`");
        if stmt::is_assign_op(found.kind) {
            diagnostic = diagnostic.with_note(
                "assignment is a statement, not an expression, so assignments cannot be chained",
            );
        }
        self.error(diagnostic)
    }

    /// Reports the current token, a closing delimiter that no opening one
    /// matches. Recovery skips it (ADR-0029).
    fn unexpected_closing_delimiter(&mut self) -> Reported {
        let found = self.cursor.current();
        self.error(
            Diagnostic::error(format!("unexpected closing delimiter {}", found.kind))
                .with_code(codes::UNEXPECTED_CLOSING_DELIMITER)
                .with_primary(found.span, "no opening delimiter matches this one"),
        )
    }

    /// Reports `diagnostic`. Either way the caller recovers as from a
    /// reported error.
    ///
    /// When the diagnostic blames the current token, the token itself may
    /// be the cause:
    ///
    /// * An [`TokenKind::Unknown`] token was reported by the lexer, and a
    ///   second diagnostic would only repeat it (ADR-0029); nothing is
    ///   reported.
    /// * A keyword reserved for a later version cannot be a name, start an
    ///   expression or appear anywhere else in v0.1; the dedicated
    ///   diagnostic that LEXICAL_STRUCTURE.md §5.2 requires replaces the one
    ///   about what was expected.
    fn error(&mut self, diagnostic: Diagnostic) -> Reported {
        let current = self.cursor.current();
        let blames_current = diagnostic
            .main_label()
            .is_some_and(|label| label.span == current.span);
        match current.kind {
            TokenKind::Unknown if blames_current => {}
            TokenKind::Keyword(keyword)
                if blames_current && keyword.is_reserved_for_future_use() =>
            {
                self.diagnostics.push(
                    Diagnostic::error(format!(
                        "`{}` is reserved for a future version of Pinguer",
                        keyword.as_str()
                    ))
                    .with_code(codes::RESERVED_KEYWORD)
                    .with_primary(current.span, "reserved keyword")
                    .with_note("it has no meaning in Pinguer v0.1 and cannot be used as a name"),
                );
            }
            _ => self.diagnostics.push(diagnostic),
        }
        Reported
    }

    /// Describes a token the way diagnostics refer to it: `` `func` ``,
    /// ``identifier `x` ``, `newline` or `end of file`. The text of a name is
    /// quoted only when it is short, so that messages stay bounded.
    fn describe(&self, token: Token) -> String {
        const MAX_QUOTED: usize = 32;
        let text = &self.text[token.span.range()];
        match token.kind {
            TokenKind::Ident if text.len() <= MAX_QUOTED => format!("identifier `{text}`"),
            kind => kind.to_string(),
        }
    }
}

/// Returns `true` for `)`, `]` and `}`.
fn is_closing_delimiter(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace
    )
}
