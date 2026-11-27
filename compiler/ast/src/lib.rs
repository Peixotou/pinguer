//! The abstract syntax tree of a Pinguer source file.
//!
//! The parser (`pinguer_parser`) produces the tree; later phases read it and
//! record what they learn in side tables keyed by [`NodeId`], without
//! modifying it ([ADR-0004], [ADR-0028]). The tree contains syntax only: no
//! types, no resolved names and no comments.
//!
//! [ADR-0004]: https://github.com/Peixotou/pinguer/blob/main/docs/adr/0004-intermediate-representations.md
//! [ADR-0028]: https://github.com/Peixotou/pinguer/blob/main/docs/adr/0028-ast-representation.md

mod expr;
mod item;
mod render;
mod stmt;
mod ty;

use std::fmt;

use pinguer_source::Span;

pub use expr::{Base, BinaryOp, BinaryOperand, Expr, ExprKind, Literal, UnaryOp};
pub use item::{File, Func, Import, Item, ItemKind, Param};
pub use render::{render, render_to_string};
pub use stmt::{AssignOp, Binding, BindingKind, Block, If, IfBranch, Stmt, StmtKind};
pub use ty::{Type, TypeKind};

/// Identifies a node of one file's syntax tree.
///
/// IDs are dense and assigned in pre-order once the tree is complete: a
/// node gets its ID before its children, and children are numbered from
/// left to right. The same input therefore always produces the same IDs,
/// and a side table can be a vector indexed by [`NodeId::index`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

impl NodeId {
    /// Creates the ID with the given index.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the zero-based index of the node within its tree.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// A name written in the source, such as a variable, function or module
/// name.
///
/// A name has no ID of its own: the node that contains it is the one later
/// phases annotate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    /// The name as written.
    pub name: Box<str>,
    /// Where the name was written.
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ids_index_side_tables_and_display_with_a_hash() {
        let id = NodeId::new(42);
        assert_eq!(id.index(), 42);
        assert_eq!(id.to_string(), "#42");
        assert!(NodeId::new(1) < NodeId::new(2));
    }
}
