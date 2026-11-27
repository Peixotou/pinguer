//! The nesting limit of ADR-0030.

use pinguer_diagnostics::Diagnostics;
use pinguer_parser::MAX_NESTING;
use pinguer_source::SourceMap;

/// Lexes and parses `text`, and returns the codes of its diagnostics and
/// the source text that each one's primary label covers.
fn diagnose(text: &str) -> (Vec<String>, Vec<String>) {
    let mut map = SourceMap::new();
    let file = map.add("limits.pgr", text).unwrap();
    let mut diagnostics = Diagnostics::new();
    let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
    let tree = pinguer_parser::parse(file, text, tokens, &mut diagnostics);
    drop(tree);
    diagnostics
        .iter()
        .map(|d| {
            let code = d.code.map_or_else(String::new, |code| code.to_string());
            let labelled = map.snippet(d.main_label().unwrap().span).to_owned();
            (code, labelled)
        })
        .unzip()
}

/// Source texts with `levels` counted levels of one kind of construct.
fn nested(kind: &str, levels: usize) -> String {
    match kind {
        "parentheses" => format!("const X = {}1{}", "(".repeat(levels), ")".repeat(levels)),
        "prefix operators" => format!("const X = {}1", "- ".repeat(levels)),
        "calls" => format!("const X = f{}", "()".repeat(levels)),
        // Arguments are parsed by recursion, unlike a chain of calls.
        "nested calls" => format!("const X = {}1{}", "f(".repeat(levels), ")".repeat(levels)),
        "member accesses" => format!("const X = a{}", ".b".repeat(levels)),
        "conversions" => format!("const X = x{}", " as int".repeat(levels)),
        // The function body is the first block.
        "blocks" => format!(
            "func f() {{{}{}}}",
            "{".repeat(levels - 1),
            "}".repeat(levels - 1)
        ),
        "conditionals" => format!(
            "func f() {{{}{}}}",
            "if x {".repeat(levels - 1),
            "} else {}".repeat(levels - 1)
        ),
        "loops" => format!(
            "func f() {{{}{}}}",
            "while x {".repeat(levels - 1),
            "}".repeat(levels - 1)
        ),
        // Each `(1 + …)` is a parenthesized expression containing a chain.
        "operator chains" => {
            let pairs = levels / 2;
            let text = format!("{}1{}", "(1 + ".repeat(pairs), ")".repeat(pairs));
            let text = if levels % 2 == 1 {
                format!("({text})")
            } else {
                text
            };
            format!("const X = {text}")
        }
        _ => unreachable!("{kind}"),
    }
}

const KINDS: [&str; 10] = [
    "parentheses",
    "prefix operators",
    "calls",
    "nested calls",
    "member accesses",
    "conversions",
    "blocks",
    "conditionals",
    "loops",
    "operator chains",
];

#[test]
fn every_construct_is_accepted_at_the_limit() {
    let limit = MAX_NESTING as usize;
    for kind in KINDS {
        let (codes, _) = diagnose(&nested(kind, limit));
        assert!(codes.is_empty(), "{kind}: {codes:?}");
    }
}

#[test]
fn every_construct_is_rejected_past_the_limit() {
    let limit = MAX_NESTING as usize;
    for (kind, labelled) in KINDS
        .into_iter()
        .zip(["(", "-", "(", "(", ".", "as", "{", "{", "{", "+"])
    {
        let (codes, labels) = diagnose(&nested(kind, limit + 1));
        assert_eq!(codes, ["P2019"], "{kind}");
        assert_eq!(labels, [labelled], "{kind}");
    }
}

#[test]
fn later_suffixes_count_the_levels_they_add_above_earlier_arguments() {
    // The first call's argument is 1 + 254 levels deep; each later call
    // encloses the first one and adds a level above the argument.
    let inner = format!("{}1{}", "(".repeat(254), ")".repeat(254));
    let (codes, _) = diagnose(&format!("const X = f({inner})()"));
    assert!(codes.is_empty(), "{codes:?}");
    let (codes, labels) = diagnose(&format!("const X = f({inner})()()"));
    assert_eq!(codes, ["P2019"]);
    assert_eq!(labels, ["("]);
}

#[test]
fn levels_add_up_across_kinds_of_construct() {
    // A block, a call and 254 parentheses.
    let parens = |n: usize| format!("{}1{}", "(".repeat(n), ")".repeat(n));
    let (codes, _) = diagnose(&format!("func f() {{\n    g({})\n}}", parens(254)));
    assert!(codes.is_empty(), "{codes:?}");
    let (codes, _) = diagnose(&format!("func f() {{\n    g({})\n}}", parens(255)));
    assert_eq!(codes, ["P2019"]);
}

#[test]
fn the_limit_is_reported_once_and_parsing_continues() {
    let deep = format!("const X = {}1{}", "(".repeat(300), ")".repeat(300));
    let text = format!("{deep}\nconst Y = (\nfunc f() {{\n    g(a\n}}");
    let (codes, _) = diagnose(&text);
    // The rest of the file is still parsed and its errors reported.
    assert_eq!(codes, ["P2019", "P2004", "P2008"], "{text}");
}

#[test]
fn a_statement_past_the_limit_is_discarded_alone() {
    let blocks = |n: usize| format!("{}{}", "{".repeat(n), "}".repeat(n));
    let text = format!("func f() {{\n    {}\n    g(-)\n}}", blocks(300));
    let (codes, _) = diagnose(&text);
    assert_eq!(codes, ["P2019", "P2004"]);
}

/// Runs `f` on a thread with 1 MiB of stack, the size of the main thread's
/// stack on Windows, so that a stack overflow fails the test.
fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

/// Lexes, parses and renders `text`, drops the tree and returns the codes
/// of the diagnostics.
fn parse_render_drop(text: &str) -> Vec<String> {
    let mut map = SourceMap::new();
    let file = map.add("stack.pgr", text).unwrap();
    let mut diagnostics = Diagnostics::new();
    let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
    let tree = pinguer_parser::parse(file, text, tokens, &mut diagnostics);
    let rendered = pinguer_ast::render_to_string(&tree, map.file(file));
    assert!(rendered.starts_with("File"));
    drop(tree);
    diagnostics
        .iter()
        .map(|d| d.code.map_or_else(String::new, |code| code.to_string()))
        .collect()
}

/// ADR-0030's stack guarantee. Tests run unoptimized unless `--release` is
/// given, which only lowers stack use.
#[test]
fn nesting_at_the_limit_fits_in_one_mebibyte_of_stack() {
    on_small_stack(|| {
        for kind in KINDS {
            let codes = parse_render_drop(&nested(kind, MAX_NESTING as usize));
            assert!(codes.is_empty(), "{kind}: {codes:?}");
        }
    });
}

/// Ten thousand levels, as in the lexer's robustness corpus: each is
/// reported once, on the same small stack.
#[test]
fn ten_thousand_levels_are_reported_once() {
    on_small_stack(|| {
        for kind in KINDS {
            let codes = parse_render_drop(&nested(kind, 10_000));
            assert_eq!(codes, ["P2019"], "{kind}");
        }
    });
}

#[test]
fn ten_thousand_unclosed_delimiters_terminate() {
    on_small_stack(|| {
        for text in [
            format!("const X = {}", "(".repeat(10_000)),
            format!("const X = {}", "- (".repeat(10_000)),
            format!("const X = {}", "f(".repeat(10_000)),
            format!("const X = {}", "[".repeat(10_000)),
        ] {
            let codes = parse_render_drop(&text);
            assert_eq!(codes.len(), 1, "{codes:?}");
        }
        // The blocks open at the end of the file, up to the limit, are
        // reported together.
        let codes = parse_render_drop(&format!("func f() {}", "{".repeat(10_000)));
        assert_eq!(codes, ["P2019", "P2008"]);
    });
}

/// Lists are parsed by loops and have no limit.
#[test]
fn long_lists_are_not_limited() {
    on_small_stack(|| {
        let n = 100_000;
        for text in [
            format!("const X = 1{}", " + 1".repeat(n)),
            format!("const X = f({})", "1, ".repeat(n)),
            format!("func f() {{\n{}}}", "g()\n".repeat(n)),
            format!(
                "func f() {{\n    if x {{\n    }}{}\n}}",
                " else if x {\n    }".repeat(n)
            ),
            "import a\n".repeat(n),
        ] {
            let codes = parse_render_drop(&text);
            assert!(codes.is_empty(), "{codes:?}");
        }
    });
}

/// Returns the depth of the tree of `text`: the largest indentation of its
/// rendering, in levels.
fn tree_depth(text: &str) -> usize {
    let mut map = SourceMap::new();
    let file = map.add("depth.pgr", text).unwrap();
    let mut diagnostics = Diagnostics::new();
    let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
    let tree = pinguer_parser::parse(file, text, tokens, &mut diagnostics);
    pinguer_ast::render_to_string(&tree, map.file(file))
        .lines()
        .map(|line| (line.len() - line.trim_start().len()) / 2)
        .max()
        .unwrap()
}

/// What the limit bounds is the depth of the tree, which recursive passes
/// and destruction recurse through: at most three lines per level, as a
/// statement, an `if` branch and its block take, plus the file, the item
/// and a leaf.
#[test]
fn the_nesting_limit_bounds_the_depth_of_the_tree() {
    let bound = 3 * MAX_NESTING as usize + 4;
    on_small_stack(move || {
        for kind in KINDS {
            for levels in [MAX_NESTING as usize, 10_000] {
                let depth = tree_depth(&nested(kind, levels));
                assert!(depth <= bound, "{kind} at {levels} levels: {depth}");
            }
        }
        // Parentheses take one line per level.
        assert_eq!(tree_depth(&nested("parentheses", 256)), 256 + 2);
    });
}
