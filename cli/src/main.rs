//! The `pinguer` command-line interface.
//!
//! In version 0.1.0-dev only the commands backed by real functionality are
//! available. Every planned command is recognised and reports that it is not
//! available yet, instead of pretending to work.

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use pinguer_diagnostics::{Diagnostic, Diagnostics, codes, human, json};
use pinguer_source::{SourceError, SourceMap};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Process exit statuses. Documented in `docs/architecture/CLI.md`.
mod status {
    /// The command succeeded.
    pub(crate) const SUCCESS: u8 = 0;
    /// The input contained errors, which were reported as diagnostics.
    pub(crate) const COMPILE_ERROR: u8 = 1;
    /// The command line was invalid.
    pub(crate) const USAGE_ERROR: u8 = 2;
    /// The command exists in the roadmap but is not implemented yet.
    pub(crate) const UNAVAILABLE: u8 = 3;
}

/// Commands that are part of the planned CLI but not implemented yet.
const PLANNED_COMMANDS: &[&str] = &[
    "new", "init", "run", "build", "check", "test", "fmt", "lint", "doc", "add", "remove",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiagnosticFormat {
    Human,
    Json,
}

/// Where command output goes. Abstracted so that output errors such as a
/// closed pipe are handled in one place.
struct Output {
    stdout: io::StdoutLock<'static>,
    stderr: io::StderrLock<'static>,
}

impl Output {
    fn out(&mut self, text: &str) {
        // A closed stdout (for example `pinguer debug tokens x | head`) is not
        // an error worth reporting.
        let _ = self.stdout.write_all(text.as_bytes());
    }

    fn err(&mut self, text: &str) {
        let _ = self.stderr.write_all(text.as_bytes());
    }

    fn diagnostics(
        &mut self,
        map: &SourceMap,
        diagnostics: &Diagnostics,
        format: DiagnosticFormat,
    ) {
        if diagnostics.is_empty() {
            return;
        }
        match format {
            DiagnosticFormat::Human => {
                let mut text = human::render_all(map, diagnostics);
                let errors = diagnostics.error_count();
                if errors > 0 {
                    let plural = if errors == 1 { "" } else { "s" };
                    text.push_str(&format!(
                        "\nerror: aborting due to {errors} previous error{plural}\n"
                    ));
                }
                self.err(&text);
            }
            DiagnosticFormat::Json => self.err(&json::render_all(map, diagnostics)),
        }
    }

    fn usage_error(&mut self, message: &str) -> u8 {
        let diagnostic = Diagnostic::error(message).with_note("run `pinguer help` for usage");
        self.err(&human::render(&SourceMap::new(), &diagnostic));
        status::USAGE_ERROR
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut output = Output {
        stdout: io::stdout().lock(),
        stderr: io::stderr().lock(),
    };
    let code = run(&args, &mut output);
    let _ = output.stdout.flush();
    ExitCode::from(code)
}

fn run(args: &[String], output: &mut Output) -> u8 {
    let Some(command) = args.first() else {
        output.out(&help_text());
        return status::SUCCESS;
    };
    let rest = &args[1..];
    let takes_no_arguments = matches!(
        command.as_str(),
        "-h" | "--help" | "help" | "-V" | "--version" | "version"
    );
    if takes_no_arguments && let Some(extra) = rest.first() {
        return output.usage_error(&format!(
            "unexpected argument `{extra}`; `{command}` takes no arguments"
        ));
    }
    match command.as_str() {
        "-h" | "--help" | "help" => {
            output.out(&help_text());
            status::SUCCESS
        }
        "-V" | "--version" | "version" => {
            output.out(&format!("pinguer {VERSION}\n"));
            status::SUCCESS
        }
        "debug" => debug(rest, output),
        planned if PLANNED_COMMANDS.contains(&planned) => {
            let diagnostic = Diagnostic::error(format!(
                "`pinguer {planned}` is not available in Pinguer {VERSION}"
            ))
            .with_note(
                "this version implements source loading, lexical analysis and diagnostics only",
            )
            .with_note("see ROADMAP.md for when this command is planned");
            output.err(&human::render(&SourceMap::new(), &diagnostic));
            status::UNAVAILABLE
        }
        unknown => output.usage_error(&format!("unknown command `{unknown}`")),
    }
}

fn help_text() -> String {
    format!(
        "\
pinguer {VERSION}

Usage: pinguer <command> [options]

Commands:
  help                  Print this help message
  version               Print version information
  debug tokens <file>   Print the token stream of a source file

Planned commands (not available in {VERSION}):
  {planned}

Options:
  -h, --help            Print this help message
  -V, --version         Print version information

Options for `debug tokens`:
  --diagnostic-format <human|json>
                        How to print diagnostics (default: human)

The `debug` commands exist for compiler development; their output format is
not stable.
",
        planned = PLANNED_COMMANDS.join(", ")
    )
}

fn debug(args: &[String], output: &mut Output) -> u8 {
    match args.first().map(String::as_str) {
        Some("tokens") => debug_tokens(&args[1..], output),
        Some(other) => output.usage_error(&format!("unknown debug command `{other}`")),
        None => output.usage_error("missing debug command; the available one is `tokens`"),
    }
}

fn debug_tokens(args: &[String], output: &mut Output) -> u8 {
    let mut format = DiagnosticFormat::Human;
    let mut path = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        let value = if let Some(value) = arg.strip_prefix("--diagnostic-format=") {
            Some(value)
        } else if arg == "--diagnostic-format" {
            index += 1;
            match args.get(index) {
                Some(value) => Some(value.as_str()),
                None => return output.usage_error("`--diagnostic-format` requires a value"),
            }
        } else {
            None
        };
        if let Some(value) = value {
            format = match value {
                "human" => DiagnosticFormat::Human,
                "json" => DiagnosticFormat::Json,
                other => {
                    return output.usage_error(&format!(
                        "invalid diagnostic format `{other}`; expected `human` or `json`"
                    ));
                }
            };
        } else if arg.starts_with('-') {
            // Reading from standard input (`-`) is not supported, and file
            // names starting with `-` must be written as `./-name`.
            return output.usage_error(&format!("unknown option `{arg}`"));
        } else if path.is_some() {
            return output.usage_error("`debug tokens` accepts exactly one file");
        } else {
            path = Some(arg);
        }
        index += 1;
    }
    let Some(path) = path else {
        return output.usage_error("missing source file for `debug tokens`");
    };

    let mut map = SourceMap::new();
    let mut diagnostics = Diagnostics::new();
    let file = match map.load(Path::new(path)) {
        Ok(file) => file,
        Err(error) => {
            diagnostics.push(load_error(path, &error));
            output.diagnostics(&map, &diagnostics, format);
            return status::COMPILE_ERROR;
        }
    };

    let source = map.file(file);
    let tokens = pinguer_lexer::tokenize(file, source.text(), &mut diagnostics);
    let mut listing = String::new();
    for token in &tokens {
        let start = source.line_col(token.span.start);
        let text = map.snippet(token.span);
        listing.push_str(&format!("{start}\t{:?}\t{text:?}\n", token.kind));
    }
    output.out(&listing);
    output.diagnostics(&map, &diagnostics, format);
    if diagnostics.has_errors() {
        status::COMPILE_ERROR
    } else {
        status::SUCCESS
    }
}

fn load_error(path: &str, error: &SourceError) -> Diagnostic {
    let code = match error {
        SourceError::Io(_) => codes::CANNOT_READ_FILE,
        SourceError::InvalidUtf8 { .. } => codes::INVALID_UTF8,
        SourceError::TooLarge { .. } => codes::FILE_TOO_LARGE,
    };
    let mut diagnostic =
        Diagnostic::error(format!("cannot load `{path}`: {error}")).with_code(code);
    if matches!(error, SourceError::InvalidUtf8 { .. }) {
        diagnostic = diagnostic.with_note("Pinguer source files must be encoded in UTF-8");
    }
    diagnostic
}
