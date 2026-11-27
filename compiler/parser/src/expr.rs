//! Expressions.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::{Keyword, TokenKind};

use crate::{PResult, Parser, literal};

impl Parser<'_> {
    /// `Expr = OrExpr`
    pub(crate) fn expr(&mut self) -> PResult<ast::Expr> {
        self.primary()
    }

    /// `PrimaryExpr = INT_LITERAL | FLOAT_LITERAL | STRING_LITERAL | CHAR_LITERAL
    /// | "true" | "false" | IDENT | "(" , Expr , ")"`
    ///
    /// A literal the lexer reported as malformed becomes an error
    /// expression without a second diagnostic.
    fn primary(&mut self) -> PResult<ast::Expr> {
        let id = self.next_id();
        let start = self.start();
        let kind = match self.cursor.kind() {
            TokenKind::Int => {
                let token = self.cursor.bump();
                match literal::decode_int(&self.text[token.span.range()]) {
                    Some((value, base)) => {
                        ast::ExprKind::Literal(ast::Literal::Int { value, base })
                    }
                    None => ast::ExprKind::Error,
                }
            }
            TokenKind::Float => {
                let token = self.cursor.bump();
                match literal::decode_float(&self.text[token.span.range()]) {
                    Some(digits) => ast::ExprKind::Literal(ast::Literal::Float(digits)),
                    None => ast::ExprKind::Error,
                }
            }
            TokenKind::String => {
                let token = self.cursor.bump();
                match literal::decode_string(&self.text[token.span.range()]) {
                    Some(value) => ast::ExprKind::Literal(ast::Literal::String(value)),
                    None => ast::ExprKind::Error,
                }
            }
            TokenKind::Char => {
                let token = self.cursor.bump();
                match literal::decode_char(&self.text[token.span.range()]) {
                    Some(value) => ast::ExprKind::Literal(ast::Literal::Char(value)),
                    None => ast::ExprKind::Error,
                }
            }
            TokenKind::Keyword(keyword @ (Keyword::True | Keyword::False)) => {
                self.cursor.bump();
                ast::ExprKind::Literal(ast::Literal::Bool(keyword == Keyword::True))
            }
            TokenKind::Ident => ast::ExprKind::Name(self.ident()?),
            TokenKind::LParen => {
                let open = self.cursor.bump();
                let inner = self.expr()?;
                self.expect_closing(open, TokenKind::RParen)?;
                ast::ExprKind::Paren(Box::new(inner))
            }
            _ => {
                let found = self.cursor.current();
                return Err(self.error(
                    Diagnostic::error(format!(
                        "expected expression, found {}",
                        self.describe(found)
                    ))
                    .with_code(codes::EXPECTED_EXPRESSION)
                    .with_primary(found.span, "expected an expression"),
                ));
            }
        };
        Ok(ast::Expr {
            id,
            span: self.span_from(start),
            kind,
        })
    }
}
