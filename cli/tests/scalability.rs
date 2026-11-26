//! Scalability regression tests for `pinguer debug tokens`.
//!
//! These tests check the properties that made the front end scale linearly:
//! output that does not grow with the input once the diagnostic limit is
//! reached, excerpts that stay narrow on very long lines, and exact positions
//! deep inside long lines with multi-byte characters and tabs. They assert
//! no timings, so they cannot fail on a slow machine; the timings themselves
//! are measured by `cargo bench -p pinguer_cli --bench scalability`.

use std::path::PathBuf;
use std::process::{Command, Output};

/// A temporary directory, unique to one test, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("pinguer-scalability-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn debug_tokens(path: &std::path::Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pinguer"))
        .args(["debug", "tokens"])
        .arg(path)
        .args(extra)
        .output()
        .expect("failed to run the pinguer executable")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output is UTF-8")
}

#[test]
fn diagnostic_output_does_not_grow_with_the_input() {
    // Before the fixes, one line of 10 KB of unknown characters produced
    // 152 MB of diagnostics and 20 KB produced 605 MB.
    let dir = TempDir::new("output-size");
    let mut sizes = Vec::new();
    for length in [10_000usize, 100_000, 1_000_000] {
        let path = dir.file("line.pgr", &"$".repeat(length));
        let output = debug_tokens(&path, &[]);
        assert_eq!(output.status.code(), Some(1));
        let stderr = text(&output.stderr);
        assert_eq!(stderr.matches("error[P1001]").count(), 100);
        assert!(
            stderr.ends_with(&format!(
                "error: aborting due to {length} previous errors\n"
            )),
            "{length}"
        );
        // The listing still covers every token.
        assert_eq!(
            output.stdout.iter().filter(|&&b| b == b'\n').count(),
            length + 1
        );
        sizes.push(stderr.len());
    }
    // Only the numbers in the two summary lines change with the input size.
    let spread = sizes.iter().max().unwrap() - sizes.iter().min().unwrap();
    assert!(spread <= 8, "{sizes:?}");
    assert!(sizes[0] < 100_000, "{sizes:?}");
}

#[test]
fn excerpts_stay_narrow_on_a_long_line_with_one_error() {
    let dir = TempDir::new("narrow");
    let source = format!("let s = \"{}", "x".repeat(2_000_000));
    let path = dir.file("unterminated.pgr", &source);
    let output = debug_tokens(&path, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    // The label covers two million characters, the excerpt 120 cells.
    assert!(stderr.contains("error[P1002]: unterminated string literal"));
    assert!(
        stderr.lines().all(|line| line.chars().count() < 200),
        "{stderr}"
    );
}

#[test]
fn positions_are_exact_deep_inside_a_long_unicode_line() {
    let dir = TempDir::new("positions");
    // Strings with multi-byte characters, separated by tabs, then an error.
    let unit = "\"é🐧ção\"\t";
    let repetitions = 100_000;
    let source = format!("{}$", unit.repeat(repetitions));
    let path = dir.file("unicode.pgr", &source);
    let output = debug_tokens(&path, &["--diagnostic-format=json"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    // Columns count Unicode scalar values: 8 per unit, then the `$`.
    let column = unit.chars().count() * repetitions + 1;
    let bytes = unit.len() * repetitions;
    assert!(
        stderr.contains(&format!(
            r#""byte_start":{bytes},"byte_end":{},"line_start":1,"column_start":{column},"line_end":1,"column_end":{}"#,
            bytes + 1,
            column + 1
        )),
        "{stderr}"
    );
    // The listing agrees with the diagnostic.
    let stdout = text(&output.stdout);
    let last = stdout.lines().rev().nth(1).unwrap();
    assert_eq!(last, format!("1:{column}\tUnknown\t\"$\""));
}

#[test]
fn many_lines_report_line_numbers_exactly() {
    let dir = TempDir::new("many-lines");
    let lines = 500_000;
    let source = format!("{}$\n", "let x = 1\n".repeat(lines));
    let path = dir.file("lines.pgr", &source);
    let output = debug_tokens(&path, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains(&format!(" --> {}:{}:1\n", path.display(), lines + 1)),
        "{stderr}"
    );
}
