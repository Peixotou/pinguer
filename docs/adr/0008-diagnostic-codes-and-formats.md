# 0008. Diagnostic codes and output formats

- Status: Accepted
- Date: 2026-11-24

## Context

Diagnostics are part of the user interface of the compiler and of the
contract with tools (editors, CI annotations). Stable codes let users search
for explanations and let tools filter diagnostics.

## Decision

* Codes are `P` followed by four digits. The first digit identifies the stage:
  `P0` driver and files, `P1` lexical, `P2` syntax, `P3` name resolution, `P4`
  types, `P5` memory and ownership, `P6` code generation and linking, `P9`
  internal errors. `Px000` is reserved.
* All codes are registered in one place (`pinguer_diagnostics::codes`), and
  tests check uniqueness and ranges. A released code is never reused.
* Two output formats:
  * **human**: `rustc`-style text with source snippets, primary (`^`) and
    secondary (`-`) labels, notes and help; terminal-safe by construction;
  * **json**: one JSON object per line (JSON Lines), versioned by a
    `format_version` field.
* A help message may only propose a change when the compiler has the semantic
  information to know the change is valid.

Details: [DIAGNOSTICS.md](../architecture/DIAGNOSTICS.md).

## Alternatives considered

* **Codes per stage without ranges.** Harder to know where a code comes from.
* **No codes.** Messages change over time; codes give a stable handle.
* **A single JSON array.** Prevents streaming; JSON Lines is the convention
  used by tools such as `cargo --message-format=json`.
* **Colored output now.** Deferred: requires reliable terminal detection on
  all platforms; plain text is always correct.

## Consequences

* New diagnostics must be added to the registry together with a test that
  triggers them.
* The JSON schema can gain fields within `format_version` 1, but not remove or
  change them.
