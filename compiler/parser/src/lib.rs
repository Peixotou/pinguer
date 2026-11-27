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
mod nesting;
mod newline;
mod stmt;
mod ty;

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, Diagnostics, codes};
use pinguer_lexer::{Token, TokenKind};
use pinguer_source::{FileId, Span};

use cursor::Cursor;

pub use nesting::MAX_NESTING;

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
    tokens: Vec<Token>,
    diagnostics: &mut Diagnostics,
) -> ast::File {
    parse_with_id_capacity(file, text, tokens, diagnostics, ids::CAPACITY)
}

/// [`parse`], with IDs for at most `id_capacity` nodes, so that tests can
/// reach the limit with small inputs.
fn parse_with_id_capacity(
    file: FileId,
    text: &str,
    mut tokens: Vec<Token>,
    diagnostics: &mut Diagnostics,
    id_capacity: u32,
) -> ast::File {
    newline::remove_ignored_newlines(&mut tokens);
    let text_len = u32::try_from(text.len()).expect("source text exceeds the maximum file size");
    let mut parser = Parser {
        cursor: Cursor::new(tokens, file, text_len),
        text,
        file,
        diagnostics,
        depth: 0,
        deepest: 0,
        last_error: None,
    };
    let mut tree = parser.file(Span::new(file, 0, text_len));
    if ids::number(&mut tree, id_capacity).is_err() {
        // ADR-0030: stop instead of reusing or wrapping IDs.
        parser.error(
            Diagnostic::error("the file has too many syntax tree nodes")
                .with_code(codes::TOO_MANY_NODES)
                .with_note(format!(
                    "the parser numbers nodes with 32-bit IDs and cannot number more than \
                     {id_capacity}"
                )),
        );
        tree.items.clear();
        tree.node_count = 0;
    }
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
    /// Number of counted constructs (ADR-0030) that enclose the position
    /// being parsed.
    depth: u32,
    /// The deepest level reached in the chain being measured; see
    /// [`Parser::open_chain`].
    deepest: u32,
    /// The primary span of the last reported diagnostic, and whether that
    /// diagnostic reported an unclosed delimiter; see [`Parser::error`].
    last_error: Option<(Span, bool)>,
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
    ///
    /// At a `}` or the end of the file nothing can complete the construct
    /// but the missing delimiter, so after reporting it parsing continues
    /// as if it had been there (ADR-0029): the construct is kept, and the
    /// `}` is left for the block that it closes. Elsewhere the caller
    /// recovers as from any other error.
    fn expect_closing(&mut self, open: Token, close: TokenKind) -> PResult<()> {
        if self.cursor.at(close) {
            self.cursor.bump();
            return Ok(());
        }
        let found = self.cursor.current();
        let reported = self.error(
            Diagnostic::error(format!(
                "expected {close} to close {}, found {}",
                open.kind,
                self.describe(found)
            ))
            .with_code(codes::UNCLOSED_DELIMITER)
            .with_primary(found.span, format!("expected {close}"))
            .with_secondary(open.span, "unclosed delimiter"),
        );
        match found.kind {
            TokenKind::RBrace | TokenKind::Eof => Ok(()),
            _ => Err(reported),
        }
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

    /// Reports the current token, a keyword that starts a declaration that
    /// is allowed only where `allowed` says, such as "inside a function".
    /// Recovery skips the declaration (ADR-0029).
    fn misplaced_declaration(&mut self, allowed: &str, note: &str) -> Reported {
        let found = self.cursor.current();
        self.error(
            Diagnostic::error(format!(
                "{} declarations are only allowed {allowed}",
                found.kind
            ))
            .with_code(codes::MISPLACED_DECLARATION)
            .with_primary(found.span, "not allowed here")
            .with_note(note),
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
    ///
    /// A diagnostic that blames the same span as the previous one is not
    /// reported either: it is a consequence of the first, as when the
    /// construct completed at a `}` (see [`Parser::expect_closing`]) still
    /// lacks something there. Unclosed delimiters are the exception, since
    /// each one reports a different opening delimiter.
    fn error(&mut self, diagnostic: Diagnostic) -> Reported {
        let current = self.cursor.current();
        let primary = diagnostic.main_label().map(|label| label.span);
        let blames_current = primary == Some(current.span);
        let diagnostic = match current.kind {
            TokenKind::Unknown if blames_current => return Reported,
            TokenKind::Keyword(keyword)
                if blames_current && keyword.is_reserved_for_future_use() =>
            {
                Diagnostic::error(format!(
                    "`{}` is reserved for a future version of Pinguer",
                    keyword.as_str()
                ))
                .with_code(codes::RESERVED_KEYWORD)
                .with_primary(current.span, "reserved keyword")
                .with_note("it has no meaning in Pinguer v0.1 and cannot be used as a name")
            }
            _ => diagnostic,
        };
        let unclosed = diagnostic.code == Some(codes::UNCLOSED_DELIMITER);
        if let (Some(span), Some((last, last_unclosed))) = (primary, self.last_error)
            && span == last
            && !(unclosed && last_unclosed)
        {
            return Reported;
        }
        self.last_error = primary.map(|span| (span, unclosed));
        self.diagnostics.push(diagnostic);
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

#[cfg(test)]
mod tests {
    use super::*;
    use pinguer_source::SourceMap;

    #[test]
    fn running_out_of_node_ids_is_reported_and_stops() {
        let text = "const X = 1 + 2";
        let mut map = SourceMap::new();
        let file = map.add("ids.pgr", text).unwrap();
        let mut diagnostics = Diagnostics::new();
        let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
        let tree = parse_with_id_capacity(file, text, tokens, &mut diagnostics, 3);
        let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, [Some(codes::TOO_MANY_NODES)]);
        assert!(tree.items.is_empty());
        assert_eq!(tree.node_count, 0);
    }
}
