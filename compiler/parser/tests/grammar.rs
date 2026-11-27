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
    /// Notes of each diagnostic.
    notes: Vec<Vec<String>>,
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
        notes: diagnostics.iter().map(|d| d.notes.clone()).collect(),
    }
}

/// Parses `text`, asserts that it has no diagnostics and returns the tree.
fn tree(text: &str) -> String {
    let parsed = parse(text);
    assert!(parsed.codes.is_empty(), "{text:?}: {:?}", parsed.messages);
    parsed.tree
}

/// Returns the structure of `expression`, parsed as the initializer of a
/// constant, in a compact form: one pair of parentheses per operator chain,
/// as in `(a - b + (c * d))`, `(- a)` for prefix operators, `(as x int)`,
/// `(call f x)`, `(. io println)` and `(paren x)`; names and literals as
/// rendered.
fn shape(expression: &str) -> String {
    struct Node {
        kind: String,
        data: String,
        children: Vec<Node>,
    }
    fn build(lines: &[(usize, Node)], index: &mut usize) -> Node {
        let depth = lines[*index].0;
        let Node { kind, data, .. } = &lines[*index].1;
        let mut node = Node {
            kind: kind.clone(),
            data: data.clone(),
            children: Vec::new(),
        };
        *index += 1;
        while *index < lines.len() && lines[*index].0 > depth {
            node.children.push(build(lines, index));
        }
        node
    }
    fn print(node: &Node) -> String {
        let children: Vec<String> = node.children.iter().map(print).collect();
        let joined = children.join(" ");
        match node.kind.as_str() {
            "Unary" => format!("({} {joined})", node.data),
            "Binary" => format!("({joined})"),
            "Int" => node.data.split(' ').next().unwrap().to_owned(),
            "Cast" => format!("(as {joined})"),
            "Call" => format!("(call {joined})"),
            "Member" => format!("(. {joined} {})", &node.data[1..]),
            "Paren" => format!("(paren {joined})"),
            "ErrorExpr" => "error".to_owned(),
            _ => node.data.clone(),
        }
    }
    let rendered = tree(&format!("const X = {expression}"));
    let lines: Vec<(usize, Node)> = rendered
        .lines()
        .skip(1)
        .map(|line| {
            let depth = line.len() - line.trim_start().len();
            // `Kind [#id] @span [data]`; operator lines have no ID.
            let mut words = line.trim_start().split(' ').peekable();
            let kind = words.next().unwrap().to_owned();
            words.next_if(|word| word.starts_with('#'));
            words.next_if(|word| word.starts_with('@'));
            let data = words.collect::<Vec<_>>().join(" ");
            (
                depth,
                Node {
                    kind,
                    data,
                    children: Vec::new(),
                },
            )
        })
        .collect();
    print(&build(&lines, &mut 0))
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

    /// Asserts that the IDs of the tree rendered for `text` are exactly
    /// `0..node_count`, in the order the lines appear, which is pre-order.
    fn assert_dense_pre_order(text: &str) {
        let mut map = SourceMap::new();
        let file = map.add("ids.pgr", text).unwrap();
        let mut diagnostics = Diagnostics::new();
        let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
        let tree = pinguer_parser::parse(file, text, tokens, &mut diagnostics);
        let rendered = pinguer_ast::render_to_string(&tree, map.file(file));
        let ids: Vec<u32> = rendered
            .split_whitespace()
            .filter_map(|word| word.strip_prefix('#')?.parse().ok())
            .collect();
        let expected: Vec<u32> = (0..tree.node_count).collect();
        assert_eq!(ids, expected, "{text:?}\n{rendered}");
    }

    #[test]
    fn node_ids_are_dense_and_in_pre_order() {
        assert_dense_pre_order("import a.b\nconst X: int = ((y))\nconst Z = 0x10");
        // An error node replaces the item; no ID is left unused.
        assert_dense_pre_order("const X: int = (y");
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
                "P2008",
                "expected `)` to close `(`, found identifier `int`",
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
    fn floating_point_literals_keep_their_digits() {
        assert_eq!(value("6.02e23"), "    Float #1 @1:11-1:18 6.02e23\n");
        assert_eq!(value("1_000.5"), "    Float #1 @1:11-1:18 1000.5\n");
        let parsed = parse("const X = 1.5e");
        assert_eq!(parsed.codes, ["P1006"]);
        assert!(parsed.tree.contains("    ErrorExpr #1"));
    }

    #[test]
    fn string_literals_are_decoded() {
        assert_eq!(
            value(r#""tab\t\u{1F427}""#),
            "    String #1 @1:11-1:27 \"tab\\t🐧\"\n"
        );
        // The rendering escapes control characters produced by escapes.
        assert!(value(r#""\u{1B}[2J""#).contains(r#""\u{1b}[2J""#));
        for text in [r#""\q""#, r#""open"#] {
            let parsed = parse(&format!("const X = {text}"));
            assert_eq!(parsed.codes.len(), 1, "{text}: {:?}", parsed.messages);
            assert!(parsed.codes[0].starts_with("P100"), "{text}");
            assert!(parsed.tree.contains("    ErrorExpr #1"), "{text}");
        }
    }

    #[test]
    fn character_literals_are_decoded() {
        assert_eq!(value("'a'"), "    Char #1 @1:11-1:14 'a'\n");
        assert_eq!(value(r"'\u{1F427}'"), "    Char #1 @1:11-1:22 '🐧'\n");
        let parsed = parse("const X = 'ab'");
        assert_eq!(parsed.codes, ["P1008"]);
        assert!(parsed.tree.contains("    ErrorExpr #1"));
    }

    #[test]
    fn boolean_literals_and_names() {
        assert_eq!(value("true"), "    Bool #1 @1:11-1:15 true\n");
        assert_eq!(value("false"), "    Bool #1 @1:11-1:16 false\n");
        // Only the lowercase keywords are literals.
        assert_eq!(value("True"), "    Name #1 @1:11-1:15 True\n");
        assert_eq!(value("_count"), "    Name #1 @1:11-1:17 _count\n");
    }

    #[test]
    fn parentheses_are_kept_in_the_tree() {
        let expected = concat!(
            "    Paren #1 @1:11-1:16\n",
            "      Paren #2 @1:12-1:15\n",
            "        Name #3 @1:13-1:14 a\n",
        );
        assert_eq!(value("((a))"), expected);
        // Line breaks inside parentheses are not tokens at all.
        assert!(value("(\n  a\n)").starts_with("    Paren #1 @1:11-3:2\n"));
    }

    #[test]
    fn unclosed_parentheses_point_at_the_opening_one() {
        let parsed = parse("const X = (a");
        assert_eq!(parsed.codes, ["P2008"]);
        assert_eq!(
            parsed.messages,
            ["expected `)` to close `(`, found end of file"]
        );
        for (text, message) in [
            ("const X = (", "expected expression, found end of file"),
            ("const X = ()", "expected expression, found `)`"),
        ] {
            assert_eq!(parse(text).messages, [message], "{text}");
        }
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

/// `UnaryExpr = ( "-" | "!" | "~" ) , UnaryExpr | PostfixExpr`
mod unary_expr {
    use super::primary_expr::value;
    use super::*;

    #[test]
    fn prefix_operators_nest() {
        let expected = concat!(
            "    Unary #1 @1:11-1:15 -\n",
            "      Unary #2 @1:12-1:15 !\n",
            "        Unary #3 @1:13-1:15 ~\n",
            "          Name #4 @1:14-1:15 x\n",
        );
        assert_eq!(value("-!~x"), expected);
        // `--x` is two prefix operators: there is no decrement operator.
        assert_eq!(value("--x").matches("Unary").count(), 2);
    }

    #[test]
    fn a_negative_literal_is_negation_of_a_literal() {
        let expected = concat!(
            "    Unary #1 @1:11-1:15 -\n",
            "      Int #2 @1:12-1:15 128 decimal\n",
        );
        assert_eq!(value("-128"), expected);
    }

    #[test]
    fn a_prefix_operator_needs_an_operand() {
        assert_eq!(
            parse("const X = -").messages,
            ["expected expression, found end of file"]
        );
        // A line break after `-` continues the line; after `!` it does not.
        assert_eq!(value("-\n1").matches("Unary").count(), 1);
        assert_eq!(
            parse("const X = !\n1").messages,
            ["expected expression, found newline"]
        );
    }
}

/// `MulExpr = CastExpr , { ( "*" | "/" | "%" ) , CastExpr }`
mod mul_expr {
    use super::*;

    #[test]
    fn multiplicative_operators_are_left_associative() {
        assert_eq!(shape("a * b"), "(a * b)");
        assert_eq!(shape("a / b % c * d"), "(a / b % c * d)");
    }

    #[test]
    fn prefix_operators_bind_more_tightly() {
        assert_eq!(shape("-a * -b"), "((- a) * (- b))");
    }

    #[test]
    fn the_right_operand_is_required() {
        assert_eq!(
            parse("const X = a *").messages,
            ["expected expression, found end of file"]
        );
        // `a * *b` is two `*` tokens, and `*` is not a prefix operator.
        assert_eq!(
            parse("const X = a * *b").messages,
            ["expected expression, found `*`"]
        );
    }
}

/// `AddExpr = MulExpr , { ( "+" | "-" ) , MulExpr }`
mod add_expr {
    use super::*;

    #[test]
    fn additive_operators_are_left_associative() {
        assert_eq!(shape("a - b + c"), "(a - b + c)");
    }

    #[test]
    fn multiplicative_operators_bind_more_tightly() {
        assert_eq!(shape("a + b * c"), "(a + (b * c))");
        assert_eq!(shape("a * b + c"), "((a * b) + c)");
        assert_eq!(shape("(a + b) * c"), "((paren (a + b)) * c)");
        assert_eq!(shape("a - -b"), "(a - (- b))");
    }

    #[test]
    fn spans_cover_both_operands_and_record_the_operator() {
        let tree = tree("const X = a +\n    b");
        assert!(tree.contains("    Binary #1 @1:11-2:6\n"), "{tree}");
        assert!(tree.contains("      Op @1:13-1:14 +\n"), "{tree}");
    }

    #[test]
    fn long_chains_are_one_node() {
        // A binary tree for this chain would be 100,000 levels deep and
        // overflow the stack when rendered or dropped.
        let chain = vec!["a"; 100_000].join(" + ");
        let parsed = parse(&format!("const X = {chain}"));
        assert!(parsed.codes.is_empty());
        assert_eq!(parsed.tree.matches("Binary").count(), 1);
        assert_eq!(parsed.tree.matches(" Op ").count(), 99_999);
    }
}

/// `ShiftExpr = AddExpr , { ( "<<" | ">>" ) , AddExpr }`
mod shift_expr {
    use super::*;

    #[test]
    fn shifts_chain_from_left_to_right() {
        assert_eq!(shape("a << b >> c"), "(a << b >> c)");
    }

    #[test]
    fn additive_operators_bind_more_tightly() {
        assert_eq!(shape("a << b + c"), "(a << (b + c))");
        assert_eq!(shape("a - b >> c"), "((a - b) >> c)");
    }

    #[test]
    fn a_shift_needs_a_right_operand() {
        assert_eq!(
            parse("const X = a <<").messages,
            ["expected expression, found end of file"]
        );
    }
}

/// `BitAndExpr = ShiftExpr , { "&" , ShiftExpr }`,
/// `BitXorExpr = BitAndExpr , { "^" , BitAndExpr }`,
/// `BitOrExpr = BitXorExpr , { "|" , BitXorExpr }`
mod bitwise_expr {
    use super::*;

    #[test]
    fn each_operator_has_its_own_level() {
        assert_eq!(shape("a & b ^ c | d"), "(((a & b) ^ c) | d)");
        assert_eq!(shape("a | b ^ c & d"), "(a | (b ^ (c & d)))");
        assert_eq!(shape("a | b | c"), "(a | b | c)");
    }

    #[test]
    fn shifts_bind_more_tightly_than_and() {
        assert_eq!(shape("a & b << c"), "(a & (b << c))");
    }

    #[test]
    fn operands_are_required() {
        for text in ["const X = a &", "const X = a ^", "const X = | a"] {
            assert_eq!(parse(text).codes, ["P2004"], "{text}");
        }
    }
}

/// `CompareExpr = BitOrExpr , [ CompareOp , BitOrExpr ]`,
/// `CompareOp = "==" | "!=" | "<" | "<=" | ">" | ">="`
mod compare_expr {
    use super::*;

    #[test]
    fn every_comparison_operator() {
        for op in ["==", "!=", "<", "<=", ">", ">="] {
            assert_eq!(shape(&format!("a {op} b")), format!("(a {op} b)"));
        }
    }

    #[test]
    fn bitwise_operators_bind_more_tightly_than_comparisons() {
        assert_eq!(shape("flags & MASK == 0"), "((flags & MASK) == 0)");
        assert_eq!(shape("a | b < c"), "((a | b) < c)");
    }

    #[test]
    fn comparisons_cannot_be_chained() {
        for text in ["a < b < c", "a == b != c", "a <= b > c"] {
            let parsed = parse(&format!("const X = {text}"));
            assert_eq!(parsed.codes, ["P2009"], "{text}");
            assert_eq!(parsed.messages, ["comparison operators cannot be chained"]);
        }
        let parsed = parse("const X = a < b < c");
        assert_eq!(parsed.labelled, ["<"]);
        assert_eq!(
            parsed.notes,
            [[
                "comparisons are non-associative: `a < b < c` does not mean `a < b && b < c`, \
                 so write each comparison separately"
            ]]
        );
        // Parentheses make the grouping explicit and are accepted.
        assert_eq!(shape("(a < b) == c"), "((paren (a < b)) == c)");
    }
}

/// `AndExpr = CompareExpr , { "&&" , CompareExpr }`,
/// `OrExpr = AndExpr , { "||" , AndExpr }`
mod logical_expr {
    use super::*;

    #[test]
    fn and_binds_more_tightly_than_or() {
        assert_eq!(shape("a || b && c"), "(a || (b && c))");
        assert_eq!(shape("a && b || c && d"), "((a && b) || (c && d))");
    }

    #[test]
    fn comparisons_bind_more_tightly_than_and() {
        assert_eq!(shape("a < b && c == d"), "((a < b) && (c == d))");
    }

    #[test]
    fn long_conditions_are_one_chain_per_level() {
        assert_eq!(shape("a && b && c && d"), "(a && b && c && d)");
        // A condition continues on the next line after an operator.
        assert_eq!(shape("a ||\n    b"), "(a || b)");
    }

    #[test]
    fn every_level_in_one_expression() {
        assert_eq!(
            shape("a || b && c == d | e ^ f & g << h + i * j"),
            "(a || (b && (c == (d | (e ^ (f & (g << (h + (i * j)))))))))"
        );
    }
}

/// `CastExpr = UnaryExpr , { "as" , Type }`
mod cast_expr {
    use super::*;

    #[test]
    fn converts_to_a_type() {
        assert_eq!(shape("x as i32"), "(as x i32)");
        assert_eq!(shape("x as f64 as f32"), "(as (as x f64) f32)");
        assert_eq!(shape("x as ()"), "(as x ())");
    }

    #[test]
    fn binds_between_prefix_and_multiplicative_operators() {
        assert_eq!(shape("-x as i32"), "(as (- x) i32)");
        assert_eq!(shape("x as int * 2"), "((as x int) * 2)");
        assert_eq!(shape("a + b as int"), "(a + (as b int))");
        assert_eq!(shape("(a + b) as int"), "(as (paren (a + b)) int)");
    }

    #[test]
    fn the_right_operand_is_a_type() {
        assert_eq!(shape("x as\n    u8"), "(as x u8)");
        for (text, message) in [
            ("const X = x as", "expected type, found end of file"),
            ("const X = x as 1", "expected type, found integer literal"),
        ] {
            assert_eq!(parse(text).messages, [message], "{text}");
        }
    }
}

/// `PostfixExpr = PrimaryExpr , { CallSuffix | … }`,
/// `CallSuffix = "(" , [ ArgList ] , ")"`, `ArgList = Expr , { "," , Expr } , [ "," ]`
mod call_suffix {
    use super::*;

    #[test]
    fn calls_with_any_number_of_arguments() {
        assert_eq!(shape("f()"), "(call f)");
        assert_eq!(shape("f(a, b + c)"), "(call f a (b + c))");
        assert_eq!(shape("f(a,)"), "(call f a)");
        assert_eq!(shape("f(g(x))"), "(call f (call g x))");
        assert_eq!(shape("f()()"), "(call (call f))");
        assert_eq!(shape("(f)(x)"), "(call (paren f) x)");
    }

    #[test]
    fn arguments_may_span_lines() {
        assert_eq!(shape("f(\n    a,\n    b,\n)"), "(call f a b)");
    }

    #[test]
    fn calls_bind_more_tightly_than_prefix_operators_and_conversions() {
        assert_eq!(shape("-f(x)"), "(- (call f x))");
        assert_eq!(shape("f(x) as int"), "(as (call f x) int)");
    }

    #[test]
    fn malformed_argument_lists() {
        for (text, code, message) in [
            (
                "const X = f(",
                "P2004",
                "expected expression, found end of file",
            ),
            (
                "const X = f(a",
                "P2008",
                "expected `)` to close `(`, found end of file",
            ),
            (
                "const X = f(a b)",
                "P2008",
                "expected `)` to close `(`, found identifier `b`",
            ),
            ("const X = f(,)", "P2004", "expected expression, found `,`"),
            (
                "const X = f(a,,)",
                "P2004",
                "expected expression, found `,`",
            ),
        ] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, [code], "{text}");
            assert_eq!(parsed.messages, [message], "{text}");
        }
    }
}

/// `MemberSuffix = "." , IDENT`
mod member_suffix {
    use super::*;

    #[test]
    fn member_accesses_chain_with_calls() {
        assert_eq!(shape("io.println"), "(. io println)");
        assert_eq!(shape("a.b.c"), "(. (. a b) c)");
        assert_eq!(shape("io.println(\"hi\")"), "(call (. io println) \"hi\")");
        assert_eq!(shape("f().x"), "(. (call f) x)");
        // Syntactically any expression has members; `1.x` is not a float.
        assert_eq!(shape("1.x"), "(. 1 x)");
    }

    #[test]
    fn lines_starting_with_a_dot_continue_the_expression() {
        assert_eq!(shape("io\n    .println"), "(. io println)");
        assert_eq!(shape("a\n\n// comment\n.b"), "(. a b)");
        // Only `.` continues a line: `(x)` on the next line is not a call.
        assert_eq!(parse("const X = io\n.println\n(x)").labelled, ["("]);
    }

    #[test]
    fn the_member_name_is_an_identifier() {
        for (text, message) in [
            ("const X = a.", "expected identifier, found end of file"),
            (
                "const X = a.1",
                "expected identifier, found integer literal",
            ),
            ("const X = a.(b)", "expected identifier, found `(`"),
        ] {
            assert_eq!(parse(text).messages, [message], "{text}");
        }
    }
}

/// Tokens the lexer recognises for planned features (OPERATORS.md,
/// FEATURE_MATRIX.md) are reported as unsupported, never accepted.
mod planned_syntax {
    use super::*;

    #[test]
    fn planned_operators_after_an_operand() {
        for (text, what) in [
            ("a ** b", "the power operator `**`"),
            ("x |> f", "the pipeline operator `|>`"),
            ("a ?? b", "the coalescing operator `??`"),
            ("a?.b", "optional member access `?.`"),
            ("f()?", "`?`"),
            ("a[0]", "indexing and array syntax `[`"),
            ("a..b", "range syntax"),
            ("a..=b", "range syntax"),
            ("a => b", "`=>`"),
        ] {
            let parsed = parse(&format!("const X = {text}"));
            assert_eq!(parsed.codes, ["P2010"], "{text}");
            assert_eq!(
                parsed.messages,
                [format!("{what} is not supported in Pinguer v0.1")],
                "{text}"
            );
        }
    }

    #[test]
    fn planned_syntax_where_an_expression_starts() {
        for (text, labelled) in [("[1, 2]", "["), ("@inline", "@"), ("..5", "..")] {
            let parsed = parse(&format!("const X = {text}"));
            assert_eq!(parsed.codes, ["P2010"], "{text}");
            assert_eq!(parsed.labelled, [labelled], "{text}");
        }
    }

    #[test]
    fn planned_operators_inside_v0_1_expressions() {
        let parsed = parse("const X = f(a, 2 ** 3)");
        assert_eq!(parsed.codes, ["P2010"]);
        assert_eq!(parsed.labelled, ["**"]);
        assert_eq!(
            parsed.notes,
            [["it is planned for a later version; see specifications/FEATURE_MATRIX.md"]]
        );
    }
}

/// `FuncDecl = "func" , IDENT , "(" , [ ParamList ] , ")" , [ "->" , Type ] , Block`
mod func_decl {
    use super::*;

    #[test]
    fn hello_world() {
        let expected = concat!(
            "  Import #0 @1:1-1:14 std.io\n",
            "  Func #1 @3:1-5:2 main\n",
            "    Block #2 @3:13-5:2\n",
            "      ExprStmt #3 @4:5-4:24\n",
            "        Call #4 @4:5-4:24\n",
            "          Member #5 @4:5-4:15 .println\n",
            "            Name #6 @4:5-4:7 io\n",
            "          String #7 @4:16-4:23 \"Hello\"\n",
        );
        let text = "import std.io\n\nfunc main() {\n    io.println(\"Hello\")\n}\n";
        assert_eq!(tree(text), expected);
    }

    #[test]
    fn parameters_and_return_type() {
        let expected = [
            "  Func #0 @1:1-1:38 add",
            "    Param #1 @1:10-1:16 a",
            "      Type #2 @1:13-1:16 int",
            "    Param #3 @1:18-1:24 b",
            "      Type #4 @1:21-1:24 int",
            "    Type #5 @1:29-1:32 int",
            "    Block #6 @1:33-1:38",
            "      ExprStmt #7 @1:35-1:36",
            "        Name #8 @1:35-1:36 a",
        ];
        let tree = tree("func add(a: int, b: int) -> int { a }");
        assert_eq!(tree.lines().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn signatures_may_span_lines() {
        let text = "func f(\n    a: int,\n    b: int,\n) ->\n    int {\n}";
        let tree = tree(text);
        assert_eq!(tree.matches("Param").count(), 2, "{tree}");
        assert!(tree.contains("    Type #5 @5:5-5:8 int\n"), "{tree}");
    }

    #[test]
    fn malformed_signatures() {
        for (text, code, message) in [
            ("func", "P2002", "expected identifier, found end of file"),
            ("func f {}", "P2005", "expected `(`, found `{`"),
            ("func f()", "P2005", "expected `{`, found end of file"),
            ("func f() -> {}", "P2007", "expected type, found `{`"),
            (
                "func f(a) {}",
                "P2011",
                "expected `:` and the parameter's type, found `)`",
            ),
            ("func f(a: ) {}", "P2007", "expected type, found `)`"),
            ("func f(,) {}", "P2002", "expected identifier, found `,`"),
            (
                "func f(a: int b: int) {}",
                "P2008",
                "expected `)` to close `(`, found identifier `b`",
            ),
        ] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, [code], "{text}");
            assert_eq!(parsed.messages, [message], "{text}");
        }
    }
}

/// `Block = "{" , { Separator } , [ Statement , { Separator , { Separator } ,
/// Statement } , { Separator } ] , "}"`
mod block {
    use super::*;

    #[test]
    fn statements_are_separated_by_line_breaks_or_semicolons() {
        let body = tree("func f() {\n\n    a(); b()\n    c()\n\n}");
        assert_eq!(body.matches("ExprStmt").count(), 3, "{body}");
        assert!(tree("func f() {}").ends_with("    Block #1 @1:10-1:12\n"));
        assert!(tree("func f() { ; ; }").ends_with("    Block #1 @1:10-1:17\n"));
    }

    #[test]
    fn statements_on_one_line_need_a_semicolon() {
        let parsed = parse("func f() { a() b() }");
        assert_eq!(parsed.codes, ["P2003"]);
        assert_eq!(
            parsed.messages,
            ["expected a line break or `;` after the statement, found identifier `b`"]
        );
    }

    #[test]
    fn an_unclosed_block_points_at_its_brace() {
        let parsed = parse("func f() {\n    a()\n");
        assert_eq!(parsed.codes, ["P2008"]);
        assert_eq!(
            parsed.messages,
            ["expected `}` to close `{`, found end of file"]
        );
    }
}
