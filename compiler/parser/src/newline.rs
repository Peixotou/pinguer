//! Removal of the line breaks that the grammar ignores.

use pinguer_lexer::{Keyword, Token, TokenKind};

/// Removes every `NEWLINE` token that GRAMMAR.ebnf says the parser
/// ignores: one directly after a binary operator, an assignment operator,
/// `as`, `->` or `,`, and one directly before `.` (ADR-0005, ADR-0014).
///
/// The rule is about tokens, not their role, so a `NEWLINE` after a prefix
/// `-` is ignored too. The lexer never produces two `NEWLINE` tokens in a
/// row, so the token before a `NEWLINE` is never one that was removed. The
/// remaining `NEWLINE` tokens separate statements and items.
pub(crate) fn remove_ignored_newlines(tokens: &mut Vec<Token>) {
    let mut kept = 0;
    for index in 0..tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Newline {
            let after_continuation = kept > 0 && continues_line(tokens[kept - 1].kind);
            let before_dot = tokens
                .get(index + 1)
                .is_some_and(|next| next.kind == TokenKind::Dot);
            if after_continuation || before_dot {
                continue;
            }
        }
        tokens[kept] = token;
        kept += 1;
    }
    tokens.truncate(kept);
}

/// The tokens after which a line break does not end a statement.
fn continues_line(kind: TokenKind) -> bool {
    use TokenKind as T;
    matches!(
        kind,
        // Binary operators.
        T::PipePipe
            | T::AmpAmp
            | T::EqEq
            | T::BangEq
            | T::Lt
            | T::LtEq
            | T::Gt
            | T::GtEq
            | T::Pipe
            | T::Caret
            | T::Amp
            | T::Shl
            | T::Shr
            | T::Plus
            | T::Minus
            | T::Star
            | T::Slash
            | T::Percent
            // Assignment operators.
            | T::Eq
            | T::PlusEq
            | T::MinusEq
            | T::StarEq
            | T::SlashEq
            | T::PercentEq
            | T::AmpEq
            | T::PipeEq
            | T::CaretEq
            | T::ShlEq
            | T::ShrEq
            // Others.
            | T::Keyword(Keyword::As)
            | T::Arrow
            | T::Comma
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinguer_diagnostics::Diagnostics;
    use pinguer_source::SourceMap;

    fn kinds(text: &str) -> Vec<TokenKind> {
        let mut map = SourceMap::new();
        let file = map.add("n.pgr", text).unwrap();
        let mut tokens = pinguer_lexer::tokenize(file, text, &mut Diagnostics::new());
        remove_ignored_newlines(&mut tokens);
        tokens.iter().map(|token| token.kind).collect()
    }

    #[test]
    fn line_breaks_after_continuation_tokens_are_ignored() {
        for operator in [
            "||", "&&", "==", "!=", "<", "<=", ">", ">=", "|", "^", "&", "<<", ">>", "+", "-", "*",
            "/", "%", "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "as",
            "->",
        ] {
            let kinds = kinds(&format!("{{ a {operator}\n  b }}"));
            assert!(
                !kinds.contains(&TokenKind::Newline),
                "{operator}: {kinds:?}"
            );
        }
    }

    #[test]
    fn line_breaks_before_a_dot_are_ignored() {
        use TokenKind as T;
        assert_eq!(
            kinds("io\n    .println\n\n// comment\n.x"),
            [T::Ident, T::Dot, T::Ident, T::Dot, T::Ident, T::Eof]
        );
        // Even after a closing brace, which ADR-0014 accepts.
        let kinds = kinds("if x {\n}\n.f()");
        let dot = kinds.iter().position(|&kind| kind == T::Dot).unwrap();
        assert_eq!(kinds[dot - 1], T::RBrace, "{kinds:?}");
    }

    #[test]
    fn other_line_breaks_are_kept() {
        use TokenKind as T;
        // A prefix operator other than `-` does not continue a line, and
        // neither does a token that can end an expression.
        for text in ["a = !\nb", "f()\n-x", "a\nb", "a\n..b"] {
            assert!(kinds(text).contains(&T::Newline), "{text:?}");
        }
        assert_eq!(
            kinds("let x = -\n1\nf()"),
            [
                T::Keyword(Keyword::Let),
                T::Ident,
                T::Eq,
                T::Minus,
                T::Int,
                T::Newline,
                T::Ident,
                T::LParen,
                T::RParen,
                T::Eof,
            ]
        );
    }
}
