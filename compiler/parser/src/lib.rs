//! Parsing for Pinguer.
//!
//! [`parse`] turns the tokens of one source file into its syntax tree
//! ([`pinguer_ast`]), following `specifications/GRAMMAR.ebnf`. Items,
//! statements and types are parsed by recursive descent; diagnostics go to
//! the caller's [`Diagnostics`], next to the lexer's. The design is recorded
//! in ADR-0027 (architecture), ADR-0028 (tree), ADR-0029 (error recovery)
//! and ADR-0030 (resource limits).

mod cursor;

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
    tokens: Vec<Token>,
    diagnostics: &mut Diagnostics,
) -> ast::File {
    let text_len = u32::try_from(text.len()).expect("source text exceeds the maximum file size");
    let mut parser = Parser {
        cursor: Cursor::new(tokens, file, text_len),
        text,
        diagnostics,
    };
    parser.file(Span::new(file, 0, text_len))
}

struct Parser<'a> {
    cursor: Cursor,
    text: &'a str,
    diagnostics: &'a mut Diagnostics,
}

impl Parser<'_> {
    /// `File = { Separator } , [ Item , … ] , EOF`
    fn file(&mut self, span: Span) -> ast::File {
        self.skip_separators();
        if !self.cursor.at(TokenKind::Eof) {
            let found = self.cursor.current();
            self.report(
                Diagnostic::error(format!(
                    "expected `import`, `func` or `const`, found {}",
                    self.describe(found)
                ))
                .with_code(codes::EXPECTED_ITEM)
                .with_primary(found.span, "expected an item"),
            );
        }
        ast::File {
            span,
            items: Vec::new(),
            node_count: 0,
        }
    }

    /// `Separator = NEWLINE | ";"`
    fn skip_separators(&mut self) {
        while matches!(
            self.cursor.kind(),
            TokenKind::Newline | TokenKind::Semicolon
        ) {
            self.cursor.bump();
        }
    }

    fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
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

#[cfg(test)]
mod tests {
    use super::*;
    use pinguer_source::SourceMap;

    fn parse_text(text: &str) -> (ast::File, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("p.pgr", text).unwrap();
        let mut diagnostics = Diagnostics::new();
        let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
        let tree = parse(file, text, tokens, &mut diagnostics);
        (tree, diagnostics)
    }

    #[test]
    fn files_of_separators_have_no_items() {
        for text in ["", "\n\n", ";;", "// comment\n;\n/* block */\n"] {
            let (tree, diagnostics) = parse_text(text);
            assert!(tree.items.is_empty(), "{text:?}");
            assert!(diagnostics.is_empty(), "{text:?}");
            assert_eq!(tree.span.range(), 0..text.len());
        }
    }

    #[test]
    fn other_tokens_are_not_items() {
        let (_, diagnostics) = parse_text("\nlet x = 1");
        let [diagnostic] = diagnostics.as_slice() else {
            panic!("{diagnostics:?}");
        };
        assert_eq!(diagnostic.code, Some(codes::EXPECTED_ITEM));
        assert_eq!(
            diagnostic.message,
            "expected `import`, `func` or `const`, found `let`"
        );
        assert_eq!(diagnostic.labels[0].span.range(), 1..4);
        let (_, diagnostics) = parse_text("main");
        assert_eq!(
            diagnostics.as_slice()[0].message,
            "expected `import`, `func` or `const`, found identifier `main`"
        );
    }
}
