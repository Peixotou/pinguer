# 0010. The CLI exposes only implemented commands

- Status: Accepted
- Date: 2026-11-24

## Context

The planned CLI has many commands (`new`, `init`, `run`, `build`, `check`,
`test`, `fmt`, `lint`, `doc`, `add`, `remove`, `version`). In Phase 01, only
source loading, lexing and diagnostics exist. A command that appears to work
but does nothing meaningful (for example a `check` that only lexes) would
mislead users.

## Decision

* Commands backed by real functionality are implemented: `help`, `version`
  and the developer command `debug tokens <file>`, whose output format is
  explicitly unstable.
* Every planned command is recognised and fails with a clear message and the
  dedicated exit status `3` ("not available in this version").
* Exit statuses: `0` success, `1` the input has errors, `2` invalid command
  line, `3` command not available yet.
* Argument parsing is hand-written while the CLI has three commands. It will
  move to a parsing library (likely `clap`) when the number of commands and
  options makes that worthwhile, in line with the dependency policy of
  [ADR-0002](0002-workspace-layout-and-engineering-policies.md).

## Alternatives considered

* **Hide unimplemented commands entirely.** Users would get "unknown command"
  for documented commands, which is less helpful.
* **Implement `check` as "lex only".** Rejected as misleading.

## Consequences

* The CLI's help text lists planned commands separately from available ones.
* Scripts can detect unavailable features through exit status `3`.
