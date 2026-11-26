//! The `pinguer` command-line interface.
//!
//! In version 0.1.0-dev only the commands backed by real functionality are
//! available. Every planned command is recognised and reports that it is not
//! available yet, instead of pretending to work.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use pinguer_diagnostics::{Diagnostic, Diagnostics, codes, human, json};
use pinguer_source::{SourceError, SourceMap};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The number of diagnostics one invocation reports.
///
/// One hundred is enough to fix many independent problems in one run (it is
/// also the default of `javac -Xmaxerrs`), while bounding the output and the
/// memory spent on diagnostics when an input produces millions of them.
/// Further diagnostics are counted, included in the error count and
/// summarized by one `P0004` note.
const DIAGNOSTIC_LIMIT: usize = 100;

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
///
/// Output is written as it is produced instead of being assembled in memory
/// first, so memory use does not grow with the size of the output.
struct Output {
    stdout: io::BufWriter<io::StdoutLock<'static>>,
    stderr: io::StderrLock<'static>,
    /// Set when writing to standard output failed, for example because it is
    /// a closed pipe (`pinguer debug tokens x | head`). That is not an error
    /// worth reporting, but there is no point in producing more output.
    stdout_closed: bool,
}

impl Output {
    fn out(&mut self, text: &str) {
        self.out_fmt(format_args!("{text}"));
    }

    fn out_fmt(&mut self, args: fmt::Arguments<'_>) {
        if !self.stdout_closed && self.stdout.write_fmt(args).is_err() {
            self.stdout_closed = true;
        }
    }

    fn err(&mut self, text: &str) {
        let _ = self.stderr.write_all(text.as_bytes());
    }

    /// Writes the retained diagnostics, then the suppression notice if there
    /// is one, rendering one diagnostic at a time.
    fn diagnostics(
        &mut self,
        map: &SourceMap,
        diagnostics: &Diagnostics,
        format: DiagnosticFormat,
    ) {
        if diagnostics.is_empty() {
            return;
        }
        // Keep the order of a terminal that shows both streams: everything
        // printed to standard output so far comes before the diagnostics.
        if !self.stdout_closed && self.stdout.flush().is_err() {
            self.stdout_closed = true;
        }
        let notice = diagnostics.suppression_notice();
        for (index, diagnostic) in diagnostics.iter().chain(notice.as_ref()).enumerate() {
            match format {
                DiagnosticFormat::Human => {
                    if index > 0 {
                        self.err("\n");
                    }
                    self.err(&human::render(map, diagnostic));
                }
                DiagnosticFormat::Json => {
                    self.err(&json::render(map, diagnostic));
                    self.err("\n");
                }
            }
        }
        let errors = diagnostics.error_count();
        if format == DiagnosticFormat::Human && errors > 0 {
            let plural = if errors == 1 { "" } else { "s" };
            self.err(&format!(
                "\nerror: aborting due to {errors} previous error{plural}\n"
            ));
        }
    }

    fn usage_error(&mut self, message: &str) -> u8 {
        let diagnostic = Diagnostic::error(message).with_note("run `pinguer help` for usage");
        self.err(&human::render(&SourceMap::new(), &diagnostic));
        status::USAGE_ERROR
    }
}

fn main() -> ExitCode {
    // `args_os`, not `args`: an argument that is not valid Unicode, such as a
    // file name on Linux, must not make the process panic.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mut output = Output {
        stdout: io::BufWriter::with_capacity(64 * 1024, io::stdout().lock()),
        stderr: io::stderr().lock(),
        stdout_closed: false,
    };
    let code = run(&args, &mut output);
    let _ = output.stdout.flush();
    ExitCode::from(code)
}

fn run(args: &[OsString], output: &mut Output) -> u8 {
    let Some(command) = args.first() else {
        output.out(&help_text());
        return status::SUCCESS;
    };
    let Some(command) = command.to_str() else {
        return output.usage_error(&format!("unknown command `{}`", command.display()));
    };
    let rest = &args[1..];
    let takes_no_arguments = matches!(
        command,
        "-h" | "--help" | "help" | "-V" | "--version" | "version"
    );
    if takes_no_arguments && let Some(extra) = rest.first() {
        return output.usage_error(&format!(
            "unexpected argument `{}`; `{command}` takes no arguments",
            extra.display()
        ));
    }
    match command {
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

fn debug(args: &[OsString], output: &mut Output) -> u8 {
    match args.first() {
        Some(command) if command == "tokens" => debug_tokens(&args[1..], output),
        Some(other) => output.usage_error(&format!("unknown debug command `{}`", other.display())),
        None => output.usage_error("missing debug command; the available one is `tokens`"),
    }
}

/// Parses the arguments of `debug tokens`. Options are compared as text; an
/// argument that is not valid Unicode can only be the file name, which is
/// passed to the operating system unchanged.
fn debug_tokens(args: &[OsString], output: &mut Output) -> u8 {
    let mut format = DiagnosticFormat::Human;
    let mut path: Option<&OsStr> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_os_str();
        let text = arg.to_str();
        let value = if let Some(value) = text.and_then(|t| t.strip_prefix("--diagnostic-format=")) {
            Some(OsStr::new(value))
        } else if text == Some("--diagnostic-format") {
            index += 1;
            match args.get(index) {
                Some(value) => Some(value.as_os_str()),
                None => return output.usage_error("`--diagnostic-format` requires a value"),
            }
        } else {
            None
        };
        if let Some(value) = value {
            format = match value.to_str() {
                Some("human") => DiagnosticFormat::Human,
                Some("json") => DiagnosticFormat::Json,
                _ => {
                    return output.usage_error(&format!(
                        "invalid diagnostic format `{}`; expected `human` or `json`",
                        value.display()
                    ));
                }
            };
        } else if arg.as_encoded_bytes().starts_with(b"-") {
            // Reading from standard input (`-`) is not supported, and file
            // names starting with `-` must be written as `./-name`.
            return output.usage_error(&format!("unknown option `{}`", arg.display()));
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
    let mut diagnostics = Diagnostics::with_limit(DIAGNOSTIC_LIMIT);
    let path = Path::new(path);
    let file = match map.load(path) {
        Ok(file) => file,
        Err(error) => {
            diagnostics.push(load_error(path, &error));
            output.diagnostics(&map, &diagnostics, format);
            return status::COMPILE_ERROR;
        }
    };

    let source = map.file(file);
    let tokens = pinguer_lexer::tokenize(file, source.text(), &mut diagnostics);
    for token in &tokens {
        if output.stdout_closed {
            break;
        }
        let start = source.line_col(token.span.start);
        let text = map.snippet(token.span);
        output.out_fmt(format_args!("{start}\t{:?}\t{text:?}\n", token.kind));
    }
    output.diagnostics(&map, &diagnostics, format);
    if diagnostics.has_errors() {
        status::COMPILE_ERROR
    } else {
        status::SUCCESS
    }
}

fn load_error(path: &Path, error: &SourceError) -> Diagnostic {
    let code = match error {
        SourceError::Io(_) => codes::CANNOT_READ_FILE,
        SourceError::InvalidUtf8 { .. } => codes::INVALID_UTF8,
        SourceError::TooLarge { .. } => codes::FILE_TOO_LARGE,
    };
    let mut diagnostic =
        Diagnostic::error(format!("cannot load `{}`: {error}", path.display())).with_code(code);
    if matches!(error, SourceError::InvalidUtf8 { .. }) {
        diagnostic = diagnostic.with_note("Pinguer source files must be encoded in UTF-8");
    }
    diagnostic
}
