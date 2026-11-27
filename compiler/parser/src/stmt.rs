//! Blocks and statements.

use pinguer_ast as ast;
use pinguer_lexer::TokenKind;

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

    /// `Statement = … | ExprStatement | …`
    fn stmt(&mut self) -> ast::Stmt {
        let start = self.start();
        let kind = self.expr_stmt();
        ast::Stmt {
            id: PENDING,
            span: self.span_from(start),
            kind: kind.unwrap_or(ast::StmtKind::Error),
        }
    }

    /// `ExprStatement = PostfixExpr`
    fn expr_stmt(&mut self) -> PResult<ast::StmtKind> {
        Ok(ast::StmtKind::Expr(self.expr()?))
    }
}
