# 0014. Lines starting with `.` continue the previous line

- Status: Accepted. Amends [ADR-0005](0005-newline-terminated-statements.md).
- Date: 2026-11-25

## Context

Under ADR-0005, a line break ends a statement unless the previous line ends
with an operator. Fluent method chains are conventionally written with the
dot at the **start** of each line:

```pinguer
let result = numbers
    .filter(n => n >= 20)
    .sum()
```

(The closure syntax `n => …` is illustrative; it is a pending decision.)

Under ADR-0005 alone, `let result = numbers` would end at the first line
break and `.filter(...)` would be a syntax error.

## Decision

The parser ignores a `NEWLINE` token that is **immediately followed** by:

* `.` — **Normative (v0.1)**;
* `?.`, `|>` and `??` — **Planned**, effective when those operators exist.

Blank lines and comment lines between the two lines are allowed, because the
lexer collapses them into the single `NEWLINE` token.

### Why this is unambiguous

None of these tokens can begin a statement or an expression in Pinguer:

* there are no implicit member expressions (Swift's `.red` shorthand);
* floating-point literals require a leading digit (`.5` is not a number);
* `..` and `..=` are different tokens and are **not** continuation tokens.

Tokens that *can* start an expression (`-`, `!`, `~`, `*`, `&`, `(`, `[`) never
continue a line, so `let x = a` followed by a line `-b` is still two
statements (and the second is rejected because it is not a call).

## Alternatives considered

* **Trailing dots only (`numbers.` at the end of the line):** consistent with
  ADR-0005 but unidiomatic and fragile when editing.
* **Continuation for every binary operator at the start of a line (Swift):**
  ambiguous for operators that are also prefix operators (`-`, `*`, `&`).

## Consequences

* Any future proposal for implicit member expressions (`.case` shorthand)
  conflicts with this rule and must address it.
* A statement that is complete on its own and is followed by a line starting
  with `.` is joined with it; for example `if x { }` followed by `.f()` is a
  syntax error, with a diagnostic explaining the joining rule.
* Lexer: unchanged (it still reports the line break). Parser: one lookahead
  check when a `NEWLINE` is encountered.
