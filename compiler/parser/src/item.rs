//! Source files and top-level items.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::{Keyword, TokenKind};
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
            let item = self.item();
            let failed = item.kind == ast::ItemKind::Error;
            items.push(item);
            // Until item-level recovery exists, the first error ends the
            // file.
            if failed {
                break;
            }
            if !self.cursor.at(TokenKind::Eof) && !self.at_separator() {
                self.missing_separator("item");
                break;
            }
            self.skip_separators();
        }
        ast::File {
            span,
            items,
            node_count: 0,
        }
    }

    /// `Item = ImportDecl | FuncDecl | ConstDecl`
    fn item(&mut self) -> ast::Item {
        let start = self.start();
        let kind = match self.cursor.kind() {
            TokenKind::Keyword(Keyword::Import) => self.import(),
            TokenKind::Keyword(Keyword::Const) => self
                .binding(ast::BindingKind::Const)
                .map(ast::ItemKind::Const),
            _ => Err(self.expected_item()),
        };
        ast::Item {
            id: PENDING,
            span: self.span_from(start),
            kind: kind.unwrap_or(ast::ItemKind::Error),
        }
    }

    fn expected_item(&mut self) -> Reported {
        let found = self.cursor.current();
        self.error(
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
        Ok(ast::ItemKind::Import(ast::Import { path }))
    }

    /// `ConstDecl = "const" , IDENT , [ ":" , Type ] , "=" , Expr`
    pub(crate) fn binding(&mut self, kind: ast::BindingKind) -> PResult<ast::Binding> {
        let keyword = self.cursor.bump();
        let name = self.ident()?;
        let ty = if self.cursor.at(TokenKind::Colon) {
            self.cursor.bump();
            Some(self.ty()?)
        } else {
            None
        };
        if self.at_separator() || self.cursor.at(TokenKind::Eof) {
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
