//! The registry of stable diagnostic codes.
//!
//! Codes have the form `P` followed by four digits. The leading digit
//! identifies the compiler stage that reports the diagnostic:
//!
//! | Range         | Stage                                   |
//! |---------------|-----------------------------------------|
//! | `P0001–P0999` | Driver, command line and file loading   |
//! | `P1001–P1999` | Lexical analysis                        |
//! | `P2001–P2999` | Syntax analysis (parsing)               |
//! | `P3001–P3999` | Name resolution and modules             |
//! | `P4001–P4999` | Type checking                           |
//! | `P5001–P5999` | Memory and ownership analysis           |
//! | `P6001–P6999` | Code generation and linking             |
//! | `P9001–P9999` | Internal compiler errors                |
//!
//! A code, once released, is never reused for a different problem. Every code
//! must be listed in [`REGISTRY`]; the tests in this module enforce that codes
//! are unique and fall within a documented range.

use std::fmt;

/// A stable diagnostic identifier such as `P1001`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(u16);

impl Code {
    /// Creates a code from its numeric part.
    ///
    /// # Panics
    ///
    /// Panics (at compile time when used in a constant) if `number` is `0` or
    /// has more than four digits.
    #[must_use]
    pub const fn new(number: u16) -> Self {
        assert!(
            number >= 1 && number <= 9999,
            "diagnostic codes are P0001-P9999"
        );
        Self(number)
    }

    /// Returns the numeric part of the code.
    #[must_use]
    pub const fn number(self) -> u16 {
        self.0
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{:04}", self.0)
    }
}

/// Documentation for a registered code.
#[derive(Clone, Copy, Debug)]
pub struct CodeInfo {
    /// The code.
    pub code: Code,
    /// A one-line description of the problem the code identifies.
    pub summary: &'static str,
}

macro_rules! define_codes {
    ($($(#[doc = $doc:literal])* $name:ident = $number:literal, $summary:literal;)*) => {
        $(
            $(#[doc = $doc])*
            pub const $name: Code = Code::new($number);
        )*

        /// Every registered diagnostic code, in ascending order.
        pub const REGISTRY: &[CodeInfo] = &[
            $(CodeInfo { code: $name, summary: $summary },)*
        ];
    };
}

define_codes! {
    /// A source file could not be read from disk.
    CANNOT_READ_FILE = 1, "source file could not be read";
    /// A source file is not valid UTF-8.
    INVALID_UTF8 = 2, "source file is not valid UTF-8";
    /// A source file exceeds the maximum supported size.
    FILE_TOO_LARGE = 3, "source file exceeds the maximum supported size";
    /// The diagnostic limit was reached; further diagnostics were counted
    /// but not reported.
    DIAGNOSTICS_SUPPRESSED = 4, "further diagnostics suppressed after reaching the limit";

    /// A character that cannot start any token.
    UNKNOWN_CHARACTER = 1001, "character not valid in this position";
    /// A string literal without a closing quote on the same line.
    UNTERMINATED_STRING = 1002, "unterminated string literal";
    /// A block comment without a matching `*/`.
    UNTERMINATED_BLOCK_COMMENT = 1003, "unterminated block comment";
    /// A character literal without a closing quote.
    UNTERMINATED_CHAR = 1004, "unterminated character literal";
    /// An escape sequence that is not defined by the language.
    INVALID_ESCAPE = 1005, "invalid escape sequence";
    /// A numeric literal with malformed digits, separators or exponent.
    INVALID_NUMBER = 1006, "malformed numeric literal";
    /// A decimal literal with a redundant leading zero.
    LEADING_ZERO = 1007, "decimal literal with a leading zero";
    /// A character literal that does not contain exactly one character.
    INVALID_CHAR_LITERAL = 1008, "character literal must contain exactly one character";
    /// A Unicode bidirectional formatting character in source text.
    BIDI_CONTROL_CHARACTER = 1009, "bidirectional formatting character in source";
    /// A carriage return that is not followed by a line feed.
    LONE_CARRIAGE_RETURN = 1010, "carriage return not followed by a line feed";
    /// An identifier containing non-ASCII characters.
    NON_ASCII_IDENTIFIER = 1011, "non-ASCII identifiers are not supported";
    /// A control character other than tab and line breaks, or U+2028 or
    /// U+2029, written directly in source text.
    FORBIDDEN_CONTROL_CHARACTER = 1012, "control character or Unicode line separator in source";

    /// A token that cannot start a top-level item.
    EXPECTED_ITEM = 2001, "expected `import`, `func` or `const`";
    /// A token other than an identifier where a name is required.
    EXPECTED_IDENTIFIER = 2002, "expected identifier";
    /// Two items or statements on one line without `;` between them.
    MISSING_SEPARATOR = 2003, "expected a line break or `;`";
    /// A token that cannot start an expression where one is required.
    EXPECTED_EXPRESSION = 2004, "expected expression";
    /// A specific token, such as `=` or `)`, is missing.
    EXPECTED_TOKEN = 2005, "expected a different token";
    /// A `let`, `var` or `const` declaration without `=` and a value.
    MISSING_INITIALIZER = 2006, "declaration without an initializer";
    /// A token that cannot start a type where one is required.
    EXPECTED_TYPE = 2007, "expected type";
    /// An opening delimiter whose closing delimiter is missing.
    UNCLOSED_DELIMITER = 2008, "unclosed delimiter";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn codes_are_unique_and_sorted() {
        let mut seen = HashSet::new();
        for pair in REGISTRY.windows(2) {
            assert!(
                pair[0].code < pair[1].code,
                "{} is not below {}",
                pair[0].code,
                pair[1].code
            );
        }
        for info in REGISTRY {
            assert!(seen.insert(info.code), "duplicate code {}", info.code);
        }
    }

    #[test]
    fn codes_fall_in_documented_ranges() {
        for info in REGISTRY {
            let stage = info.code.number() / 1000;
            assert!(
                matches!(stage, 0..=6 | 9),
                "{} is outside every documented range",
                info.code
            );
            assert_ne!(
                info.code.number() % 1000,
                0,
                "{} uses a reserved x000 slot",
                info.code
            );
            assert!(!info.summary.is_empty());
        }
    }

    #[test]
    fn display_is_zero_padded() {
        assert_eq!(CANNOT_READ_FILE.to_string(), "P0001");
        assert_eq!(UNKNOWN_CHARACTER.to_string(), "P1001");
    }
}
