//! Type syntax.

use pinguer_source::Span;

use crate::{Ident, NodeId};

/// A type as written in the source, before any resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type {
    /// The node's ID.
    pub id: NodeId,
    /// The tokens of the type.
    pub span: Span,
    /// What kind of type syntax this is.
    pub kind: TypeKind,
}

/// The kinds of type syntax in v0.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    /// A type name such as `int` or `string`. Which type it denotes is
    /// decided by name resolution.
    Name(Ident),
    /// The unit type `()`.
    Unit,
}
