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
//! # Long lines
//!
//! An excerpt shows at most [`MAX_EXCERPT_WIDTH`] cells of a source line. A
//! longer line is cut around its labels and every cut is marked with `...`;
//! labels too far apart to share one excerpt get one excerpt each. The size
//! of a rendered diagnostic therefore depends on its number of labels, not on
//! the length of the lines they point into, and rendering never scans a
//! whole line.
//!
//! # Known limitations
//!
//! * Every displayed character is assumed to occupy one terminal cell; wide
//!   East Asian characters and most emoji will misalign the markers.
//! * A label whose span covers several lines is underlined only up to the end
//!   of its first line.
//! * Output is never colored.

use std::fmt::Write as _;
use std::ops::Range;

use pinguer_source::{FileId, SourceFile, SourceMap};

use crate::{Diagnostic, Label, LabelStyle};

const TAB_WIDTH: usize = 4;

/// The widest part of a source line, in terminal cells, shown by one
/// excerpt. 120 cells keep an excerpt on one line of a typical terminal
/// while leaving room for the gutter and the cut markers.
pub const MAX_EXCERPT_WIDTH: usize = 120;

/// Cells of source text kept before the first label of a cut excerpt.
const EXCERPT_CONTEXT: usize = 24;

/// Marks the place where an excerpt was cut.
const ELLIPSIS: &str = "...";

/// A label clamped to the line it is displayed on: its start and end byte
/// offsets, both character boundaries within the line.
type Mark<'a> = (usize, usize, &'a Label);

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
    let text = file.text();
    let mut by_line: Vec<(u32, &Label)> = labels
        .iter()
        .map(|&label| (file.line_col(label.span.start).line, label))
        .collect();
    by_line.sort_by_key(|&(line, label)| (line, label.span.start, label.span.end));

    for group in by_line.chunk_by(|a, b| a.0 == b.0) {
        let line = group[0].0;
        let range = file.line_range(line as usize);
        let marks: Vec<Mark<'_>> = group
            .iter()
            .map(|&(_, label)| {
                let start = floor_char_boundary(text, (label.span.start as usize).min(range.end));
                let end =
                    floor_char_boundary(text, (label.span.end as usize).min(range.end)).max(start);
                (start, end, label)
            })
            .collect();
        let mut first = 0;
        while first < marks.len() {
            let (window, count) = excerpt_window(text, range.clone(), &marks[first..]);
            let excerpt = Excerpt {
                text,
                line,
                cut_before: window.start > range.start,
                cut_after: window.end < range.end,
                window,
            };
            excerpt.write(out, gutter, &marks[first..first + count]);
            first += count;
        }
    }
}

/// Chooses the part of the line `range` that is shown for `marks`, which are
/// sorted by start offset. Returns the shown byte range and the number of
/// marks, at least one, that fall inside it.
fn excerpt_window(text: &str, range: Range<usize>, marks: &[Mark<'_>]) -> (Range<usize>, usize) {
    if advance(text, range.start, range.end, MAX_EXCERPT_WIDTH) == range.end {
        return (range, marks.len());
    }
    let anchor = marks[0].0;
    let mut start = retreat(text, anchor, range.start, EXCERPT_CONTEXT);
    let end = advance(text, start, range.end, MAX_EXCERPT_WIDTH);
    if end == range.end {
        // The line ends inside the window: use the free space for context
        // on the left instead.
        start = retreat(text, range.end, range.start, MAX_EXCERPT_WIDTH);
    }
    let visible = marks[1..]
        .iter()
        .take_while(|&&(mark_start, _, _)| mark_start < end || end == range.end)
        .count();
    (start..end, 1 + visible)
}

/// One displayed piece of a source line, with its labels.
struct Excerpt<'a> {
    text: &'a str,
    line: u32,
    /// The shown byte range of `text`.
    window: Range<usize>,
    cut_before: bool,
    cut_after: bool,
}

impl Excerpt<'_> {
    fn write(&self, out: &mut String, gutter: usize, marks: &[Mark<'_>]) {
        let line = self.line;
        let mut shown = String::new();
        if self.cut_before {
            shown.push_str(ELLIPSIS);
        }
        for c in self.text[self.window.clone()].chars() {
            push_display(&mut shown, c);
        }
        if self.cut_after {
            shown.push_str(ELLIPSIS);
        }
        if shown.is_empty() {
            let _ = writeln!(out, "{line:>gutter$} |");
        } else {
            let _ = writeln!(out, "{line:>gutter$} | {shown}");
        }

        let indent = if self.cut_before { ELLIPSIS.len() } else { 0 };
        let pad = " ".repeat(gutter);
        for &(start, end, label) in marks {
            let column = indent + text_width(&self.text[self.window.start..start]);
            let width = text_width(&self.text[start..end.min(self.window.end)]).max(1);
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
        }
    }
}

/// Returns the end of the longest prefix of `text[from..limit]` whose
/// display width is at most `budget` cells.
fn advance(text: &str, from: usize, limit: usize, budget: usize) -> usize {
    let mut cells = 0;
    let mut end = from;
    for c in text[from..limit].chars() {
        cells += cell_width(c);
        if cells > budget {
            break;
        }
        end += c.len_utf8();
    }
    end
}

/// Returns the start of the longest suffix of `text[limit..from]` whose
/// display width is at most `budget` cells.
fn retreat(text: &str, from: usize, limit: usize, budget: usize) -> usize {
    let mut cells = 0;
    let mut start = from;
    for c in text[limit..from].chars().rev() {
        cells += cell_width(c);
        if cells > budget {
            break;
        }
        start -= c.len_utf8();
    }
    start
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Characters other than controls that are invisible or change how the text
/// around them is displayed: Unicode's default-ignorable code points
/// (DerivedCoreProperties.txt, `Default_Ignorable_Code_Point`), the
/// interlinear annotation characters and the line and paragraph separators.
/// `docs/architecture/DIAGNOSTICS.md` lists the same ranges.
const INVISIBLE: &[(char, char)] = &[
    ('\u{00AD}', '\u{00AD}'),   // soft hyphen
    ('\u{034F}', '\u{034F}'),   // combining grapheme joiner
    ('\u{061C}', '\u{061C}'),   // Arabic letter mark
    ('\u{115F}', '\u{1160}'),   // Hangul choseong and jungseong fillers
    ('\u{17B4}', '\u{17B5}'),   // Khmer inherent vowels
    ('\u{180B}', '\u{180F}'),   // Mongolian variation selectors, vowel separator
    ('\u{200B}', '\u{200F}'),   // zero-width space and joiners, LRM, RLM
    ('\u{2028}', '\u{202E}'),   // line and paragraph separators, bidi embeddings
    ('\u{2060}', '\u{206F}'),   // word joiner, invisible operators, isolates, deprecated
    ('\u{3164}', '\u{3164}'),   // Hangul filler
    ('\u{FE00}', '\u{FE0F}'),   // variation selectors
    ('\u{FEFF}', '\u{FEFF}'),   // zero-width no-break space
    ('\u{FFA0}', '\u{FFA0}'),   // halfwidth Hangul filler
    ('\u{FFF0}', '\u{FFFB}'),   // reserved, interlinear annotation characters
    ('\u{1BCA0}', '\u{1BCA3}'), // shorthand format controls
    ('\u{1D173}', '\u{1D17A}'), // musical symbol formatting
    ('\u{E0000}', '\u{E0FFF}'), // tags, variation selectors supplement, reserved
];

/// Returns `true` for characters that must not be written to a terminal
/// verbatim because they are invisible or alter how surrounding text is
/// shown: control characters and the [`INVISIBLE`] ranges.
pub(crate) fn needs_escape(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_control();
    }
    c.is_control()
        || INVISIBLE
            .iter()
            .any(|&(first, last)| (first..=last).contains(&c))
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

/// Returns the number of cells `c` occupies after [`push_display`].
fn cell_width(c: char) -> usize {
    if c == '\t' {
        TAB_WIDTH
    } else if needs_escape(c) {
        // `<U+` and `>` around at least four hexadecimal digits.
        let digits = match u32::from(c) {
            0..=0xFFFF => 4,
            0x1_0000..=0xF_FFFF => 5,
            _ => 6,
        };
        digits + 4
    } else {
        1
    }
}

/// Returns the number of cells `text` occupies after [`sanitize`].
fn text_width(text: &str) -> usize {
    text.chars().map(cell_width).sum()
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
    fn invisible_characters_are_escaped() {
        for &(first, last) in INVISIBLE {
            assert!(needs_escape(first), "{:04X}", u32::from(first));
            assert!(needs_escape(last), "{:04X}", u32::from(last));
        }
        // The characters missing from the original list.
        for c in [
            '\u{AD}',
            '\u{34F}',
            '\u{180E}',
            '\u{206A}',
            '\u{206F}',
            '\u{3164}',
            '\u{FE0F}',
            '\u{FFF9}',
            '\u{FFFB}',
            '\u{E0001}',
            '\u{E0041}',
            '\u{E007F}',
        ] {
            assert!(needs_escape(c), "{:04X}", u32::from(c));
        }
        // Controls, including C1 controls such as next line.
        for c in ['\u{0}', '\u{1B}', '\u{7F}', '\u{85}', '\u{9B}'] {
            assert!(needs_escape(c), "{:04X}", u32::from(c));
        }
        // Visible text, including spaces that are visible as spaces.
        for c in [
            'a',
            ' ',
            'é',
            '€',
            '🐧',
            '\u{A0}',
            '\u{3000}',
            '\u{FFFD}',
            '\u{E1000}',
        ] {
            assert!(!needs_escape(c), "{:04X}", u32::from(c));
        }
    }

    #[test]
    fn supplementary_invisible_characters_are_escaped_and_markers_follow() {
        // A tag character (`<U+E0041>`, 9 cells) before the label.
        let (map, file) = setup("tag.pgr", "a\u{E0041}b");
        let diagnostic = Diagnostic::error("tag").with_primary(Span::new(file, 5, 6), "here");
        let rendered = render(&map, &diagnostic);
        assert!(rendered.contains("1 | a<U+E0041>b\n"), "{rendered}");
        assert!(rendered.contains("  |           ^ here\n"), "{rendered}");
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

    fn render_one_label(text: &str, span: (u32, u32)) -> String {
        let (map, file) = setup("long.pgr", text);
        let diagnostic =
            Diagnostic::error("long").with_primary(Span::new(file, span.0, span.1), "here");
        render(&map, &diagnostic)
    }

    #[test]
    fn long_line_is_cut_around_the_label() {
        let text = format!("{}${}", "a".repeat(1_000), "b".repeat(1_000));
        let expected = format!(
            "\
error: long
 --> long.pgr:1:1001
  |
1 | ...{}${}...
  | {}^ here
",
            "a".repeat(EXCERPT_CONTEXT),
            "b".repeat(MAX_EXCERPT_WIDTH - EXCERPT_CONTEXT - 1),
            " ".repeat(ELLIPSIS.len() + EXCERPT_CONTEXT),
        );
        assert_eq!(render_one_label(&text, (1_000, 1_001)), expected);
    }

    #[test]
    fn label_near_the_start_of_a_long_line_is_not_cut_on_the_left() {
        let text = "a".repeat(1_000);
        let rendered = render_one_label(&text, (5, 6));
        let source = format!("1 | {}...\n", "a".repeat(MAX_EXCERPT_WIDTH));
        assert!(rendered.contains(&source), "{rendered}");
        assert!(rendered.contains("  |      ^ here\n"), "{rendered}");
    }

    #[test]
    fn label_at_the_end_of_a_long_line_fills_the_excerpt_with_context() {
        let text = format!("{}$", "a".repeat(1_000));
        let rendered = render_one_label(&text, (1_000, 1_001));
        let source = format!("1 | ...{}$\n", "a".repeat(MAX_EXCERPT_WIDTH - 1));
        let marker = format!("  | {}^ here\n", " ".repeat(ELLIPSIS.len() + 119));
        assert!(rendered.contains(&source), "{rendered}");
        assert!(rendered.contains(&marker), "{rendered}");
        // An empty span at the end of the line still gets one marker.
        let rendered = render_one_label(&text, (1_001, 1_001));
        let marker = format!("  | {}^ here\n", " ".repeat(ELLIPSIS.len() + 120));
        assert!(rendered.contains(&marker), "{rendered}");
    }

    #[test]
    fn distant_labels_on_one_long_line_get_separate_excerpts() {
        let text = "x".repeat(2_000);
        let (map, file) = setup("far.pgr", &text);
        let diagnostic = Diagnostic::error("far")
            .with_primary(Span::new(file, 1_500, 1_501), "second")
            .with_secondary(Span::new(file, 10, 12), "first");
        let rendered = render(&map, &diagnostic);
        let rows: Vec<&str> = rendered.lines().collect();
        assert_eq!(rows[3], format!("1 | {}...", "x".repeat(MAX_EXCERPT_WIDTH)));
        assert_eq!(rows[4], format!("  | {}-- first", " ".repeat(10)));
        assert_eq!(
            rows[5],
            format!("1 | ...{}...", "x".repeat(MAX_EXCERPT_WIDTH))
        );
        assert_eq!(rows[6], format!("  | {}^ second", " ".repeat(3 + 24)));
        assert_eq!(rows.len(), 7, "{rendered}");
    }

    #[test]
    fn cut_excerpts_measure_tabs_escapes_and_multibyte_characters() {
        // Before the label: a tab (4 cells), U+202E (8 cells as `<U+202E>`)
        // and `é` (1 cell), so 11 more `x` complete the 24 cells of context.
        let text = format!("{}\t\u{202E}é${}", "x".repeat(500), "y".repeat(500));
        let start = text.find('$').unwrap() as u32;
        let rendered = render_one_label(&text, (start, start + 1));
        let source = format!(
            "1 | ...{}    <U+202E>é${}...\n",
            "x".repeat(11),
            "y".repeat(MAX_EXCERPT_WIDTH - EXCERPT_CONTEXT - 1)
        );
        let marker = format!("  | {}^ here\n", " ".repeat(ELLIPSIS.len() + 24));
        assert!(rendered.contains(&source), "{rendered}");
        assert!(rendered.contains(&marker), "{rendered}");
        assert!(!rendered.contains('\u{202E}'));
    }

    #[test]
    fn rendered_size_does_not_depend_on_line_length() {
        for length in [1_000usize, 100_000, 1_000_000] {
            let text = "z".repeat(length);
            let middle = (length / 2) as u32;
            // A label covering most of the line is underlined only as far as
            // the excerpt reaches.
            let rendered = render_one_label(&text, (middle, length as u32));
            assert!(
                rendered.len() < 2 * MAX_EXCERPT_WIDTH + 100,
                "{length}: {rendered}"
            );
        }
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
