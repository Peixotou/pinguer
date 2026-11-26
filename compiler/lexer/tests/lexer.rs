//! Behavioural tests for the Pinguer lexer.

use pinguer_diagnostics::{Diagnostic, Diagnostics};
use pinguer_lexer::{Keyword, Token, TokenKind, tokenize};
use pinguer_source::{FileId, SourceMap};

use TokenKind as T;

struct Lexed {
    map: SourceMap,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl Lexed {
    /// Token kinds, excluding the final end-of-file token.
    fn kinds(&self) -> Vec<TokenKind> {
        let (eof, rest) = self.tokens.split_last().unwrap();
        assert_eq!(eof.kind, T::Eof);
        rest.iter().map(|token| token.kind).collect()
    }

    /// Token kinds paired with their source text, excluding end of file.
    fn pairs(&self) -> Vec<(TokenKind, &str)> {
        self.tokens[..self.tokens.len() - 1]
            .iter()
            .map(|token| (token.kind, self.map.snippet(token.span)))
            .collect()
    }

    fn codes(&self) -> Vec<String> {
        self.diagnostics
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .code
                    .expect("lexer diagnostics have codes")
                    .to_string()
            })
            .collect()
    }

    /// The text covered by the primary label of each diagnostic.
    fn labelled(&self) -> Vec<&str> {
        self.diagnostics
            .iter()
            .map(|diagnostic| self.map.snippet(diagnostic.main_label().unwrap().span))
            .collect()
    }

    fn assert_clean(&self) {
        assert!(
            self.diagnostics.is_empty(),
            "unexpected diagnostics: {:#?}",
            self.diagnostics
        );
    }
}

fn lex(text: &str) -> Lexed {
    let mut map = SourceMap::new();
    let file: FileId = map.add("test.pgr", text).unwrap();
    let mut diagnostics = Diagnostics::new();
    let tokens = tokenize(file, map.file(file).text(), &mut diagnostics);
    Lexed {
        map,
        tokens,
        diagnostics: diagnostics.into_vec(),
    }
}

fn kinds(text: &str) -> Vec<TokenKind> {
    let lexed = lex(text);
    lexed.assert_clean();
    lexed.kinds()
}

fn kw(keyword: Keyword) -> TokenKind {
    T::Keyword(keyword)
}

// Basic structure

#[test]
fn empty_input_is_only_end_of_file() {
    let lexed = lex("");
    lexed.assert_clean();
    assert_eq!(lexed.tokens.len(), 1);
    assert_eq!(lexed.tokens[0].kind, T::Eof);
    assert!(lexed.tokens[0].span.is_empty());
}

#[test]
fn end_of_file_span_is_at_end_of_text() {
    let lexed = lex("let x = 1   ");
    let eof = lexed.tokens.last().unwrap();
    assert_eq!((eof.span.start, eof.span.end), (12, 12));
}

#[test]
fn hello_world() {
    let source = "import std.io\n\nfunc main() {\n    io.print(\"Hello, Pinguer!\")\n}\n";
    let lexed = lex(source);
    lexed.assert_clean();
    assert_eq!(
        lexed.pairs(),
        vec![
            (kw(Keyword::Import), "import"),
            (T::Ident, "std"),
            (T::Dot, "."),
            (T::Ident, "io"),
            (T::Newline, "\n"),
            (kw(Keyword::Func), "func"),
            (T::Ident, "main"),
            (T::LParen, "("),
            (T::RParen, ")"),
            (T::LBrace, "{"),
            (T::Newline, "\n"),
            (T::Ident, "io"),
            (T::Dot, "."),
            (T::Ident, "print"),
            (T::LParen, "("),
            (T::String, "\"Hello, Pinguer!\""),
            (T::RParen, ")"),
            (T::Newline, "\n"),
            (T::RBrace, "}"),
            (T::Newline, "\n"),
        ]
    );
}

#[test]
fn function_with_types_and_return() {
    assert_eq!(
        kinds("func sum(a: int, b: int) -> int {\n    return a + b\n}"),
        vec![
            kw(Keyword::Func),
            T::Ident,
            T::LParen,
            T::Ident,
            T::Colon,
            T::Ident,
            T::Comma,
            T::Ident,
            T::Colon,
            T::Ident,
            T::RParen,
            T::Arrow,
            T::Ident,
            T::LBrace,
            T::Newline,
            kw(Keyword::Return),
            T::Ident,
            T::Plus,
            T::Ident,
            T::Newline,
            T::RBrace,
        ]
    );
}

// Identifiers and keywords

#[test]
fn identifiers_and_keywords() {
    assert_eq!(
        kinds("let var const Func func_ _x x1 _ __"),
        vec![
            kw(Keyword::Let),
            kw(Keyword::Var),
            kw(Keyword::Const),
            T::Ident,
            T::Ident,
            T::Ident,
            T::Ident,
            T::Underscore,
            T::Ident,
        ]
    );
}

#[test]
fn every_keyword_is_recognised() {
    for &keyword in Keyword::ALL {
        assert_eq!(kinds(keyword.as_str()), vec![kw(keyword)], "{keyword}");
    }
}

#[test]
fn primitive_type_names_are_identifiers() {
    assert_eq!(kinds("int string bool f64 u8"), vec![T::Ident; 5]);
}

#[test]
fn non_ascii_identifier_is_rejected_as_one_token() {
    let lexed = lex("let café = 1");
    assert_eq!(lexed.codes(), ["P1011"]);
    assert_eq!(lexed.labelled(), ["café"]);
    assert_eq!(
        lexed.kinds(),
        vec![kw(Keyword::Let), T::Ident, T::Eq, T::Int]
    );
}

// Line breaks

#[test]
fn consecutive_line_breaks_collapse() {
    assert_eq!(
        kinds("a\n\n\n   \n\tb"),
        vec![T::Ident, T::Newline, T::Ident]
    );
}

#[test]
fn leading_line_breaks_produce_no_token() {
    assert_eq!(kinds("\n\n// comment\n\na"), vec![T::Ident]);
}

#[test]
fn trailing_line_break_produces_a_token() {
    assert_eq!(kinds("a\n"), vec![T::Ident, T::Newline]);
}

#[test]
fn crlf_is_a_single_line_break() {
    let lexed = lex("a\r\nb\r\n");
    lexed.assert_clean();
    assert_eq!(
        lexed.pairs(),
        vec![
            (T::Ident, "a"),
            (T::Newline, "\r\n"),
            (T::Ident, "b"),
            (T::Newline, "\r\n"),
        ]
    );
}

#[test]
fn line_breaks_inside_parentheses_and_brackets_are_ignored() {
    assert_eq!(
        kinds("f(\n  a,\n  b\n)\n[\n1\n]"),
        vec![
            T::Ident,
            T::LParen,
            T::Ident,
            T::Comma,
            T::Ident,
            T::RParen,
            T::Newline,
            T::LBracket,
            T::Int,
            T::RBracket,
        ]
    );
}

#[test]
fn braces_inside_parentheses_make_line_breaks_significant_again() {
    assert_eq!(
        kinds("f({\na\nb\n})"),
        vec![
            T::Ident,
            T::LParen,
            T::LBrace,
            T::Newline,
            T::Ident,
            T::Newline,
            T::Ident,
            T::Newline,
            T::RBrace,
            T::RParen,
        ]
    );
}

#[test]
fn unmatched_closing_delimiter_does_not_disturb_line_breaks() {
    let lexed = lex(")\na\nb");
    lexed.assert_clean();
    assert_eq!(
        lexed.kinds(),
        vec![T::RParen, T::Newline, T::Ident, T::Newline, T::Ident]
    );
}

#[test]
fn closing_brace_closes_unclosed_parenthesis_inside_it() {
    // The parser reports the missing `)`; the lexer only needs to recover so
    // that the following line break is significant again.
    assert_eq!(
        kinds("{ f( }\na"),
        vec![
            T::LBrace,
            T::Ident,
            T::LParen,
            T::RBrace,
            T::Newline,
            T::Ident,
        ]
    );
}

#[test]
fn comments_between_line_breaks_collapse() {
    assert_eq!(
        kinds("a // first\n// second\n\n/* third */\nb"),
        vec![T::Ident, T::Newline, T::Ident]
    );
}

#[test]
fn block_comment_containing_line_break_counts_as_line_break() {
    assert_eq!(
        kinds("a /* x\n y */ b"),
        vec![T::Ident, T::Newline, T::Ident]
    );
    assert_eq!(kinds("a /* x y */ b"), vec![T::Ident, T::Ident]);
}

#[test]
fn line_comment_at_end_of_file() {
    assert_eq!(kinds("a // trailing"), vec![T::Ident]);
}

// Comments

#[test]
fn block_comments_nest() {
    assert_eq!(kinds("/* a /* b */ c */ x"), vec![T::Ident]);
}

#[test]
fn unterminated_block_comment_is_reported_at_its_start() {
    let lexed = lex("x /* a /* b */");
    assert_eq!(lexed.codes(), ["P1003"]);
    assert_eq!(lexed.labelled(), ["/*"]);
    assert_eq!(lexed.kinds(), vec![T::Ident]);
}

#[test]
fn doc_comment_is_a_comment() {
    assert_eq!(kinds("/// docs\nfunc"), vec![kw(Keyword::Func)]);
}

// Numeric literals

#[test]
fn integer_literals() {
    let lexed = lex("0 7 42 1_000_000 0xFF 0xdead_BEEF 0o17 0b1010_0101");
    lexed.assert_clean();
    assert_eq!(lexed.kinds(), vec![T::Int; 8]);
}

#[test]
fn float_literals() {
    let lexed = lex("0.5 1.5 3.141_592 1e10 1E10 2.5e-3 6.02e+23 1_0.0_1e1_0");
    lexed.assert_clean();
    assert_eq!(lexed.kinds(), vec![T::Float; 8]);
}

#[test]
fn dot_after_integer_is_not_part_of_the_literal() {
    assert_eq!(kinds("1.foo"), vec![T::Int, T::Dot, T::Ident]);
    assert_eq!(kinds("1..2"), vec![T::Int, T::DotDot, T::Int]);
    assert_eq!(kinds("1."), vec![T::Int, T::Dot]);
    assert_eq!(kinds("1.e5"), vec![T::Int, T::Dot, T::Ident]);
}

#[test]
fn minus_is_a_separate_token() {
    assert_eq!(kinds("-1"), vec![T::Minus, T::Int]);
}

#[test]
fn leading_zeros_are_rejected() {
    for source in ["007", "00", "00.5", "0_1"] {
        let lexed = lex(source);
        assert!(
            lexed.codes().contains(&"P1007".to_owned()),
            "{source}: {:?}",
            lexed.codes()
        );
        assert_eq!(lexed.kinds().len(), 1, "{source} must stay one token");
    }
}

#[test]
fn misplaced_separators_are_rejected() {
    for (source, misplaced_at) in [
        ("1_", 1),
        ("1__0", 1),
        ("1.5_", 3),
        ("1e1_", 3),
        ("0x_1", 2),
        ("0b1_", 3),
    ] {
        let lexed = lex(source);
        assert_eq!(lexed.codes(), ["P1006"], "{source}");
        let span = lexed.diagnostics[0].main_label().unwrap().span;
        assert_eq!(span.start, misplaced_at, "{source}");
    }
}

#[test]
fn leading_underscore_makes_an_identifier() {
    assert_eq!(kinds("_1"), vec![T::Ident]);
}

#[test]
fn invalid_digits_for_base_are_rejected() {
    let lexed = lex("0b102");
    assert_eq!(lexed.codes(), ["P1006"]);
    assert_eq!(lexed.labelled(), ["2"]);
    assert_eq!(lexed.kinds(), vec![T::Int]);

    let lexed = lex("0o78");
    assert_eq!(lexed.labelled(), ["8"]);

    let lexed = lex("0xFG");
    assert_eq!(lexed.labelled(), ["G"]);
}

#[test]
fn missing_digits_after_prefix() {
    for source in ["0x", "0b", "0o", "0x_"] {
        let lexed = lex(source);
        assert_eq!(lexed.codes(), ["P1006"], "{source}");
        assert_eq!(lexed.labelled(), [source]);
    }
}

#[test]
fn uppercase_prefix_is_not_a_prefix() {
    let lexed = lex("0X1F");
    assert_eq!(lexed.codes(), ["P1006"]);
    assert_eq!(lexed.labelled(), ["X1F"]);
}

#[test]
fn missing_exponent_digits() {
    for source in ["1e", "1e+", "2.5E-"] {
        let lexed = lex(source);
        assert!(
            lexed.diagnostics[0].message.contains("exponent"),
            "{source}: {:?}",
            lexed.diagnostics
        );
    }
}

#[test]
fn suffixes_are_rejected() {
    let lexed = lex("10u8 1.5f32");
    assert_eq!(lexed.codes(), ["P1006", "P1006"]);
    assert_eq!(lexed.labelled(), ["u8", "f32"]);
    assert_eq!(lexed.kinds(), vec![T::Int, T::Float]);
}

// String literals

#[test]
fn string_literals_with_escapes() {
    let lexed = lex(r#""" "a" "tab\tnew\nline" "q\"\'\\\0\r" "\u{0}\u{41}\u{1F427}\u{10FFFF}""#);
    lexed.assert_clean();
    assert_eq!(lexed.kinds(), vec![T::String; 5]);
}

#[test]
fn strings_may_contain_any_non_control_text() {
    let lexed = lex("\"Olá, mundo! 🐧 // not a comment /* nor this\"");
    lexed.assert_clean();
    assert_eq!(lexed.kinds(), vec![T::String]);
}

#[test]
fn unknown_escape_is_reported() {
    let lexed = lex(r#""a\qb" x"#);
    assert_eq!(lexed.codes(), ["P1005"]);
    assert_eq!(lexed.labelled(), [r"\q"]);
    assert_eq!(lexed.kinds(), vec![T::String, T::Ident]);
}

#[test]
fn invalid_unicode_escapes_are_reported() {
    for (source, labelled) in [
        (r#""\uZ""#, r"\u"),
        (r#""\u{}""#, r"\u{}"),
        (r#""\u{1234567}""#, r"\u{1234567}"),
        (r#""\u{D800}""#, r"\u{D800}"),
        (r#""\u{110000}""#, r"\u{110000}"),
        (r#""\u{41""#, r"\u{41"),
    ] {
        let lexed = lex(source);
        assert_eq!(lexed.codes(), ["P1005"], "{source}");
        assert_eq!(lexed.labelled(), [labelled], "{source}");
        assert_eq!(lexed.kinds(), vec![T::String], "{source}");
    }
}

#[test]
fn unterminated_string_stops_at_end_of_line() {
    let lexed = lex("let s = \"abc\r\nnext");
    assert_eq!(lexed.codes(), ["P1002"]);
    assert_eq!(lexed.labelled(), ["\"abc"]);
    assert_eq!(
        lexed.pairs(),
        vec![
            (kw(Keyword::Let), "let"),
            (T::Ident, "s"),
            (T::Eq, "="),
            (T::String, "\"abc"),
            (T::Newline, "\r\n"),
            (T::Ident, "next"),
        ]
    );
}

#[test]
fn unterminated_string_at_end_of_file() {
    let lexed = lex("\"abc");
    assert_eq!(lexed.codes(), ["P1002"]);
    assert_eq!(lexed.kinds(), vec![T::String]);
}

#[test]
fn backslash_before_line_break_does_not_continue_the_string() {
    let lexed = lex("\"abc\\\nx");
    assert_eq!(lexed.codes(), ["P1002"]);
    assert_eq!(lexed.kinds(), vec![T::String, T::Newline, T::Ident]);
}

// Character literals

#[test]
fn character_literals() {
    let lexed = lex(r"'a' '\n' '\'' '\u{1F427}' '🐧' 'é'");
    lexed.assert_clean();
    assert_eq!(lexed.kinds(), vec![T::Char; 6]);
}

#[test]
fn empty_character_literal() {
    let lexed = lex("''");
    assert_eq!(lexed.codes(), ["P1008"]);
    assert_eq!(lexed.kinds(), vec![T::Char]);
}

#[test]
fn multi_character_literal() {
    let lexed = lex("'ab' x");
    assert_eq!(lexed.codes(), ["P1008"]);
    assert_eq!(lexed.labelled(), ["'ab'"]);
    assert_eq!(lexed.kinds(), vec![T::Char, T::Ident]);
}

#[test]
fn decomposed_character_is_two_scalar_values() {
    // 'e' followed by U+0301 COMBINING ACUTE ACCENT.
    let lexed = lex("'e\u{301}'");
    assert_eq!(lexed.codes(), ["P1008"]);
}

#[test]
fn unterminated_character_literal() {
    let lexed = lex("'a\nb");
    assert_eq!(lexed.codes(), ["P1004"]);
    assert_eq!(lexed.labelled(), ["'a"]);
    assert_eq!(lexed.kinds(), vec![T::Char, T::Newline, T::Ident]);
}

// Operators and punctuation

#[test]
fn every_operator_and_punctuation_token() {
    let source = "<<= >>= **= ..= -> => == != <= >= && || << >> ** ?? ?. |> \
                  += -= *= /= %= &= |= ^= .. \
                  ( ) { } [ ] , . : ; ? @ + - * / % & | ^ ~ ! = < >";
    let lexed = lex(source);
    lexed.assert_clean();
    for (kind, text) in lexed.pairs() {
        assert_eq!(kind.punctuation(), Some(text), "{text}");
    }
    assert_eq!(lexed.kinds().len(), 52);
}

// Tokens added by ADR-0013 to ADR-0019 (lexical layer only; the operators
// have no meaning in v0.1 and are rejected by the parser)

#[test]
fn power_operator_tokens() {
    assert_eq!(kinds("a**b"), vec![T::Ident, T::StarStar, T::Ident]);
    assert_eq!(kinds("a **= 2"), vec![T::Ident, T::StarStarEq, T::Int]);
    assert_eq!(kinds("2**-1"), vec![T::Int, T::StarStar, T::Minus, T::Int]);
    // Separated stars remain two multiplication tokens.
    assert_eq!(kinds("a * *b"), vec![T::Ident, T::Star, T::Star, T::Ident]);
    // `*=` followed by `*` is not a power assignment.
    assert_eq!(
        kinds("a *=*b"),
        vec![T::Ident, T::StarEq, T::Star, T::Ident]
    );
    // Three stars: the longest token first.
    assert_eq!(
        kinds("a***b"),
        vec![T::Ident, T::StarStar, T::Star, T::Ident]
    );
}

#[test]
fn optional_operator_tokens() {
    assert_eq!(
        kinds("a ?? b"),
        vec![T::Ident, T::QuestionQuestion, T::Ident]
    );
    assert_eq!(
        kinds("user?.name"),
        vec![T::Ident, T::QuestionDot, T::Ident]
    );
    // A space breaks the token.
    assert_eq!(
        kinds("user? .name"),
        vec![T::Ident, T::Question, T::Dot, T::Ident]
    );
    assert_eq!(
        kinds("a ? ? b"),
        vec![T::Ident, T::Question, T::Question, T::Ident]
    );
    // `T??` is one token; the parser splits it in type position.
    assert_eq!(kinds("int??"), vec![T::Ident, T::QuestionQuestion]);
    // `?..` is `?.` followed by `.`, never `?` followed by `..`.
    assert_eq!(
        kinds("a?..b"),
        vec![T::Ident, T::QuestionDot, T::Dot, T::Ident]
    );
}

#[test]
fn pipeline_operator_token() {
    assert_eq!(kinds("data |> parse"), vec![T::Ident, T::PipeGt, T::Ident]);
    assert_eq!(kinds("a | >b"), vec![T::Ident, T::Pipe, T::Gt, T::Ident]);
    assert_eq!(
        kinds("a || >b"),
        vec![T::Ident, T::PipePipe, T::Gt, T::Ident]
    );
    assert_eq!(kinds("a |>= b"), vec![T::Ident, T::PipeGt, T::Eq, T::Ident]);
}

#[test]
fn leading_operators_on_continuation_lines_still_produce_newlines() {
    // Line continuation for a leading `.`, `?.`, `|>` or `??` is decided by
    // the parser (ADR-0014); the lexer still reports the line break.
    assert_eq!(
        kinds("items\n    .filter(f)\n    |> sum"),
        vec![
            T::Ident,
            T::Newline,
            T::Dot,
            T::Ident,
            T::LParen,
            T::Ident,
            T::RParen,
            T::Newline,
            T::PipeGt,
            T::Ident,
        ]
    );
}

#[test]
fn visibility_and_propagation_keywords_are_reserved() {
    assert_eq!(
        kinds("public private try class"),
        vec![
            kw(Keyword::Public),
            kw(Keyword::Private),
            kw(Keyword::Try),
            kw(Keyword::Class),
        ]
    );
    for keyword in [
        Keyword::Public,
        Keyword::Private,
        Keyword::Try,
        Keyword::Class,
    ] {
        assert!(keyword.is_reserved_for_future_use(), "{keyword}");
    }
}

#[test]
fn replaced_and_rejected_words_are_identifiers() {
    // `pub` was replaced by `public` (ADR-0012). Words proposed for
    // inheritance and other unadopted features are not reserved (ADR-0011,
    // ADR-0015), so programs may use them as names.
    for word in [
        "pub",
        "extends",
        "super",
        "virtual",
        "override",
        "protected",
        "internal",
        "readonly",
        "init",
        "borrow",
        "parallel",
        "supervisor",
        "module",
        "export",
        "none",
        "repeat",
        "void",
        "float",
    ] {
        assert_eq!(kinds(word), vec![T::Ident], "{word}");
    }
}

#[test]
fn query_clause_words_are_not_reserved() {
    // Contextual keywords (ADR-0015): ordinary identifiers outside a query.
    assert_eq!(
        kinds("let order = select(from, by, query)"),
        vec![
            kw(Keyword::Let),
            T::Ident,
            T::Eq,
            T::Ident,
            T::LParen,
            T::Ident,
            T::Comma,
            T::Ident,
            T::Comma,
            T::Ident,
            T::RParen,
        ]
    );
}

#[test]
fn operators_are_read_greedily() {
    assert_eq!(
        kinds("a>>=b<<c"),
        vec![T::Ident, T::ShrEq, T::Ident, T::Shl, T::Ident]
    );
    assert_eq!(kinds("x=-1"), vec![T::Ident, T::Eq, T::Minus, T::Int]);
    assert_eq!(kinds("a->b"), vec![T::Ident, T::Arrow, T::Ident]);
    assert_eq!(kinds("1/2"), vec![T::Int, T::Slash, T::Int]);
}

// Rejected characters

#[test]
fn unknown_characters_are_reported_individually() {
    let lexed = lex("a # $ b");
    assert_eq!(lexed.codes(), ["P1001", "P1001"]);
    assert_eq!(
        lexed.kinds(),
        vec![T::Ident, T::Unknown, T::Unknown, T::Ident]
    );
}

#[test]
fn non_breaking_space_gets_specific_help() {
    let lexed = lex("a\u{A0}b");
    assert_eq!(lexed.codes(), ["P1001"]);
    assert!(lexed.diagnostics[0].help[0].contains("whitespace"));
}

#[test]
fn typographic_quotes_get_specific_help() {
    let lexed = lex("\u{201C}hi\u{201D}");
    assert_eq!(lexed.codes(), ["P1001", "P1001"]);
    assert!(lexed.diagnostics[0].help[0].contains('"'));
}

#[test]
fn bidirectional_characters_are_rejected_everywhere() {
    for source in [
        "a \u{202E} b",
        "// comment \u{2066}\na",
        "/* \u{2069} */ a",
        "\"text \u{202D}\"",
        "'\u{200F}'",
        "a\u{061C}",
    ] {
        let lexed = lex(source);
        assert_eq!(lexed.codes(), ["P1009"], "{source:?}");
    }
}

#[test]
fn bidirectional_escape_in_string_is_allowed() {
    lex(r#""\u{202E}""#).assert_clean();
}

/// Vertical tab, form feed, next line, line separator, paragraph separator,
/// NUL, escape, delete and a C1 control (CSI).
const FORBIDDEN_CONTROLS: [char; 9] = [
    '\u{B}', '\u{C}', '\u{85}', '\u{2028}', '\u{2029}', '\u{0}', '\u{1B}', '\u{7F}', '\u{9B}',
];

#[test]
fn control_characters_and_line_separators_are_rejected_everywhere() {
    for c in FORBIDDEN_CONTROLS {
        for source in [
            format!("a {c} b"),
            format!("// comment {c} text\na"),
            format!("/* {c} */ a"),
            format!("\"text {c} more\""),
            format!("'{c}'"),
        ] {
            let lexed = lex(&source);
            assert_eq!(lexed.codes(), ["P1012"], "{source:?}");
            assert_eq!(lexed.labelled(), [c.to_string()], "{source:?}");
        }
    }
}

#[test]
fn line_separator_cannot_hide_code_in_a_comment() {
    // An editor that breaks the line at U+2028 shows `return grant()` as
    // code; the compiler reads it as part of the comment.
    let lexed = lex("func f() {\n    // check access\u{2028}return grant()\n}");
    assert_eq!(lexed.codes(), ["P1012"]);
    assert!(!lexed.kinds().contains(&kw(Keyword::Return)));
    let diagnostic = &lexed.diagnostics[0];
    assert_eq!(diagnostic.message, "line separator U+2028 in source");
    assert!(diagnostic.labels[0].message.contains("line break"));
    assert_eq!(
        diagnostic.help,
        ["inside a string or character literal, write the character as the escape `\\u{2028}`"]
    );
}

#[test]
fn other_control_characters_are_described_as_invisible() {
    let lexed = lex("// \u{1B}[2K");
    assert_eq!(lexed.codes(), ["P1012"]);
    assert_eq!(
        lexed.diagnostics[0].message,
        "control character U+001B in source"
    );
    assert!(lexed.diagnostics[0].labels[0].message.contains("terminal"));
}

#[test]
fn escapes_may_produce_forbidden_characters() {
    lex(r#""\u{2028}\u{2029}\u{B}\u{C}\u{85}\u{1B}\u{7F}\0" '\u{B}'"#).assert_clean();
}

#[test]
fn tab_is_allowed_in_comments_and_literals() {
    lex("\"a\tb\" '\t' // c\td\n/* e\tf */").assert_clean();
}

#[test]
fn forbidden_control_in_code_is_one_unknown_token_and_one_diagnostic() {
    let lexed = lex("a\u{C}b");
    assert_eq!(lexed.codes(), ["P1012"]);
    assert_eq!(lexed.kinds(), vec![T::Ident, T::Unknown, T::Ident]);
    assert_eq!(lexed.pairs()[1], (T::Unknown, "\u{C}"));
}

#[test]
fn backslash_before_forbidden_control_is_not_an_escape() {
    let lexed = lex("\"a\\\u{B}b\"");
    assert_eq!(lexed.codes(), ["P1012"]);
    assert_eq!(lexed.kinds(), vec![T::String]);
}

#[test]
fn lone_carriage_return_is_rejected() {
    let lexed = lex("a\rb");
    assert_eq!(lexed.codes(), ["P1010"]);
    assert_eq!(lexed.kinds(), vec![T::Ident, T::Ident]);
}

#[test]
fn diagnostics_are_reported_in_source_order() {
    // The bidirectional character is found by a separate pass but must still
    // be reported after the earlier unknown character.
    let lexed = lex("# \"x\" \u{202E} $");
    assert_eq!(lexed.codes(), ["P1001", "P1009", "P1001"]);
}

#[test]
fn lexing_continues_after_errors() {
    let lexed = lex("let x = 0b2\nlet y = \"ok\"\nlet z = 'zz'\nfunc");
    assert_eq!(lexed.codes(), ["P1006", "P1008"]);
    assert_eq!(lexed.kinds().last(), Some(&kw(Keyword::Func)));
}

// Diagnostic limit

fn lex_limited(text: &str, limit: usize) -> (Vec<Token>, Diagnostics) {
    let mut map = SourceMap::new();
    let file = map.add("limited.pgr", text).unwrap();
    let mut diagnostics = Diagnostics::with_limit(limit);
    let tokens = tokenize(file, map.file(file).text(), &mut diagnostics);
    (tokens, diagnostics)
}

#[test]
fn diagnostics_beyond_the_limit_are_counted_but_not_stored() {
    let (tokens, diagnostics) = lex_limited(&"$".repeat(10_000), 10);
    // Recovery is unaffected: one token per character, then end of file.
    assert_eq!(tokens.len(), 10_001);
    assert_eq!(diagnostics.len(), 10);
    assert_eq!(diagnostics.error_count(), 10_000);
    assert_eq!(diagnostics.suppressed_count(), 9_990);
    let starts: Vec<u32> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.main_label().unwrap().span.start)
        .collect();
    assert_eq!(starts, (0..10).collect::<Vec<u32>>());
}

#[test]
fn limit_keeps_the_earliest_diagnostics_in_source_order() {
    // The bidirectional character is found by the first pass, the unknown
    // characters by the second; the unterminated string is reported after
    // the invalid escape inside it, although it starts earlier.
    let source = "a\u{202E} $ \"\\q $";
    let unlimited = lex(source);
    assert_eq!(unlimited.codes(), ["P1009", "P1001", "P1002", "P1005"]);
    for limit in 1..=4 {
        let (_, limited) = lex_limited(source, limit);
        let codes: Vec<String> = limited
            .iter()
            .map(|diagnostic| diagnostic.code.unwrap().to_string())
            .collect();
        assert_eq!(codes, unlimited.codes()[..limit], "limit {limit}");
        assert_eq!(limited.error_count(), 4, "limit {limit}");
    }
}

// Regression tests

#[test]
fn zero_followed_by_separator_is_not_a_leading_zero() {
    let lexed = lex("0_");
    assert_eq!(lexed.codes(), ["P1006"]);
}

#[test]
fn suffix_after_complete_exponent_is_an_invalid_suffix() {
    let lexed = lex("1e5e");
    assert_eq!(lexed.codes(), ["P1006"]);
    assert_eq!(lexed.labelled(), ["e"]);
    assert!(
        lexed.diagnostics[0].message.contains("invalid suffix"),
        "{:?}",
        lexed.diagnostics
    );
}

#[test]
fn backslash_before_lone_carriage_return_is_not_an_escape() {
    // A quote, `a`, a backslash, a lone carriage return, `b`, a quote.
    let source: String = ['"', 'a', '\\', '\r', 'b', '"'].iter().collect();
    let lexed = lex(&source);
    assert_eq!(lexed.codes(), ["P1010"]);
    assert_eq!(lexed.kinds(), vec![T::String]);
}

#[test]
fn decomposed_non_ascii_identifier_is_one_token() {
    // "cafe" followed by U+0301 COMBINING ACUTE ACCENT, then "s".
    let lexed = lex("let cafe\u{301}s = 1");
    assert_eq!(lexed.codes(), ["P1011"]);
    assert_eq!(lexed.labelled(), ["cafe\u{301}s"]);
    assert_eq!(
        lexed.kinds(),
        vec![kw(Keyword::Let), T::Ident, T::Eq, T::Int]
    );
}
