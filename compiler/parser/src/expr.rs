//! Expressions.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::TokenKind;

use crate::{PResult, Parser};

impl Parser<'_> {
    /// `Expr = OrExpr`
    pub(crate) fn expr(&mut self) -> PResult<ast::Expr> {
        self.primary()
    }

    /// `PrimaryExpr = … | IDENT | …`
    fn primary(&mut self) -> PResult<ast::Expr> {
        let id = self.next_id();
        let start = self.start();
        let kind = match self.cursor.kind() {
            TokenKind::Ident => ast::ExprKind::Name(self.ident()?),
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
