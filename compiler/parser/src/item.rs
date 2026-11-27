//! Source files and top-level items.

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostic, codes};
use pinguer_lexer::{Keyword, TokenKind};
use pinguer_source::Span;

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
            node_count: self.next_id,
        }
    }

    /// `Item = ImportDecl | FuncDecl | ConstDecl`
    fn item(&mut self) -> ast::Item {
        let id = self.next_id();
        let start = self.start();
        let kind = match self.cursor.kind() {
            TokenKind::Keyword(Keyword::Import) => self.import(),
            _ => Err(self.expected_item()),
        };
        ast::Item {
            id,
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
}
