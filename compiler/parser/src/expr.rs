//! Expressions.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::{Keyword, TokenKind};

use crate::ids::PENDING;
use crate::{PResult, Parser, literal};

impl Parser<'_> {
    /// `Expr = OrExpr`
    pub(crate) fn expr(&mut self) -> PResult<ast::Expr> {
        self.binary(0)
    }

    /// Parses a binary expression whose operators bind at least as
    /// tightly as `min`, by precedence climbing (ADR-0027).
    ///
    /// Operators of one level are collected into one chain node by a loop
    /// (ADR-0028), so neither the parser nor the tree grows deeper with the
    /// length of a chain. The operand to the right of an operator recurses
    /// only into operators that bind more tightly; an operator that binds
    /// less tightly ends the chain, which becomes the first operand of the
    /// next one.
    fn binary(&mut self, min: u8) -> PResult<ast::Expr> {
        let start = self.start();
        let mut left = self.unary()?;
        while let Some((_, level)) = binary_op(self.cursor.kind()) {
            if level < min {
                break;
            }
            let mut rest = Vec::new();
            while let Some((op, precedence)) = binary_op(self.cursor.kind())
                && precedence == level
            {
                let op_span = self.cursor.bump().span;
                let operand = self.binary(level + 1)?;
                rest.push(ast::BinaryOperand {
                    op,
                    op_span,
                    operand,
                });
            }
            left = ast::Expr {
                id: PENDING,
                span: self.span_from(start),
                kind: ast::ExprKind::Binary {
                    first: Box::new(left),
                    rest,
                },
            };
        }
        Ok(left)
    }

    /// `UnaryExpr = ( "-" | "!" | "~" ) , UnaryExpr | PostfixExpr`
    fn unary(&mut self) -> PResult<ast::Expr> {
        let op = match self.cursor.kind() {
            TokenKind::Minus => ast::UnaryOp::Neg,
            TokenKind::Bang => ast::UnaryOp::Not,
            TokenKind::Tilde => ast::UnaryOp::BitNot,
            _ => return self.primary(),
        };
        let start = self.start();
        self.cursor.bump();
        let operand = self.unary()?;
        Ok(ast::Expr {
            id: PENDING,
            span: self.span_from(start),
            kind: ast::ExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
        })
    }

    /// `PrimaryExpr = INT_LITERAL | FLOAT_LITERAL | STRING_LITERAL | CHAR_LITERAL
    /// | "true" | "false" | IDENT | "(" , Expr , ")"`
    ///
    /// A literal the lexer reported as malformed becomes an error
    /// expression without a second diagnostic.
    fn primary(&mut self) -> PResult<ast::Expr> {
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
            id: PENDING,
            span: self.span_from(start),
            kind,
        })
    }
}

/// Precedence levels of the binary operators: a larger number binds more
/// tightly. The order is that of OPERATORS.md §1, which numbers the same
/// levels from the tightest; the planned operators have no level yet.
mod precedence {
    pub(super) const ADDITIVE: u8 = 9;
    pub(super) const MULTIPLICATIVE: u8 = 10;
}

/// Returns the binary operator a token spells in v0.1 and its precedence.
fn binary_op(kind: TokenKind) -> Option<(ast::BinaryOp, u8)> {
    use ast::BinaryOp as B;
    use precedence::*;
    Some(match kind {
        TokenKind::Star => (B::Mul, MULTIPLICATIVE),
        TokenKind::Slash => (B::Div, MULTIPLICATIVE),
        TokenKind::Percent => (B::Rem, MULTIPLICATIVE),
        TokenKind::Plus => (B::Add, ADDITIVE),
        TokenKind::Minus => (B::Sub, ADDITIVE),
        _ => return None,
    })
}
