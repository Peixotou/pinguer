# 0013. Prefix `try` for error propagation

- Status: Accepted (feature status: Planned)
- Date: 2026-11-25

## Context

Error propagation was specified as Rust's postfix `?` (`read(path)?`). A
proposal under review suggested Swift/Zig's prefix `try` (`try read(path)`).

## Decision

`try expr` is a **prefix operator** at the precedence of the other prefix
operators (level 3 in [OPERATORS.md](../../specifications/OPERATORS.md)). Its
full semantics are in
[ERROR_HANDLING.md §1.1](../../specifications/ERROR_HANDLING.md#11-propagation-with-try).
In short:

* the operand has type `Result<T, E>` or `Option<T>`;
* on `Ok(v)`/`Some(v)` the expression evaluates to `v`;
* on `Err(e)`/`None` the enclosing function returns immediately with that
  error/`None`;
* the enclosing function must return a compatible `Result` or `Option`.

The postfix `?` operator is **removed**. The `?` token remains reserved for
optional types (`T?`) and is part of the `?.` and `??` tokens.

## Rationale

* Readability: `try` is a word that states what happens; every fallible call
  is visibly marked at the start of the expression.
* It frees `?` for the optional operators (`?.`, `??`, `T?`), avoiding three
  different meanings for one symbol.

## Alternatives considered

| Alternative | Assessment |
|---|---|
| Postfix `?` (Rust) | Concise and chains well (`a()?.b()?`), but cryptic for newcomers and overloaded with optional syntax. |
| Swift-style `try` covering the whole expression to its right | One `try` can mark several calls, so readers cannot see which call fails; rejected in favour of a tightly binding operator. |
| Exceptions | Rejected by the error-handling model. |

## Consequences

* Known cost: because prefix operators bind less tightly than postfix ones,
  `try read(path).trim()` applies `try` to `read(path).trim()`. Calling a
  method on the unwrapped value needs parentheses: `(try read(path)).trim()`.
  The type checker will detect the common mistake (method not found on
  `Result` but found on its `Ok` type) and suggest the parentheses.
* `try await fetch()` means `try (await fetch())`, the natural order.
* Lexer: `try` is a reserved keyword (implemented, with tests).
* AST: a `Try` unary expression node. Types: rules above. PIR: lowered to a
  discriminant test and an early-return edge that runs destructors.
