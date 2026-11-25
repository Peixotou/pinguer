# 0005. Newline-terminated statements

- Status: Accepted; amended by [ADR-0014](0014-leading-dot-continuation.md) (lines starting with `.`)
- Date: 2026-11-24

## Context

The proposed syntax has no semicolons (`let name = "Pinguer"` on its own
line). A language without statement terminators needs precise rules for where
a line break ends a statement, or programs become ambiguous: JavaScript's
automatic semicolon insertion is a well-known source of bugs.

## Decision

* The **lexer** emits a `NEWLINE` token for a run of line breaks only when
  the innermost open delimiter is `{` (or none). Line breaks inside `()` and
  `[]` never matter.
* The **parser** ignores a `NEWLINE` directly after a binary operator, an
  assignment operator, `as`, `->` or `,`, so expressions can be continued by
  ending a line with an operator.
* Everywhere else, `NEWLINE` (or `;`) separates statements and items.
* `{` must be on the same line as its construct, and `else` on the same line
  as the preceding `}`.
* Only call expressions are allowed as expression statements, so a line
  starting with `-x` after an expression cannot silently become a separate,
  meaningless statement.

The precise rules are in
[LEXICAL_STRUCTURE.md §6](../../specifications/LEXICAL_STRUCTURE.md#6-line-breaks)
and [GRAMMAR.ebnf](../../specifications/GRAMMAR.ebnf).

## Alternatives considered

| Alternative | Assessment |
|---|---|
| Mandatory semicolons (C, Rust) | Unambiguous, but contradicts the readability goal. |
| Go's lexer-only rule (insert `;` after a line ending in an identifier, literal, `)`, `]`, `}` …) | Simple and proven, but forces trailing commas in multi-line calls and interacts badly with `>` closing generic arguments. |
| Indentation-sensitive blocks (Python) | Rejected: braces are already chosen and indentation sensitivity complicates tooling and copy-paste. |
| Kotlin/Swift-style parser-driven rules (chosen) | Flexible and familiar; requires care in the grammar, which this ADR provides. |

## Consequences

* Leading-dot method chains (`.map(…)` at the start of a line) were not
  continuations under these rules; ADR-0014 adds that rule.
* The formatter must preserve the brace and `else` placement rules.
* Allman-style braces are not supported.
