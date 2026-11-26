# 0025. String interpolation syntax

- Status: Proposed. Records pending decision C6
  ([FEATURE_MATRIX.md §5](../../specifications/FEATURE_MATRIX.md#5-pending-decisions));
  nothing is implemented.
- Date: 2026-11-25

## Context

String interpolation is planned (`"Hello, ${name}"` is the current
recommendation) and so is a compile-time checked `format` intrinsic whose
placeholders follow the same syntax
([STANDARD_LIBRARY.md](../../specifications/STANDARD_LIBRARY.md)). The syntax
must be fixed before v0.1.0 is released, because some choices change the
meaning of string literals that are valid in v0.1.

What v0.1 accepts today (verified with `pinguer debug tokens`):

| Literal | v0.1 lexing |
|---|---|
| `"Hello, ${name}"` | One valid string; `$`, `{` and `}` are ordinary text. |
| `"Hello, {name}"` | One valid string. |
| `"Hello, \(name)"` | A string with an invalid escape (`P1005`). |
| `f"Hello, {name}"` | The identifier `f` followed by a string. No production of the grammar accepts an identifier directly followed by a string literal. |

A program that prints `"${x}"` literally in v0.1 would silently print the
value of `x` once interpolation exists, unless v0.1 rejects that sequence.

## Options

| Option | Example | Precedent | What v0.1 must do before v0.1.0 |
|---|---|---|---|
| A. `${…}` inside every literal | `"Hello, ${name}"` | Kotlin, Dart, JavaScript template literals | Reject `${` in literals (new diagnostic) and add an escape for a literal `$`, such as `\$` |
| B. `{…}` inside every literal | `"Hello, {name}"` | Python `str.format` placeholders | Reject `{` and `}` in literals and add escapes for them; every JSON or code snippet in a string needs escaping |
| C. Prefixed literals | `f"Hello, {name}"` | Python f-strings, C# `$"…"` | Nothing: the prefixed form is not valid v0.1 syntax, and unprefixed literals never interpolate |
| D. An escape sequence | `"Hello, \(name)"` | Swift | Nothing: `\(` is already an invalid escape in v0.1 |

## Consequences to weigh

* **Compatibility.** A and B require a lexer change and a new diagnostic in
  v0.1, plus a new escape; C and D keep every valid v0.1 literal valid and
  unchanged without any v0.1 work.
* **Readability and familiarity.** A and C read naturally for users of
  Kotlin, JavaScript, Python and C#. D is compact but less common. B makes
  braces, frequent in text, special everywhere.
* **Lexer design for all options.** An interpolated literal is no longer one
  token: the lexer must produce literal parts and the tokens of each
  embedded expression, track nesting (an embedded expression can contain
  string literals and braces), and decide whether line breaks are allowed
  inside the embedded expression although they are not allowed in literals.
* **Prefix forms (C)** reserve a lexical pattern, identifier immediately
  followed by `"`, which the proposed byte strings (`b"…"`) and raw strings
  would share.
* **Formatting directives** (width, precision, radix, as in `{x:08}`) and
  the `format` intrinsic should use the same placeholder syntax as
  interpolation, so this decision also fixes the `format` template language.

## Decision

Not taken. The maintainer chooses an option; if it is A or B, the
reservation it requires is implemented and tested before v0.1.0, as
[ROADMAP.md](../../ROADMAP.md) records. The chosen option then gets an
accepted ADR that supersedes this one.
