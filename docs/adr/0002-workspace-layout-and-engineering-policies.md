# 0002. Workspace layout and engineering policies

- Status: Accepted, partially superseded by [0023](0023-no-placeholder-directories.md)
- Date: 2026-11-24

## Context

The repository started with a directory skeleton that mirrors a full
compiler pipeline (`compiler/lexer`, `parser`, `ast`, `semantic`, `types`,
`ir`, `optimizer`, `codegen`, plus `runtime`, `stdlib`, `cli`, …). Creating a
crate for every directory immediately would produce empty abstractions, and
crate boundaries are expensive to change once code depends on them.

## Decision

### Crates

The Cargo workspace contains only crates with real code:

| Crate | Directory | Responsibility |
|---|---|---|
| `pinguer_source` | `compiler/source` | Source files, file IDs, spans, line/column mapping |
| `pinguer_diagnostics` | `compiler/diagnostics` | Diagnostic data model, code registry, human and JSON renderers |
| `pinguer_lexer` | `compiler/lexer` | Tokens and lexical analysis |
| `pinguer_cli` (binary `pinguer`) | `cli` | Command-line interface |

`compiler/source` and `compiler/diagnostics` were added to the original
skeleton because every stage depends on them and they have no dependencies on
any stage.

The remaining skeleton directories stay as placeholders with a README that
states their planned role (superseded by ADR-0023: placeholder directories
were removed). A new crate is created when its first real code is
written, and a component is split into several crates only when a dependency
boundary or build-time concern justifies it. The planned mapping is in
[COMPILER_ARCHITECTURE.md](../architecture/COMPILER_ARCHITECTURE.md).

### Policies

* **No unsafe Rust.** `unsafe_code = "forbid"` is set for the whole
  workspace. A crate that needs unsafe code must override the lint locally,
  document every invariant next to each `unsafe` block, and record the reason
  in an ADR.
* **No third-party dependencies in Phase 01.** The standard library covers the
  current needs. Each future dependency must be justified (problem solved,
  maintenance status, transitive dependencies, licence compatibility with
  Apache-2.0) in the pull request that adds it.
* **Warnings are errors in CI** (`-D warnings`, including `missing_docs` for
  public items and selected Clippy lints).
* **Edition 2024** (Rust 1.85 or newer; the code also uses `let` chains, which
  need Rust 1.88). The minimum supported
  Rust version is not yet tested and is therefore not declared.

## Alternatives considered

* **One crate per pipeline stage now.** Rejected: empty crates, premature
  boundaries.
* **A single compiler crate.** Simpler today, but diagnostics and source
  management are consumed by every stage and by future tools (formatter,
  language server), so a separate base layer is justified now.
* **`clap` for argument parsing.** Rejected for Phase 01; see
  [ADR-0010](0010-cli-exposes-only-implemented-commands.md).

## Consequences

* The dependency graph is `source ← diagnostics ← lexer ← cli`, with no
  cycles.
* Adding the parser means adding a crate (`compiler/parser`) and deciding at
  that time whether the AST lives with it or in `compiler/ast`.
