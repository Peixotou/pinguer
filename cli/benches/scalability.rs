//! Scalability measurements for the front end.
//!
//! Run with `cargo bench -p pinguer_cli --bench scalability`. For inputs of
//! growing size and several shapes it measures each phase separately:
//!
//! * `read`: reading the file from disk (I/O only);
//! * `load`: `SourceMap::load`, which reads, validates UTF-8 and indexes;
//! * `index`: `SourceMap::add` on text already in memory (line index only);
//! * `lex`: `tokenize` with the CLI's diagnostic limit;
//! * `parse`: `pinguer_parser::parse` on tokens already produced, including
//!   its line-break filter and node numbering;
//! * `tree`: rendering the syntax tree, as `debug ast` does, into a sink;
//! * `drop`: dropping the syntax tree;
//! * `positions`: converting the start of every token to a line and column;
//! * `render`: rendering the reported diagnostics in both formats;
//! * `listing`: formatting the `debug tokens` listing into a sink.
//!
//! Each value is the fastest of several runs. After each shape, the growth
//! of every phase from one size to twice that size is printed; a linear
//! phase grows by about 2x. Nothing is asserted, because timings depend on
//! the machine: this is a measurement tool, and the deterministic properties
//! are tested in `cli/tests/scalability.rs`.
//!
//! For the syntax tree it also prints the bytes per node and allocations per
//! node of its heap: every `Box`, and every boxed slice or string that is
//! not empty, is one allocation of exactly the size of its contents. They
//! are counted by walking the tree, so they do not depend on the allocator,
//! whose own overhead is not included.

#![allow(clippy::print_stdout)] // Printing the measurements is the purpose.

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::{Duration, Instant};

use pinguer_ast as ast;
use pinguer_diagnostics::{Diagnostics, human, json};
use pinguer_lexer::tokenize;
use pinguer_parser::parse;
use pinguer_source::SourceMap;

/// The same limit as the CLI.
const DIAGNOSTIC_LIMIT: usize = 100;

const SIZES: [usize; 8] = [
    10_000, 20_000, 160_000, 320_000, 640_000, 1_000_000, 5_000_000, 10_000_000,
];

/// Input shapes: a prefix, then a unit repeated up to the requested size.
/// The first five were chosen for the lexer; at the top level, `let` is a
/// misplaced declaration, reported once, after which recovery skips to the
/// next item, so all of `many_valid_lines` becomes one error item. The last
/// three are valid programs, or nearly: the body of `many_statements` is
/// not closed.
const SHAPES: [(&str, &str, &str); 8] = [
    ("long_valid_line", "", "let x = y + 1 "),
    ("long_invalid_line", "", "$"),
    ("many_valid_lines", "", "let x = y + 1\n"),
    ("many_invalid_lines", "", "$\n"),
    ("unicode_tabs_line", "", "\"é🐧ção\"\t"),
    ("long_expression", "const X = 0", " + x * 2"),
    ("many_statements", "func f() {\n", "    g(x, y + 1)\n"),
    (
        "many_functions",
        "",
        "func f(a: int) -> int {\n    return a * 2 + 1\n}\n",
    ),
];

const PHASES: [&str; 10] = [
    "read",
    "load",
    "index",
    "lex",
    "parse",
    "tree",
    "drop",
    "positions",
    "render",
    "listing",
];

fn input(prefix: &str, unit: &str, size: usize) -> String {
    let mut text = prefix.to_owned();
    text.push_str(&unit.repeat(size / unit.len() + 1));
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

/// Counts what is written to it and discards it.
struct Sink(usize);

impl std::fmt::Write for Sink {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0 += text.len();
        Ok(())
    }
}

/// The heap of a syntax tree: the bytes it asks the allocator for and the
/// number of allocations.
#[derive(Default)]
struct Heap {
    bytes: usize,
    allocations: usize,
}

impl Heap {
    fn of(file: &ast::File) -> Self {
        let mut heap = Heap::default();
        heap.list::<ast::Item>(file.items.len());
        for item in &file.items {
            heap.item(item);
        }
        heap
    }

    /// Counts an allocation of `count` values of type `T`, if it is one.
    fn list<T>(&mut self, count: usize) {
        if count > 0 {
            self.bytes += count * size_of::<T>();
            self.allocations += 1;
        }
    }

    fn ident(&mut self, ident: &ast::Ident) {
        self.list::<u8>(ident.name.len());
    }

    fn item(&mut self, item: &ast::Item) {
        match &item.kind {
            ast::ItemKind::Import(import) => {
                self.list::<ast::Ident>(import.path.len());
                for name in &import.path {
                    self.ident(name);
                }
            }
            ast::ItemKind::Func(func) => {
                self.ident(&func.name);
                self.list::<ast::Param>(func.params.len());
                for param in &func.params {
                    self.ident(&param.name);
                    self.ty(&param.ty);
                }
                if let Some(ret) = &func.ret {
                    self.ty(ret);
                }
                self.block(&func.body);
            }
            ast::ItemKind::Const(binding) => self.binding(binding),
            ast::ItemKind::Error => {}
        }
    }

    fn ty(&mut self, ty: &ast::Type) {
        match &ty.kind {
            ast::TypeKind::Name(name) => self.ident(name),
            ast::TypeKind::Unit => {}
        }
    }

    fn binding(&mut self, binding: &ast::Binding) {
        self.ident(&binding.name);
        if let Some(ty) = &binding.ty {
            self.list::<ast::Type>(1);
            self.ty(ty);
        }
        self.expr(&binding.value);
    }

    fn block(&mut self, block: &ast::Block) {
        self.list::<ast::Stmt>(block.stmts.len());
        for stmt in &block.stmts {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &ast::Stmt) {
        match &stmt.kind {
            ast::StmtKind::Binding(binding) => self.binding(binding),
            ast::StmtKind::Assign { target, value, .. } => {
                self.ident(target);
                self.expr(value);
            }
            ast::StmtKind::If(if_stmt) => {
                self.list::<ast::IfBranch>(if_stmt.branches.len());
                for branch in &if_stmt.branches {
                    self.expr(&branch.cond);
                    self.block(&branch.block);
                }
                if let Some(block) = &if_stmt.else_block {
                    self.block(block);
                }
            }
            ast::StmtKind::While { cond, body } => {
                self.expr(cond);
                self.block(body);
            }
            ast::StmtKind::Expr(expr) | ast::StmtKind::Return(Some(expr)) => self.expr(expr),
            ast::StmtKind::Block(block) => self.block(block),
            ast::StmtKind::Return(None)
            | ast::StmtKind::Break
            | ast::StmtKind::Continue
            | ast::StmtKind::Error => {}
        }
    }

    fn boxed(&mut self, expr: &ast::Expr) {
        self.list::<ast::Expr>(1);
        self.expr(expr);
    }

    fn expr(&mut self, expr: &ast::Expr) {
        match &expr.kind {
            ast::ExprKind::Literal(ast::Literal::Float(text) | ast::Literal::String(text)) => {
                self.list::<u8>(text.len());
            }
            ast::ExprKind::Literal(_) | ast::ExprKind::Error => {}
            ast::ExprKind::Name(name) => self.ident(name),
            ast::ExprKind::Paren(inner) => self.boxed(inner),
            ast::ExprKind::Unary { operand, .. } => self.boxed(operand),
            ast::ExprKind::Binary { first, rest } => {
                self.boxed(first);
                self.list::<ast::BinaryOperand>(rest.len());
                for pair in rest {
                    self.expr(&pair.operand);
                }
            }
            ast::ExprKind::Cast { operand, ty } => {
                self.boxed(operand);
                self.list::<ast::Type>(1);
                self.ty(ty);
            }
            ast::ExprKind::Call { callee, args } => {
                self.boxed(callee);
                self.list::<ast::Expr>(args.len());
                for arg in args {
                    self.expr(arg);
                }
            }
            ast::ExprKind::Member { base, name } => {
                self.boxed(base);
                self.ident(name);
            }
        }
    }
}

/// Runs `f` on a fresh value from `setup` `runs` times, timing only `f`,
/// and returns the fastest duration and the last result.
fn fastest_with<S, T>(
    runs: usize,
    mut setup: impl FnMut() -> S,
    mut f: impl FnMut(S) -> T,
) -> (Duration, T) {
    let mut best = Duration::MAX;
    let mut result = None;
    for _ in 0..runs {
        let input = setup();
        let start = Instant::now();
        let value = f(input);
        best = best.min(start.elapsed());
        result = Some(value);
    }
    (best, result.expect("at least one run"))
}

struct Measurement {
    size: usize,
    phases: [Duration; 10],
    tokens: usize,
    nodes: u32,
    heap: Heap,
    errors: usize,
    rendered_bytes: usize,
}

impl Measurement {
    /// Heap bytes and allocations of the syntax tree per node.
    fn per_node(&self) -> (f64, f64) {
        let nodes = f64::from(self.nodes.max(1));
        (
            self.heap.bytes as f64 / nodes,
            self.heap.allocations as f64 / nodes,
        )
    }
}

fn measure(prefix: &str, unit: &str, size: usize, path: &std::path::Path) -> Measurement {
    let text = input(prefix, unit, size);
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
    // Parsing consumes the tokens, so each run gets a copy, made outside the
    // timed part. Parse diagnostics go to a collection of their own, so that
    // the lexer's are rendered alone below as before.
    let (parse_time, tree) = fastest_with(
        runs,
        || (tokens.clone(), Diagnostics::with_limit(DIAGNOSTIC_LIMIT)),
        |(tokens, mut parse_diagnostics)| {
            parse(file, source.text(), tokens, &mut parse_diagnostics)
        },
    );
    let (tree_time, _) = fastest(runs, || {
        let mut sink = Sink(0);
        pinguer_ast::render(&mut sink, &tree, source).expect("the sink accepts everything");
        sink.0
    });
    let nodes = tree.node_count;
    let heap = Heap::of(&tree);
    drop(tree);
    let (drop_time, ()) = fastest_with(
        runs,
        || {
            let mut parse_diagnostics = Diagnostics::with_limit(DIAGNOSTIC_LIMIT);
            parse(file, source.text(), tokens.clone(), &mut parse_diagnostics)
        },
        drop,
    );
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
        phases: [
            read, load, index, lex, parse_time, tree_time, drop_time, positions, render, listing,
        ],
        tokens: tokens.len(),
        nodes,
        heap,
        errors: diagnostics.error_count(),
        rendered_bytes,
    }
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn main() {
    let path = std::env::temp_dir().join(format!("pinguer-scalability-{}.pgr", std::process::id()));
    for (shape, prefix, unit) in SHAPES {
        println!("\n{shape}");
        let mut header = format!("{:>10}", "bytes");
        for phase in PHASES {
            let _ = write!(header, " {phase:>10}");
        }
        let _ = write!(
            header,
            " {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
            "tokens", "nodes", "B/node", "allocs/n", "errors", "rendered"
        );
        println!("{header}");
        let mut results = Vec::new();
        for size in SIZES {
            let m = measure(prefix, unit, size, &path);
            let mut row = format!("{:>10}", m.size);
            for phase in m.phases {
                let _ = write!(row, " {:>10.3}", milliseconds(phase));
            }
            let (bytes, allocations) = m.per_node();
            let _ = write!(
                row,
                " {:>10} {:>10} {bytes:>10.1} {allocations:>10.2} {:>10} {:>10}",
                m.tokens, m.nodes, m.errors, m.rendered_bytes
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
    println!("B/node, allocs/n: heap bytes and allocations of the syntax tree per node");
}
