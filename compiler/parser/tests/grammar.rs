//! Conformance tests for `specifications/GRAMMAR.ebnf`.
//!
//! There is one module per production of the grammar, named after it, with
//! positive tests (the rendered tree) and negative tests (the diagnostics).
//! The module list is the traceability matrix between the grammar and the
//! tests.

use pinguer_diagnostics::Diagnostics;
use pinguer_source::SourceMap;

/// The result of lexing and parsing one source text.
struct Parsed {
    /// The rendered tree without its `File` line.
    tree: String,
    /// Codes of the lexer and parser diagnostics, in order.
    codes: Vec<String>,
    /// Messages of the diagnostics, in order.
    messages: Vec<String>,
    /// Source text covered by each diagnostic's primary label.
    labelled: Vec<String>,
}

fn parse(text: &str) -> Parsed {
    let mut map = SourceMap::new();
    let file = map.add("g.pgr", text).unwrap();
    let mut diagnostics = Diagnostics::new();
    let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
    let tree = pinguer_parser::parse(file, text, tokens, &mut diagnostics);
    let rendered = pinguer_ast::render_to_string(&tree, map.file(file));
    let tree = rendered
        .split_once('\n')
        .map_or(String::new(), |(_, rest)| rest.to_owned());
    Parsed {
        tree,
        codes: diagnostics
            .iter()
            .map(|d| d.code.map_or_else(String::new, |code| code.to_string()))
            .collect(),
        messages: diagnostics.iter().map(|d| d.message.clone()).collect(),
        labelled: diagnostics
            .iter()
            .map(|d| map.snippet(d.main_label().unwrap().span).to_owned())
            .collect(),
    }
}

/// Parses `text`, asserts that it has no diagnostics and returns the tree.
fn tree(text: &str) -> String {
    let parsed = parse(text);
    assert!(parsed.codes.is_empty(), "{text:?}: {:?}", parsed.messages);
    parsed.tree
}

mod file {
    use super::*;

    #[test]
    fn separators_alone_make_an_empty_file() {
        for text in ["", "\n\n", ";;", "// comment\n;\n/* block */\n"] {
            assert_eq!(tree(text), "", "{text:?}");
        }
    }

    #[test]
    fn items_are_separated_by_line_breaks_or_semicolons() {
        let expected = concat!(
            "  Import #0 @1:1-1:9 a\n",
            "  Import #1 @1:10-1:18 b\n",
            "  Import #2 @4:1-4:9 c\n",
        );
        assert_eq!(tree("import a;import b\n\n\nimport c\n;"), expected);
    }

    #[test]
    fn other_tokens_are_not_items() {
        let parsed = parse("\nlet x = 1");
        assert_eq!(parsed.codes, ["P2001"]);
        assert_eq!(
            parsed.messages,
            ["expected `import`, `func` or `const`, found `let`"]
        );
        assert_eq!(parsed.labelled, ["let"]);
        let parsed = parse("main");
        assert_eq!(
            parsed.messages,
            ["expected `import`, `func` or `const`, found identifier `main`"]
        );
    }

    #[test]
    fn items_on_one_line_need_a_semicolon() {
        let parsed = parse("import a import b");
        assert_eq!(parsed.codes, ["P2003"]);
        assert_eq!(
            parsed.messages,
            ["expected a line break or `;` after the item, found `import`"]
        );
        assert_eq!(parsed.labelled, ["import"]);
    }
}

mod import_decl {
    use super::*;

    #[test]
    fn imports_a_module_path() {
        assert_eq!(tree("import std.io"), "  Import #0 @1:1-1:14 std.io\n");
        assert_eq!(tree("import a.b.c.d"), "  Import #0 @1:1-1:15 a.b.c.d\n");
    }

    #[test]
    fn a_path_continues_on_lines_that_start_with_a_dot() {
        assert_eq!(tree("import std\n    .io"), "  Import #0 @1:1-2:8 std.io\n");
    }

    #[test]
    fn path_components_are_identifiers() {
        for (text, labelled) in [
            ("import", ""),
            ("import std.", ""),
            ("import 1", "1"),
            ("import std.func", "func"),
        ] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, ["P2002"], "{text:?}");
            assert_eq!(parsed.labelled, [labelled], "{text:?}");
        }
        assert_eq!(
            parse("import").messages,
            ["expected identifier, found end of file"]
        );
        // `..` is one token, so the path ends after `std`.
        assert_eq!(parse("import std..io").codes, ["P2003"]);
    }
}
