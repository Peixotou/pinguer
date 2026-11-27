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
        "member accesses" => format!("const X = a{}", ".b".repeat(levels)),
        "conversions" => format!("const X = x{}", " as int".repeat(levels)),
        // The function body is the first block.
        "blocks" => format!(
            "func f() {{{}{}}}",
            "{".repeat(levels - 1),
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

const KINDS: [&str; 7] = [
    "parentheses",
    "prefix operators",
    "calls",
    "member accesses",
    "conversions",
    "blocks",
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
    for (kind, labelled) in KINDS.into_iter().zip(["(", "-", "(", ".", "as", "{", "+"]) {
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
