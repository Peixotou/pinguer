//! Statements and blocks.

use pinguer_source::Span;

use crate::{Expr, Ident, NodeId, Type};

/// A block, `{ statements }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The node's ID.
    pub id: NodeId,
    /// The tokens from `{` to `}`.
    pub span: Span,
    /// The statements, in order.
    pub stmts: Vec<Stmt>,
}

/// A statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stmt {
    /// The node's ID.
    pub id: NodeId,
    /// The tokens of the statement, without its separator.
    pub span: Span,
    /// What kind of statement this is.
    pub kind: StmtKind,
}

/// The kinds of statement in v0.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StmtKind {
    /// A `let`, `var` or `const` declaration.
    Binding(Binding),
    /// An assignment such as `x = 1` or `x += 1`. Only variables can be
    /// assigned in v0.1.
    Assign {
        /// The assigned variable.
        target: Ident,
        /// The assignment operator.
        op: AssignOp,
        /// Where the operator was written.
        op_span: Span,
        /// The assigned value.
        value: Expr,
    },
    /// An expression used as a statement. The grammar allows any postfix
    /// expression; the rule that it must be a call is checked by semantic
    /// analysis.
    Expr(Expr),
    /// `return`, with an optional value.
    Return(Option<Expr>),
    /// `break`.
    Break,
    /// `continue`.
    Continue,
    /// A nested block.
    Block(Block),
    /// A statement that could not be parsed. A diagnostic has been
    /// reported.
    Error,
}

/// A `let`, `var` or `const` declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    /// Which keyword introduced the declaration.
    pub kind: BindingKind,
    /// The declared name.
    pub name: Ident,
    /// The type annotation, if one was written.
    pub ty: Option<Type>,
    /// The initializer.
    pub value: Expr,
}

/// The keyword of a [`Binding`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BindingKind {
    /// `let`, an immutable binding.
    Let,
    /// `var`, a mutable binding.
    Var,
    /// `const`, a compile-time constant.
    Const,
}

impl BindingKind {
    /// Returns the keyword.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BindingKind::Let => "let",
            BindingKind::Var => "var",
            BindingKind::Const => "const",
        }
    }
}

/// An assignment operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssignOp {
    /// `=`
    Assign,
    /// `+=`
    Add,
    /// `-=`
    Sub,
    /// `*=`
    Mul,
    /// `/=`
    Div,
    /// `%=`
    Rem,
    /// `&=`
    BitAnd,
    /// `|=`
    BitOr,
    /// `^=`
    BitXor,
    /// `<<=`
    Shl,
    /// `>>=`
    Shr,
}

impl AssignOp {
    /// Every assignment operator, in the order of GRAMMAR.ebnf.
    pub const ALL: &'static [AssignOp] = &[
        AssignOp::Assign,
        AssignOp::Add,
        AssignOp::Sub,
        AssignOp::Mul,
        AssignOp::Div,
        AssignOp::Rem,
        AssignOp::BitAnd,
        AssignOp::BitOr,
        AssignOp::BitXor,
        AssignOp::Shl,
        AssignOp::Shr,
    ];

    /// Returns the operator's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AssignOp::Assign => "=",
            AssignOp::Add => "+=",
            AssignOp::Sub => "-=",
            AssignOp::Mul => "*=",
            AssignOp::Div => "/=",
            AssignOp::Rem => "%=",
            AssignOp::BitAnd => "&=",
            AssignOp::BitOr => "|=",
            AssignOp::BitXor => "^=",
            AssignOp::Shl => "<<=",
            AssignOp::Shr => ">>=",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_operator_spellings_follow_the_grammar() {
        let spellings: Vec<&str> = AssignOp::ALL.iter().map(|op| op.as_str()).collect();
        assert_eq!(
            spellings,
            [
                "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="
            ]
        );
        let keywords: Vec<&str> = [BindingKind::Let, BindingKind::Var, BindingKind::Const]
            .into_iter()
            .map(BindingKind::as_str)
            .collect();
        assert_eq!(keywords, ["let", "var", "const"]);
    }
}
