//! Property-style robustness tests for the parser.
//!
//! As in the lexer's robustness tests, a small deterministic generator
//! replaces an external property-testing crate: every run parses the same
//! inputs, and a failure names the seed or input that caused it.

use pinguer_diagnostics::Diagnostics;
use pinguer_source::SourceMap;

/// Xorshift64* pseudo-random generator.
struct Rng(u64);

impl Rng {
    fn new(seed: u64, salt: u64) -> Self {
        Self(seed.wrapping_mul(salt) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

/// A position as rendered: line and column, both 1-based.
type Position = (u32, u32);

/// What parsing one input produced.
struct Outcome {
    rendered: String,
    codes: Vec<String>,
    node_count: u32,
    /// The start of the primary label of every diagnostic, lexical ones
    /// included.
    diagnostic_starts: Vec<Position>,
    /// The primary span of every parser diagnostic, as a byte range.
    parser_spans: Vec<(u32, u32)>,
    /// The start of every token other than a line break, in order; the last
    /// one is the end of the file.
    token_starts: Vec<Position>,
}

fn parse(text: &str) -> Outcome {
    let mut map = SourceMap::new();
    let file = map.add("fuzz.pgr", text).unwrap();
    let mut diagnostics = Diagnostics::new();
    let tokens = pinguer_lexer::tokenize(file, text, &mut diagnostics);
    let source = map.file(file);
    let position = |offset: u32| {
        let at = source.line_col(offset);
        (at.line, at.column)
    };
    let token_starts = tokens
        .iter()
        .filter(|token| token.kind != pinguer_lexer::TokenKind::Newline)
        .map(|token| position(token.span.start))
        .collect();
    let tree = pinguer_parser::parse(file, text, tokens, &mut diagnostics);
    // Rendering panics if a span lies outside the file.
    let rendered = pinguer_ast::render_to_string(&tree, source);
    let primary = |d: &pinguer_diagnostics::Diagnostic| d.main_label().map(|label| label.span);
    Outcome {
        rendered,
        codes: diagnostics
            .iter()
            .map(|d| d.code.map_or_else(String::new, |code| code.to_string()))
            .collect(),
        node_count: tree.node_count,
        diagnostic_starts: diagnostics
            .iter()
            .filter_map(|d| primary(d).map(|span| position(span.start)))
            .collect(),
        parser_spans: diagnostics
            .iter()
            .filter(|d| d.code.is_some_and(|code| code.number() / 1000 == 2))
            .filter_map(|d| primary(d).map(|span| (span.start, span.end)))
            .collect(),
        token_starts,
    }
}

/// Returns the span of every error node in `rendered`, whose lines read
/// `ErrorStmt #3 @2:5-2:16`.
fn error_nodes(rendered: &str) -> Vec<(Position, Position)> {
    let number = |text: &str| text.parse::<u32>().unwrap();
    rendered
        .lines()
        .filter(|line| line.trim_start().starts_with("Error"))
        .map(|line| {
            let span = line.split_whitespace().nth(2).unwrap();
            let (start, end) = span.strip_prefix('@').unwrap().split_once('-').unwrap();
            let (start_line, start_column) = start.split_once(':').unwrap();
            let (end_line, end_column) = end.split_once(':').unwrap();
            (
                (number(start_line), number(start_column)),
                (number(end_line), number(end_column)),
            )
        })
        .collect()
}

/// Checks the invariants that hold for any input, and returns the outcome.
fn check(text: &str, context: &dyn Fn() -> String) -> Outcome {
    let outcome = parse(text);

    // IDs are exactly `0..node_count`, in the order of the rendered lines,
    // which is pre-order.
    let ids: Vec<u32> = outcome
        .rendered
        .split_whitespace()
        .filter_map(|word| word.strip_prefix('#')?.parse().ok())
        .collect();
    let expected: Vec<u32> = (0..outcome.node_count).collect();
    assert_eq!(ids, expected, "{}", context());

    // Every error node has a diagnostic of its own: one that points into
    // the node or at the token that follows it, where recovery found the
    // problem.
    for (start, end) in error_nodes(&outcome.rendered) {
        let next = outcome
            .token_starts
            .iter()
            .copied()
            .find(|&token| token >= end)
            .expect("the end-of-file token follows every node");
        assert!(
            outcome
                .diagnostic_starts
                .iter()
                .any(|&at| start <= at && at <= next),
            "error node {start:?}-{end:?} without a diagnostic: {}\n{}",
            context(),
            outcome.rendered
        );
    }

    // No two parser diagnostics blame the same span (ADR-0029); unclosed
    // delimiters closed at the same token are reported together.
    let mut seen = std::collections::HashSet::new();
    for &span in &outcome.parser_spans {
        assert!(
            seen.insert(span),
            "two diagnostics at {span:?}: {:?}: {}",
            outcome.codes,
            context()
        );
    }

    // Parsing is deterministic.
    let again = parse(text);
    assert_eq!(again.rendered, outcome.rendered, "{}", context());
    assert_eq!(again.codes, outcome.codes, "{}", context());
    outcome
}

/// Fragments that reach the interesting paths of the parser: every kind of
/// token, the tokens of planned syntax, reserved keywords, separators and
/// unbalanced delimiters, and a character the lexer rejects.
const FRAGMENTS: &[&str] = &[
    " ", "\n", ";", "x", "io", "f", "func", "import", "const", "let", "var", "if", "else", "while",
    "return", "break", "continue", "true", "as", "int", "0", "1_000", "0x1F", "1.5", "\"s\"",
    "'c'", "(", ")", "{", "}", "[", "]", ",", ".", ":", "->", "=", "+=", "<<=", "+", "-", "*", "/",
    "%", "<<", ">>", "&", "^", "|", "==", "!=", "<", ">=", "&&", "||", "!", "~", "**", "|>", "??",
    "?.", "?", "..", "=>", "@", "for", "match", "struct", "$", "// c\n", "/* c */",
];

#[test]
fn random_token_sequences_uphold_invariants() {
    for seed in 1..=5_000u64 {
        let mut rng = Rng::new(seed, 0x9E37_79B9_7F4A_7C15);
        let length = rng.below(64);
        let text: String = (0..length).map(|_| rng.pick(FRAGMENTS)).collect();
        check(&text, &|| format!("seed {seed}, input {text:?}"));
    }
}

/// Random fragments inside a function body, where statements are parsed.
#[test]
fn random_function_bodies_uphold_invariants() {
    for seed in 1..=3_000u64 {
        let mut rng = Rng::new(seed, 0xD1B5_4A32_D192_ED03);
        let length = rng.below(48);
        let body: String = (0..length).map(|_| rng.pick(FRAGMENTS)).collect();
        let text = format!("func f() {{\n{body}\n}}\nfunc g() {{\n}}\n");
        check(&text, &|| format!("seed {seed}, input {text:?}"));
    }
}

fn examples() -> Vec<String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    ["hello.pgr", "fibonacci.pgr"]
        .iter()
        .map(|name| std::fs::read_to_string(dir.join(name)).unwrap())
        .collect()
}

#[test]
fn every_truncation_of_the_examples_upholds_invariants() {
    for text in examples() {
        for end in (0..=text.len()).filter(|&end| text.is_char_boundary(end)) {
            let prefix = &text[..end];
            check(prefix, &|| format!("prefix {prefix:?}"));
        }
    }
}

#[test]
fn deleting_any_token_of_the_examples_upholds_invariants() {
    for text in examples() {
        let mut map = SourceMap::new();
        let file = map.add("t.pgr", &text).unwrap();
        let tokens = pinguer_lexer::tokenize(file, &text, &mut Diagnostics::new());
        for token in &tokens {
            let range = token.span.range();
            let edited = format!("{}{}", &text[..range.start], &text[range.end..]);
            check(&edited, &|| format!("without {range:?}: {edited:?}"));
        }
    }
}

/// Generates well-formed programs from the grammar.
struct Generator {
    rng: Rng,
}

impl Generator {
    fn name(&mut self) -> String {
        ["a", "b", "count", "x1", "_tmp"][self.rng.below(5)].to_owned()
    }

    fn expr(&mut self, depth: usize) -> String {
        const BINARY: &[&str] = &[
            "*", "/", "%", "+", "-", "<<", ">>", "&", "^", "|", "&&", "||",
        ];
        const COMPARISON: &[&str] = &["==", "!=", "<", "<=", ">", ">="];
        let choice = if depth == 0 {
            self.rng.below(4)
        } else {
            self.rng.below(11)
        };
        match choice {
            0 => self.name(),
            1 => ["0", "42", "0xFF", "1_000", "0b101"][self.rng.below(5)].to_owned(),
            2 => ["1.5", "2.0e3", "\"text\"", "'c'", "true", "false"][self.rng.below(6)].to_owned(),
            3 => format!("io.{}", self.name()),
            4 => format!("({})", self.expr(depth - 1)),
            5 => format!(
                "{}{}",
                ["-", "!", "~"][self.rng.below(3)],
                self.expr(depth - 1)
            ),
            6 => format!("{} as int", self.expr(depth - 1)),
            7 => {
                let args: Vec<String> = (0..self.rng.below(3))
                    .map(|_| self.expr(depth - 1))
                    .collect();
                format!("{}({})", self.name(), args.join(", "))
            }
            // Comparisons are non-associative, and every binary operator
            // but `&&` and `||` binds more tightly: the comparison and its
            // operands are parenthesized.
            8 => format!(
                "(({}) {} ({}))",
                self.expr(depth - 1),
                self.rng.pick(COMPARISON),
                self.expr(depth - 1)
            ),
            _ => format!(
                "{} {} {}",
                self.expr(depth - 1),
                self.rng.pick(BINARY),
                self.expr(depth - 1)
            ),
        }
    }

    fn block(&mut self, depth: usize, indent: usize) -> String {
        let mut out = String::from("{\n");
        for _ in 0..self.rng.below(4) {
            out.push_str(&"    ".repeat(indent + 1));
            out.push_str(&self.stmt(depth, indent + 1));
            out.push('\n');
        }
        out.push_str(&"    ".repeat(indent));
        out.push('}');
        out
    }

    fn stmt(&mut self, depth: usize, indent: usize) -> String {
        let choice = if depth == 0 {
            self.rng.below(6)
        } else {
            self.rng.below(9)
        };
        match choice {
            0 => format!("let {} = {}", self.name(), self.expr(2)),
            1 => format!("var {}: int = {}", self.name(), self.expr(2)),
            2 => {
                let op = ["=", "+=", "<<=", "|="][self.rng.below(4)];
                format!("{} {op} {}", self.name(), self.expr(2))
            }
            3 => format!("{}({})", self.name(), self.expr(1)),
            4 => ["return", "break", "continue"][self.rng.below(3)].to_owned(),
            5 => format!("return {}", self.expr(2)),
            6 => format!(
                "if {} {} else if {} {} else {}",
                self.expr(1),
                self.block(depth - 1, indent),
                self.expr(1),
                self.block(depth - 1, indent),
                self.block(depth - 1, indent)
            ),
            7 => format!("while {} {}", self.expr(1), self.block(depth - 1, indent)),
            _ => self.block(depth - 1, indent),
        }
    }

    fn program(&mut self) -> String {
        let mut out = String::from("import std.io\n\n");
        for _ in 0..1 + self.rng.below(3) {
            if self.rng.below(3) == 0 {
                out.push_str(&format!("const {} = {}\n", self.name(), self.expr(3)));
            } else {
                let name = self.name();
                let body = self.block(3, 0);
                out.push_str(&format!("func {name}(n: int, s: string) -> int {body}\n"));
            }
        }
        out
    }
}

#[test]
fn generated_programs_parse_without_diagnostics() {
    for seed in 1..=2_000u64 {
        let mut generator = Generator {
            rng: Rng::new(seed, 0xA24B_AED4_963E_E407),
        };
        let text = generator.program();
        let outcome = check(&text, &|| format!("seed {seed}:\n{text}"));
        assert!(
            outcome.codes.is_empty(),
            "seed {seed}: {:?}\n{text}",
            outcome.codes
        );
        assert!(!outcome.rendered.contains("Error"), "seed {seed}:\n{text}");
    }
}
