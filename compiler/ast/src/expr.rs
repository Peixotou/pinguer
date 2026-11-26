//! Expressions.

use pinguer_source::Span;

use crate::{Ident, NodeId, Type};

/// An expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    /// The node's ID.
    pub id: NodeId,
    /// The tokens of the expression.
    pub span: Span,
    /// What kind of expression this is.
    pub kind: ExprKind,
}

/// The kinds of expression in v0.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    /// A literal value.
    Literal(Literal),
    /// A name: a variable, constant, parameter, function or module.
    Name(Ident),
    /// An expression in parentheses. Kept because the negative-literal rule
    /// and diagnostics depend on whether parentheses were written.
    Paren(Box<Expr>),
    /// A prefix operator applied to an operand, such as `-x`.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: Box<Expr>,
    },
    /// A binary operator applied to two operands, such as `a + b`.
    Binary {
        /// The operator.
        op: BinaryOp,
        /// Where the operator was written.
        op_span: Span,
        /// The left operand.
        left: Box<Expr>,
        /// The right operand.
        right: Box<Expr>,
    },
    /// An explicit conversion, `operand as ty`.
    Cast {
        /// The value converted.
        operand: Box<Expr>,
        /// The target type.
        ty: Type,
    },
    /// An expression that could not be parsed. A diagnostic has been
    /// reported, by the lexer or the parser.
    Error,
}

/// A prefix operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// `-`, arithmetic negation.
    Neg,
    /// `!`, logical not.
    Not,
    /// `~`, bitwise not.
    BitNot,
}

impl UnaryOp {
    /// Returns the operator's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
        }
    }
}

/// A binary operator of v0.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `&`
    BitAnd,
    /// `^`
    BitXor,
    /// `|`
    BitOr,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`, which evaluates its right operand only if the left is `true`.
    And,
    /// `||`, which evaluates its right operand only if the left is `false`.
    Or,
}

impl BinaryOp {
    /// Every binary operator, from the highest precedence level to the
    /// lowest.
    pub const ALL: &'static [BinaryOp] = &[
        BinaryOp::Mul,
        BinaryOp::Div,
        BinaryOp::Rem,
        BinaryOp::Add,
        BinaryOp::Sub,
        BinaryOp::Shl,
        BinaryOp::Shr,
        BinaryOp::BitAnd,
        BinaryOp::BitXor,
        BinaryOp::BitOr,
        BinaryOp::Eq,
        BinaryOp::Ne,
        BinaryOp::Lt,
        BinaryOp::Le,
        BinaryOp::Gt,
        BinaryOp::Ge,
        BinaryOp::And,
        BinaryOp::Or,
    ];

    /// Returns the operator's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitXor => "^",
            BinaryOp::BitOr => "|",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        }
    }

    /// Returns `true` for `==`, `!=`, `<`, `<=`, `>` and `>=`.
    #[must_use]
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        )
    }
}

/// A literal, decoded from its token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    /// An integer literal. Its type, and whether the value fits it, are
    /// decided by the type checker.
    Int {
        /// The value, or `None` if it needs more than 128 bits.
        value: Option<u128>,
        /// The base it was written in.
        base: Base,
    },
    /// A floating-point literal, as its digits with separators removed (for
    /// example `6.02e23`). It is rounded once, at the type the type checker
    /// chooses, by a correctly rounded parser.
    Float(Box<str>),
    /// A string literal, with its escape sequences decoded.
    String(Box<str>),
    /// A character literal, with its escape sequence decoded.
    Char(char),
    /// `true` or `false`.
    Bool(bool),
}

/// The base of an integer literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Base {
    /// `0b`
    Binary,
    /// `0o`
    Octal,
    /// No prefix.
    Decimal,
    /// `0x`
    Hexadecimal,
}

impl Base {
    /// Returns the number base, such as 16 for hexadecimal.
    #[must_use]
    pub fn radix(self) -> u32 {
        match self {
            Base::Binary => 2,
            Base::Octal => 8,
            Base::Decimal => 10,
            Base::Hexadecimal => 16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_spellings_are_distinct() {
        let mut spellings: Vec<&str> = BinaryOp::ALL.iter().map(|op| op.as_str()).collect();
        assert_eq!(spellings.len(), 18);
        spellings.sort_unstable();
        spellings.dedup();
        assert_eq!(spellings.len(), 18);
        let comparisons: Vec<&str> = BinaryOp::ALL
            .iter()
            .filter(|op| op.is_comparison())
            .map(|op| op.as_str())
            .collect();
        assert_eq!(comparisons, ["==", "!=", "<", "<=", ">", ">="]);
        let unary: Vec<&str> = [UnaryOp::Neg, UnaryOp::Not, UnaryOp::BitNot]
            .into_iter()
            .map(UnaryOp::as_str)
            .collect();
        assert_eq!(unary, ["-", "!", "~"]);
    }

    #[test]
    fn bases_report_their_radix() {
        let radixes: Vec<u32> = [Base::Binary, Base::Octal, Base::Decimal, Base::Hexadecimal]
            .into_iter()
            .map(Base::radix)
            .collect();
        assert_eq!(radixes, [2, 8, 10, 16]);
    }
}
