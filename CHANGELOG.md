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
- ADR-0024: control characters other than tab and line breaks, and
  U+2028 and U+2029, are rejected anywhere in source text (`P1012`).
  ADR-0025 (proposed) records the options for string interpolation (C6);
  ADR-0026 (proposed) the requirements for locating and invoking `clang`.
- A limit of 100 reported diagnostics per invocation; further diagnostics
  are counted and summarized by a `P0004` note in both formats.
- Scalability tests with inputs of up to 2 MB and a front-end benchmark
  (`cargo bench -p pinguer_cli --bench scalability`).

### Fixed

- Line and column lookup, and therefore `debug tokens`, took time
  proportional to the square of the line length; it no longer depends on
  the line length.
- Diagnostics repeated the whole source line: excerpts now show at most 120
  cells around the labels, and the CLI streams its output instead of
  building it in memory.
- Arguments that are not valid Unicode made the CLI panic; they are now
  accepted as file names and are usage errors elsewhere.
- Command-line errors ignored `--diagnostic-format json`.
- File errors embedded the operating system's localized message in the
  English diagnostic, and a directory was reported as "permission denied"
  on Windows.
- Source files were read completely before their size was checked.
- The `P0002` offset in a file with a byte order mark counted the mark.
- A lone carriage return at the end of a file was cut from excerpts.
- `\u{12G4}` was reported as an unterminated escape; uppercase numeric
  prefixes such as `0XFF` as an invalid suffix; a bidirectional character or
  lone carriage return outside literals produced no token.
- Several invisible characters (soft hyphen, variation selectors, tag
  characters and others) were printed verbatim in diagnostics, and escaping
  a character outside the Basic Multilingual Plane produced invalid JSON.
