//! Expressions.

use std::fmt;

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
    /// Binary operators of one precedence level applied from left to
    /// right: `a - b + c` is one node meaning `(a - b) + c`. Keeping a chain
    /// in one node bounds the depth of the tree (ADR-0028, ADR-0030).
    ///
    /// The chain means exactly that left fold. Operands are evaluated in
    /// order, and `&&` and `||` evaluate the operand to their right only
    /// when the result so far requires it. A chain must never be
    /// reassociated or reordered: with checked overflow, `-`, `/`, `%`,
    /// shifts and floating-point arithmetic, `(a op b) op c` and
    /// `a op (b op c)` differ. Parentheses stay `Paren` operands, never
    /// merged into the chain.
    Binary {
        /// The leftmost operand.
        first: Box<Expr>,
        /// The operators and the operands to their right, in order. Never
        /// empty; all operators have the same precedence.
        rest: Box<[BinaryOperand]>,
    },
    /// An explicit conversion, `operand as ty`.
    Cast {
        /// The value converted.
        operand: Box<Expr>,
        /// The target type, boxed because conversions are rare and an inline
        /// type would enlarge every expression.
        ty: Box<Type>,
    },
    /// A call, `callee(args)`.
    Call {
        /// The called expression.
        callee: Box<Expr>,
        /// The arguments, in order.
        args: Box<[Expr]>,
    },
    /// A member access, `base.name`. In v0.1 the base must be a module
    /// name; name resolution checks that.
    Member {
        /// The expression before the `.`.
        base: Box<Expr>,
        /// The name after the `.`.
        name: Ident,
    },
    /// An expression that could not be parsed. A diagnostic has been
    /// reported, by the lexer or the parser.
    Error,
}

/// An operator of a binary chain and the operand to its right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryOperand {
    /// The operator.
    pub op: BinaryOp,
    /// Where the operator was written.
    pub op_span: Span,
    /// The operand to the right of the operator.
    pub operand: Expr,
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
        value: Option<IntValue>,
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

/// The value of an integer literal, up to 128 bits.
///
/// Stored as two halves: a `u128` field would raise the alignment, and with
/// it the size, of every expression to 16 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntValue {
    high: u64,
    low: u64,
}

impl IntValue {
    /// Stores `value`.
    #[must_use]
    pub const fn new(value: u128) -> Self {
        Self {
            high: (value >> 64) as u64,
            low: value as u64,
        }
    }

    /// Returns the value.
    #[must_use]
    pub const fn get(self) -> u128 {
        (self.high as u128) << 64 | self.low as u128
    }
}

impl fmt::Debug for IntValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
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

    #[test]
    fn integer_values_keep_all_128_bits() {
        for value in [
            0,
            1,
            u128::from(u64::MAX),
            1 << 64,
            u128::MAX - 1,
            u128::MAX,
        ] {
            assert_eq!(IntValue::new(value).get(), value);
        }
        assert_eq!(
            format!("{:?}", IntValue::new(1 << 100)),
            (1u128 << 100).to_string()
        );
    }

    /// The sizes that the layout of expressions was chosen for (ADR-0028).
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn expression_sizes() {
        assert_eq!(size_of::<Expr>(), 64);
        assert_eq!(size_of::<Literal>(), 32);
        assert_eq!(size_of::<BinaryOperand>(), 80);
    }
}
