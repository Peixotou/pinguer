//! Type syntax.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::TokenKind;

use crate::{PResult, Parser};

impl Parser<'_> {
    /// `Type = IDENT | UnitType`, with `UnitType = "(" , ")"`
    ///
    /// Names are not resolved: `int` and `Point` are both type names here.
    pub(crate) fn ty(&mut self) -> PResult<ast::Type> {
        let id = self.next_id();
        let start = self.start();
        let kind = match self.cursor.kind() {
            TokenKind::Ident => ast::TypeKind::Name(self.ident()?),
            TokenKind::LParen => {
                let open = self.cursor.bump();
                self.expect_closing(open, TokenKind::RParen)?;
                ast::TypeKind::Unit
            }
            _ => {
                let found = self.cursor.current();
                return Err(self.error(
                    Diagnostic::error(format!("expected type, found {}", self.describe(found)))
                        .with_code(codes::EXPECTED_TYPE)
                        .with_primary(found.span, "expected a type"),
                ));
            }
        };
        Ok(ast::Type {
            id,
            span: self.span_from(start),
            kind,
        })
    }
}
