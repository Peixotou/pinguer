//! Deterministic text rendering of syntax trees.
//!
//! Every node is one line: its kind, its ID, its span as
//! `@line:column-line:column` (1-based, the end just past the node) and its
//! data, followed by its children indented by two spaces. String and
//! character values, and names that are not ASCII, are written with Rust's
//! escaping, so the output never contains control or invisible characters.
//!
//! The format is an internal interface for `pinguer debug ast` and tests;
//! it may change between versions.

use std::fmt::{self, Write};

use pinguer_source::{SourceFile, Span};

use crate::{
    Base, Binding, BindingKind, Block, Expr, ExprKind, File, Item, ItemKind, Literal, NodeId,
    Param, Stmt, StmtKind, Type, TypeKind,
};

/// Writes the tree of `file`, whose spans refer to `source`, to `out`.
///
/// # Errors
///
/// Returns an error if writing to `out` fails.
///
/// # Panics
///
/// Panics if a span lies outside `source`.
pub fn render(out: &mut impl Write, file: &File, source: &SourceFile) -> fmt::Result {
    let mut renderer = Renderer { out, source };
    renderer.line(
        0,
        "File",
        None,
        file.span,
        &format_args!("nodes={}", file.node_count),
    )?;
    for item in &file.items {
        renderer.item(1, item)?;
    }
    Ok(())
}

struct Renderer<'a, W> {
    out: &'a mut W,
    source: &'a SourceFile,
}

/// Writes a name, escaped unless it is plain ASCII.
struct Name<'a>(&'a str);

impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.bytes().all(|b| b.is_ascii_graphic()) {
            f.write_str(self.0)
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}

impl<W: Write> Renderer<'_, W> {
    fn line(
        &mut self,
        depth: usize,
        kind: &str,
        id: Option<NodeId>,
        span: Span,
        data: &dyn fmt::Display,
    ) -> fmt::Result {
        let start = self.source.line_col(span.start);
        let end = self.source.line_col(span.end);
        write!(self.out, "{:indent$}{kind}", "", indent = 2 * depth)?;
        if let Some(id) = id {
            write!(self.out, " {id}")?;
        }
        write!(self.out, " @{start}-{end}")?;
        let data = data.to_string();
        if !data.is_empty() {
            write!(self.out, " {data}")?;
        }
        writeln!(self.out)
    }

    fn item(&mut self, depth: usize, item: &Item) -> fmt::Result {
        let (id, span) = (Some(item.id), item.span);
        match &item.kind {
            ItemKind::Import(import) => {
                let path: Vec<String> = import
                    .path
                    .iter()
                    .map(|part| Name(&part.name).to_string())
                    .collect();
                self.line(depth, "Import", id, span, &path.join("."))
            }
            ItemKind::Func(func) => {
                self.line(depth, "Func", id, span, &Name(&func.name.name))?;
                for param in &func.params {
                    self.param(depth + 1, param)?;
                }
                if let Some(ret) = &func.ret {
                    self.ty(depth + 1, ret)?;
                }
                self.block(depth + 1, &func.body)
            }
            ItemKind::Const(binding) => self.binding(depth, id, span, binding),
            ItemKind::Error => self.line(depth, "ErrorItem", id, span, &""),
        }
    }

    fn param(&mut self, depth: usize, param: &Param) -> fmt::Result {
        self.line(
            depth,
            "Param",
            Some(param.id),
            param.span,
            &Name(&param.name.name),
        )?;
        self.ty(depth + 1, &param.ty)
    }

    fn ty(&mut self, depth: usize, ty: &Type) -> fmt::Result {
        let (id, span) = (Some(ty.id), ty.span);
        match &ty.kind {
            TypeKind::Name(name) => self.line(depth, "Type", id, span, &Name(&name.name)),
            TypeKind::Unit => self.line(depth, "Type", id, span, &"()"),
            TypeKind::Error => self.line(depth, "ErrorType", id, span, &""),
        }
    }

    fn block(&mut self, depth: usize, block: &Block) -> fmt::Result {
        self.line(depth, "Block", Some(block.id), block.span, &"")?;
        for stmt in &block.stmts {
            self.stmt(depth + 1, stmt)?;
        }
        Ok(())
    }

    fn binding(
        &mut self,
        depth: usize,
        id: Option<NodeId>,
        span: Span,
        binding: &Binding,
    ) -> fmt::Result {
        let kind = match binding.kind {
            BindingKind::Let => "Let",
            BindingKind::Var => "Var",
            BindingKind::Const => "Const",
        };
        self.line(depth, kind, id, span, &Name(&binding.name.name))?;
        if let Some(ty) = &binding.ty {
            self.ty(depth + 1, ty)?;
        }
        self.expr(depth + 1, &binding.value)
    }

    fn stmt(&mut self, depth: usize, stmt: &Stmt) -> fmt::Result {
        let (id, span) = (Some(stmt.id), stmt.span);
        match &stmt.kind {
            StmtKind::Binding(binding) => self.binding(depth, id, span, binding),
            StmtKind::Assign {
                target, op, value, ..
            } => {
                let data = format!("{} {}", Name(&target.name), op.as_str());
                self.line(depth, "Assign", id, span, &data)?;
                self.expr(depth + 1, value)
            }
            StmtKind::If(if_stmt) => {
                self.line(depth, "If", id, span, &"")?;
                for branch in &if_stmt.branches {
                    self.line(depth + 1, "Branch", None, branch.span, &"")?;
                    self.expr(depth + 2, &branch.cond)?;
                    self.block(depth + 2, &branch.block)?;
                }
                if let Some(block) = &if_stmt.else_block {
                    self.line(depth + 1, "Else", None, block.span, &"")?;
                    self.block(depth + 2, block)?;
                }
                Ok(())
            }
            StmtKind::While { cond, body } => {
                self.line(depth, "While", id, span, &"")?;
                self.expr(depth + 1, cond)?;
                self.block(depth + 1, body)
            }
            StmtKind::Expr(expr) => {
                self.line(depth, "ExprStmt", id, span, &"")?;
                self.expr(depth + 1, expr)
            }
            StmtKind::Return(value) => {
                self.line(depth, "Return", id, span, &"")?;
                match value {
                    Some(value) => self.expr(depth + 1, value),
                    None => Ok(()),
                }
            }
            StmtKind::Break => self.line(depth, "Break", id, span, &""),
            StmtKind::Continue => self.line(depth, "Continue", id, span, &""),
            StmtKind::Block(block) => {
                self.line(depth, "BlockStmt", id, span, &"")?;
                self.block(depth + 1, block)
            }
            StmtKind::Error => self.line(depth, "ErrorStmt", id, span, &""),
        }
    }

    fn expr(&mut self, depth: usize, expr: &Expr) -> fmt::Result {
        let (id, span) = (Some(expr.id), expr.span);
        match &expr.kind {
            ExprKind::Literal(literal) => self.literal(depth, id, span, literal),
            ExprKind::Name(name) => self.line(depth, "Name", id, span, &Name(&name.name)),
            ExprKind::Paren(inner) => {
                self.line(depth, "Paren", id, span, &"")?;
                self.expr(depth + 1, inner)
            }
            ExprKind::Unary { op, operand } => {
                self.line(depth, "Unary", id, span, &op.as_str())?;
                self.expr(depth + 1, operand)
            }
            ExprKind::Binary {
                op, left, right, ..
            } => {
                self.line(depth, "Binary", id, span, &op.as_str())?;
                self.expr(depth + 1, left)?;
                self.expr(depth + 1, right)
            }
            ExprKind::Cast { operand, ty } => {
                self.line(depth, "Cast", id, span, &"")?;
                self.expr(depth + 1, operand)?;
                self.ty(depth + 1, ty)
            }
            ExprKind::Call { callee, args } => {
                self.line(depth, "Call", id, span, &"")?;
                self.expr(depth + 1, callee)?;
                for arg in args {
                    self.expr(depth + 1, arg)?;
                }
                Ok(())
            }
            ExprKind::Member { base, name } => {
                let data = format!(".{}", Name(&name.name));
                self.line(depth, "Member", id, span, &data)?;
                self.expr(depth + 1, base)
            }
            ExprKind::Error => self.line(depth, "ErrorExpr", id, span, &""),
        }
    }

    fn literal(
        &mut self,
        depth: usize,
        id: Option<NodeId>,
        span: Span,
        literal: &Literal,
    ) -> fmt::Result {
        match literal {
            Literal::Int { value, base } => {
                let base = match base {
                    Base::Binary => "binary",
                    Base::Octal => "octal",
                    Base::Decimal => "decimal",
                    Base::Hexadecimal => "hexadecimal",
                };
                let data = match value {
                    Some(value) => format!("{value} {base}"),
                    None => format!("more-than-128-bits {base}"),
                };
                self.line(depth, "Int", id, span, &data)
            }
            Literal::Float(digits) => self.line(depth, "Float", id, span, digits),
            Literal::String(value) => self.line(depth, "String", id, span, &format!("{value:?}")),
            Literal::Char(value) => self.line(depth, "Char", id, span, &format!("{value:?}")),
            Literal::Bool(value) => self.line(depth, "Bool", id, span, value),
        }
    }
}

/// Renders a tree to a string; see [`render`].
///
/// # Panics
///
/// Panics if a span lies outside `source`.
#[must_use]
pub fn render_to_string(file: &File, source: &SourceFile) -> String {
    let mut out = String::new();
    render(&mut out, file, source).expect("writing to a string cannot fail");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssignOp, BinaryOp, Func, Ident, If, IfBranch, Import, UnaryOp};
    use pinguer_source::SourceMap;

    /// Builds nodes with consecutive IDs over a source of spaces, where
    /// byte `n` is column `n + 1`.
    struct Builder {
        file: pinguer_source::FileId,
        next: u32,
    }

    impl Builder {
        fn id(&mut self) -> NodeId {
            self.next += 1;
            NodeId::new(self.next - 1)
        }
        fn span(&self, start: u32, end: u32) -> Span {
            Span::new(self.file, start, end)
        }
        fn ident(&self, name: &str, start: u32) -> Ident {
            Ident {
                name: name.into(),
                span: self.span(start, start + name.len() as u32),
            }
        }
        fn expr(&mut self, start: u32, end: u32, kind: ExprKind) -> Expr {
            Expr {
                id: self.id(),
                span: self.span(start, end),
                kind,
            }
        }
        fn ty(&mut self, start: u32, end: u32, kind: TypeKind) -> Type {
            Type {
                id: self.id(),
                span: self.span(start, end),
                kind,
            }
        }
    }

    #[test]
    fn renders_every_kind_of_node() {
        let mut map = SourceMap::new();
        let file_id = map.add("t.pgr", " ".repeat(90)).unwrap();
        let mut b = Builder {
            file: file_id,
            next: 0,
        };
        let import = Item {
            id: b.id(),
            span: b.span(0, 13),
            kind: ItemKind::Import(Import {
                path: vec![b.ident("std", 7), b.ident("io", 11)],
            }),
        };
        let const_id = b.id();
        let const_ty = b.ty(10, 13, TypeKind::Name(b.ident("int", 10)));
        let neg_operand = b.expr(
            17,
            19,
            ExprKind::Literal(Literal::Int {
                value: Some(255),
                base: Base::Hexadecimal,
            }),
        );
        let const_value = Expr {
            id: b.id(),
            span: b.span(16, 19),
            kind: ExprKind::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(neg_operand),
            },
        };
        let constant = Item {
            id: const_id,
            span: b.span(0, 19),
            kind: ItemKind::Const(Binding {
                kind: BindingKind::Const,
                name: b.ident("MAX", 6),
                ty: Some(const_ty),
                value: const_value,
            }),
        };
        let func_id = b.id();
        let param = Param {
            id: b.id(),
            span: b.span(7, 13),
            name: b.ident("x", 7),
            ty: b.ty(10, 13, TypeKind::Name(b.ident("f64", 10))),
        };
        let ret = b.ty(18, 20, TypeKind::Unit);
        let body_id = b.id();
        let stmts = {
            let let_id = b.id();
            let left = b.expr(30, 31, ExprKind::Name(b.ident("x", 30)));
            let right = b.expr(34, 39, ExprKind::Literal(Literal::Float("1.5e3".into())));
            let sum = Expr {
                id: b.id(),
                span: b.span(30, 39),
                kind: ExprKind::Binary {
                    op: BinaryOp::Add,
                    op_span: b.span(32, 33),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
            let let_stmt = Stmt {
                id: let_id,
                span: b.span(22, 39),
                kind: StmtKind::Binding(Binding {
                    kind: BindingKind::Let,
                    name: b.ident("y", 26),
                    ty: None,
                    value: sum,
                }),
            };
            let assign_id = b.id();
            let cast_operand = b.expr(45, 46, ExprKind::Name(b.ident("y", 45)));
            let cast_ty = b.ty(50, 53, TypeKind::Name(b.ident("int", 50)));
            let cast = Expr {
                id: b.id(),
                span: b.span(45, 53),
                kind: ExprKind::Cast {
                    operand: Box::new(cast_operand),
                    ty: cast_ty,
                },
            };
            let assign = Stmt {
                id: assign_id,
                span: b.span(40, 53),
                kind: StmtKind::Assign {
                    target: b.ident("z", 40),
                    op: AssignOp::Add,
                    op_span: b.span(42, 44),
                    value: cast,
                },
            };
            let if_id = b.id();
            let cond = b.expr(57, 61, ExprKind::Literal(Literal::Bool(true)));
            let then_block = Block {
                id: b.id(),
                span: b.span(62, 64),
                stmts: Vec::new(),
            };
            let else_block = Block {
                id: b.id(),
                span: b.span(70, 72),
                stmts: vec![Stmt {
                    id: b.id(),
                    span: b.span(70, 71),
                    kind: StmtKind::Break,
                }],
            };
            let if_stmt = Stmt {
                id: if_id,
                span: b.span(54, 72),
                kind: StmtKind::If(If {
                    branches: vec![IfBranch {
                        span: b.span(54, 64),
                        cond,
                        block: then_block,
                    }],
                    else_block: Some(else_block),
                }),
            };
            let call_stmt_id = b.id();
            let call_id = b.id();
            let base = b.expr(73, 75, ExprKind::Name(b.ident("io", 73)));
            let member = Expr {
                id: b.id(),
                span: b.span(73, 77),
                kind: ExprKind::Member {
                    base: Box::new(base),
                    name: b.ident("p", 76),
                },
            };
            let inner = b.expr(
                78,
                81,
                ExprKind::Literal(Literal::String("\u{1b}\"".into())),
            );
            let arg = Expr {
                id: b.id(),
                span: b.span(77, 80),
                kind: ExprKind::Paren(Box::new(inner)),
            };
            let call_stmt = Stmt {
                id: call_stmt_id,
                span: b.span(73, 80),
                kind: StmtKind::Expr(Expr {
                    id: call_id,
                    span: b.span(73, 80),
                    kind: ExprKind::Call {
                        callee: Box::new(member),
                        args: vec![arg],
                    },
                }),
            };
            vec![let_stmt, assign, if_stmt, call_stmt]
        };
        let body = Block {
            id: body_id,
            span: b.span(21, 80),
            stmts,
        };
        let func = Item {
            id: func_id,
            span: b.span(0, 80),
            kind: ItemKind::Func(Func {
                name: b.ident("f", 5),
                params: vec![param],
                ret: Some(ret),
                body,
            }),
        };
        let file = File {
            span: b.span(0, 80),
            items: vec![import, constant, func],
            node_count: b.next,
        };
        let expected = "\
File @1:1-1:81 nodes=29
  Import #0 @1:1-1:14 std.io
  Const #1 @1:1-1:20 MAX
    Type #2 @1:11-1:14 int
    Unary #4 @1:17-1:20 -
      Int #3 @1:18-1:20 255 hexadecimal
  Func #5 @1:1-1:81 f
    Param #6 @1:8-1:14 x
      Type #7 @1:11-1:14 f64
    Type #8 @1:19-1:21 ()
    Block #9 @1:22-1:81
      Let #10 @1:23-1:40 y
        Binary #13 @1:31-1:40 +
          Name #11 @1:31-1:32 x
          Float #12 @1:35-1:40 1.5e3
      Assign #14 @1:41-1:54 z +=
        Cast #17 @1:46-1:54
          Name #15 @1:46-1:47 y
          Type #16 @1:51-1:54 int
      If #18 @1:55-1:73
        Branch @1:55-1:65
          Bool #19 @1:58-1:62 true
          Block #20 @1:63-1:65
        Else @1:71-1:73
          Block #21 @1:71-1:73
            Break #22 @1:71-1:72
      ExprStmt #23 @1:74-1:81
        Call #24 @1:74-1:81
          Member #26 @1:74-1:78 .p
            Name #25 @1:74-1:76 io
          Paren #28 @1:78-1:81
            String #27 @1:79-1:82 \"\\u{1b}\\\"\"
";
        assert_eq!(render_to_string(&file, map.file(file_id)), expected);
    }

    #[test]
    fn errors_and_unusual_values_are_rendered() {
        let mut map = SourceMap::new();
        let file_id = map.add("e.pgr", " ".repeat(20)).unwrap();
        let mut b = Builder {
            file: file_id,
            next: 0,
        };
        let value = b.expr(0, 3, ExprKind::Error);
        let big = b.expr(
            10,
            12,
            ExprKind::Literal(Literal::Int {
                value: None,
                base: Base::Binary,
            }),
        );
        let file = File {
            span: b.span(0, 20),
            items: vec![
                Item {
                    id: b.id(),
                    span: b.span(0, 2),
                    kind: ItemKind::Error,
                },
                Item {
                    id: b.id(),
                    span: b.span(3, 9),
                    kind: ItemKind::Const(Binding {
                        kind: BindingKind::Const,
                        name: b.ident("café", 3),
                        ty: Some(b.ty(4, 5, TypeKind::Error)),
                        value,
                    }),
                },
                Item {
                    id: b.id(),
                    span: b.span(10, 12),
                    kind: ItemKind::Const(Binding {
                        kind: BindingKind::Const,
                        name: b.ident("B", 10),
                        ty: None,
                        value: big,
                    }),
                },
            ],
            node_count: b.next,
        };
        let rendered = render_to_string(&file, map.file(file_id));
        assert!(rendered.contains("  ErrorItem #2 @1:1-1:3\n"), "{rendered}");
        assert!(
            rendered.contains("  Const #3 @1:4-1:10 \"café\"\n"),
            "{rendered}"
        );
        assert!(
            rendered.contains("    ErrorType #4 @1:5-1:6\n"),
            "{rendered}"
        );
        assert!(
            rendered.contains("    ErrorExpr #0 @1:1-1:4\n"),
            "{rendered}"
        );
        assert!(
            rendered.contains("    Int #1 @1:11-1:13 more-than-128-bits binary\n"),
            "{rendered}"
        );
    }
}
