//! Token definitions.

use std::fmt;

use pinguer_source::Span;

/// A lexical token: a kind and the source range it was read from.
///
/// Tokens do not store their text or literal value. The text is recovered from
/// the source through the span, and literal values are decoded by later
/// stages, which know the type the literal is expected to have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// The bytes the token was read from.
    pub span: Span,
}

/// Every keyword recognised by the lexer.
///
/// Keywords that have no meaning in Pinguer v0.1 are still reserved, so that
/// introducing them later does not break existing programs. See
/// [`Keyword::is_reserved_for_future_use`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Keyword {
    // Keywords with meaning in v0.1.
    /// `as`
    As,
    /// `break`
    Break,
    /// `const`
    Const,
    /// `continue`
    Continue,
    /// `else`
    Else,
    /// `false`
    False,
    /// `func`
    Func,
    /// `if`
    If,
    /// `import`
    Import,
    /// `let`
    Let,
    /// `return`
    Return,
    /// `true`
    True,
    /// `var`
    Var,
    /// `while`
    While,

    // Keywords reserved for future language features.
    /// `actor`
    Actor,
    /// `async`
    Async,
    /// `await`
    Await,
    /// `class`
    Class,
    /// `comptime`
    Comptime,
    /// `defer`
    Defer,
    /// `enum`
    Enum,
    /// `extern`
    Extern,
    /// `for`
    For,
    /// `impl`
    Impl,
    /// `in`
    In,
    /// `interface`
    Interface,
    /// `loop`
    Loop,
    /// `match`
    Match,
    /// `move`
    Move,
    /// `private`
    Private,
    /// `public`
    Public,
    /// `self`
    SelfValue,
    /// `Self`
    SelfType,
    /// `spawn`
    Spawn,
    /// `static`
    Static,
    /// `struct`
    Struct,
    /// `try`
    Try,
    /// `type`
    Type,
    /// `unsafe`
    Unsafe,
    /// `where`
    Where,
    /// `yield`
    Yield,
}

impl Keyword {
    /// Every keyword, in declaration order.
    pub const ALL: &'static [Keyword] = &[
        Keyword::As,
        Keyword::Break,
        Keyword::Const,
        Keyword::Continue,
        Keyword::Else,
        Keyword::False,
        Keyword::Func,
        Keyword::If,
        Keyword::Import,
        Keyword::Let,
        Keyword::Return,
        Keyword::True,
        Keyword::Var,
        Keyword::While,
        Keyword::Actor,
        Keyword::Async,
        Keyword::Await,
        Keyword::Class,
        Keyword::Comptime,
        Keyword::Defer,
        Keyword::Enum,
        Keyword::Extern,
        Keyword::For,
        Keyword::Impl,
        Keyword::In,
        Keyword::Interface,
        Keyword::Loop,
        Keyword::Match,
        Keyword::Move,
        Keyword::Private,
        Keyword::Public,
        Keyword::SelfValue,
        Keyword::SelfType,
        Keyword::Spawn,
        Keyword::Static,
        Keyword::Struct,
        Keyword::Try,
        Keyword::Type,
        Keyword::Unsafe,
        Keyword::Where,
        Keyword::Yield,
    ];

    /// Returns the keyword spelled `text`, if any.
    #[must_use]
    pub fn lookup(text: &str) -> Option<Keyword> {
        Keyword::ALL
            .iter()
            .copied()
            .find(|keyword| keyword.as_str() == text)
    }

    /// Returns the source spelling of the keyword.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Keyword::As => "as",
            Keyword::Break => "break",
            Keyword::Const => "const",
            Keyword::Continue => "continue",
            Keyword::Else => "else",
            Keyword::False => "false",
            Keyword::Func => "func",
            Keyword::If => "if",
            Keyword::Import => "import",
            Keyword::Let => "let",
            Keyword::Return => "return",
            Keyword::True => "true",
            Keyword::Var => "var",
            Keyword::While => "while",
            Keyword::Actor => "actor",
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Class => "class",
            Keyword::Comptime => "comptime",
            Keyword::Defer => "defer",
            Keyword::Enum => "enum",
            Keyword::Extern => "extern",
            Keyword::For => "for",
            Keyword::Impl => "impl",
            Keyword::In => "in",
            Keyword::Interface => "interface",
            Keyword::Loop => "loop",
            Keyword::Match => "match",
            Keyword::Move => "move",
            Keyword::Private => "private",
            Keyword::Public => "public",
            Keyword::SelfValue => "self",
            Keyword::SelfType => "Self",
            Keyword::Spawn => "spawn",
            Keyword::Static => "static",
            Keyword::Struct => "struct",
            Keyword::Try => "try",
            Keyword::Type => "type",
            Keyword::Unsafe => "unsafe",
            Keyword::Where => "where",
            Keyword::Yield => "yield",
        }
    }

    /// Returns `true` if the keyword is reserved but has no meaning in v0.1.
    ///
    /// The parser reports a dedicated diagnostic when it meets such a keyword.
    #[must_use]
    pub fn is_reserved_for_future_use(self) -> bool {
        self >= Keyword::Actor
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kind of a [`Token`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// An identifier such as `counter` or `_value`.
    Ident,
    /// A keyword.
    Keyword(Keyword),
    /// A lone `_`.
    Underscore,
    /// An integer literal such as `42`, `1_000`, `0xFF`, `0o17` or `0b1010`.
    Int,
    /// A floating-point literal such as `1.5`, `2e10` or `6.02e-23`.
    Float,
    /// A string literal such as `"Hello"`, including its quotes.
    String,
    /// A character literal such as `'a'` or `'\n'`, including its quotes.
    Char,

    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `..=`
    DotDotEq,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,
    /// `?`
    Question,
    /// `??`
    QuestionQuestion,
    /// `?.`
    QuestionDot,
    /// `@`
    At,

    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `&`
    Amp,
    /// `|`
    Pipe,
    /// `^`
    Caret,
    /// `~`
    Tilde,
    /// `!`
    Bang,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `&&`
    AmpAmp,
    /// `||`
    PipePipe,
    /// `**`
    StarStar,
    /// `|>`
    PipeGt,

    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!=`
    BangEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,

    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,
    /// `*=`
    StarEq,
    /// `/=`
    SlashEq,
    /// `%=`
    PercentEq,
    /// `&=`
    AmpEq,
    /// `|=`
    PipeEq,
    /// `^=`
    CaretEq,
    /// `<<=`
    ShlEq,
    /// `>>=`
    ShrEq,
    /// `**=`
    StarStarEq,

    /// One or more line breaks that may terminate a statement.
    ///
    /// See the crate documentation for when line breaks produce this token.
    Newline,
    /// Input that does not form any valid token. A diagnostic has been
    /// reported for it.
    Unknown,
    /// The end of the input. Always the last token, with an empty span.
    Eof,
}

impl TokenKind {
    /// Every punctuation and operator kind with its spelling. This table is
    /// the single source of truth used by the lexer (longest match) and by
    /// [`TokenKind::punctuation`].
    pub const PUNCTUATION: &'static [(TokenKind, &'static str)] = &[
        (TokenKind::LParen, "("),
        (TokenKind::RParen, ")"),
        (TokenKind::LBrace, "{"),
        (TokenKind::RBrace, "}"),
        (TokenKind::LBracket, "["),
        (TokenKind::RBracket, "]"),
        (TokenKind::Comma, ","),
        (TokenKind::Dot, "."),
        (TokenKind::DotDot, ".."),
        (TokenKind::DotDotEq, "..="),
        (TokenKind::Colon, ":"),
        (TokenKind::Semicolon, ";"),
        (TokenKind::Arrow, "->"),
        (TokenKind::FatArrow, "=>"),
        (TokenKind::Question, "?"),
        (TokenKind::QuestionQuestion, "??"),
        (TokenKind::QuestionDot, "?."),
        (TokenKind::At, "@"),
        (TokenKind::Plus, "+"),
        (TokenKind::Minus, "-"),
        (TokenKind::Star, "*"),
        (TokenKind::Slash, "/"),
        (TokenKind::Percent, "%"),
        (TokenKind::Amp, "&"),
        (TokenKind::Pipe, "|"),
        (TokenKind::Caret, "^"),
        (TokenKind::Tilde, "~"),
        (TokenKind::Bang, "!"),
        (TokenKind::Shl, "<<"),
        (TokenKind::Shr, ">>"),
        (TokenKind::AmpAmp, "&&"),
        (TokenKind::PipePipe, "||"),
        (TokenKind::StarStar, "**"),
        (TokenKind::PipeGt, "|>"),
        (TokenKind::Eq, "="),
        (TokenKind::EqEq, "=="),
        (TokenKind::BangEq, "!="),
        (TokenKind::Lt, "<"),
        (TokenKind::LtEq, "<="),
        (TokenKind::Gt, ">"),
        (TokenKind::GtEq, ">="),
        (TokenKind::PlusEq, "+="),
        (TokenKind::MinusEq, "-="),
        (TokenKind::StarEq, "*="),
        (TokenKind::SlashEq, "/="),
        (TokenKind::PercentEq, "%="),
        (TokenKind::AmpEq, "&="),
        (TokenKind::PipeEq, "|="),
        (TokenKind::CaretEq, "^="),
        (TokenKind::ShlEq, "<<="),
        (TokenKind::ShrEq, ">>="),
        (TokenKind::StarStarEq, "**="),
    ];

    /// Returns the fixed spelling of punctuation and operator tokens, as
    /// listed in [`TokenKind::PUNCTUATION`].
    #[must_use]
    pub fn punctuation(self) -> Option<&'static str> {
        TokenKind::PUNCTUATION
            .iter()
            .find(|(kind, _)| *kind == self)
            .map(|&(_, spelling)| spelling)
    }
}

impl fmt::Display for TokenKind {
    /// Describes the token kind the way diagnostics refer to it, for example
    /// ``identifier``, `` `func` `` or `` `(` ``.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(spelling) = self.punctuation() {
            return write!(f, "`{spelling}`");
        }
        match self {
            TokenKind::Ident => f.write_str("identifier"),
            TokenKind::Keyword(keyword) => write!(f, "`{keyword}`"),
            TokenKind::Underscore => f.write_str("`_`"),
            TokenKind::Int => f.write_str("integer literal"),
            TokenKind::Float => f.write_str("floating-point literal"),
            TokenKind::String => f.write_str("string literal"),
            TokenKind::Char => f.write_str("character literal"),
            TokenKind::Newline => f.write_str("newline"),
            TokenKind::Unknown => f.write_str("unknown token"),
            TokenKind::Eof => f.write_str("end of file"),
            _ => unreachable!("punctuation is handled above"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_round_trip() {
        for &keyword in Keyword::ALL {
            assert_eq!(Keyword::lookup(keyword.as_str()), Some(keyword));
        }
        assert_eq!(Keyword::lookup("Func"), None);
        assert_eq!(Keyword::lookup("int"), None);
    }

    #[test]
    fn keyword_list_is_complete_and_ordered() {
        // `ALL` must follow declaration order so that the reserved-keyword
        // boundary used by `is_reserved_for_future_use` is meaningful.
        for pair in Keyword::ALL.windows(2) {
            assert!(pair[0] < pair[1], "{} before {}", pair[0], pair[1]);
        }
        assert_eq!(Keyword::ALL.len(), 41);
    }

    #[test]
    fn reserved_keywords_are_split_correctly() {
        assert!(!Keyword::Func.is_reserved_for_future_use());
        assert!(!Keyword::While.is_reserved_for_future_use());
        assert!(Keyword::Struct.is_reserved_for_future_use());
        assert!(Keyword::Match.is_reserved_for_future_use());
        assert!(Keyword::Actor.is_reserved_for_future_use());
    }

    #[test]
    fn punctuation_table_is_consistent() {
        let mut kinds = std::collections::HashSet::new();
        let mut spellings = std::collections::HashSet::new();
        for &(kind, spelling) in TokenKind::PUNCTUATION {
            assert!(kinds.insert(kind), "duplicate kind {kind:?}");
            assert!(spellings.insert(spelling), "duplicate spelling {spelling}");
            assert_eq!(kind.punctuation(), Some(spelling));
        }
        assert_eq!(TokenKind::PUNCTUATION.len(), 52);
        // Kinds outside the table have no fixed spelling.
        for kind in [
            TokenKind::Ident,
            TokenKind::Int,
            TokenKind::Newline,
            TokenKind::Eof,
        ] {
            assert_eq!(kind.punctuation(), None);
        }
    }

    #[test]
    fn display_quotes_fixed_spellings() {
        assert_eq!(TokenKind::Ident.to_string(), "identifier");
        assert_eq!(TokenKind::Keyword(Keyword::Func).to_string(), "`func`");
        assert_eq!(TokenKind::ShrEq.to_string(), "`>>=`");
        assert_eq!(TokenKind::Eof.to_string(), "end of file");
    }
}
