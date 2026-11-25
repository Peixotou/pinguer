# Compiler Architecture

This document describes how the Pinguer compiler is structured, which parts
exist today and how the planned parts will fit in. The observable behaviour
of compilation is specified in
[COMPILATION_MODEL.md](../../specifications/COMPILATION_MODEL.md); the
reasoning behind the structure is in the [ADRs](../adr/README.md).

## 1. Pipeline

```text
                ┌────────────────────┐
 .pgr file ───▶ │ pinguer_source     │ SourceMap, FileId, Span, line index   [implemented]
                └─────────┬──────────┘
                          ▼
                ┌────────────────────┐
                │ pinguer_lexer      │ Vec<Token>                            [implemented]
                └─────────┬──────────┘
                          ▼
                ┌────────────────────┐
                │ parser             │ AST with NodeIds, syntax recovery     [planned: compiler/parser]
                └─────────┬──────────┘
                          ▼
                ┌────────────────────┐
                │ semantic analysis  │ name resolution, type checking,       [planned: compiler/semantic,
                │                    │ control-flow checks → side tables      compiler/types]
                └─────────┬──────────┘
                          ▼
                ┌────────────────────┐
                │ PIR lowering       │ typed CFG per function                [planned: compiler/ir]
                └─────────┬──────────┘
                          ▼
                ┌────────────────────┐
                │ LLVM codegen       │ textual LLVM IR (.ll)                 [planned: compiler/codegen]
                └─────────┬──────────┘
                          ▼
                     clang (external): optimize, assemble, link → executable

 Every stage reports to pinguer_diagnostics (Diagnostic, Diagnostics, renderers) [implemented]
```

Stages of a conventional multi-IR compiler pipeline that are **not** separate
components in v0.1, and why ([ADR-0004](../adr/0004-intermediate-representations.md)):

| Conventional stage | v0.1 treatment |
|---|---|
| Token stream | `Vec<Token>`; no separate abstraction is needed. |
| Name resolution, semantic analysis, type checking | One semantic phase producing side tables; may become separate passes inside the same crate. |
| High-level IR | Not needed yet; the typed AST serves. Introduced with desugaring-heavy features. |
| Mid-level IR | PIR. |
| Low-level IR | LLVM IR. |
| Optimization | LLVM's pipeline (`clang -O2`). PIR-level passes are added only when a concrete need appears. |
| Object file, linker | Delegated to `clang` ([ADR-0003](../adr/0003-llvm-via-textual-ir-and-clang.md)). |

## 2. Crate map

Existing crates:

| Directory | Crate | Status | Depends on |
|---|---|---|---|
| `compiler/source` | `pinguer_source` | Implemented | — |
| `compiler/diagnostics` | `pinguer_diagnostics` | Implemented | source |
| `compiler/lexer` | `pinguer_lexer` | Implemented | source, diagnostics |
| `cli` | `pinguer_cli` (binary `pinguer`) | Partially implemented | all stages |

Planned components. A directory is created together with its first real code
([ADR-0023](../adr/0023-no-placeholder-directories.md)); the
names below are the intended locations, not existing directories.

| Directory | Phase | Role | Depends on |
|---|---|---|---|
| `compiler/parser` | 02 | Tokens to AST, with recovery at statement and item boundaries ([GRAMMAR.ebnf](../../specifications/GRAMMAR.ebnf)). Whether the AST data model gets its own `compiler/ast` crate is decided when the parser is written. | lexer |
| `compiler/semantic` | 03 | Name resolution, scopes, definite-return and `break`/`continue` checks, producing side tables keyed by AST node IDs | parser |
| `compiler/types` | 03 | Type representation, literal typing and type checking ([TYPE_SYSTEM.md](../../specifications/TYPE_SYSTEM.md)). May be merged with `compiler/semantic` if the boundary proves artificial. | parser |
| `compiler/ir` | 04 | PIR: typed, target-independent control-flow graph ([ADR-0004](../adr/0004-intermediate-representations.md)) | semantic |
| `compiler/codegen` | 04 | PIR to textual LLVM IR, driving `clang` ([ADR-0003](../adr/0003-llvm-via-textual-ir-and-clang.md)) | ir |
| `runtime` | 04 | Entry point, output buffering, number formatting and trap reporting; the implementation strategy is an open question in ADR-0003 | — |
| `stdlib` | 04 | Standard library; v0.1 provides only `std.io` ([STANDARD_LIBRARY.md](../../specifications/STANDARD_LIBRARY.md)) | — |

A PIR optimizer is deferred: v0.1 relies on LLVM's optimization pipeline.

A **driver** crate that owns a compilation session (options, source map,
diagnostics, stage orchestration) will be extracted from the CLI when the
second consumer appears (the test harness for compiled programs or the
language server). Today the CLI calls the stages directly.

## 3. Cross-cutting design rules

### 3.1 Spans everywhere

Every token, AST node and diagnostic carries a `Span` (file ID plus a byte
range). Spans are 12 bytes and `Copy`. Line and column numbers are computed
only when a diagnostic is rendered.

### 3.2 Error recovery

Each stage reports as many independent errors as possible:

* the lexer produces tokens for invalid input
  ([LEXICAL_STRUCTURE.md §7](../../specifications/LEXICAL_STRUCTURE.md#7-error-recovery));
* the parser will use synchronization at statement and item boundaries
  (`NEWLINE`, `;`, `}`, and item keywords) and produce error nodes;
* the type checker will assign an *error type* to ill-typed expressions and
  suppress diagnostics that would only be consequences of earlier errors.

### 3.3 Determinism

No stage may depend on hash-map iteration order, time, or absolute paths for
its output. Diagnostics are reported in a deterministic order (the lexer sorts
by source position). This underpins reproducible builds and stable test
snapshots.

### 3.4 No panics on user input

Panics are reserved for violated internal invariants (compiler bugs). Any
condition that malformed input can trigger must be a diagnostic. Robustness
tests feed random input to every stage that accepts text
([TESTING_STRATEGY.md](TESTING_STRATEGY.md)).

### 3.5 Backend independence

The front end and PIR know nothing about LLVM. Only `compiler/codegen` emits
LLVM IR. Future backends (WebAssembly, bytecode VM, JIT, embedded, GPU) are
additional consumers of PIR
([COMPILATION_MODEL.md §6](../../specifications/COMPILATION_MODEL.md#6-future-backends-and-modes-proposal)).

### 3.6 Shared with tooling

The formatter and language server will reuse the lexer, parser and semantic
crates. Two consequences for the design of those crates:

* the parser must be able to produce a tree for incomplete code;
* the formatter needs comments, which the lexer currently discards. A
  trivia-preserving mode (or a separate lossless syntax tree, as in
  rust-analyzer) will be decided when the formatter is designed.
