//! Source files and top-level items.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::{Keyword, Token, TokenKind};
use pinguer_source::Span;

use crate::ids::PENDING;
use crate::{PResult, Parser, Reported};

impl Parser<'_> {
    /// `File = { Separator } , [ Item , { Separator , { Separator } , Item } ,
    /// { Separator } ] , EOF`
    pub(crate) fn file(&mut self, span: Span) -> ast::File {
        let mut items = Vec::new();
        self.skip_separators();
        while !self.cursor.at(TokenKind::Eof) {
            let mut item = self.item();
            if item.kind == ast::ItemKind::Error {
                // The error node covers what recovery skipped.
                let end = self.skip_item().max(item.span.end);
                item.span = Span::new(self.file, item.span.start, end);
            } else if !self.cursor.at(TokenKind::Eof) && !self.at_separator() {
                self.missing_separator("item");
                self.skip_item();
            }
            items.push(item);
            self.skip_separators();
        }
        ast::File {
            span,
            items: items.into(),
            node_count: 0,
        }
    }

    /// Skips the rest of an item after an error (ADR-0029): up to `func`,
    /// `import` or `const` at the start of a line and outside every
    /// delimiter opened while skipping, or to the end of the file. A
    /// function body is therefore skipped as a whole, even when it contains
    /// `const` declarations. Every token is consumed at most once.
    ///
    /// Returns the end of the last token skipped other than a line break,
    /// or the end of the last consumed token if nothing was skipped.
    fn skip_item(&mut self) -> u32 {
        let mut depth = 0usize;
        let mut end = self.cursor.prev_end();
        loop {
            let token = self.cursor.current();
            match token.kind {
                TokenKind::Eof => return end,
                TokenKind::Keyword(Keyword::Func | Keyword::Import | Keyword::Const)
                    if depth == 0 && self.starts_line(token) =>
                {
                    return end;
                }
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            if token.kind != TokenKind::Newline {
                end = token.span.end;
            }
            self.cursor.bump();
        }
    }

    /// Returns `true` if only spaces and tabs precede `token` on its line.
    ///
    /// The source text is consulted rather than `NEWLINE` tokens, because
    /// inside an unclosed `(` or `[` the lexer produces none. The scan stops
    /// at the first other character, so it reads only the whitespace
    /// between `token` and the token before it.
    fn starts_line(&self, token: Token) -> bool {
        self.text[..token.span.start as usize]
            .bytes()
            .rev()
            .find(|&byte| byte != b' ' && byte != b'\t')
            .is_none_or(|byte| byte == b'\n')
    }

    /// `Item = ImportDecl | FuncDecl | ConstDecl`
    fn item(&mut self) -> ast::Item {
        let start = self.start();
        // See `stmt`.
        let depth = self.depth;
        let kind = match self.cursor.kind() {
            TokenKind::Keyword(Keyword::Import) => self.import(),
            TokenKind::Keyword(Keyword::Func) => self.func(),
            TokenKind::Keyword(Keyword::Const) => self
                .binding(ast::BindingKind::Const)
                .map(ast::ItemKind::Const),
            TokenKind::Keyword(Keyword::Let | Keyword::Var) => Err(self.misplaced_declaration(
                "inside a function",
                "at the top level, only `import`, `func` and `const` declarations are allowed",
            )),
            kind if crate::is_closing_delimiter(kind) => Err(self.unexpected_closing_delimiter()),
            kind if crate::expr::planned_syntax(kind).is_some() => Err(self.unsupported()),
            _ => Err(self.expected_item()),
        };
        self.depth = depth;
        ast::Item {
            id: PENDING,
            span: self.span_from(start),
            kind: kind.unwrap_or(ast::ItemKind::Error),
        }
    }

    fn expected_item(&mut self) -> Reported {
        let found = self.cursor.current();
        self.expected(
            Diagnostic::error(format!(
                "expected `import`, `func` or `const`, found {}",
                self.describe(found)
            ))
            .with_code(codes::EXPECTED_ITEM)
            .with_primary(found.span, "expected an item"),
        )
    }

    /// `ImportDecl = "import" , ModulePath`, with
    /// `ModulePath = IDENT , { "." , IDENT }`
    fn import(&mut self) -> PResult<ast::ItemKind> {
        self.cursor.bump();
        let mut path = vec![self.ident()?];
        while self.cursor.at(TokenKind::Dot) {
            self.cursor.bump();
            path.push(self.ident()?);
        }
        Ok(ast::ItemKind::Import(ast::Import { path: path.into() }))
    }

    /// `FuncDecl = "func" , IDENT , "(" , [ ParamList ] , ")" , [ "->" ,
    /// Type ] , Block`
    fn func(&mut self) -> PResult<ast::ItemKind> {
        self.cursor.bump();
        let name = self.ident()?;
        if self.cursor.at(TokenKind::Lt) {
            return Err(self.unsupported_syntax("a generic parameter list"));
        }
        let open = self.expect(TokenKind::LParen)?;
        let params = self.params(open)?;
        let ret = if self.cursor.at(TokenKind::Arrow) {
            self.cursor.bump();
            Some(self.declared_ty()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(ast::ItemKind::Func(ast::Func {
            name,
            params: params.into(),
            ret,
            body,
        }))
    }

    /// The rest of `"(" , [ ParamList ] , ")"` after `open`, with
    /// `ParamList = Param , { "," , Param } , [ "," ]`.
    fn params(&mut self, open: Token) -> PResult<Vec<ast::Param>> {
        let mut params = Vec::new();
        while !self.cursor.at(TokenKind::RParen) {
            params.push(self.param()?);
            if !self.cursor.at(TokenKind::Comma) {
                break;
            }
            self.cursor.bump();
        }
        self.expect_closing(open, TokenKind::RParen)?;
        Ok(params)
    }

    /// `Param = IDENT , ":" , Type`
    fn param(&mut self) -> PResult<ast::Param> {
        let start = self.start();
        let name = self.ident()?;
        if !self.cursor.at(TokenKind::Colon) {
            let found = self.cursor.current();
            return Err(self.expected(
                Diagnostic::error(format!(
                    "expected `:` and the parameter's type, found {}",
                    self.describe(found)
                ))
                .with_code(codes::MISSING_PARAMETER_TYPE)
                .with_primary(found.span, "expected `:`")
                .with_secondary(name.span, "this parameter has no type")
                .with_note("parameter types are always written: `name: Type`"),
            ));
        }
        self.cursor.bump();
        let ty = self.declared_ty()?;
        Ok(ast::Param {
            id: PENDING,
            span: self.span_from(start),
            name,
            ty,
        })
    }

    /// `ConstDecl = "const" , IDENT , [ ":" , Type ] , "=" , Expr`, and
    /// `LetDecl` and `VarDecl`, which have the same form.
    pub(crate) fn binding(&mut self, kind: ast::BindingKind) -> PResult<ast::Binding> {
        let keyword = self.cursor.bump();
        let name = self.ident()?;
        let ty = if self.cursor.at(TokenKind::Colon) {
            self.cursor.bump();
            Some(Box::new(self.declared_ty()?))
        } else {
            None
        };
        if self.at_separator()
            || self.cursor.at(TokenKind::Eof)
            || self.cursor.at(TokenKind::RBrace)
        {
            let found = self.cursor.current();
            return Err(self.error(
                Diagnostic::error(format!(
                    "`{}` declaration has no initializer",
                    kind.as_str()
                ))
                .with_code(codes::MISSING_INITIALIZER)
                .with_primary(found.span, "expected `=` and a value")
                .with_secondary(keyword.span.to(name.span), "declared here")
                .with_note("every `let`, `var` and `const` declaration has an initializer"),
            ));
        }
        self.expect(TokenKind::Eq)?;
        let value = self.expr()?;
        Ok(ast::Binding {
            kind,
            name,
            ty,
            value,
        })
    }
}
