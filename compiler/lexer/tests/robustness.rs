//! Property-style robustness tests for the lexer.
//!
//! These tests feed large numbers of pseudo-random inputs to the lexer and
//! check structural invariants that must hold for *any* input. They use a
//! small deterministic generator instead of an external property-testing
//! crate, so every run exercises the same inputs and failures are
//! reproducible from the printed seed. Coverage-guided fuzzing is planned
//! separately (see docs/architecture/TESTING_STRATEGY.md).

use pinguer_diagnostics::Diagnostics;
use pinguer_lexer::{TokenKind, tokenize};
use pinguer_source::SourceMap;

/// Xorshift64* pseudo-random generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

/// Fragments chosen to hit the interesting paths of the lexer: delimiters,
/// literal prefixes, escapes, comment markers, line endings and characters
/// that must be rejected.
const FRAGMENTS: &[&str] = &[
    " ", "\t", "\n", "\r\n", "\r", "a", "Z", "_", "func", "let", "0", "1", "9", "0x", "0b", "0o",
    "e", "E", ".", "..", "+", "-", "*", "/", "//", "/*", "*/", "\"", "'", "\\", "\\u{", "}", "{",
    "(", ")", "[", "]", "=", "<", ">", "!", "&", "|", "#", "é", "🐧", "\u{202E}", "\u{A0}",
    "\u{301}", "\u{0}", "\u{7F}", "\u{FEFF}",
];

fn random_input(rng: &mut Rng) -> String {
    let length = rng.below(48);
    (0..length)
        .map(|_| FRAGMENTS[rng.below(FRAGMENTS.len())])
        .collect()
}

fn check_invariants(text: &str, seed: u64) {
    let mut map = SourceMap::new();
    let file = map.add("fuzz.pgr", text).unwrap();
    let text = map.file(file).text();
    let mut diagnostics = Diagnostics::new();
    let tokens = tokenize(file, text, &mut diagnostics);
    let context = || format!("seed {seed}, input {text:?}");

    // Exactly one end-of-file token, last, with an empty span at the end.
    let (eof, rest) = tokens
        .split_last()
        .unwrap_or_else(|| panic!("no tokens: {}", context()));
    assert_eq!(eof.kind, TokenKind::Eof, "{}", context());
    assert_eq!(eof.span.start as usize, text.len(), "{}", context());
    assert!(eof.span.is_empty(), "{}", context());
    assert!(
        rest.iter().all(|t| t.kind != TokenKind::Eof),
        "{}",
        context()
    );

    // Spans are in bounds, non-empty, on character boundaries, ordered and
    // non-overlapping.
    let mut previous_end = 0u32;
    for token in rest {
        assert_eq!(token.span.file, file);
        assert!(!token.span.is_empty(), "empty {:?}: {}", token, context());
        assert!(
            token.span.start >= previous_end,
            "overlap at {:?}: {}",
            token,
            context()
        );
        assert!(token.span.end as usize <= text.len(), "{}", context());
        assert!(
            text.is_char_boundary(token.span.start as usize),
            "{}",
            context()
        );
        assert!(
            text.is_char_boundary(token.span.end as usize),
            "{}",
            context()
        );
        previous_end = token.span.end;
    }

    // Newline tokens never start the stream and never repeat.
    assert_ne!(
        rest.first().map(|t| t.kind),
        Some(TokenKind::Newline),
        "{}",
        context()
    );
    for pair in rest.windows(2) {
        assert!(
            !(pair[0].kind == TokenKind::Newline && pair[1].kind == TokenKind::Newline),
            "{}",
            context()
        );
    }

    // Every diagnostic carries a code and a primary label inside the file,
    // and an `Unknown` token is always accompanied by an error.
    for diagnostic in &diagnostics {
        assert!(diagnostic.code.is_some(), "{}", context());
        let span = diagnostic
            .main_label()
            .expect("lexer diagnostics are labelled")
            .span;
        assert!(span.end as usize <= text.len(), "{}", context());
    }
    if rest.iter().any(|t| t.kind == TokenKind::Unknown) {
        assert!(diagnostics.has_errors(), "{}", context());
    }

    // Lexing is deterministic.
    let mut again = Diagnostics::new();
    assert_eq!(tokenize(file, text, &mut again), tokens, "{}", context());
    assert_eq!(again.as_slice(), diagnostics.as_slice(), "{}", context());
}

#[test]
fn random_fragment_sequences_uphold_invariants() {
    for seed in 1..=5_000u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let input = random_input(&mut rng);
        check_invariants(&input, seed);
    }
}

#[test]
fn random_characters_uphold_invariants() {
    for seed in 1..=2_000u64 {
        let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
        let length = rng.below(32);
        let input: String = (0..length)
            .map(|_| {
                // Mostly ASCII, sometimes any Unicode scalar value.
                let code = if rng.below(4) == 0 {
                    rng.below(0x11_0000) as u32
                } else {
                    rng.below(0x80) as u32
                };
                char::from_u32(code).unwrap_or('\u{FFFD}')
            })
            .collect();
        check_invariants(&input, seed);
    }
}

#[test]
fn deeply_nested_block_comments() {
    let input = "/*".repeat(10_000) + &"*/".repeat(10_000) + "x";
    check_invariants(&input, 0);
}

#[test]
fn deeply_nested_delimiters() {
    let input = "(".repeat(10_000) + "\n" + &")".repeat(10_000);
    check_invariants(&input, 0);
}
