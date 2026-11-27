//! Blocks and statements.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::{Keyword, TokenKind};

use crate::ids::PENDING;
use crate::{PResult, Parser};

impl Parser<'_> {
    /// `Block = "{" , { Separator } , [ Statement , { Separator ,
    /// { Separator } , Statement } , { Separator } ] , "}"`
    pub(crate) fn block(&mut self) -> PResult<ast::Block> {
        let start = self.start();
        let open = self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        self.skip_separators();
        while !self.cursor.at(TokenKind::RBrace) && !self.cursor.at(TokenKind::Eof) {
            let stmt = self.stmt();
            let failed = stmt.kind == ast::StmtKind::Error;
            stmts.push(stmt);
            // Until statement-level recovery exists, an error ends the
            // block, and with it the enclosing item.
            if failed {
                return Err(crate::Reported);
            }
            if self.cursor.at(TokenKind::RBrace) {
                break;
            }
            if !self.at_separator() {
                return Err(self.missing_separator("statement"));
            }
            self.skip_separators();
        }
        self.expect_closing(open, TokenKind::RBrace)?;
        Ok(ast::Block {
            id: PENDING,
            span: self.span_from(start),
            stmts,
        })
    }

    /// `Statement = LetDecl | VarDecl | ConstDecl | … | ExprStatement | …`
    fn stmt(&mut self) -> ast::Stmt {
        let start = self.start();
        let binding = match self.cursor.kind() {
            TokenKind::Keyword(Keyword::Let) => Some(ast::BindingKind::Let),
            TokenKind::Keyword(Keyword::Var) => Some(ast::BindingKind::Var),
            TokenKind::Keyword(Keyword::Const) => Some(ast::BindingKind::Const),
            _ => None,
        };
        let kind = match binding {
            Some(kind) => self.binding(kind).map(ast::StmtKind::Binding),
            None => self.expr_stmt(),
        };
        ast::Stmt {
            id: PENDING,
            span: self.span_from(start),
            kind: kind.unwrap_or(ast::StmtKind::Error),
        }
    }

    /// `ExprStatement = PostfixExpr`
    ///
    /// The whole expression is parsed so that the diagnostic can cover it.
    /// A postfix expression that is not a call, such as `x`, is valid
    /// syntax; semantic analysis rejects it.
    fn expr_stmt(&mut self) -> PResult<ast::StmtKind> {
        let expr = self.expr()?;
        if matches!(
            expr.kind,
            ast::ExprKind::Unary { .. } | ast::ExprKind::Binary { .. } | ast::ExprKind::Cast { .. }
        ) {
            return Err(self.error(
                Diagnostic::error("expression cannot be used as a statement")
                    .with_code(codes::EXPRESSION_STATEMENT)
                    .with_primary(expr.span, "its value would be discarded")
                    .with_note("only calls can be used as expression statements"),
            ));
        }
        Ok(ast::StmtKind::Expr(expr))
    }
}
