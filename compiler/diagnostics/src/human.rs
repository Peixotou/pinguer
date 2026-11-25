//! Renders diagnostics as human-readable text.
//!
//! The format follows the conventions popularised by `rustc`:
//!
//! ```text
//! error[P1002]: unterminated string literal
//!  --> main.pgr:4:15
//!   |
//! 4 | let message = "hello
//!   |               ^^^^^^ missing closing `"`
//!   |
//!   = note: string literals cannot contain line breaks; use `\n` instead
//! ```
//!
//! # Terminal safety
//!
//! Source lines are untrusted input. Before display, tabs are expanded to four
//! spaces and control characters, bidirectional formatting characters and
//! invisible formatting characters are replaced with a visible `<U+XXXX>`
//! placeholder, so that a diagnostic can never reorder or hide text in the
//! user's terminal. Column markers are computed from the sanitised text.
//!
//! # Known limitations
//!
//! * Every displayed character is assumed to occupy one terminal cell; wide
//!   East Asian characters and most emoji will misalign the markers.
//! * A label whose span covers several lines is underlined only up to the end
//!   of its first line.
//! * Output is never colored.

use std::fmt::Write as _;

use pinguer_source::{FileId, SourceFile, SourceMap};

use crate::{Diagnostic, Label, LabelStyle};

const TAB_WIDTH: usize = 4;

/// Renders one diagnostic. The result always ends with a newline.
///
/// # Panics
///
/// Panics if a label refers to a file that is not part of `map`.
#[must_use]
pub fn render(map: &SourceMap, diagnostic: &Diagnostic) -> String {
    let mut out = String::new();
    write_header(&mut out, diagnostic);

    let gutter = gutter_width(map, diagnostic);
    let pad = " ".repeat(gutter);

    for (index, file_id) in files_in_display_order(diagnostic).into_iter().enumerate() {
        let file = map.file(file_id);
        let labels: Vec<&Label> = diagnostic
            .labels
            .iter()
            .filter(|label| label.span.file == file_id)
            .collect();
        let location_label = if index == 0 {
            diagnostic
                .main_label()
                .expect("a file implies at least one label")
        } else {
            labels[0]
        };
        let arrow = if index == 0 { "-->" } else { ":::" };
        let location = file.line_col(location_label.span.start);
        let _ = writeln!(out, "{pad}{arrow} {}:{location}", sanitize(file.name()));
        let _ = writeln!(out, "{pad} |");
        write_snippet(&mut out, file, &labels, gutter);
    }

    if !diagnostic.notes.is_empty() || !diagnostic.help.is_empty() {
        if !diagnostic.labels.is_empty() {
            let _ = writeln!(out, "{pad} |");
        }
        for note in &diagnostic.notes {
            let _ = writeln!(out, "{pad} = note: {}", sanitize(note));
        }
        for help in &diagnostic.help {
            let _ = writeln!(out, "{pad} = help: {}", sanitize(help));
        }
    }
    out
}

/// Renders several diagnostics separated by blank lines.
#[must_use]
pub fn render_all<'a>(
    map: &SourceMap,
    diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
) -> String {
    diagnostics
        .into_iter()
        .map(|diagnostic| render(map, diagnostic))
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_header(out: &mut String, diagnostic: &Diagnostic) {
    let message = sanitize(&diagnostic.message);
    match diagnostic.code {
        Some(code) => {
            let _ = writeln!(out, "{}[{code}]: {message}", diagnostic.severity);
        }
        None => {
            let _ = writeln!(out, "{}: {message}", diagnostic.severity);
        }
    }
}

/// Returns the width of the line-number gutter: the number of digits in the
/// largest line number that will be displayed, and at least one.
fn gutter_width(map: &SourceMap, diagnostic: &Diagnostic) -> usize {
    diagnostic
        .labels
        .iter()
        .map(|label| map.file(label.span.file).line_col(label.span.start).line)
        .max()
        .map_or(1, |line| line.to_string().len())
}

/// Returns the files referenced by labels, with the main label's file first
/// and the others in order of first appearance.
fn files_in_display_order(diagnostic: &Diagnostic) -> Vec<FileId> {
    let mut files = Vec::new();
    if let Some(main) = diagnostic.main_label() {
        files.push(main.span.file);
    }
    for label in &diagnostic.labels {
        if !files.contains(&label.span.file) {
            files.push(label.span.file);
        }
    }
    files
}

fn write_snippet(out: &mut String, file: &SourceFile, labels: &[&Label], gutter: usize) {
    let pad = " ".repeat(gutter);
    let mut by_line: Vec<(u32, &Label)> = labels
        .iter()
        .map(|&label| (file.line_col(label.span.start).line, label))
        .collect();
    by_line.sort_by_key(|&(line, label)| (line, label.span.start, label.span.end));

    let mut index = 0;
    while index < by_line.len() {
        let line = by_line[index].0;
        let line_range = file.line_range(line as usize);
        let line_text = &file.text()[line_range.clone()];
        let shown = sanitize(line_text);
        if shown.is_empty() {
            let _ = writeln!(out, "{line:>gutter$} |");
        } else {
            let _ = writeln!(out, "{line:>gutter$} | {shown}");
        }

        while index < by_line.len() && by_line[index].0 == line {
            let label = by_line[index].1;
            let text = file.text();
            let start = floor_char_boundary(text, (label.span.start as usize).min(line_range.end));
            let end =
                floor_char_boundary(text, (label.span.end as usize).min(line_range.end)).max(start);
            let column = display_width(&text[line_range.start..start]);
            let width = display_width(&text[start..end]).max(1);
            let marker = match label.style {
                LabelStyle::Primary => '^',
                LabelStyle::Secondary => '-',
            };
            let mut row = format!("{pad} | {}", " ".repeat(column));
            row.extend(std::iter::repeat_n(marker, width));
            if !label.message.is_empty() {
                row.push(' ');
                row.push_str(&sanitize(&label.message));
            }
            let _ = writeln!(out, "{row}");
            index += 1;
        }
    }
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Returns `true` for characters that must not be written to a terminal
/// verbatim because they are invisible or alter how surrounding text is shown.
pub(crate) fn needs_escape(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{061C}'              // Arabic letter mark
            | '\u{200B}'..='\u{200F}' // zero-width characters, LRM, RLM
            | '\u{2028}' | '\u{2029}' // line and paragraph separators
            | '\u{202A}'..='\u{202E}' // bidirectional embeddings and overrides
            | '\u{2060}'..='\u{2069}' // word joiner, invisible operators, isolates
            | '\u{FEFF}'              // zero-width no-break space
        )
}

fn push_display(out: &mut String, c: char) {
    if c == '\t' {
        out.extend(std::iter::repeat_n(' ', TAB_WIDTH));
    } else if needs_escape(c) {
        let _ = write!(out, "<U+{:04X}>", u32::from(c));
    } else {
        out.push(c);
    }
}

/// Returns `text` with tabs expanded and unsafe characters escaped.
fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        push_display(&mut out, c);
    }
    out
}

/// Returns the number of characters `text` occupies after [`sanitize`].
fn display_width(text: &str) -> usize {
    let mut buffer = String::new();
    text.chars()
        .map(|c| {
            buffer.clear();
            push_display(&mut buffer, c);
            buffer.chars().count()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes;
    use pinguer_source::Span;

    fn setup(name: &str, text: &str) -> (SourceMap, FileId) {
        let mut map = SourceMap::new();
        let id = map.add(name, text).unwrap();
        (map, id)
    }

    #[test]
    fn renders_primary_label_with_help() {
        let text = "func main() {\n    let x = 1\n\n    let age: int = \"twenty\"\n}\n";
        let (map, file) = setup("main.pgr", text);
        let start = text.find("\"twenty\"").unwrap() as u32;
        let diagnostic = Diagnostic::error("incompatible types")
            .with_code(codes::Code::new(4001))
            .with_primary(
                Span::new(file, start, start + 8),
                "expected int, found string",
            )
            .with_help("assign an integer value");
        let expected = "\
error[P4001]: incompatible types
 --> main.pgr:4:20
  |
4 |     let age: int = \"twenty\"
  |                    ^^^^^^^^ expected int, found string
  |
  = help: assign an integer value
";
        assert_eq!(render(&map, &diagnostic), expected);
    }

    #[test]
    fn renders_secondary_labels_and_wide_gutters() {
        let mut text = "\n".repeat(9);
        text.push_str("let a = b + c\n");
        let (map, file) = setup("x.pgr", &text);
        let diagnostic = Diagnostic::error("example")
            .with_secondary(Span::new(file, 17, 18), "left")
            .with_primary(Span::new(file, 21, 22), "right");
        let expected = "\
error: example
  --> x.pgr:10:13
   |
10 | let a = b + c
   |         - left
   |             ^ right
";
        assert_eq!(render(&map, &diagnostic), expected);
    }

    #[test]
    fn diagnostic_without_labels_renders_notes() {
        let map = SourceMap::new();
        let diagnostic = Diagnostic::error("no input files").with_note("pass a .pgr file");
        assert_eq!(
            render(&map, &diagnostic),
            "error: no input files\n  = note: pass a .pgr file\n"
        );
    }

    #[test]
    fn tabs_are_expanded_and_markers_follow() {
        let (map, file) = setup("t.pgr", "\tx");
        let diagnostic = Diagnostic::error("tab").with_primary(Span::new(file, 1, 2), "");
        let rendered = render(&map, &diagnostic);
        assert!(rendered.contains("1 |     x\n"), "{rendered}");
        assert!(rendered.contains("  |     ^\n"), "{rendered}");
    }

    #[test]
    fn bidi_characters_are_escaped_and_markers_follow() {
        let text = "a\u{202E}b";
        let (map, file) = setup("b.pgr", text);
        let diagnostic = Diagnostic::error("bidi").with_primary(Span::new(file, 1, 4), "here");
        let rendered = render(&map, &diagnostic);
        assert!(rendered.contains("1 | a<U+202E>b\n"), "{rendered}");
        assert!(rendered.contains("  |  ^^^^^^^^ here\n"), "{rendered}");
        assert!(!rendered.contains('\u{202E}'));
    }

    #[test]
    fn empty_span_at_end_of_file_renders_one_marker() {
        let (map, file) = setup("e.pgr", "abc");
        let diagnostic = Diagnostic::error("eof").with_primary(Span::new(file, 3, 3), "");
        let rendered = render(&map, &diagnostic);
        assert!(rendered.contains("  |    ^\n"), "{rendered}");
    }

    #[test]
    fn multi_line_span_is_underlined_to_end_of_first_line() {
        let (map, file) = setup("m.pgr", "ab\r\ncd");
        let diagnostic = Diagnostic::error("multi").with_primary(Span::new(file, 1, 6), "");
        let rendered = render(&map, &diagnostic);
        assert!(rendered.contains("1 | ab\n  |  ^\n"), "{rendered}");
    }

    #[test]
    fn labels_in_other_files_get_their_own_section() {
        let mut map = SourceMap::new();
        let a = map.add("a.pgr", "first").unwrap();
        let b = map.add("b.pgr", "second").unwrap();
        let diagnostic = Diagnostic::error("two files")
            .with_primary(Span::new(a, 0, 5), "here")
            .with_secondary(Span::new(b, 0, 6), "and here");
        let expected = "\
error: two files
 --> a.pgr:1:1
  |
1 | first
  | ^^^^^ here
 ::: b.pgr:1:1
  |
1 | second
  | ------ and here
";
        assert_eq!(render(&map, &diagnostic), expected);
    }

    #[test]
    fn render_all_separates_with_blank_lines() {
        let map = SourceMap::new();
        let diagnostics = [Diagnostic::error("one"), Diagnostic::warning("two")];
        assert_eq!(
            render_all(&map, &diagnostics),
            "error: one\n\nwarning: two\n"
        );
    }
}
