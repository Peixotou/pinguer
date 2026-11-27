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
        let parsed = parse("\nreturn 1");
        assert_eq!(parsed.codes, ["P2001"]);
        assert_eq!(
            parsed.messages,
            ["expected `import`, `func` or `const`, found `return`"]
        );
        assert_eq!(parsed.labelled, ["return"]);
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

/// `ExprStatement = PostfixExpr`
mod expr_statement {
    use super::*;

    #[test]
    fn postfix_expressions_are_statements() {
        // Only calls are meaningful, but that is checked by semantic
        // analysis: syntactically these are all expression statements.
        for text in ["f()", "io.println(x)", "x", "1", "(f)()", "io.println"] {
            let body = tree(&format!("func f() {{ {text} }}"));
            assert!(body.contains("ExprStmt"), "{text}: {body}");
        }
    }

    #[test]
    fn other_expressions_are_not_statements() {
        for text in ["a + b", "-x", "!f()", "x as int", "a == b"] {
            let parsed = parse(&format!("func f() {{ {text} }}"));
            assert_eq!(parsed.codes, ["P2012"], "{text}");
            assert_eq!(parsed.labelled, [text], "{text}");
        }
    }

    #[test]
    fn a_line_starting_with_minus_does_not_continue_the_previous_one() {
        // ADR-0005: `-x` on its own line is a separate, invalid statement.
        let parsed = parse("func f() {\n    g()\n    -x\n}");
        assert_eq!(parsed.codes, ["P2012"]);
        assert_eq!(parsed.labelled, ["-x"]);
    }
}

/// `LetDecl = "let" , IDENT , [ ":" , Type ] , "=" , Expr`,
/// `VarDecl = "var" , …`, and `ConstDecl` inside blocks
mod local_bindings {
    use super::*;

    /// The rendered statements of `func f() { <body> }`.
    fn stmts(body: &str) -> String {
        let rendered = tree(&format!("func f() {{\n{body}\n}}"));
        rendered
            .lines()
            .skip(2)
            .map(|line| format!("{}\n", &line[6..]))
            .collect()
    }

    #[test]
    fn let_var_and_const_declare_bindings() {
        let expected = concat!(
            "Let #2 @2:1-2:10 x\n",
            "  Int #3 @2:9-2:10 1 decimal\n",
            "Var #4 @3:1-3:15 y\n",
            "  Type #5 @3:8-3:11 int\n",
            "  Name #6 @3:14-3:15 x\n",
            "Const #7 @4:1-4:15 Z\n",
            "  Bool #8 @4:11-4:15 true\n",
        );
        assert_eq!(stmts("let x = 1\nvar y: int = x\nconst Z = true"), expected);
    }

    #[test]
    fn every_binding_needs_an_initializer() {
        for text in ["func f() { let x }", "func f() {\n    var y: int\n}"] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, ["P2006"], "{text}");
        }
        assert_eq!(
            parse("func f() { var y: int }").messages,
            ["`var` declaration has no initializer"]
        );
    }

    #[test]
    fn malformed_bindings() {
        for (text, code) in [
            ("func f() { let = 1 }", "P2002"),
            ("func f() { let x: = 1 }", "P2007"),
            ("func f() { let x 1 }", "P2005"),
            ("func f() { let x = }", "P2004"),
        ] {
            assert_eq!(parse(text).codes, [code], "{text}");
        }
    }
}

/// `Assignment = IDENT , AssignOp , Expr`
mod assignment {
    use super::*;

    #[test]
    fn every_assignment_operator() {
        for op in [
            "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=",
        ] {
            let body = tree(&format!("func f() {{ x {op} 1 }}"));
            assert!(
                body.contains(&format!(
                    "      Assign #2 @1:12-1:{} x {op}\n",
                    16 + op.len()
                )),
                "{op}: {body}"
            );
        }
    }

    #[test]
    fn the_value_is_a_full_expression_and_may_continue_on_the_next_line() {
        let body = tree("func f() {\n    total +=\n        a * b\n}");
        assert!(
            body.contains("      Assign #2 @2:5-3:14 total +=\n"),
            "{body}"
        );
        assert!(body.contains("        Binary #3 @3:9-3:14\n"), "{body}");
    }

    #[test]
    fn only_variables_can_be_assigned() {
        for target in ["a.b", "f()", "(x)", "-x", "a + b", "1"] {
            let parsed = parse(&format!("func f() {{ {target} = 1 }}"));
            assert_eq!(parsed.codes, ["P2013"], "{target}");
            assert_eq!(parsed.labelled, [target], "{target}");
        }
    }

    #[test]
    fn assignments_are_statements_not_expressions() {
        let parsed = parse("func f() { x = y = 1 }");
        assert_eq!(parsed.codes, ["P2003"]);
        assert_eq!(
            parsed.notes[0][1],
            "assignment is a statement, not an expression, so assignments cannot be chained"
        );
        assert_eq!(parse("func f() { x = }").codes, ["P2004"]);
        assert_eq!(parse("func f() { x **= 2 }").codes, ["P2010"]);
    }
}

/// `ReturnStatement = "return" , [ Expr ]`
mod return_statement {
    use super::*;

    #[test]
    fn returns_a_value() {
        let body = tree("func f() -> int { return a + 1 }");
        assert!(
            body.contains("      Return #3 @1:19-1:31\n        Binary #4"),
            "{body}"
        );
    }

    #[test]
    fn returns_nothing_before_a_separator_brace_or_end() {
        for text in [
            "func f() { return }",
            "func f() { return; g() }",
            "func f() {\n    return\n}",
        ] {
            let body = tree(text);
            let line = body.lines().find(|line| line.contains("Return")).unwrap();
            let next = body.lines().skip_while(|l| !l.contains("Return")).nth(1);
            assert!(
                next.is_none_or(|next| !next.starts_with("        ")),
                "{text}: {line}\n{body}"
            );
        }
    }

    #[test]
    fn a_value_on_the_next_line_is_a_separate_statement() {
        let body = tree("func f() {\n    return\n    g()\n}");
        assert_eq!(body.matches("Return").count(), 1);
        assert!(body.contains("      ExprStmt #3 @3:5-3:8\n"), "{body}");
    }
}

/// `Statement = … | "break" | "continue" | …`
mod loop_control {
    use super::*;

    #[test]
    fn break_and_continue_are_statements() {
        let body = tree("func f() {\n    break\n    continue\n}");
        assert!(body.contains("      Break #2 @2:5-2:10\n"), "{body}");
        assert!(body.contains("      Continue #3 @3:5-3:13\n"), "{body}");
    }

    #[test]
    fn they_take_no_operand() {
        // Labels and values are not part of v0.1.
        assert_eq!(parse("func f() { break x }").codes, ["P2003"]);
    }
}

/// `WhileStatement = "while" , Expr , Block`
mod while_statement {
    use super::*;

    #[test]
    fn a_condition_and_a_body() {
        let expected = concat!(
            "      While #2 @2:5-4:6\n",
            "        Binary #3 @2:11-2:17\n",
            "          Name #4 @2:11-2:12 i\n",
            "          Op @2:13-2:14 <\n",
            "          Int #5 @2:15-2:17 10 decimal\n",
            "        Block #6 @2:18-4:6\n",
            "          Assign #7 @3:9-3:15 i +=\n",
            "            Int #8 @3:14-3:15 1 decimal\n",
        );
        let body = tree("func f() {\n    while i < 10 {\n        i += 1\n    }\n}");
        assert!(body.contains(expected), "{body}");
    }

    #[test]
    fn needs_a_condition_and_a_block() {
        for (text, message) in [
            ("func f() { while {} }", "expected expression, found `{`"),
            ("func f() { while x }", "expected `{`, found `}`"),
            (
                "func f() { while x y }",
                "expected `{`, found identifier `y`",
            ),
        ] {
            assert_eq!(parse(text).messages, [message], "{text}");
        }
    }
}

/// `IfStatement = "if" , Expr , Block , [ "else" , ( IfStatement | Block ) ]`
mod if_statement {
    use super::*;

    #[test]
    fn if_else_if_else_is_one_node() {
        let text = "func f() {\n    if a {\n    } else if b {\n        g()\n    } else {\n    }\n}";
        let expected = concat!(
            "      If #2 @2:5-6:6\n",
            "        Branch @2:5-3:6\n",
            "          Name #3 @2:8-2:9 a\n",
            "          Block #4 @2:10-3:6\n",
            "        Branch @3:12-5:6\n",
            "          Name #5 @3:15-3:16 b\n",
            "          Block #6 @3:17-5:6\n",
            "            ExprStmt #7 @4:9-4:12\n",
            "              Call #8 @4:9-4:12\n",
            "                Name #9 @4:9-4:10 g\n",
            "        Else @5:12-6:6\n",
            "          Block #10 @5:12-6:6\n",
        );
        let body = tree(text);
        assert!(body.contains(expected), "{body}");
    }

    #[test]
    fn without_else() {
        let body = tree("func f() { if a {} }");
        assert_eq!(body.matches("Branch").count(), 1);
        assert!(!body.contains("Else"));
    }

    #[test]
    fn long_else_if_chains_do_not_nest() {
        let mut text = String::from("func f() {\n    if c0 {\n    }");
        for index in 1..5_000 {
            text.push_str(&format!(" else if c{index} {{\n    }}"));
        }
        text.push_str("\n}");
        let body = tree(&text);
        assert_eq!(body.matches("Branch").count(), 5_000);
        // Every branch is at the same depth.
        assert!(
            body.lines()
                .filter(|l| l.contains("Branch"))
                .all(|l| l.starts_with("        Branch"))
        );
    }

    #[test]
    fn malformed_conditionals() {
        for (text, message) in [
            ("func f() { if {} }", "expected expression, found `{`"),
            ("func f() { if a }", "expected `{`, found `}`"),
            ("func f() { if a {} else }", "expected `{`, found `}`"),
            (
                "func f() { if a {} else b {} }",
                "expected `{`, found identifier `b`",
            ),
        ] {
            assert_eq!(parse(text).messages, [message], "{text}");
        }
    }
}

/// `Statement = … | Block`
mod block_statement {
    use super::*;

    #[test]
    fn blocks_nest_as_statements() {
        let expected = concat!(
            "      BlockStmt #2 @1:12-1:23\n",
            "        Block #3 @1:12-1:23\n",
            "          BlockStmt #4 @1:14-1:21\n",
            "            Block #5 @1:14-1:21\n",
            "              ExprStmt #6 @1:16-1:19\n",
        );
        let body = tree("func f() { { { g() } } }");
        assert!(body.contains(expected), "{body}");
    }

    #[test]
    fn an_unclosed_inner_block_is_reported_at_its_brace() {
        let parsed = parse("func f() {\n    {\n        g()\n}");
        assert_eq!(parsed.codes, ["P2008"]);
        assert_eq!(
            parsed.messages,
            ["expected `}` to close `{`, found end of file"]
        );
    }
}

/// Layout rule of GRAMMAR.ebnf: `{` must be on the same line as the
/// construct that introduces it.
mod brace_placement {
    use super::*;

    #[test]
    fn a_brace_on_the_next_line_is_reported_once() {
        for text in [
            "func f()\n{\n}",
            "func f() {\n    if a\n    {\n    }\n}",
            "func f() {\n    while a\n    {\n    }\n}",
            "func f() {\n    if a {\n    } else\n    {\n    }\n}",
        ] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, ["P2014"], "{text}");
            assert_eq!(parsed.labelled, ["{"], "{text}");
            assert_eq!(
                parsed.messages,
                ["`{` must be on the same line as the code it belongs to"]
            );
        }
    }

    #[test]
    fn the_construct_is_still_parsed() {
        let parsed = parse("func f()\n{\n    g()\n}");
        assert!(parsed.tree.contains("ExprStmt"), "{}", parsed.tree);
    }
}

/// Layout rule of GRAMMAR.ebnf: `else` must be on the same line as the
/// preceding `}`.
mod else_placement {
    use super::*;

    #[test]
    fn else_on_the_next_line_is_reported_once() {
        for text in [
            "func f() {\n    if a {\n    }\n    else {\n    }\n}",
            "func f() {\n    if a {\n    }\n\n    // comment\n    else if b {\n    }\n}",
        ] {
            let parsed = parse(text);
            assert_eq!(parsed.codes, ["P2015"], "{text}");
            assert_eq!(parsed.labelled, ["else"], "{text}");
            // The `else` still belongs to the `if`.
            assert!(parsed.tree.contains("Else") || parsed.tree.matches("Branch").count() == 2);
        }
    }
}

/// Statement-level recovery (ADR-0029).
mod statement_recovery {
    use super::*;

    #[test]
    fn each_broken_statement_is_reported_once_and_the_rest_parses() {
        let text = "func f() {\n    let = 1\n    a + b = 1\n    h()\n    -x\n    m()\n}";
        let parsed = parse(text);
        assert_eq!(
            parsed.codes,
            ["P2002", "P2013", "P2012"],
            "{:?}",
            parsed.messages
        );
        assert_eq!(
            parsed.tree.matches("ExprStmt").count(),
            2,
            "{}",
            parsed.tree
        );
        assert_eq!(
            parsed.tree.matches("ErrorStmt").count(),
            3,
            "{}",
            parsed.tree
        );
    }

    #[test]
    fn an_unclosed_parenthesis_extends_the_statement_to_the_closing_brace() {
        // Inside `(` the lexer produces no line breaks until the enclosing
        // `}` closes it, so the whole rest of the block is one statement.
        let parsed = parse("func f() {\n    g(\n    h()\n}\nfunc k() {\n}");
        assert_eq!(parsed.codes, ["P2008"]);
        assert!(
            parsed.tree.contains("Func #") && parsed.tree.contains(" k\n"),
            "{}",
            parsed.tree
        );
    }

    #[test]
    fn nested_blocks_inside_a_broken_statement_are_skipped_whole() {
        let parsed = parse("func f() {\n    if a + { g()\n        h() }\n    k()\n}");
        assert_eq!(parsed.codes, ["P2004"]);
        assert!(
            parsed.tree.contains("Name #") && parsed.tree.contains(" k\n"),
            "{}",
            parsed.tree
        );
    }

    #[test]
    fn a_missing_separator_drops_the_rest_of_the_line_only() {
        let parsed = parse("func f() {\n    a() b() c()\n    d()\n}");
        assert_eq!(parsed.codes, ["P2003"]);
        assert_eq!(
            parsed.tree.matches("ExprStmt").count(),
            2,
            "{}",
            parsed.tree
        );
    }

    #[test]
    fn errors_in_later_functions_are_still_found() {
        let parsed = parse("func f() {\n    -x\n}\n\nfunc g() {\n    a ** b\n}");
        assert_eq!(parsed.codes, ["P2012", "P2010"]);
    }
}

mod item_recovery {
    use super::*;

    /// Returns the names of the functions and constants in the tree.
    fn names(tree: &str) -> Vec<&str> {
        tree.lines()
            .filter(|line| line.starts_with("  Func ") || line.starts_with("  Const "))
            .map(|line| line.rsplit(' ').next().unwrap())
            .collect()
    }

    #[test]
    fn each_broken_item_is_reported_once_and_the_rest_parses() {
        let text = "import\nconst = 1\nfunc f() -> {\n}\nfunc g() {\n    h()\n}\nconst X = 1";
        let parsed = parse(text);
        assert_eq!(
            parsed.codes,
            ["P2002", "P2002", "P2007"],
            "{:?}",
            parsed.messages
        );
        assert_eq!(names(&parsed.tree), ["g", "X"], "{}", parsed.tree);
        assert_eq!(
            parsed.tree.matches("ErrorItem").count(),
            3,
            "{}",
            parsed.tree
        );
    }

    #[test]
    fn a_function_body_is_skipped_whole() {
        // `const` starts a line inside the skipped body, at depth 1.
        let parsed = parse("func f() x {\n    const Y = 1\n}\nfunc g() {\n}");
        assert_eq!(parsed.codes, ["P2005"]);
        assert_eq!(names(&parsed.tree), ["g"], "{}", parsed.tree);
    }

    #[test]
    fn an_unclosed_parenthesis_ends_at_the_next_item() {
        // The lexer produces no line breaks inside `(`, so line starts are
        // found in the source text.
        let parsed = parse("const X = (1\n  \tfunc g() {\n}");
        assert_eq!(parsed.codes, ["P2008"]);
        assert_eq!(names(&parsed.tree), ["g"], "{}", parsed.tree);
    }

    #[test]
    fn an_item_keyword_in_the_middle_of_a_line_does_not_end_recovery() {
        let parsed = parse("func f( import a\nimport b");
        assert_eq!(parsed.codes, ["P2002"]);
        assert!(parsed.tree.contains("Import #"), "{}", parsed.tree);
        assert_eq!(parsed.tree.matches("Import #").count(), 1);
    }

    #[test]
    fn a_missing_separator_skips_to_the_next_line() {
        let parsed = parse("func f() {} x y\nconst Z = 2");
        assert_eq!(parsed.codes, ["P2003"]);
        assert_eq!(names(&parsed.tree), ["f", "Z"], "{}", parsed.tree);
    }

    #[test]
    fn tokens_that_cannot_start_an_item_are_skipped() {
        let parsed = parse("x + 1\n)\nfunc g() {\n}");
        assert_eq!(parsed.codes, ["P2001"]);
        assert_eq!(names(&parsed.tree), ["g"], "{}", parsed.tree);
    }
}

mod unknown_token_recovery {
    use super::*;

    #[test]
    fn an_unknown_token_is_an_error_expression() {
        let parsed = parse("const X = $\nconst Y = 1 + $");
        assert_eq!(parsed.codes, ["P1001", "P1001"]);
        assert_eq!(
            parsed.tree.matches("ErrorExpr").count(),
            2,
            "{}",
            parsed.tree
        );
    }

    #[test]
    fn errors_at_an_unknown_token_are_not_reported_again() {
        let text = "$ func\nfunc f() {\n    let $ = 1\n    a $ b\n    g()\n}\nfunc h($) {\n}";
        let parsed = parse(text);
        assert_eq!(parsed.codes, ["P1001"; 4], "{:?}", parsed.messages);
        for (node, count) in [("ErrorItem", 2), ("ErrorStmt", 1), ("ExprStmt", 2)] {
            assert_eq!(parsed.tree.matches(node).count(), count, "{}", parsed.tree);
        }
    }

    #[test]
    fn errors_about_other_code_are_still_reported() {
        let parsed = parse("func f() {\n    -x $\n}");
        assert_eq!(parsed.codes, ["P1001", "P2012"]);
    }
}

mod unexpected_closing_delimiter {
    use super::*;

    #[test]
    fn a_closer_where_an_item_starts() {
        let parsed = parse("}\n)\nconst X = 1");
        assert_eq!(parsed.codes, ["P2016"]);
        assert_eq!(parsed.messages, ["unexpected closing delimiter `}`"]);
        assert_eq!(parsed.labelled, ["}"]);
        assert!(parsed.tree.contains("Const #"), "{}", parsed.tree);
    }

    #[test]
    fn a_closer_after_an_item() {
        let parsed = parse("func f() {\n}}\nconst X = (1))");
        assert_eq!(parsed.codes, ["P2016", "P2016"]);
        assert_eq!(parsed.labelled, ["}", ")"]);
        assert!(parsed.tree.contains("Const #"), "{}", parsed.tree);
    }

    #[test]
    fn closers_in_a_block_are_reported_and_skipped() {
        let parsed = parse("func f() {\n    g())\n    ]\n    h()\n}");
        assert_eq!(parsed.codes, ["P2016", "P2016"]);
        assert_eq!(parsed.labelled, [")", "]"]);
        assert_eq!(
            parsed.tree.matches("ExprStmt").count(),
            2,
            "{}",
            parsed.tree
        );
    }
}

mod reserved_keyword {
    use super::*;

    /// The keywords that LEXICAL_STRUCTURE.md §5.2 reserves for future use.
    const RESERVED: [&str; 27] = [
        "actor",
        "async",
        "await",
        "class",
        "comptime",
        "defer",
        "enum",
        "extern",
        "for",
        "impl",
        "in",
        "interface",
        "loop",
        "match",
        "move",
        "private",
        "public",
        "self",
        "Self",
        "spawn",
        "static",
        "struct",
        "try",
        "type",
        "unsafe",
        "where",
        "yield",
    ];

    #[test]
    fn reserved_keywords_are_not_names() {
        for keyword in RESERVED {
            let parsed = parse(&format!("const {keyword} = 1"));
            assert_eq!(parsed.codes, ["P2017"], "{keyword}");
            assert_eq!(
                parsed.messages,
                [format!(
                    "`{keyword}` is reserved for a future version of Pinguer"
                )]
            );
            assert_eq!(parsed.labelled, [keyword]);
        }
    }

    #[test]
    fn reserved_keywords_start_no_item_statement_type_or_expression() {
        for text in [
            "struct S {\n    x: int\n}",
            "func f() {\n    for x in y {\n        g()\n    }\n}",
            "func f(x: type) {\n}",
            "const X = 1 + await",
            "func f() {\n    let x = 1 in y\n}",
        ] {
            assert_eq!(parse(text).codes, ["P2017"], "{text:?}");
        }
    }
}

mod misplaced_declaration {
    use super::*;

    #[test]
    fn local_bindings_are_not_items() {
        for keyword in ["let", "var"] {
            let parsed = parse(&format!("{keyword} x = 1\nconst Y = 2"));
            assert_eq!(parsed.codes, ["P2018"], "{keyword}");
            assert_eq!(
                parsed.messages,
                [format!(
                    "`{keyword}` declarations are only allowed inside a function"
                )]
            );
            assert_eq!(parsed.labelled, [keyword]);
            assert!(parsed.tree.contains("Const #"), "{}", parsed.tree);
        }
    }

    #[test]
    fn functions_and_imports_are_not_statements() {
        let text = "func f() {\n    import a\n    func g() {\n        h()\n    }\n    k()\n}";
        let parsed = parse(text);
        assert_eq!(parsed.codes, ["P2018", "P2018"]);
        assert_eq!(
            parsed.messages,
            [
                "`import` declarations are only allowed at the top level of a file",
                "`func` declarations are only allowed at the top level of a file",
            ]
        );
        // The nested function is skipped whole; `k()` is still parsed.
        assert_eq!(
            parsed.tree.matches("ExprStmt").count(),
            1,
            "{}",
            parsed.tree
        );
    }

    #[test]
    fn a_missing_brace_is_reported_with_the_declaration_it_misplaces() {
        let parsed = parse("func f() {\n    a()\n\nfunc g() {\n}\n");
        assert_eq!(parsed.codes, ["P2018", "P2008"]);
        assert_eq!(parsed.labelled, ["func", ""]);
    }
}

mod missing_closing_delimiter {
    use super::*;

    #[test]
    fn a_construct_before_a_closing_brace_is_kept() {
        let parsed = parse("func f() {\n    g(a, (b\n}");
        assert_eq!(parsed.codes, ["P2008", "P2008"]);
        assert_eq!(parsed.labelled, ["}", "}"]);
        assert!(parsed.tree.contains("Call #"), "{}", parsed.tree);
        assert!(!parsed.tree.contains("Error"), "{}", parsed.tree);
    }

    #[test]
    fn constructs_at_the_end_of_the_file_are_kept() {
        let parsed = parse("func f() {\n    if x {\n        g()\n");
        assert_eq!(parsed.codes, ["P2008", "P2008"]);
        assert_eq!(parsed.labelled, ["", ""]);
        assert!(parsed.tree.contains("If #"), "{}", parsed.tree);
        assert!(!parsed.tree.contains("Error"), "{}", parsed.tree);
    }

    #[test]
    fn other_tokens_make_the_construct_an_error() {
        let parsed = parse("const X = (1 2)\nconst Y = 3");
        assert_eq!(parsed.codes, ["P2008"]);
        assert_eq!(parsed.labelled, ["2"]);
        assert!(parsed.tree.contains("ErrorItem"), "{}", parsed.tree);
        assert!(parsed.tree.contains("Const #"), "{}", parsed.tree);
    }
}

mod error_node_spans {
    use super::*;

    #[test]
    fn error_nodes_cover_the_skipped_tokens() {
        let parsed = parse("func f() {\n    let = 1 + 2\n    g()\n}");
        assert!(
            parsed.tree.contains("      ErrorStmt #2 @2:5-2:16\n"),
            "{}",
            parsed.tree
        );
        // Line breaks after the last skipped token are not covered.
        let parsed = parse("const = (1,\n  2)\n\nfunc g() {\n}");
        assert!(
            parsed.tree.starts_with("  ErrorItem #0 @1:1-2:5\n"),
            "{}",
            parsed.tree
        );
    }
}
