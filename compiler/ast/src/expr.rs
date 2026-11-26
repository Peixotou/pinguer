//! Expressions.

use pinguer_source::Span;

use crate::{Ident, NodeId};

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
    /// An expression that could not be parsed. A diagnostic has been
    /// reported, by the lexer or the parser.
    Error,
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
    fn bases_report_their_radix() {
        let radixes: Vec<u32> = [Base::Binary, Base::Octal, Base::Decimal, Base::Hexadecimal]
            .into_iter()
            .map(Base::radix)
            .collect();
        assert_eq!(radixes, [2, 8, 10, 16]);
    }
}
