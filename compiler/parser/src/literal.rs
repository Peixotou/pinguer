//! Decoding of literal tokens into values.
//!
//! Each decoder accepts exactly the spelling that
//! `specifications/LEXICAL_STRUCTURE.md` §5 defines and returns `None` for
//! anything else. The lexer reports every malformed literal and still
//! produces a token for it, so a literal that does not decode has already
//! been diagnosed and becomes an error expression without a new diagnostic.

use pinguer_ast::Base;

/// Decodes an integer literal such as `1_000`, `0xFF`, `0o17` or `0b1010`.
///
/// Returns the value, or `Some(None)` with the base when the literal is
/// well formed but its value needs more than 128 bits; the type checker
/// reports that it fits no type. Never truncates.
pub(crate) fn decode_int(text: &str) -> Option<(Option<u128>, Base)> {
    let (base, digits) = match text.as_bytes() {
        [b'0', b'x', ..] => (Base::Hexadecimal, &text[2..]),
        [b'0', b'o', ..] => (Base::Octal, &text[2..]),
        [b'0', b'b', ..] => (Base::Binary, &text[2..]),
        _ => {
            // `decimal_literal = "0" | nonzero_digit , { [ "_" ] , digit }`
            if text.len() > 1 && text.starts_with('0') {
                return None;
            }
            (Base::Decimal, text)
        }
    };
    let value = digit_run(digits, base.radix())?;
    Some((value, base))
}

/// Decodes `digit , { [ "_" ] , digit }` in `radix`. Returns `None` if
/// `text` does not have that form, and `Some(None)` if the value does not
/// fit in a `u128`.
fn digit_run(text: &str, radix: u32) -> Option<Option<u128>> {
    let mut value = Some(0u128);
    let mut previous_was_digit = false;
    for c in text.chars() {
        if c == '_' {
            if !previous_was_digit {
                return None;
            }
            previous_was_digit = false;
            continue;
        }
        let digit = c.to_digit(radix)?;
        value = value
            .and_then(|value| value.checked_mul(u128::from(radix)))
            .and_then(|value| value.checked_add(u128::from(digit)));
        previous_was_digit = true;
    }
    // Empty, or ending with a separator.
    previous_was_digit.then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_in_every_base() {
        for (text, value, base) in [
            ("0", 0, Base::Decimal),
            ("42", 42, Base::Decimal),
            ("1_000_000", 1_000_000, Base::Decimal),
            ("0xFF", 255, Base::Hexadecimal),
            ("0xdead_BEEF", 0xDEAD_BEEF, Base::Hexadecimal),
            ("0o17", 15, Base::Octal),
            ("0b1010", 10, Base::Binary),
            ("0b0", 0, Base::Binary),
            ("0x0_0", 0, Base::Hexadecimal),
        ] {
            assert_eq!(decode_int(text), Some((Some(value), base)), "{text}");
        }
    }

    #[test]
    fn values_up_to_128_bits_are_exact() {
        let max = u128::MAX.to_string();
        assert_eq!(decode_int(&max), Some((Some(u128::MAX), Base::Decimal)));
        assert_eq!(
            decode_int("18446744073709551616"),
            Some((Some(1 << 64), Base::Decimal))
        );
        let hex = format!("0x{}", "F".repeat(32));
        assert_eq!(decode_int(&hex), Some((Some(u128::MAX), Base::Hexadecimal)));
    }

    #[test]
    fn larger_values_are_kept_as_too_large_not_truncated() {
        // u128::MAX + 1, and far larger values.
        assert_eq!(
            decode_int("340282366920938463463374607431768211456"),
            Some((None, Base::Decimal))
        );
        let long = "9".repeat(10_000);
        assert_eq!(decode_int(&long), Some((None, Base::Decimal)));
        let hex = format!("0x1{}", "0".repeat(32));
        assert_eq!(decode_int(&hex), Some((None, Base::Hexadecimal)));
    }

    #[test]
    fn malformed_literals_do_not_decode() {
        // Each of these is reported by the lexer.
        for text in [
            "", "007", "00", "0x", "0b", "0o_", "0b102", "0o8", "0xG", "1__0", "1_", "_1", "0x_1",
            "10u8", "0X1F", "0B1", "1e5", "1.5",
        ] {
            assert_eq!(decode_int(text), None, "{text}");
        }
    }
}
