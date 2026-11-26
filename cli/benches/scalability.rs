//! Scalability measurements for the front end.
//!
//! Run with `cargo bench -p pinguer_cli --bench scalability`. For inputs of
//! growing size and several shapes it measures each phase separately:
//!
//! * `read`: reading the file from disk (I/O only);
//! * `load`: `SourceMap::load`, which reads, validates UTF-8 and indexes;
//! * `index`: `SourceMap::add` on text already in memory (line index only);
//! * `lex`: `tokenize` with the CLI's diagnostic limit;
//! * `positions`: converting the start of every token to a line and column;
//! * `render`: rendering the reported diagnostics in both formats;
//! * `listing`: formatting the `debug tokens` listing into a sink.
//!
//! Each value is the fastest of several runs. After each shape, the growth
//! of every phase from one size to twice that size is printed; a linear
//! phase grows by about 2x. Nothing is asserted, because timings depend on
//! the machine: this is a measurement tool, and the deterministic properties
//! are tested in `cli/tests/scalability.rs`.

#![allow(clippy::print_stdout)] // Printing the measurements is the purpose.

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::{Duration, Instant};

use pinguer_diagnostics::{Diagnostics, human, json};
use pinguer_lexer::tokenize;
use pinguer_source::SourceMap;

/// The same limit as the CLI.
const DIAGNOSTIC_LIMIT: usize = 100;

const SIZES: [usize; 8] = [
    10_000, 20_000, 160_000, 320_000, 640_000, 1_000_000, 5_000_000, 10_000_000,
];

/// Input shapes: a unit repeated up to the requested size.
const SHAPES: [(&str, &str); 5] = [
    ("long_valid_line", "let x = y + 1 "),
    ("long_invalid_line", "$"),
    ("many_valid_lines", "let x = y + 1\n"),
    ("many_invalid_lines", "$\n"),
    ("unicode_tabs_line", "\"é🐧ção\"\t"),
];

const PHASES: [&str; 7] = [
    "read",
    "load",
    "index",
    "lex",
    "positions",
    "render",
    "listing",
];

fn input(unit: &str, size: usize) -> String {
    let mut text = unit.repeat(size / unit.len() + 1);
    let mut end = size;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}

/// Runs `f` `runs` times and returns the fastest duration and the last
/// result.
fn fastest<T>(runs: usize, mut f: impl FnMut() -> T) -> (Duration, T) {
    let mut best = Duration::MAX;
    let mut result = None;
    for _ in 0..runs {
        let start = Instant::now();
        let value = f();
        best = best.min(start.elapsed());
        result = Some(value);
    }
    (best, result.expect("at least one run"))
}

struct Measurement {
    size: usize,
    phases: [Duration; 7],
    tokens: usize,
    errors: usize,
    rendered_bytes: usize,
}

fn measure(unit: &str, size: usize, path: &std::path::Path) -> Measurement {
    let text = input(unit, size);
    std::fs::write(path, &text).expect("cannot write the input file");
    let runs = if size <= 1_000_000 { 5 } else { 3 };

    let (read, _) = fastest(runs, || std::fs::read(path).expect("cannot read the input"));
    let (load, _) = fastest(runs, || {
        SourceMap::new().load(path).expect("the input loads");
    });
    // `index` includes copying the text into the source map.
    let (index, (map, file)) = fastest(runs, || {
        let mut map = SourceMap::new();
        let file = map.add("input.pgr", text.clone()).expect("the input fits");
        (map, file)
    });
    let source = map.file(file);

    let (lex, (tokens, diagnostics)) = fastest(runs, || {
        let mut diagnostics = Diagnostics::with_limit(DIAGNOSTIC_LIMIT);
        let tokens = tokenize(file, source.text(), &mut diagnostics);
        (tokens, diagnostics)
    });
    let (positions, _) = fastest(runs, || {
        tokens
            .iter()
            .map(|token| source.line_col(token.span.start).column as u64)
            .sum::<u64>()
    });
    let notice = diagnostics.suppression_notice();
    let (render, rendered_bytes) = fastest(runs, || {
        let mut bytes = 0;
        for diagnostic in diagnostics.iter().chain(notice.as_ref()) {
            bytes += human::render(&map, diagnostic).len();
            bytes += json::render(&map, diagnostic).len() + 1;
        }
        bytes
    });
    let (listing, _) = fastest(runs, || {
        let mut sink = std::io::BufWriter::new(std::io::sink());
        for token in &tokens {
            let start = source.line_col(token.span.start);
            let text = map.snippet(token.span);
            let _ = writeln!(sink, "{start}\t{:?}\t{text:?}", token.kind);
        }
    });

    Measurement {
        size,
        phases: [read, load, index, lex, positions, render, listing],
        tokens: tokens.len(),
        errors: diagnostics.error_count(),
        rendered_bytes,
    }
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn main() {
    let path = std::env::temp_dir().join(format!("pinguer-scalability-{}.pgr", std::process::id()));
    for (shape, unit) in SHAPES {
        println!("\n{shape}");
        let mut header = format!("{:>10}", "bytes");
        for phase in PHASES {
            let _ = write!(header, " {phase:>10}");
        }
        let _ = write!(
            header,
            " {:>10} {:>10} {:>10}",
            "tokens", "errors", "rendered"
        );
        println!("{header}");
        let mut results = Vec::new();
        for size in SIZES {
            let m = measure(unit, size, &path);
            let mut row = format!("{:>10}", m.size);
            for phase in m.phases {
                let _ = write!(row, " {:>10.3}", milliseconds(phase));
            }
            let _ = write!(
                row,
                " {:>10} {:>10} {:>10}",
                m.tokens, m.errors, m.rendered_bytes
            );
            println!("{row}");
            results.push(m);
        }
        for pair in results.windows(2) {
            if pair[1].size != 2 * pair[0].size {
                continue;
            }
            let mut row = format!("{:>10}", format!("x2 @{}", pair[0].size));
            for (before, after) in pair[0].phases.iter().zip(&pair[1].phases) {
                let ratio = after.as_secs_f64() / before.as_secs_f64().max(1e-9);
                let _ = write!(row, " {ratio:>10.2}");
            }
            println!("{row}");
        }
    }
    let _ = std::fs::remove_file(&path);
    println!("\ntimes in milliseconds, fastest of 5 runs (3 above 1 MB)");
}
