# 0015. Contextual keywords for declarative queries

- Status: Accepted (feature status: Experimental, after iterators and closures)
- Date: 2026-11-25

## Context

A proposal under review suggested C#-LINQ-style queries:

```pinguer
let names = query {
    from user in users
    where user.active
    order by user.name
    select user.name
}
```

Reserving `query`, `from`, `order`, `by` and `select` globally would forbid
very common names (`order`, `select()`, `from`) in every program.

## Decision

* Query syntax is an **Experimental** feature, specified in
  [FUNCTIONAL_PROGRAMMING.md §4](../../specifications/FUNCTIONAL_PROGRAMMING.md#4-iteration-and-queries).
* `query`, `from`, `order`, `by`, `select`, `ascending` and `descending` are
  **contextual keywords**: ordinary identifiers everywhere except in the
  positions listed below. They are not reserved by the lexer.
  * `query` is a keyword only when it is immediately followed by `{` where an
    expression is expected, and not directly in the condition of `if`,
    `while` or the scrutinee of `match` (there, `query {` is an identifier
    followed by a block; a query can be parenthesized).
  * `from`, `order`, `select` are keywords only at the start of a clause
    inside a query block; `by` only directly after `order`; `ascending` and
    `descending` only at the end of an `order by` key.
* `where` and `in` are already reserved keywords and keep that status.
* A query **desugars** to iterator combinators (`filter`, `orderBy`, `map`)
  and has no semantics of its own beyond that translation.

## Alternatives considered

* **Global reservation:** simplest parser; rejected for its cost to every
  program (every reserved word is a name no program can use).
* **No query syntax (combinators only):** the previous position; replaced by
  an experimental, contained syntax.

## Consequences

* The lexer needs no change: all clause words lex as identifiers (tests
  verify this).
* The parser must handle context-dependent keywords, which is a bounded,
  local decision at clause starts.
* Contextual keywords also offer a migration path for other features without
  breaking programs (for example `parallel` blocks).
