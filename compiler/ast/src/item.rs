//! Source files and their top-level items.

use pinguer_source::Span;

use crate::{Binding, Block, Ident, NodeId, Type};

/// The syntax tree of one source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    /// The whole text of the file.
    pub span: Span,
    /// The top-level items, in order.
    pub items: Vec<Item>,
    /// How many node IDs were assigned: every ID is below this number, so a
    /// side table indexed by [`NodeId::index`] needs this many entries.
    pub node_count: u32,
}

/// A top-level item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The node's ID.
    pub id: NodeId,
    /// The tokens of the item.
    pub span: Span,
    /// What kind of item this is.
    pub kind: ItemKind,
}

/// The kinds of top-level item in v0.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// `import a.b.c`.
    Import(Import),
    /// A function declaration.
    Func(Func),
    /// A `const` declaration; its [`Binding::kind`] is
    /// [`BindingKind::Const`](crate::BindingKind::Const).
    Const(Binding),
    /// An item that could not be parsed. A diagnostic has been reported.
    Error,
}

/// `import a.b.c`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    /// The components of the module path. Never empty.
    pub path: Vec<Ident>,
}

/// `func name(params) -> ret { body }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Func {
    /// The function's name.
    pub name: Ident,
    /// The parameters, in order.
    pub params: Vec<Param>,
    /// The return type, or `None` when no `-> Type` was written and the
    /// function returns the unit type.
    pub ret: Option<Type>,
    /// The body.
    pub body: Block,
}

/// A function parameter, `name: Type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    /// The node's ID.
    pub id: NodeId,
    /// The tokens of the parameter.
    pub span: Span,
    /// The parameter's name.
    pub name: Ident,
    /// The parameter's type.
    pub ty: Type,
}
