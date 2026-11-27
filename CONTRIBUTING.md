# Contributing to Pinguer

The project is in an early phase (0.1.0-dev), so design discussions are as
valuable as code.

## Before you start

* Read the [specification](specifications/LANGUAGE_SPECIFICATION.md), the
  [v0.1 scope](specifications/V0_1_SCOPE.md) and the
  [ADRs](docs/adr/README.md).
* For language changes, open an issue describing the problem, the proposed
  semantics, the alternatives and the interactions with existing features.
  Accepted changes update the specification and, where they change a
  decision, add an ADR.

## Repository layout

| Path | Contents |
|---|---|
| `specifications/` | Language specification and grammar; start at `LANGUAGE_SPECIFICATION.md` |
| `docs/adr/` | Architecture Decision Records |
| `docs/architecture/` | Compiler architecture, diagnostics, CLI, testing strategy |
| `compiler/source` | Source files, spans, line/column mapping |
| `compiler/diagnostics` | Diagnostic model, code registry, human and JSON renderers |
| `compiler/lexer` | Tokens and lexical analysis |
| `compiler/ast` | Syntax tree data model and its text rendering |
| `compiler/parser` | Parsing, syntax error recovery and limits |
| `cli/` | The `pinguer` binary and its end-to-end tests |
| `examples/` | Example programs in v0.1 syntax |

Planned components and their future locations are listed in
[COMPILER_ARCHITECTURE.md](docs/architecture/COMPILER_ARCHITECTURE.md#2-crate-map).

## Development workflow

1. Fork the repository and create a branch from `main`.
2. Follow [docs/BUILDING.md](docs/BUILDING.md) to set up the toolchain.
3. Make focused changes with tests. Do not rewrite unrelated code.
4. Run the checks; all of them must pass. CI runs the same commands, plus
   `cargo build --workspace --all-targets --locked`, with
   `RUSTFLAGS="-D warnings"`:

   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo test --workspace --locked
   RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
   ```

5. Open a pull request explaining what changed and why, and which parts of the
   specification it implements or modifies.

## Engineering rules

* **Specification first.** Language behaviour is specified before or together
  with its implementation. Do not implement undocumented features or change
  semantics silently.
* **No placeholder functionality.** Never simulate compilation, hardcode
  output or present unfinished features as complete. Unavailable features
  must say so explicitly, as the CLI does with exit status 3.
* **Native compilation only.** Do not transpile to another high-level
  language as a substitute for code generation.
* **Tests are mandatory.** Every change includes tests, including malformed
  input and edge cases. Every bug fix includes a regression test.
* **Diagnostics** follow [DIAGNOSTICS.md](docs/architecture/DIAGNOSTICS.md):
  a new compiler diagnostic gets a registered code and a test that triggers
  it, and a help message proposes a change only when the compiler knows it is
  valid.
* **No `unsafe` Rust.** The workspace forbids it; an exception needs
  documented invariants and an ADR.
* **Dependencies** require a justification in the pull request: the problem
  solved, maintenance status, transitive dependencies and licence
  compatibility with Apache-2.0.
* **No unsupported claims.** Performance or safety claims require
  reproducible measurements or tests.
* Public items are documented (`missing_docs` is enforced).
* `LICENSE` and the logo in `assets/` are not modified in ordinary
  contributions.
* Code, comments, diagnostics, tests and documentation are written in
  English.

## License

By contributing, you agree that your contributions are licensed under the
[Apache License, Version 2.0](LICENSE).
