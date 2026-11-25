# 0023. No placeholder directories

- Status: Accepted
- Date: 2026-11-25
- Supersedes: the placeholder rule of [ADR-0002](0002-workspace-layout-and-engineering-policies.md)

## Context

ADR-0002 kept the original directory skeleton (`compiler/parser`, `ast`,
`semantic`, `types`, `ir`, `optimizer`, `codegen`, `runtime`, `stdlib`, and
empty `tests`, `tools`, `packages`, `benchmarks`, `extensions/vscode`) as
placeholders, each with a short README or a `.gitkeep`. In a public
repository these directories suggest components that do not exist, and their
READMEs repeated the crate map in
[COMPILER_ARCHITECTURE.md](../architecture/COMPILER_ARCHITECTURE.md).

## Decision

* The repository contains a directory only when it holds real content: code,
  tests, specifications, documentation or assets.
* Planned components, their intended locations and their roles are recorded
  in the crate map of COMPILER_ARCHITECTURE.md. A directory is created
  together with the first code that belongs in it.
* The crate rules of ADR-0002 are unchanged: a crate is created when its first
  real code is written and split only when a dependency boundary or
  build-time concern justifies it.

## Alternatives considered

* **Keep the placeholders.** Rejected: they advertise unimplemented
  components and duplicate the architecture document.

## Consequences

* The placeholder READMEs and `.gitkeep` files were removed; their content is
  in COMPILER_ARCHITECTURE.md.
* Directory names for planned components in earlier ADRs (for example
  `compiler/optimizer` in ADR-0004) name future locations, not existing
  directories.
* Adding the parser still means creating `compiler/parser` and deciding at
  that time whether the AST lives with it or in `compiler/ast`.
