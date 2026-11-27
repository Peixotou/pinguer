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

mod const_decl {
    use super::*;

    #[test]
    fn declares_a_constant() {
        let expected = concat!("  Const #0 @1:1-1:16 LIMIT\n", "    Name #1 @1:15-1:16 x\n",);
        assert_eq!(tree("const LIMIT = x"), expected);
        // A line break after `=` continues the declaration.
        assert_eq!(tree("const LIMIT =\n    x").lines().count(), 2);
    }

    #[test]
    fn needs_an_initializer() {
        for text in ["const LIMIT", "const LIMIT\nimport a", "const LIMIT;"] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, ["P2006"], "{text:?}");
            assert_eq!(
                parsed.messages,
                ["`const` declaration has no initializer"],
                "{text:?}"
            );
        }
    }

    #[test]
    fn needs_a_name_an_equals_sign_and_an_expression() {
        for (text, code, message) in [
            ("const = x", "P2002", "expected identifier, found `=`"),
            ("const X y", "P2005", "expected `=`, found identifier `y`"),
            ("const X = ;", "P2004", "expected expression, found `;`"),
            (
                "const X =",
                "P2004",
                "expected expression, found end of file",
            ),
        ] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, [code], "{text:?}");
            assert_eq!(parsed.messages, [message], "{text:?}");
        }
    }

    #[test]
    fn may_have_a_type_annotation() {
        let expected = concat!(
            "  Const #0 @1:1-1:17 X\n",
            "    Type #1 @1:10-1:13 int\n",
            "    Name #2 @1:16-1:17 y\n",
        );
        assert_eq!(tree("const X: int = y"), expected);
    }
}

/// `Type = IDENT | UnitType`, `UnitType = "(" , ")"`
mod ty {
    use super::*;

    #[test]
    fn names_and_the_unit_type() {
        assert!(tree("const X: string = y").contains("    Type #1 @1:10-1:16 string\n"));
        assert!(tree("const X: () = y").contains("    Type #1 @1:10-1:12 ()\n"));
        // Whitespace inside `()` is allowed: they are two tokens.
        assert!(tree("const X: ( ) = y").contains("    Type #1 @1:10-1:13 ()\n"));
    }

    #[test]
    fn other_tokens_are_not_types() {
        for (text, code, message) in [
            ("const X: = y", "P2007", "expected type, found `=`"),
            ("const X:", "P2007", "expected type, found end of file"),
            (
                "const X: 1 = y",
                "P2007",
                "expected type, found integer literal",
            ),
            (
                "const X: (int) = y",
                "P2005",
                "expected `)`, found identifier `int`",
            ),
        ] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, [code], "{text:?}");
            assert_eq!(parsed.messages, [message], "{text:?}");
        }
    }
}

/// `PrimaryExpr = INT_LITERAL | FLOAT_LITERAL | STRING_LITERAL | CHAR_LITERAL
/// | "true" | "false" | IDENT | "(" , Expr , ")"`
mod primary_expr {
    use super::*;

    /// The rendered initializer of `const X = <expression>`.
    pub(crate) fn value(expression: &str) -> String {
        let tree = tree(&format!("const X = {expression}"));
        let (_, value) = tree.split_once('\n').unwrap();
        value.to_owned()
    }

    #[test]
    fn integer_literals_keep_their_value_and_base() {
        assert_eq!(value("1_000"), "    Int #1 @1:11-1:16 1000 decimal\n");
        assert_eq!(value("0xFF"), "    Int #1 @1:11-1:15 255 hexadecimal\n");
        assert_eq!(value("0o17"), "    Int #1 @1:11-1:15 15 octal\n");
        assert_eq!(value("0b1010"), "    Int #1 @1:11-1:17 10 binary\n");
        // Wider than any v0.1 type, but kept exactly for the type checker.
        assert_eq!(
            value("18446744073709551616"),
            "    Int #1 @1:11-1:31 18446744073709551616 decimal\n"
        );
        assert_eq!(
            value(&"9".repeat(40)),
            "    Int #1 @1:11-1:51 more-than-128-bits decimal\n"
        );
    }

    #[test]
    fn malformed_integer_literals_are_reported_once_by_the_lexer() {
        for (text, code) in [
            ("0b12", "P1006"),
            ("007", "P1007"),
            ("10u8", "P1006"),
            ("0X1F", "P1006"),
        ] {
            let parsed = parse(&format!("const X = {text}"));
            assert_eq!(parsed.codes, [code], "{text}");
            assert!(parsed.tree.contains("    ErrorExpr #1 @1:11-"), "{text}");
        }
    }
}
