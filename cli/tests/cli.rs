//! End-to-end tests that run the `pinguer` executable.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Runs `pinguer` with `args` inside the fixtures directory.
fn pinguer(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pinguer"))
        .args(args)
        .current_dir(fixtures())
        .output()
        .expect("failed to run the pinguer executable")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

/// A temporary directory, unique to one test, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path = std::env::temp_dir().join(format!("pinguer-cli-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, contents: &[u8]) -> PathBuf {
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

#[test]
fn version_flags_print_the_version() {
    for flag in ["version", "--version", "-V"] {
        let output = pinguer(&[flag]);
        assert_eq!(output.status.code(), Some(0), "{flag}");
        assert_eq!(stdout(&output), "pinguer 0.1.0-dev\n", "{flag}");
        assert!(output.stderr.is_empty(), "{flag}");
    }
}

#[test]
fn help_lists_available_and_planned_commands() {
    for args in [&[][..], &["help"], &["--help"], &["-h"]] {
        let output = pinguer(args);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        let text = stdout(&output);
        assert!(
            text.starts_with("pinguer 0.1.0-dev\n\nUsage: pinguer <command>"),
            "{text}"
        );
        assert!(text.contains("debug tokens <file>"), "{text}");
        assert!(text.contains("debug ast <file>"), "{text}");
        assert!(text.contains("new, init, run, build, check, test, fmt, lint, doc, add, remove"));
    }
}

#[test]
fn planned_commands_report_that_they_are_unavailable() {
    for command in [
        "new", "init", "run", "build", "check", "test", "fmt", "lint", "doc", "add", "remove",
    ] {
        let output = pinguer(&[command, "hello.pgr"]);
        assert_eq!(output.status.code(), Some(3), "{command}");
        assert!(output.stdout.is_empty(), "{command}");
        let text = stderr(&output);
        assert!(
            text.starts_with(&format!(
                "error: `pinguer {command}` is not available in Pinguer 0.1.0-dev\n"
            )),
            "{text}"
        );
        assert!(text.contains("ROADMAP.md"), "{text}");
    }
}

#[test]
fn unknown_command_is_a_usage_error() {
    let output = pinguer(&["compile"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output),
        "error: unknown command `compile`\n  = note: run `pinguer help` for usage\n"
    );
}

#[test]
fn help_and_version_reject_extra_arguments() {
    for args in [
        &["version", "--bogus"][..],
        &["--version", "x"],
        &["help", "build"],
        &["-h", "x"],
    ] {
        let output = pinguer(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(
            stderr(&output).contains("takes no arguments"),
            "{args:?}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn debug_usage_errors() {
    for (args, message) in [
        (&["debug", "tokens", "-"][..], "unknown option `-`"),
        (&["debug"], "missing debug command"),
        (&["debug", "hir"], "unknown debug command `hir`"),
        (&["debug", "tokens"], "missing source file"),
        (&["debug", "tokens", "a.pgr", "b.pgr"], "exactly one file"),
        (&["debug", "ast"], "missing source file for `debug ast`"),
        (
            &["debug", "ast", "a.pgr", "b.pgr"],
            "`debug ast` accepts exactly one file",
        ),
        (
            &["debug", "ast", "--tree", "a.pgr"],
            "unknown option `--tree`",
        ),
        (
            &["debug", "tokens", "--verbose", "a.pgr"],
            "unknown option `--verbose`",
        ),
        (
            &["debug", "tokens", "a.pgr", "--diagnostic-format"],
            "requires a value",
        ),
        (
            &["debug", "tokens", "a.pgr", "--diagnostic-format=xml"],
            "invalid diagnostic format `xml`",
        ),
    ] {
        let output = pinguer(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(
            stderr(&output).contains(message),
            "{args:?}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn command_line_errors_honor_the_json_format() {
    for (args, status, message) in [
        (
            &["compile", "--diagnostic-format", "json"][..],
            2,
            "unknown command `compile`",
        ),
        (
            &["version", "--diagnostic-format=json"],
            2,
            "unexpected argument `--diagnostic-format=json`; `version` takes no arguments",
        ),
        (
            &["debug", "tokens", "--diagnostic-format=json"],
            2,
            "missing source file for `debug tokens`",
        ),
        (
            &[
                "debug",
                "tokens",
                "--diagnostic-format=json",
                "a.pgr",
                "b.pgr",
            ],
            2,
            "`debug tokens` accepts exactly one file",
        ),
        (
            &["debug", "ast", "--diagnostic-format=json"],
            2,
            "missing source file for `debug ast`",
        ),
        (
            &[
                "debug",
                "tokens",
                "--diagnostic-format=json",
                "--diagnostic-format=xml",
            ],
            2,
            "invalid diagnostic format `xml`; expected `human` or `json`",
        ),
        (
            &["build", "hello.pgr", "--diagnostic-format", "json"],
            3,
            "`pinguer build` is not available in Pinguer 0.1.0-dev",
        ),
    ] {
        let output = pinguer(args);
        assert_eq!(output.status.code(), Some(status), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        let text = stderr(&output);
        assert_eq!(text.lines().count(), 1, "{args:?}: {text}");
        let expected = format!(
            r#"{{"format_version":1,"severity":"error","code":null,"message":"{message}","labels":[],"notes":["#
        );
        assert!(text.starts_with(&expected), "{args:?}: {text}");
        assert!(text.ends_with("]}\n"), "{args:?}: {text}");
    }
}

#[test]
fn command_line_errors_are_human_when_the_format_is_unknown() {
    for args in [
        &["debug", "tokens", "a.pgr", "--diagnostic-format"][..],
        &["debug", "tokens", "a.pgr", "--diagnostic-format=xml"],
        // A malformed option hides any later one.
        &[
            "debug",
            "tokens",
            "--diagnostic-format=xml",
            "--diagnostic-format=json",
        ],
        &["compile"],
    ] {
        let output = pinguer(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(stderr(&output).starts_with("error: "), "{args:?}");
    }
}

#[test]
fn debug_tokens_lists_tokens_of_a_valid_file() {
    let output = pinguer(&["debug", "tokens", "hello.pgr"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let expected = "\
1:1\tKeyword(Import)\t\"import\"
1:8\tIdent\t\"std\"
1:11\tDot\t\".\"
1:12\tIdent\t\"io\"
1:14\tNewline\t\"\\n\"
3:1\tKeyword(Func)\t\"func\"
3:6\tIdent\t\"main\"
3:10\tLParen\t\"(\"
3:11\tRParen\t\")\"
3:13\tLBrace\t\"{\"
3:14\tNewline\t\"\\n\"
4:5\tIdent\t\"io\"
4:7\tDot\t\".\"
4:8\tIdent\t\"println\"
4:15\tLParen\t\"(\"
4:16\tString\t\"\\\"Hello, Pinguer!\\\"\"
4:33\tRParen\t\")\"
4:34\tNewline\t\"\\n\"
5:1\tRBrace\t\"}\"
5:2\tNewline\t\"\\n\"
6:1\tEof\t\"\"
";
    assert_eq!(stdout(&output), expected);
}

#[test]
fn debug_tokens_reports_lexical_errors() {
    let output = pinguer(&["debug", "tokens", "lexical_errors.pgr"]);
    assert_eq!(output.status.code(), Some(1));
    let expected = "\
error[P1002]: unterminated string literal
 --> lexical_errors.pgr:2:19
  |
2 |     let message = \"unterminated
  |                   ^^^^^^^^^^^^^ missing closing `\"`
  |
  = note: string literals cannot contain line breaks; use `\\n` instead

error[P1006]: invalid digit `2` in binary literal
 --> lexical_errors.pgr:3:21
  |
3 |     let count = 0b102
  |                     ^ not a binary digit

error: aborting due to 2 previous errors
";
    assert_eq!(stderr(&output), expected);
    // Tokens are still printed so that recovery can be inspected.
    assert!(stdout(&output).ends_with("5:1\tEof\t\"\"\n"));
}

#[test]
fn debug_ast_prints_the_tree_of_a_valid_file() {
    let output = pinguer(&["debug", "ast", "hello.pgr"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let expected = "\
File @1:1-6:1 nodes=8
  Import #0 @1:1-1:14 std.io
  Func #1 @3:1-5:2 main
    Block #2 @3:13-5:2
      ExprStmt #3 @4:5-4:34
        Call #4 @4:5-4:34
          Member #5 @4:5-4:15 .println
            Name #6 @4:5-4:7 io
          String #7 @4:16-4:33 \"Hello, Pinguer!\"
";
    assert_eq!(stdout(&output), expected);
}

#[test]
fn debug_ast_reports_syntax_errors_and_prints_the_recovered_tree() {
    let output = pinguer(&["debug", "ast", "syntax_errors.pgr"]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(
        text.starts_with("error[P2002]: expected identifier, found `=`\n"),
        "{text}"
    );
    assert!(
        text.contains("error[P2008]: expected `)` to close `(`, found `}`\n"),
        "{text}"
    );
    assert!(
        text.ends_with("error: aborting due to 2 previous errors\n"),
        "{text}"
    );
    // The broken statement is an error node; the next one is still parsed.
    let tree = stdout(&output);
    assert!(tree.contains("      ErrorStmt #2 @2:5-"), "{tree}");
    assert!(tree.contains("        Call #4 @3:5-3:20\n"), "{tree}");
}

#[test]
fn debug_ast_emits_json_diagnostics() {
    let output = pinguer(&[
        "debug",
        "ast",
        "syntax_errors.pgr",
        "--diagnostic-format=json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].contains(r#""code":"P2002""#), "{text}");
    assert!(lines[1].contains(r#""code":"P2008""#), "{text}");
    assert!(stdout(&output).starts_with("File @1:1-5:1 nodes=8\n"));
}

#[test]
fn debug_ast_survives_deep_nesting() {
    let dir = TempDir::new("deep-nesting");
    let text = format!("const X = {}1{}\n", "(".repeat(10_000), ")".repeat(10_000));
    let path = dir.file("deep.pgr", text.as_bytes());
    let output = pinguer(&["debug", "ast", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert_eq!(text.matches("error[P2019]").count(), 1, "{text}");
    assert!(
        text.ends_with("error: aborting due to 1 previous error\n"),
        "{text}"
    );
    assert!(stdout(&output).contains("  ErrorItem #0 @1:1-"));
}

#[test]
fn debug_tokens_emits_json_diagnostics() {
    let output = pinguer(&[
        "debug",
        "tokens",
        "lexical_errors.pgr",
        "--diagnostic-format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].starts_with(r#"{"format_version":1,"severity":"error","code":"P1002""#));
    assert!(lines[1].contains(r#""code":"P1006""#));
    assert!(lines[1].contains(r#""line_start":3,"column_start":21"#));
}

#[test]
fn diagnostics_beyond_the_limit_are_summarized() {
    let dir = TempDir::new("limit-human");
    let path = dir.file("dollars.pgr", "$".repeat(10_000).as_bytes());
    let output = pinguer(&["debug", "tokens", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert_eq!(text.matches("error[P1001]").count(), 100, "{text}");
    assert!(
        text.ends_with(
            "note[P0004]: 9900 more diagnostics not shown\n  \
             = note: at most 100 diagnostics are reported in one compilation\n\n\
             error: aborting due to 10000 previous errors\n"
        ),
        "{text}"
    );
    // Before the limit and the excerpt cut, this input printed 152 MB.
    assert!(text.len() < 100_000, "{} bytes", text.len());
    // Every token is still listed.
    assert_eq!(stdout(&output).lines().count(), 10_001);
}

#[test]
fn json_diagnostics_beyond_the_limit_are_summarized() {
    let dir = TempDir::new("limit-json");
    let path = dir.file("dollars.pgr", "$\n".repeat(500).as_bytes());
    let output = pinguer(&[
        "debug",
        "tokens",
        path.to_str().unwrap(),
        "--diagnostic-format=json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 101, "{text}");
    assert!(
        lines[..100]
            .iter()
            .all(|line| line.contains(r#""code":"P1001""#))
    );
    assert!(lines[99].contains(r#""line_start":100,"#), "{}", lines[99]);
    assert!(
        lines[100].starts_with(
            r#"{"format_version":1,"severity":"note","code":"P0004","message":"400 more diagnostics not shown","labels":[]"#
        ),
        "{}",
        lines[100]
    );
}

#[test]
fn closed_standard_output_is_not_an_error() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;

    let dir = TempDir::new("closed-stdout");
    // Far more output than any pipe buffer holds.
    let path = dir.file("many.pgr", "let x = 1\n".repeat(200_000).as_bytes());
    let mut child = Command::new(env!("CARGO_BIN_EXE_pinguer"))
        .args(["debug", "tokens", path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run the pinguer executable");
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    // The reader is dropped here, closing the pipe like `| head -1`.
    assert_eq!(first, "1:1\tKeyword(Let)\t\"let\"\n");
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
}

#[test]
fn closed_standard_output_stops_the_tree() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;

    let dir = TempDir::new("closed-stdout-ast");
    let text = format!("func f() {{\n{}}}\n", "g()\n".repeat(200_000));
    let path = dir.file("many.pgr", text.as_bytes());
    let mut child = Command::new(env!("CARGO_BIN_EXE_pinguer"))
        .args(["debug", "ast", path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run the pinguer executable");
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    assert_eq!(first, "File @1:1-200003:1 nodes=600002\n");
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
}

/// Returns `prefix` followed by something that is not valid Unicode but is a
/// valid argument and file name: an unpaired surrogate on Windows, the byte
/// 0xFF on Unix.
#[cfg(windows)]
fn non_unicode(prefix: &str) -> OsString {
    use std::os::windows::ffi::OsStringExt;
    let mut wide: Vec<u16> = prefix.encode_utf16().collect();
    wide.push(0xD800);
    OsString::from_wide(&wide)
}

#[cfg(unix)]
fn non_unicode(prefix: &str) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    let mut bytes = prefix.as_bytes().to_vec();
    bytes.push(0xFF);
    OsString::from_vec(bytes)
}

fn pinguer_os(args: &[OsString]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pinguer"))
        .args(args)
        .current_dir(fixtures())
        .output()
        .expect("failed to run the pinguer executable")
}

#[test]
fn non_unicode_arguments_are_usage_errors_where_text_is_expected() {
    let os = |s: &str| OsString::from(s);
    for (args, message) in [
        (vec![non_unicode("x")], "unknown command `x\u{FFFD}`"),
        (
            vec![os("version"), non_unicode("y")],
            "unexpected argument `y\u{FFFD}`",
        ),
        (vec![os("debug"), non_unicode("t")], "unknown debug command"),
        (
            vec![os("debug"), os("tokens"), non_unicode("--opt")],
            "unknown option `--opt\u{FFFD}`",
        ),
        (
            vec![
                os("debug"),
                os("tokens"),
                os("hello.pgr"),
                os("--diagnostic-format"),
                non_unicode("json"),
            ],
            "invalid diagnostic format `json\u{FFFD}`",
        ),
    ] {
        let output = pinguer_os(&args);
        // Before `args_os`, these panicked with exit status 101.
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(
            stderr(&output).contains(message),
            "{args:?}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn non_unicode_path_to_a_missing_file_is_reported() {
    let output = pinguer_os(&[
        OsString::from("debug"),
        OsString::from("tokens"),
        non_unicode("missing"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).starts_with("error[P0001]: cannot load `missing\u{FFFD}`: "),
        "{}",
        stderr(&output)
    );
}

// macOS file systems require file names to be valid UTF-8.
#[cfg(any(windows, target_os = "linux"))]
#[test]
fn file_names_that_are_not_unicode_can_be_read() {
    let dir = TempDir::new("non-unicode-name");
    let path = dir.0.join(non_unicode("source"));
    std::fs::write(&path, "let x = 1\n").unwrap();
    let output = pinguer_os(&[
        OsString::from("debug"),
        OsString::from("tokens"),
        path.into(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).starts_with("1:1\tKeyword(Let)\t\"let\"\n"));
}

#[test]
fn missing_file_is_reported() {
    let output = pinguer(&["debug", "tokens", "does-not-exist.pgr"]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    // The message is the compiler's English text; the operating system's
    // message, which may be localized, is quoted in a note with its code
    // (2 is ENOENT on Unix and ERROR_FILE_NOT_FOUND on Windows).
    assert!(
        text.starts_with(
            "error[P0001]: cannot load `does-not-exist.pgr`: file not found\n  \
             = note: the operating system reported: "
        ),
        "{text}"
    );
    assert!(text.contains("(os error 2)\n"), "{text}");
}

#[test]
fn directory_is_reported_as_a_directory() {
    let dir = TempDir::new("directory-input");
    let output = pinguer(&["debug", "tokens", dir.0.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    let first = text.lines().next().unwrap();
    assert!(
        first.starts_with("error[P0001]: cannot load `")
            && first.ends_with("`: is a directory, not a file"),
        "{text}"
    );
    // Windows refuses to open a directory, Linux and macOS refuse to read
    // one; either way the original error is kept.
    assert!(
        text.contains("= note: the operating system reported: "),
        "{text}"
    );
    assert!(text.contains("(os error "), "{text}");
}

#[test]
fn invalid_utf8_is_reported() {
    let dir = std::env::temp_dir().join(format!("pinguer-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("latin1.pgr");
    std::fs::write(&path, b"let s = \"caf\xE9\"\n").unwrap();
    let output = pinguer(&["debug", "tokens", path.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(text.starts_with("error[P0002]: cannot load `"), "{text}");
    assert!(text.contains("invalid byte at offset 12"), "{text}");
    assert!(text.contains("must be encoded in UTF-8"), "{text}");
}

#[test]
fn invalid_utf8_offset_excludes_the_byte_order_mark() {
    let dir = TempDir::new("bom-invalid-utf8");
    let path = dir.file("bom.pgr", b"\xEF\xBB\xBFlet s = \"caf\xE9\"\n");
    let output = pinguer(&["debug", "tokens", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(text.starts_with("error[P0002]: cannot load `"), "{text}");
    // The same offset as without the mark, as LEXICAL_STRUCTURE.md §1 requires.
    assert!(text.contains("invalid byte at offset 12"), "{text}");
}
