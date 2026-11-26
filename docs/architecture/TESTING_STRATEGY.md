# Testing Strategy

## 1. Principles

* Every implemented component has tests for valid input, malformed input and
  edge cases. A feature without tests is not done.
* Every normative rule in the specification has a test; every diagnostic code
  has a test that triggers it.
* Tests are deterministic. Randomized tests use fixed seeds and print the
  seed on failure.
* Expected output is written out in full in the test (snapshot style), so
  that any change in diagnostics or token streams is a visible diff in review.

## 2. Current test suite (Phase 01.5)

| Layer | Location | What it covers |
|---|---|---|
| Unit | `compiler/source/src/lib.rs` | Line/column mapping (LF, CRLF, lone CR, multi-byte characters, end of file), checked against a direct definition at every offset of pseudo-random text; BOM removal and offsets after it; line text keeping lone carriage returns; span operations; UTF-8 validation on load; bounded reading (endless sources, wrong size hints, files at and over a reduced limit); English load errors, including directories |
| Unit | `compiler/diagnostics/src/*.rs` | Code registry uniqueness and ranges; the diagnostic limit and suppression notice; human renderer layout (gutters, multiple labels, multiple files, tabs, empty and multi-line spans, cut excerpts of long lines); escaping of every invisible-character range; JSON rendering and escaping, including surrogate pairs |
| Unit | `compiler/lexer/src/token.rs` | Keyword table consistency |
| Integration | `compiler/lexer/tests/lexer.rs` | Every token kind, newline significance rules, every lexical diagnostic, recovery after errors (including `Unknown` tokens for rejected characters), source ordering of diagnostics, the diagnostic limit |
| Property | `compiler/lexer/tests/robustness.rs` | 9,000 pseudo-random inputs plus deep nesting: no panics, span invariants, newline invariants, determinism, and limited lexing reporting a prefix of unlimited lexing |
| End-to-end | `cli/tests/cli.rs` | Running the `pinguer` binary: version, help, unavailable commands, usage errors in both formats, arguments that are not valid Unicode, token listing, a closed standard output, human and JSON diagnostics, the diagnostic limit, missing, non-UTF-8 and directory inputs |
| Scalability | `cli/tests/scalability.rs` | Inputs of up to 2 MB: diagnostic output independent of input size, narrow excerpts on long lines, exact positions deep inside long Unicode lines and after many lines. No timings are asserted. |
| Benchmark | `cli/benches/scalability.rs` | Not a test: times reading, loading, indexing, lexing, position lookup, diagnostic rendering and listing separately for inputs from 10 KB to 10 MB, and prints the growth per doubling |

Commands:

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo bench -p pinguer_cli --bench scalability   # measurements only
```

Ordinary tests never assert timings, which vary between machines and CI
runners; performance is measured by the benchmark and compared by its
growth ratios, which are about 2 per doubling for every phase that depends
on the input size.

## 3. Planned layers

| Layer | When | Approach |
|---|---|---|
| Parser tests and AST snapshots | Phase 02 | Pretty-printed AST compared against expected text; malformed input for every production |
| Type-checking tests | Phase 03 | A directory of `.pgr` files annotated with expected diagnostics (`// error[P4001]` comments), checked by a harness |
| Diagnostic snapshots | Phase 02+ | Full rendered output of every diagnostic |
| Compile-and-run tests | Phase 04 | Programs with expected stdout, stderr and exit status, run on every CI platform; includes one test per trap condition |
| LLVM IR snapshots | Phase 04 | Generated `.ll` for small programs |
| Fuzzing | Before v0.1.0 | `cargo-fuzz` (libFuzzer) targets for the lexer and parser; at least 24 CPU-hours per target before release |
| Property-based tests | Phase 03+ | Considered: `proptest` for type-checker invariants, if the dependency is justified |
| Differential tests | Phase 04+ | Constant folding versus run-time evaluation of the same expressions |
| Benchmarks | After codegen | Compile-time benchmarks of the whole pipeline, extending the front-end benchmark; run-time benchmarks only with documented methodology. No performance claims without published measurements. |
| Reproducible builds | Phase 04 | Build twice, compare executables |

Windows Application Control can prevent test executables from running; see
[BUILDING.md](../BUILDING.md#windows).

## 4. Continuous integration

`.github/workflows/ci.yml` runs on every push to `main` and every pull
request:

* formatting, Clippy (warnings denied) and documentation build on Linux;
* build and full test suite on Linux x86-64, Windows x86-64 and macOS ARM64.

Linux ARM64 and Windows ARM64 runners will be added when they are available
to the repository.

## 5. Milestone completion criteria

A milestone is complete when:

1. its features match the specification;
2. its tests cover positive, negative and edge cases, and all pass on CI;
3. its diagnostics have codes, tests and documentation;
4. the documentation describes only behaviour that exists.
