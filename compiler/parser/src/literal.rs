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

/// Decodes a floating-point literal such as `1.5`, `6.02e23` or `1e-9` into
/// its digits without separators.
///
/// The value is not computed here: rounding happens once, when the type
/// checker parses these digits at the expected type (`f32` or `f64`) with
/// the standard library, which rounds correctly and does not depend on the
/// locale.
pub(crate) fn decode_float(text: &str) -> Option<Box<str>> {
    // `float_literal = decimal_part , "." , digit_run , [ exponent ]
    //                | decimal_part , exponent`
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(index) => (&text[..index], Some(&text[index + 1..])),
        None => (text, None),
    };
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (mantissa, None),
    };
    if fraction.is_none() && exponent.is_none() {
        return None;
    }
    if integer.len() > 1 && integer.starts_with('0') {
        return None;
    }
    digit_run(integer, 10)?;
    if let Some(fraction) = fraction {
        digit_run(fraction, 10)?;
    }
    if let Some(exponent) = exponent {
        digit_run(exponent.strip_prefix(['+', '-']).unwrap_or(exponent), 10)?;
    }
    Some(text.replace('_', "").into_boxed_str())
}

/// Decodes a string literal, quotes included, into its value with escape
/// sequences resolved.
///
/// Characters that the lexer rejects anywhere in a file (LEXICAL_STRUCTURE.md
/// §3) are not checked again: the lexer has reported them, and they do not
/// change the structure of the literal.
pub(crate) fn decode_string(text: &str) -> Option<Box<str>> {
    let mut chars = text.strip_prefix('"')?.chars();
    let mut value = String::with_capacity(text.len());
    loop {
        match chars.next()? {
            '"' => return chars.as_str().is_empty().then(|| value.into_boxed_str()),
            '\\' => value.push(escape(&mut chars)?),
            c => value.push(c),
        }
    }
}

/// Decodes the escape sequence after a `\`: `\n \r \t \\ \0 \" \'` or
/// `\u{X}` with one to six hexadecimal digits naming a Unicode scalar value.
fn escape(chars: &mut std::str::Chars<'_>) -> Option<char> {
    Some(match chars.next()? {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' => '\\',
        '0' => '\0',
        '"' => '"',
        '\'' => '\'',
        'u' => {
            if chars.next()? != '{' {
                return None;
            }
            let mut value = 0u32;
            let mut digits = 0;
            loop {
                let c = chars.next()?;
                if c == '}' {
                    break;
                }
                digits += 1;
                if digits > 6 {
                    return None;
                }
                value = value * 16 + c.to_digit(16)?;
            }
            if digits == 0 {
                return None;
            }
            char::from_u32(value)?
        }
        _ => return None,
    })
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
    fn floats_keep_their_digits_without_separators() {
        for (text, digits) in [
            ("1.5", "1.5"),
            ("0.25", "0.25"),
            ("6.02e23", "6.02e23"),
            ("1e-9", "1e-9"),
            ("2E+10", "2E+10"),
            ("1_000.000_1", "1000.0001"),
            ("1e1_0", "1e10"),
        ] {
            let decoded = decode_float(text);
            assert_eq!(decoded.as_deref(), Some(digits), "{text}");
            // The type checker can parse every decoded literal.
            assert!(digits.parse::<f64>().is_ok() && digits.parse::<f32>().is_ok());
        }
    }

    #[test]
    fn malformed_floats_do_not_decode() {
        for text in [
            "1", "1.", "1e", "1e+", "2.5E-", "01.5", "00.5", "1._5", "1_.5", "1.5_", "1.5f32",
            "1e5e", "1.5.2", "0x1.5",
        ] {
            assert_eq!(decode_float(text), None, "{text}");
        }
    }

    #[test]
    fn strings_resolve_every_escape() {
        let text = r#""a\n\r\t\\\0\"\'\u{41}\u{1F427}\u{10FFFF}é""#;
        assert_eq!(
            decode_string(text).as_deref(),
            Some("a\n\r\t\\\0\"'A🐧\u{10FFFF}é")
        );
        assert_eq!(decode_string(r#""""#).as_deref(), Some(""));
        // A quote that is not escaped ends the literal, so `'` is ordinary.
        assert_eq!(decode_string(r#""it's""#).as_deref(), Some("it's"));
    }

    #[test]
    fn malformed_strings_do_not_decode() {
        for text in [
            r#"""#,
            r#""abc"#,
            r#""abc\""#,
            r#""\q""#,
            r#""\u41""#,
            r#""\u{}""#,
            r#""\u{1234567}""#,
            r#""\u{D800}""#,
            r#""\u{110000}""#,
            r#""\u{12G4}""#,
            r#""\u{41""#,
            r#""a"b""#,
            "abc",
        ] {
            assert_eq!(decode_string(text), None, "{text}");
        }
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
