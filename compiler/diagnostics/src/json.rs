//! Renders diagnostics as machine-readable JSON.
//!
//! Each diagnostic becomes one JSON object on a single line (the
//! [JSON Lines](https://jsonlines.org/) convention), so that tools can process
//! diagnostics as a stream. The schema is documented in
//! `docs/architecture/DIAGNOSTICS.md` and is versioned through the
//! `format_version` field; fields may be added within a version but are never
//! removed or repurposed.
//!
//! ```json
//! {"format_version":1,"severity":"error","code":"P1002","message":"unterminated string literal","labels":[{"file":"main.pgr","byte_start":14,"byte_end":20,"line_start":1,"column_start":15,"line_end":1,"column_end":21,"style":"primary","message":"missing closing `\"`"}],"notes":["string literals cannot contain line breaks; use `\\n` instead"],"help":[]}
//! ```
//!
//! Lines and columns are 1-based and columns count Unicode scalar values.
//! `line_end`/`column_end` describe the position just past the span.

use std::fmt::Write as _;

use pinguer_source::SourceMap;

use crate::{Diagnostic, LabelStyle};

/// The version of the JSON diagnostic schema produced by this module.
pub const FORMAT_VERSION: u32 = 1;

/// Renders one diagnostic as a single-line JSON object without a trailing
/// newline.
///
/// # Panics
///
/// Panics if a label refers to a file that is not part of `map`.
#[must_use]
pub fn render(map: &SourceMap, diagnostic: &Diagnostic) -> String {
    let mut out = String::new();
    let _ = write!(out, "{{\"format_version\":{FORMAT_VERSION},\"severity\":");
    write_string(&mut out, diagnostic.severity.as_str());
    out.push_str(",\"code\":");
    match diagnostic.code {
        Some(code) => write_string(&mut out, &code.to_string()),
        None => out.push_str("null"),
    }
    out.push_str(",\"message\":");
    write_string(&mut out, &diagnostic.message);

    out.push_str(",\"labels\":[");
    for (index, label) in diagnostic.labels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let file = map.file(label.span.file);
        let start = file.line_col(label.span.start);
        let end = file.line_col(label.span.end);
        out.push_str("{\"file\":");
        write_string(&mut out, file.name());
        let _ = write!(
            out,
            ",\"byte_start\":{},\"byte_end\":{},\"line_start\":{},\"column_start\":{},\"line_end\":{},\"column_end\":{},\"style\":",
            label.span.start, label.span.end, start.line, start.column, end.line, end.column
        );
        write_string(
            &mut out,
            match label.style {
                LabelStyle::Primary => "primary",
                LabelStyle::Secondary => "secondary",
            },
        );
        out.push_str(",\"message\":");
        write_string(&mut out, &label.message);
        out.push('}');
    }
    out.push(']');

    out.push_str(",\"notes\":");
    write_string_array(&mut out, &diagnostic.notes);
    out.push_str(",\"help\":");
    write_string_array(&mut out, &diagnostic.help);
    out.push('}');
    out
}

/// Renders several diagnostics, one JSON object per line. Every line,
/// including the last, ends with a newline.
#[must_use]
pub fn render_all<'a>(
    map: &SourceMap,
    diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        out.push_str(&render(map, diagnostic));
        out.push('\n');
    }
    out
}

fn write_string_array(out: &mut String, items: &[String]) {
    out.push('[');
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        write_string(out, item);
    }
    out.push(']');
}

/// Writes `value` as a JSON string literal (RFC 8259).
///
/// Besides the mandatory escapes, every character that the human renderer
/// considers unsafe for a terminal is escaped as well: DEL and C1 controls,
/// bidirectional formatting characters, the other invisible characters and
/// U+2028 and U+2029 (which also keeps the output valid JavaScript).
/// Characters outside the Basic Multilingual Plane are escaped as a UTF-16
/// surrogate pair, the only form JSON allows for them.
fn write_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if crate::human::needs_escape(c) => {
                for unit in c.encode_utf16(&mut [0; 2]) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes;
    use pinguer_source::Span;

    #[test]
    fn renders_full_diagnostic() {
        let mut map = SourceMap::new();
        let file = map.add("main.pgr", "let s = \"abc\nx").unwrap();
        let diagnostic = Diagnostic::error("unterminated string literal")
            .with_code(codes::UNTERMINATED_STRING)
            .with_primary(Span::new(file, 8, 12), "string starts here")
            .with_help("close the string");
        let expected = concat!(
            r#"{"format_version":1,"severity":"error","code":"P1002","#,
            r#""message":"unterminated string literal","labels":[{"file":"main.pgr","#,
            r#""byte_start":8,"byte_end":12,"line_start":1,"column_start":9,"#,
            r#""line_end":1,"column_end":13,"style":"primary","#,
            r#""message":"string starts here"}],"notes":[],"help":["close the string"]}"#
        );
        assert_eq!(render(&map, &diagnostic), expected);
    }

    #[test]
    fn diagnostic_without_code_uses_null() {
        let map = SourceMap::new();
        let rendered = render(&map, &Diagnostic::warning("w"));
        assert!(rendered.contains(r#""code":null"#), "{rendered}");
        assert!(rendered.contains(r#""labels":[]"#), "{rendered}");
    }

    #[test]
    fn strings_are_escaped() {
        let mut out = String::new();
        write_string(&mut out, "q\"b\\n\nt\tc\u{1}d\u{7f}l\u{2028}é🐧");
        // Built from pieces so that no tool or editor can turn the expected
        // escape sequences into the characters they denote.
        let bs = '\\';
        let expected = format!(
            "\"q{bs}\"b{bs}{bs}n{bs}nt{bs}tc{bs}u0001d{bs}u007fl{bs}u2028{}{}\"",
            char::from_u32(0xE9).unwrap(),
            char::from_u32(0x1F427).unwrap()
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn bidirectional_and_invisible_characters_are_escaped() {
        let mut out = String::new();
        write_string(&mut out, "evil\u{202E}name\u{200B}.pgr");
        let bs = '\\';
        assert_eq!(out, format!("\"evil{bs}u202ename{bs}u200b.pgr\""));
    }

    #[test]
    fn invisible_characters_are_escaped_as_valid_json() {
        let mut out = String::new();
        write_string(&mut out, "soft\u{AD}hyphen tag\u{E0041}x");
        let bs = '\\';
        // U+E0041 is the surrogate pair DB40 DC41; `1` would not be JSON.
        assert_eq!(
            out,
            format!("\"soft{bs}u00adhyphen tag{bs}udb40{bs}udc41x\"")
        );
    }

    #[test]
    fn render_all_emits_one_line_per_diagnostic() {
        let map = SourceMap::new();
        let diagnostics = [Diagnostic::error("a\nb"), Diagnostic::error("c")];
        let rendered = render_all(&map, &diagnostics);
        assert_eq!(rendered.lines().count(), 2);
        assert!(rendered.ends_with('\n'));
    }
}
