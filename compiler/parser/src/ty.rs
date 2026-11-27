//! Type syntax.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::TokenKind;

use crate::expr::planned_syntax;
use crate::ids::PENDING;
use crate::{PResult, Parser};

impl Parser<'_> {
    /// `Type = IDENT | UnitType`, with `UnitType = "(" , ")"`
    ///
    /// Names are not resolved: `int` and `Point` are both type names here.
    /// The type syntax of later versions is reported as such: optional
    /// types `T?` and the tokens of planned syntax, such as `[`.
    pub(crate) fn ty(&mut self) -> PResult<ast::Type> {
        let start = self.start();
        let kind = match self.cursor.kind() {
            kind if planned_syntax(kind).is_some() => return Err(self.unsupported()),
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
        match self.cursor.kind() {
            // `T??` is lexed as `T` and `??`.
            TokenKind::Question | TokenKind::QuestionQuestion => {
                return Err(self.unsupported_syntax("the optional type syntax `T?`"));
            }
            TokenKind::LBracket => return Err(self.unsupported()),
            _ => {}
        }
        Ok(ast::Type {
            id: PENDING,
            span: self.span_from(start),
            kind,
        })
    }
}
