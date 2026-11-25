# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Language specification drafts for v0.1, initial grammar (`GRAMMAR.ebnf`),
  explicit v0.1 scope, `specifications/OPERATORS.md` (precedence and
  semantics of every operator) and `specifications/FEATURE_MATRIX.md`
  (features by classification and target version).
- Compiler architecture documentation and Architecture Decision Records
  0001–0023. ADR-0011 to ADR-0022 record the syntax decisions: composition
  instead of class inheritance, `public`/`private` visibility, prefix `try`
  instead of postfix `?`, leading-dot line continuation, contextual query
  keywords, `i128`/`u128`/`byte`, `**`, `??`/`?.`, `|>`, compiler
  attributes, fundamental library functions, and the feature classification
  labels.
- Rust workspace with the crates `pinguer_source` (source files, spans),
  `pinguer_diagnostics` (diagnostics, human and JSON renderers) and
  `pinguer_lexer` (complete v0.1 lexer, including the tokens `**`, `**=`,
  `??`, `?.` and `|>` and the reserved keywords `public`, `private`, `try`
  and `class`).
- `pinguer` CLI with `help`, `version` and `debug tokens`; planned commands
  report that they are not available yet.
- Unit, integration, property-style and end-to-end tests.
- GitHub Actions CI for Linux, Windows and macOS.
- Project logo.
