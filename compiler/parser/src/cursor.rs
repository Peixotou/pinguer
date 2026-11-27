//! Reading position in a token vector.

use pinguer_lexer::{Token, TokenKind};
use pinguer_source::{FileId, Span};

/// A position in a token vector that always ends with [`TokenKind::Eof`].
///
/// The end-of-file token is never consumed: [`Cursor::bump`] at the end
/// returns it again without moving. Every parsing loop therefore stops at
/// the end of the file, and the current position is always a valid index.
pub(crate) struct Cursor {
    tokens: Vec<Token>,
    /// Index of the current token. Invariant: `pos < tokens.len()`.
    pos: usize,
    /// End offset of the last consumed token, or 0 before the first.
    prev_end: u32,
}

impl Cursor {
    /// Creates a cursor at the first token. If `tokens` does not end with an
    /// end-of-file token, one is added at `text_len`, so the invariant holds
    /// for any input.
    pub(crate) fn new(mut tokens: Vec<Token>, file: FileId, text_len: u32) -> Self {
        if tokens
            .last()
            .is_none_or(|token| token.kind != TokenKind::Eof)
        {
            tokens.push(Token {
                kind: TokenKind::Eof,
                span: Span::new(file, text_len, text_len),
            });
        }
        Self {
            tokens,
            pos: 0,
            prev_end: 0,
        }
    }

    /// Returns the current token.
    pub(crate) fn current(&self) -> Token {
        self.tokens[self.pos]
    }

    /// Returns the kind of the current token.
    pub(crate) fn kind(&self) -> TokenKind {
        self.current().kind
    }

    /// Returns `true` if the current token has the given kind.
    pub(crate) fn at(&self, kind: TokenKind) -> bool {
        self.kind() == kind
    }

    /// Consumes and returns the current token. At the end of the file the
    /// end-of-file token is returned and the position does not change.
    pub(crate) fn bump(&mut self) -> Token {
        let token = self.current();
        if token.kind != TokenKind::Eof {
            self.pos += 1;
            self.prev_end = token.span.end;
        }
        token
    }

    /// Returns the end offset of the last consumed token.
    pub(crate) fn prev_end(&self) -> u32 {
        self.prev_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinguer_diagnostics::Diagnostics;
    use pinguer_source::SourceMap;

    fn cursor(text: &str) -> Cursor {
        let mut map = SourceMap::new();
        let file = map.add("c.pgr", text).unwrap();
        let tokens = pinguer_lexer::tokenize(file, text, &mut Diagnostics::new());
        Cursor::new(tokens, file, text.len() as u32)
    }

    #[test]
    fn moves_through_tokens_and_stops_at_the_end() {
        let mut c = cursor("a (b");
        assert_eq!(c.kind(), TokenKind::Ident);
        assert_eq!(c.bump().kind, TokenKind::Ident);
        assert!(c.at(TokenKind::LParen));
        c.bump();
        assert_eq!(c.prev_end(), 3);
        assert_eq!(c.bump().span.range(), 3..4);
        for _ in 0..3 {
            assert_eq!(c.bump().kind, TokenKind::Eof);
        }
        assert!(c.at(TokenKind::Eof));
        assert_eq!(c.prev_end(), 4);
    }

    #[test]
    fn adds_a_missing_end_of_file_token() {
        let mut map = SourceMap::new();
        let file = map.add("c.pgr", "ab").unwrap();
        let c = Cursor::new(Vec::new(), file, 2);
        assert_eq!(c.current().kind, TokenKind::Eof);
        assert_eq!(c.current().span, Span::new(file, 2, 2));
    }
}
