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

/// Numbers every node of `file` in pre-order and records the count.
pub(crate) fn number(file: &mut ast::File) {
    let mut numbering = Numbering { next: 0 };
    for item in &mut file.items {
        numbering.item(item);
    }
    file.node_count = numbering.next;
}

struct Numbering {
    next: u32,
}

impl Numbering {
    fn id(&mut self) -> ast::NodeId {
        let id = ast::NodeId::new(self.next);
        // A file of at most `u32::MAX` bytes cannot reach this count;
        // ADR-0030 requires the check that replaces this saturation.
        self.next = self.next.saturating_add(1);
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
            ast::ExprKind::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
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
