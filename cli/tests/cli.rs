//! End-to-end tests that run the `pinguer` executable.

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
fn debug_tokens_usage_errors() {
    for (args, message) in [
        (&["debug", "tokens", "-"][..], "unknown option `-`"),
        (&["debug"], "missing debug command"),
        (&["debug", "ast"], "unknown debug command `ast`"),
        (&["debug", "tokens"], "missing source file"),
        (&["debug", "tokens", "a.pgr", "b.pgr"], "exactly one file"),
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
fn missing_file_is_reported() {
    let output = pinguer(&["debug", "tokens", "does-not-exist.pgr"]);
    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(
        text.starts_with("error[P0001]: cannot load `does-not-exist.pgr`: "),
        "{text}"
    );
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
