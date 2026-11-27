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
        // Layout rule: `{` is on the same line as what it belongs to. After
        // reporting a brace on the next line, parse the block as if it were
        // in place, so the rest of the construct is not reported again.
        if self.cursor.at(TokenKind::Newline) && self.cursor.peek(1) == TokenKind::LBrace {
            self.cursor.bump();
            let brace = self.cursor.current();
            self.error(
                Diagnostic::error("`{` must be on the same line as the code it belongs to")
                    .with_code(codes::BRACE_ON_NEXT_LINE)
                    .with_primary(brace.span, "this `{` starts a new line")
                    .with_note("a line break before `{` ends the declaration or statement")
                    .with_help("move the `{` to the end of the previous line"),
            );
        }
        let start = self.start();
        let open = self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        self.skip_separators();
        while !self.cursor.at(TokenKind::RBrace) && !self.cursor.at(TokenKind::Eof) {
            let stmt = self.stmt();
            if stmt.kind == ast::StmtKind::Error {
                self.skip_statement();
            }
            stmts.push(stmt);
            // The end of the file is left for `expect_closing`, which
            // reports the unclosed `{`.
            if self.cursor.at(TokenKind::RBrace) || self.cursor.at(TokenKind::Eof) {
                break;
            }
            if !self.at_separator() {
                // Report once and drop the rest of the line, rather than
                // reporting every token that follows.
                self.missing_separator("statement");
                self.skip_statement();
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

    /// Skips the rest of a statement after an error (ADR-0029): up to the
    /// next separator at the statement's own nesting level, the `}` that
    /// closes the enclosing block, which is left for the block, or the end
    /// of the file. Delimiters are counted, so a nested block or argument
    /// list is skipped as a whole; a closer without an opener is skipped.
    /// Every token is consumed at most once, so recovery is linear.
    fn skip_statement(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.cursor.kind() {
                TokenKind::Eof => return,
                TokenKind::Newline | TokenKind::Semicolon | TokenKind::RBrace if depth == 0 => {
                    return;
                }
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            self.cursor.bump();
        }
    }

    /// `Statement = LetDecl | VarDecl | ConstDecl | Assignment | ExprStatement
    /// | IfStatement | WhileStatement | ReturnStatement | "break" | "continue"
    /// | Block`
    fn stmt(&mut self) -> ast::Stmt {
        let start = self.start();
        let kind = match self.cursor.kind() {
            TokenKind::Keyword(Keyword::Let) => self
                .binding(ast::BindingKind::Let)
                .map(ast::StmtKind::Binding),
            TokenKind::Keyword(Keyword::Var) => self
                .binding(ast::BindingKind::Var)
                .map(ast::StmtKind::Binding),
            TokenKind::Keyword(Keyword::Const) => self
                .binding(ast::BindingKind::Const)
                .map(ast::StmtKind::Binding),
            TokenKind::Keyword(Keyword::If) => self.if_stmt(),
            TokenKind::LBrace => self.block().map(ast::StmtKind::Block),
            TokenKind::Keyword(Keyword::While) => self.while_stmt(),
            TokenKind::Keyword(Keyword::Return) => self.return_stmt(),
            // Whether they are inside a loop is checked by semantic
            // analysis.
            TokenKind::Keyword(Keyword::Break) => {
                self.cursor.bump();
                Ok(ast::StmtKind::Break)
            }
            TokenKind::Keyword(Keyword::Continue) => {
                self.cursor.bump();
                Ok(ast::StmtKind::Continue)
            }
            _ => self.expr_stmt(),
        };
        ast::Stmt {
            id: PENDING,
            span: self.span_from(start),
            kind: kind.unwrap_or(ast::StmtKind::Error),
        }
    }

    /// `IfStatement = "if" , Expr , Block , [ "else" , ( IfStatement |
    /// Block ) ]`
    ///
    /// The `else if` branches are collected into one list by a loop
    /// (ADR-0028), so a long chain neither recurses nor deepens the tree.
    fn if_stmt(&mut self) -> PResult<ast::StmtKind> {
        let mut branches = Vec::new();
        let mut else_block = None;
        loop {
            let start = self.start();
            self.cursor.bump();
            let cond = self.expr()?;
            let block = self.block()?;
            branches.push(ast::IfBranch {
                span: self.span_from(start),
                cond,
                block,
            });
            self.else_on_next_line();
            if !self.cursor.at(TokenKind::Keyword(Keyword::Else)) {
                break;
            }
            self.cursor.bump();
            if !self.cursor.at(TokenKind::Keyword(Keyword::If)) {
                else_block = Some(self.block()?);
                break;
            }
        }
        Ok(ast::StmtKind::If(ast::If {
            branches,
            else_block,
        }))
    }

    /// Layout rule: `else` is on the same line as the `}` before it.
    /// Otherwise the `if` statement has ended and `else` cannot start a
    /// statement; after reporting it, parse the `else` as if it were in
    /// place.
    fn else_on_next_line(&mut self) {
        if self.cursor.at(TokenKind::Newline)
            && self.cursor.peek(1) == TokenKind::Keyword(Keyword::Else)
        {
            self.cursor.bump();
            let keyword = self.cursor.current();
            self.error(
                Diagnostic::error("`else` must be on the same line as the `}` before it")
                    .with_code(codes::ELSE_ON_NEXT_LINE)
                    .with_primary(keyword.span, "this `else` starts a new line")
                    .with_note("a line break after `}` ends the `if` statement")
                    .with_help("write `} else` on one line"),
            );
        }
    }

    /// `WhileStatement = "while" , Expr , Block`
    fn while_stmt(&mut self) -> PResult<ast::StmtKind> {
        self.cursor.bump();
        let cond = self.expr()?;
        let body = self.block()?;
        Ok(ast::StmtKind::While { cond, body })
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
