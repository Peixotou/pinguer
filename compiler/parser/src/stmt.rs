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

    /// `Statement = LetDecl | VarDecl | ConstDecl | Assignment | ExprStatement
    /// | … | ReturnStatement | …`
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
            None if self.cursor.at(TokenKind::Keyword(Keyword::Return)) => self.return_stmt(),
            None => self.expr_stmt(),
        };
        ast::Stmt {
            id: PENDING,
            span: self.span_from(start),
            kind: kind.unwrap_or(ast::StmtKind::Error),
        }
    }

    /// `ReturnStatement = "return" , [ Expr ]`, without a value when a
    /// separator, the closing `}` or the end of the file follows.
    fn return_stmt(&mut self) -> PResult<ast::StmtKind> {
        self.cursor.bump();
        if self.at_separator()
            || self.cursor.at(TokenKind::RBrace)
            || self.cursor.at(TokenKind::Eof)
        {
            return Ok(ast::StmtKind::Return(None));
        }
        Ok(ast::StmtKind::Return(Some(self.expr()?)))
    }

    /// `ExprStatement = PostfixExpr`
    ///
    /// The whole expression is parsed so that the diagnostic can cover it.
    /// A postfix expression that is not a call, such as `x`, is valid
    /// syntax; semantic analysis rejects it.
    fn expr_stmt(&mut self) -> PResult<ast::StmtKind> {
        let expr = self.expr()?;
        if let Some(op) = assign_op(self.cursor.kind()) {
            return self.assignment(expr, op);
        }
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

    /// `Assignment = IDENT , AssignOp , Expr`, after the target has been
    /// parsed as an expression. Only a name is a valid target in v0.1.
    fn assignment(&mut self, target: ast::Expr, op: ast::AssignOp) -> PResult<ast::StmtKind> {
        let ast::ExprKind::Name(target) = target.kind else {
            return Err(self.error(
                Diagnostic::error("invalid assignment target")
                    .with_code(codes::INVALID_ASSIGNMENT_TARGET)
                    .with_primary(target.span, "cannot be assigned")
                    .with_note("only variables can be assigned in Pinguer v0.1"),
            ));
        };
        let op_span = self.cursor.bump().span;
        let value = self.expr()?;
        Ok(ast::StmtKind::Assign {
            target,
            op,
            op_span,
            value,
        })
    }
}

/// `AssignOp = "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^="
/// | "<<=" | ">>="`
fn assign_op(kind: TokenKind) -> Option<ast::AssignOp> {
    use ast::AssignOp as A;
    Some(match kind {
        TokenKind::Eq => A::Assign,
        TokenKind::PlusEq => A::Add,
        TokenKind::MinusEq => A::Sub,
        TokenKind::StarEq => A::Mul,
        TokenKind::SlashEq => A::Div,
        TokenKind::PercentEq => A::Rem,
        TokenKind::AmpEq => A::BitAnd,
        TokenKind::PipeEq => A::BitOr,
        TokenKind::CaretEq => A::BitXor,
        TokenKind::ShlEq => A::Shl,
        TokenKind::ShrEq => A::Shr,
        _ => return None,
    })
}

/// Returns `true` for the tokens that are assignment operators, which can
/// only follow the target of an assignment statement.
pub(crate) fn is_assign_op(kind: TokenKind) -> bool {
    assign_op(kind).is_some()
}
