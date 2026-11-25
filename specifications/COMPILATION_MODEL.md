# Compilation Model

Status: sections 1–4 are **Normative (v0.1)** for observable behaviour;
implementation choices are recorded in ADRs and may change without changing
the language. Sections 5–6 are **Proposals**.
Implementation: source loading, spans, diagnostics and lexing exist; the
remaining stages are not started.

## 1. Overview

Pinguer is compiled **ahead of time** to native executables. There is no
interpreter and no virtual machine in v0.1. A JIT is a future proposal and
must not shape the v0.1 pipeline.

## 2. Compilation unit (Normative v0.1)

* A v0.1 compilation unit is one `.pgr` file containing `main`.
* The compiler processes the whole file before generating code; declaration
  order does not matter.
* The output is one native executable for the target platform.

## 3. Pipeline

The v0.1 pipeline, with the crate that owns each stage (see
[docs/architecture/COMPILER_ARCHITECTURE.md](../docs/architecture/COMPILER_ARCHITECTURE.md)):

```text
source file
  │  pinguer_source       load, validate UTF-8, index lines
  ▼
text + FileId
  │  pinguer_lexer        tokens, lexical diagnostics          [implemented]
  ▼
tokens
  │  parser               syntax tree, syntax diagnostics      [not started]
  ▼
AST
  │  semantic analysis    name resolution, type checking,      [not started]
  │                       control-flow checks (returns, loops)
  ▼
typed AST (AST + side tables)
  │  PIR lowering         typed control-flow graph              [not started]
  ▼
PIR
  │  LLVM codegen         textual LLVM IR                       [not started]
  ▼
.ll file
  │  clang                optimization, object code, linking    [external]
  ▼
native executable
```

Each stage runs only if the previous stages reported no errors, except that
the parser runs on the lexer's recovered tokens so that lexical and syntax
errors are reported together. Diagnostics from all stages are reported
through `pinguer_diagnostics`.

Intermediate layers were chosen deliberately small; see
[ADR-0004](../docs/adr/0004-intermediate-representations.md). There is no
separate HIR or LIR in v0.1.

## 4. Native code generation (Normative v0.1 for behaviour)

* **Backend:** LLVM. In v0.1 the compiler writes textual LLVM IR and invokes
  `clang` to optimize, assemble and link it
  ([ADR-0003](../docs/adr/0003-llvm-via-textual-ir-and-clang.md)).
  `clang` must therefore be installed to build executables.
* **Targets:** x86-64 and ARM64 on Windows (MSVC environment), Linux (glibc)
  and macOS. The default target is the host.
* **Optimization levels:** a debug mode without optimization and a release
  mode (`-O2`). Optimization must not change observable behaviour, including
  which traps occur (see [ERROR_HANDLING.md §2](ERROR_HANDLING.md#2-traps-normative-v01)).
* **Runtime:** generated programs link against the platform C runtime for
  process start-up, output and termination. The Pinguer runtime support code
  (trap reporting, output formatting) is kept minimal. **Unresolved:** whether
  it is generated directly as LLVM IR or written in another language and
  linked as a library ([ADR-0003](../docs/adr/0003-llvm-via-textual-ir-and-clang.md)).

## 5. Reproducible builds (Proposal, design requirement)

* Compiler output depends only on the input files, the compiler version, the
  target and the documented options — never on the time, the host name, the
  order of hash-map iteration or absolute paths. The compiler avoids
  nondeterministic iteration in all stages.
* Absolute paths embedded in debug information will be remappable.
* The CI will build the same program twice and compare the outputs.

## 6. Future backends and modes (Proposal)

The code generator consumes PIR only, and PIR contains no LLVM-specific
concepts. Future backends are separate crates that consume PIR:

| Backend | Purpose | Notes |
|---|---|---|
| LLVM C API | Faster code generation without a text round-trip | Replaces textual IR when the toolchain story is solved |
| WebAssembly | Browser and sandboxed execution | Can reuse LLVM's wasm target first |
| Bytecode VM | Fast start-up, embedding, REPL | Interprets PIR or a bytecode derived from it |
| JIT | REPL and dynamic loading | LLVM ORC on top of the LLVM backend |
| Embedded | Microcontrollers | Requires `core`-only standard library |
| GPU | Data-parallel kernels | Research; would require restricted kernel subset |

Compile-time execution ([METAPROGRAMMING.md](METAPROGRAMMING.md)) will
interpret PIR, which ensures that compile-time and run-time semantics match.

## 7. Incremental compilation (Proposal)

Query-based incremental compilation (as in rustc and rust-analyzer) is the
long-term direction, so that the compiler and the language server share one
implementation. v0.1 recompiles the whole file.
