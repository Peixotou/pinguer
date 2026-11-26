# Roadmap

Pinguer is developed in phases. Each phase has completion criteria; a phase is
done only when its criteria are met and CI is green. Dates are not promised.

## Toward v0.1.0

### Phase 01 — Foundation (complete)

* Toolchain verification.
* Language specification drafts, initial grammar, v0.1 scope.
* Compiler architecture and ADRs.
* Rust workspace: source management, diagnostics (human and JSON), lexer.
* CLI scaffolding: `help`, `version`, `debug tokens`.
* Tests (unit, integration, property-style, end-to-end) and CI.

### Phase 02 — Parser and AST

* AST with node IDs and spans; parser for the full v0.1 grammar.
* Newline handling rules from ADR-0005; reserved-keyword diagnostics.
* Error recovery at statement and item boundaries.
* `pinguer debug ast <file>`.
* **Done when:** every production has positive and negative tests, AST
  snapshot tests exist, and the parser survives the robustness corpus.

### Phase 03 — Semantic analysis

* Name resolution, scopes, imports of `std.io`.
* Type checking: primitive types, literal typing, operators, `as`,
  definite-return and loop-control checks, constant evaluation.
* `pinguer check <file>`.
* **Done when:** every normative rule in TYPE_SYSTEM.md and
  LANGUAGE_SPECIFICATION.md has tests; every diagnostic has a code and test.

### Phase 04 — PIR, code generation and runtime

* PIR lowering with explicit overflow checks and trap edges.
* Textual LLVM IR generation; `clang` driver; runtime support for output and
  traps; `std.io.print`/`println`.
* `pinguer build` and `pinguer run`.
* **Done when:** compile-and-run tests (stdout, stderr, exit status, every
  trap) pass on Windows, Linux and macOS.

### Phase 05 — v0.1 hardening and release

* Fuzzing of lexer and parser (≥ 24 CPU-hours per target).
* Reproducible-build check, documentation review, release builds.
* **Done when:** the release criteria in
  [V0_1_SCOPE.md](specifications/V0_1_SCOPE.md#3-release-criteria-for-v010) hold.

## After v0.1

Every feature, its classification and its tentative target version are
listed in [specifications/FEATURE_MATRIX.md](specifications/FEATURE_MATRIX.md).
Order is tentative and depends on design validation:

1. **v0.2:** structs, enums, `match`, tuples and arrays; `i128`, `u128`,
   `byte`; `**`; `@test`, `@inline`, `@deprecated`; `assert`, `panic`, `exit`.
2. **v0.3:** ownership model and heap types; `Option`, `Result`, `T?`,
   `try`, `??`, `?.` (`Option` and `Result` built in); closures; `@bench`.
3. **v0.4:** generics and interfaces; multi-file modules and packages;
   `public`/`private`; `|>`; `@derive`, `@export`, `@entry`, `@target`.
4. Tooling: formatter, language server, VS Code extension, test runner.
5. **v0.5+:** concurrency model (decision ADR first); C interoperability and
   `unsafe`; `@packed`, `@align`; compile-time execution; query expressions
   (experimental).
6. Additional backends: WebAssembly, JIT, bytecode VM.

## Open questions

| Question | Blocks | Next step |
|---|---|---|
| Runtime implementation strategy, trap exit status and trap message format | Phase 04 | ADR at the start of Phase 04 |
| Minimum supported Rust and LLVM versions | Release | Establish through the CI matrix |
| Cross-platform CI results (Linux, macOS ARM64) | Release | Observe the workflow on GitHub |
| Ownership model | Heap types after v0.1 | Prototype per [OWNERSHIP_MODEL.md §6](specifications/OWNERSHIP_MODEL.md#6-validation-plan) |
| Concurrency model | Async features | Prototype and ADR ([CONCURRENCY_MODEL.md](specifications/CONCURRENCY_MODEL.md)) |
| Explicit generic argument syntax | Generics | ADR together with the generics design |
| Coverage-guided fuzzing | v0.1.0 release | `cargo-fuzz` targets for lexer and parser (Phase 05) |
| String interpolation syntax (C6) | v0.1.0 release, if the chosen syntax gives a meaning to text that is valid in v0.1 string literals: that text must be rejected first | Maintainer decision; options and consequences in [ADR-0025](docs/adr/0025-string-interpolation-syntax.md) |
| Other pending syntax decisions (C4, C5, C7, C9, C11, C12) | Their target versions | Maintainer decisions; same table |
| `actions/checkout` pinned by tag rather than commit SHA | Supply-chain hardening | Pin by SHA together with automated update tooling |
