//! Expressions.

use std::mem;

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::{Keyword, Token, TokenKind};

use crate::ids::PENDING;
use crate::{PResult, Parser, Reported, literal};

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
        let enclosing = self.open_chain();
        let mut left = self.operand()?;
        loop {
            if planned_syntax(self.cursor.kind()).is_some() {
                return Err(self.unsupported());
            }
            match binary_op(self.cursor.kind()) {
                Some((_, level)) if level >= min => self.chain(start, level, &mut left)?,
                _ => break,
            }
        }
        self.close_chain(enclosing);
        Ok(left)
    }

    /// Parses the operators of precedence `level` that follow `left`, which
    /// starts at `start`, and their right operands, and replaces `left`
    /// with the chain. A separate function, so that the temporaries of
    /// building the chain are not part of the frame of
    /// [`Parser::binary`], which a parenthesized expression recurses
    /// through (see [`Parser::operand`]).
    fn chain(&mut self, start: u32, level: u8, left: &mut ast::Expr) -> PResult<()> {
        self.wrap()?;
        self.depth += 1;
        let mut rest = Vec::new();
        while let Some((op, precedence)) = binary_op(self.cursor.kind())
            && precedence == level
        {
            if let Some(previous) = rest.last()
                && op.is_comparison()
            {
                return Err(self.chained_comparison(previous));
            }
            let op_span = self.cursor.bump().span;
            let operand = self.binary(level + 1)?;
            rest.push(ast::BinaryOperand {
                op,
                op_span,
                operand,
            });
        }
        self.depth -= 1;
        self.enclose(start, left, |first| ast::ExprKind::Binary { first, rest });
        Ok(())
    }

    /// Reports the current token, which belongs to syntax that later
    /// versions add (see [`planned_syntax`]).
    pub(crate) fn unsupported(&mut self) -> Reported {
        let found = self.cursor.current();
        let what = planned_syntax(found.kind).unwrap_or("this syntax");
        self.error(
            Diagnostic::error(format!("{what} is not supported in Pinguer v0.1"))
                .with_code(codes::UNSUPPORTED_SYNTAX)
                .with_primary(found.span, "not supported in v0.1")
                .with_note(
                    "it is planned for a later version; see specifications/FEATURE_MATRIX.md",
                ),
        )
    }

    /// Reports a comparison operator that follows another one: comparisons
    /// are non-associative (`CompareExpr = BitOrExpr , [ CompareOp ,
    /// BitOrExpr ]`).
    fn chained_comparison(&mut self, previous: &ast::BinaryOperand) -> Reported {
        let second = self.cursor.current();
        self.error(
            Diagnostic::error("comparison operators cannot be chained")
                .with_code(codes::CHAINED_COMPARISON)
                .with_primary(second.span, "second comparison")
                .with_secondary(previous.op_span, "first comparison")
                .with_note(
                    "comparisons are non-associative: `a < b < c` does not mean \
                     `a < b && b < c`, so write each comparison separately",
                ),
        )
    }

    /// `CastExpr = UnaryExpr , { "as" , Type }`, with
    /// `UnaryExpr = ( "-" | "!" | "~" ) , UnaryExpr | PostfixExpr` and
    /// `PostfixExpr = PrimaryExpr , { CallSuffix | MemberSuffix }`.
    ///
    /// The three productions are parsed by loops in one function, which
    /// keeps the stack used per level of nesting small (ADR-0030): prefix
    /// operators are collected before the postfix expression and applied
    /// to it afterwards, innermost first, and a parenthesized expression is
    /// the only recursion besides call arguments. `as` binds less tightly
    /// than the prefix operators, so `-x as T` converts `-x`.
    fn operand(&mut self) -> PResult<ast::Expr> {
        // The work after the recursion is done by helpers that update
        // `expr` in place, so that their temporaries are not part of this
        // recursive frame: unoptimized builds give every temporary of a
        // function its own stack slot.
        let start = self.start();
        let conversions = self.open_chain();
        let prefixes = self.prefix_operators()?;
        let postfix_start = self.start();
        let suffixes = self.open_chain();
        let mut expr = if self.cursor.at(TokenKind::LParen) {
            // `"(" , Expr , ")"`, the rest of `PrimaryExpr` being in
            // `primary`, which does not recurse.
            self.enter()?;
            let open = self.cursor.bump();
            let inner = self.binary(0)?;
            self.expect_closing(open, TokenKind::RParen)?;
            self.leave();
            self.node(postfix_start, ast::ExprKind::Paren(Box::new(inner)))
        } else {
            self.primary()?
        };
        self.suffixes(postfix_start, &mut expr)?;
        self.close_chain(suffixes);
        self.apply_prefix_operators(prefixes, &mut expr);
        self.conversions(start, &mut expr)?;
        self.close_chain(conversions);
        Ok(expr)
    }

    /// Consumes the prefix operators before an operand and returns them
    /// with their start offsets, entering one level for each.
    fn prefix_operators(&mut self) -> PResult<Vec<(ast::UnaryOp, u32)>> {
        let mut prefixes = Vec::new();
        while let Some(op) = unary_op(self.cursor.kind()) {
            self.enter()?;
            prefixes.push((op, self.start()));
            self.cursor.bump();
        }
        Ok(prefixes)
    }

    /// `{ CallSuffix | MemberSuffix }` after the primary expression `expr`,
    /// which starts at `start`.
    fn suffixes(&mut self, start: u32, expr: &mut ast::Expr) -> PResult<()> {
        loop {
            match self.cursor.kind() {
                TokenKind::LParen => {
                    self.wrap()?;
                    let open = self.cursor.bump();
                    self.depth += 1;
                    let args = self.args(open)?;
                    self.depth -= 1;
                    self.enclose(start, expr, |callee| ast::ExprKind::Call { callee, args });
                }
                // `MemberSuffix = "." , IDENT`. Whether the base is a
                // module, as v0.1 requires, is checked by name resolution.
                TokenKind::Dot => {
                    self.wrap()?;
                    self.cursor.bump();
                    let name = self.ident()?;
                    self.enclose(start, expr, |base| ast::ExprKind::Member { base, name });
                }
                _ => return Ok(()),
            }
        }
    }

    /// Applies the prefix operators that precede `expr`, innermost first,
    /// leaving the level that each entered.
    fn apply_prefix_operators(
        &mut self,
        mut prefixes: Vec<(ast::UnaryOp, u32)>,
        expr: &mut ast::Expr,
    ) {
        while let Some((op, start)) = prefixes.pop() {
            self.leave();
            self.enclose(start, expr, |operand| ast::ExprKind::Unary { op, operand });
        }
    }

    /// `{ "as" , Type }` after `expr`, which starts at `start`.
    fn conversions(&mut self, start: u32, expr: &mut ast::Expr) -> PResult<()> {
        while self.cursor.at(TokenKind::Keyword(Keyword::As)) {
            self.wrap()?;
            self.cursor.bump();
            let ty = self.ty()?;
            self.enclose(start, expr, |operand| ast::ExprKind::Cast { operand, ty });
        }
        Ok(())
    }

    /// Replaces `expr` with a node from `start` to the last consumed token,
    /// whose kind `kind` builds around the boxed `expr`.
    fn enclose(
        &self,
        start: u32,
        expr: &mut ast::Expr,
        kind: impl FnOnce(Box<ast::Expr>) -> ast::ExprKind,
    ) {
        let placeholder = ast::Expr {
            id: PENDING,
            span: expr.span,
            kind: ast::ExprKind::Error,
        };
        let inner = Box::new(mem::replace(expr, placeholder));
        *expr = self.node(start, kind(inner));
    }

    /// Returns an expression node from `start` to the last consumed token.
    fn node(&self, start: u32, kind: ast::ExprKind) -> ast::Expr {
        ast::Expr {
            id: PENDING,
            span: self.span_from(start),
            kind,
        }
    }

    /// The rest of `CallSuffix = "(" , [ ArgList ] , ")"` after `open`, with
    /// `ArgList = Expr , { "," , Expr } , [ "," ]`.
    fn args(&mut self, open: Token) -> PResult<Vec<ast::Expr>> {
        let mut args = Vec::new();
        while !self.cursor.at(TokenKind::RParen) {
            args.push(self.binary(0)?);
            if !self.cursor.at(TokenKind::Comma) {
                break;
            }
            self.cursor.bump();
        }
        self.expect_closing(open, TokenKind::RParen)?;
        Ok(args)
    }

    /// `PrimaryExpr = INT_LITERAL | FLOAT_LITERAL | STRING_LITERAL | CHAR_LITERAL
    /// | "true" | "false" | IDENT | "(" , Expr , ")"`, except the
    /// parenthesized expression, which `operand` parses.
    ///
    /// A literal the lexer reported as malformed, and a token the lexer
    /// could not recognise, become an error expression without a second
    /// diagnostic.
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
            TokenKind::Unknown => {
                self.cursor.bump();
                ast::ExprKind::Error
            }
            kind if planned_syntax(kind).is_some() => return Err(self.unsupported()),
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
    pub(super) const OR: u8 = 2;
    pub(super) const AND: u8 = 3;
    pub(super) const COMPARISON: u8 = 4;
    pub(super) const BIT_OR: u8 = 5;
    pub(super) const BIT_XOR: u8 = 6;
    pub(super) const BIT_AND: u8 = 7;
    pub(super) const SHIFT: u8 = 8;
    pub(super) const ADDITIVE: u8 = 9;
    pub(super) const MULTIPLICATIVE: u8 = 10;
}

/// Returns the prefix operator a token spells.
fn unary_op(kind: TokenKind) -> Option<ast::UnaryOp> {
    Some(match kind {
        TokenKind::Minus => ast::UnaryOp::Neg,
        TokenKind::Bang => ast::UnaryOp::Not,
        TokenKind::Tilde => ast::UnaryOp::BitNot,
        _ => return None,
    })
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
        TokenKind::Shl => (B::Shl, SHIFT),
        TokenKind::Shr => (B::Shr, SHIFT),
        TokenKind::Amp => (B::BitAnd, BIT_AND),
        TokenKind::Caret => (B::BitXor, BIT_XOR),
        TokenKind::Pipe => (B::BitOr, BIT_OR),
        TokenKind::EqEq => (B::Eq, COMPARISON),
        TokenKind::BangEq => (B::Ne, COMPARISON),
        TokenKind::Lt => (B::Lt, COMPARISON),
        TokenKind::LtEq => (B::Le, COMPARISON),
        TokenKind::Gt => (B::Gt, COMPARISON),
        TokenKind::GtEq => (B::Ge, COMPARISON),
        TokenKind::AmpAmp => (B::And, AND),
        TokenKind::PipePipe => (B::Or, OR),
        _ => return None,
    })
}

/// Names the planned construct that a token belongs to, for tokens that
/// the lexer recognises but v0.1 syntax does not use.
pub(crate) fn planned_syntax(kind: TokenKind) -> Option<&'static str> {
    Some(match kind {
        TokenKind::StarStar => "the power operator `**`",
        TokenKind::StarStarEq => "the compound assignment `**=`",
        TokenKind::PipeGt => "the pipeline operator `|>`",
        TokenKind::QuestionQuestion => "the coalescing operator `??`",
        TokenKind::QuestionDot => "optional member access `?.`",
        TokenKind::Question => "`?`",
        TokenKind::LBracket => "indexing and array syntax `[`",
        TokenKind::DotDot | TokenKind::DotDotEq => "range syntax",
        TokenKind::FatArrow => "`=>`",
        TokenKind::At => "an attribute",
        _ => return None,
    })
}
