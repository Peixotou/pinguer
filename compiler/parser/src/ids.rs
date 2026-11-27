//! Node numbering.
//!
//! Nodes are created with [`PENDING`] IDs and numbered once the tree is
//! complete, by one pre-order walk (ADR-0028). Numbering after parsing makes
//! the IDs independent of the order in which the parser builds nodes:
//! precedence climbing builds the left operand of a binary expression before
//! the expression itself. It also keeps the IDs dense, because nodes that
//! recovery discarded never reach the tree.

use pinguer_ast as ast;

/// The ID of a node that has not been numbered yet.
pub(crate) const PENDING: ast::NodeId = ast::NodeId::new(u32::MAX);

/// The number of IDs that can be assigned: every `u32` except [`PENDING`].
pub(crate) const CAPACITY: u32 = u32::MAX;

/// The file has more nodes than [`number`] may assign IDs to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TooManyNodes;

/// Numbers every node of `file` in pre-order and records the count, using
/// IDs below `capacity`.
///
/// A file of at most [`pinguer_source::MAX_FILE_SIZE`] bytes can have about
/// as many nodes as there are IDs, so the count is checked (ADR-0030): when
/// the IDs run out, the remaining nodes keep [`PENDING`] and the caller
/// reports the error, instead of IDs being reused or wrapped.
pub(crate) fn number(file: &mut ast::File, capacity: u32) -> Result<(), TooManyNodes> {
    let mut numbering = Numbering {
        next: 0,
        capacity,
        exhausted: false,
    };
    for item in &mut file.items {
        numbering.item(item);
    }
    file.node_count = numbering.next;
    if numbering.exhausted {
        Err(TooManyNodes)
    } else {
        Ok(())
    }
}

struct Numbering {
    next: u32,
    capacity: u32,
    exhausted: bool,
}

impl Numbering {
    fn id(&mut self) -> ast::NodeId {
        if self.next == self.capacity {
            self.exhausted = true;
            return PENDING;
        }
        let id = ast::NodeId::new(self.next);
        self.next += 1;
        id
    }

    fn item(&mut self, item: &mut ast::Item) {
        item.id = self.id();
        match &mut item.kind {
            ast::ItemKind::Import(_) | ast::ItemKind::Error => {}
            ast::ItemKind::Func(func) => {
                for param in &mut func.params {
                    param.id = self.id();
                    self.ty(&mut param.ty);
                }
                if let Some(ret) = &mut func.ret {
                    self.ty(ret);
                }
                self.block(&mut func.body);
            }
            ast::ItemKind::Const(binding) => self.binding(binding),
        }
    }

    fn binding(&mut self, binding: &mut ast::Binding) {
        if let Some(ty) = &mut binding.ty {
            self.ty(ty);
        }
        self.expr(&mut binding.value);
    }

    fn ty(&mut self, ty: &mut ast::Type) {
        ty.id = self.id();
    }

    fn block(&mut self, block: &mut ast::Block) {
        block.id = self.id();
        for stmt in &mut block.stmts {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &mut ast::Stmt) {
        stmt.id = self.id();
        match &mut stmt.kind {
            ast::StmtKind::Binding(binding) => self.binding(binding),
            ast::StmtKind::Assign { value, .. } => self.expr(value),
            ast::StmtKind::If(if_stmt) => {
                for branch in &mut if_stmt.branches {
                    self.expr(&mut branch.cond);
                    self.block(&mut branch.block);
                }
                if let Some(block) = &mut if_stmt.else_block {
                    self.block(block);
                }
            }
            ast::StmtKind::While { cond, body } => {
                self.expr(cond);
                self.block(body);
            }
            ast::StmtKind::Expr(expr) => self.expr(expr),
            ast::StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            ast::StmtKind::Block(block) => self.block(block),
            ast::StmtKind::Break | ast::StmtKind::Continue | ast::StmtKind::Error => {}
        }
    }

    fn expr(&mut self, expr: &mut ast::Expr) {
        expr.id = self.id();
        match &mut expr.kind {
            ast::ExprKind::Literal(_) | ast::ExprKind::Name(_) | ast::ExprKind::Error => {}
            ast::ExprKind::Paren(inner) => self.expr(inner),
            ast::ExprKind::Unary { operand, .. } => self.expr(operand),
            ast::ExprKind::Binary { first, rest } => {
                self.expr(first);
                for operand in rest {
                    self.expr(&mut operand.operand);
                }
            }
            ast::ExprKind::Cast { operand, ty } => {
                self.expr(operand);
                self.ty(ty);
            }
            ast::ExprKind::Call { callee, args } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
            }
            ast::ExprKind::Member { base, .. } => self.expr(base),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinguer_diagnostics::Diagnostics;
    use pinguer_source::SourceMap;

    fn tree(text: &str) -> ast::File {
        let mut map = SourceMap::new();
        let file = map.add("ids.pgr", text).unwrap();
        let mut diagnostics = Diagnostics::new();
        let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
        crate::parse(file, text, tokens, &mut diagnostics)
    }

    #[test]
    fn numbering_stops_when_the_ids_run_out() {
        // `Const`, `Binary`, two `Int` literals.
        let mut file = tree("const X = 1 + 2");
        assert_eq!(file.node_count, 4);
        assert_eq!(number(&mut file, 4), Ok(()));
        assert_eq!(number(&mut file, 3), Err(TooManyNodes));
        assert_eq!(file.node_count, 3);
        let ast::ItemKind::Const(binding) = &file.items[0].kind else {
            unreachable!()
        };
        let ast::ExprKind::Binary { rest, .. } = &binding.value.kind else {
            unreachable!()
        };
        // The last node in pre-order has no ID; none is reused.
        assert_eq!(rest[0].operand.id, PENDING);
    }
}
