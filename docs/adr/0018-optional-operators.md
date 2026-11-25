# 0018. Optional operators `??` and `?.`

- Status: Accepted (feature status: Planned, with `Option<T>`)
- Date: 2026-11-25

## Decision

Specified in [OPERATORS.md §4–5](../../specifications/OPERATORS.md#4-optional-member-access--planned):

* `a ?? b` (coalescing): the value inside `a: Option<T>`, or `b` when `a` is
  `None`. `b` is evaluated lazily. If `b: Option<T>`, the result is an
  `Option<T>`. Level 12, right-associative.
* `a?.b` (optional chaining): `None` if `a` is `None`, otherwise `v.b` wrapped
  in `Some` (flattened if already optional). The short-circuit covers the rest
  of the postfix chain, as in Swift. Level 1.
* Both work only on `Option`, not on `Result`.

Together with `T?` (sugar for `Option<T>`) and `try`, these make absent values
explicit and convenient without null.

## Alternatives considered

* **Kotlin's `?:` (Elvis):** same meaning as `??`; `??` is used by C#, Swift,
  JavaScript and TypeScript and does not resemble a ternary operator.
* **Per-step short-circuit (Kotlin):** `a?.b.c` would fail on `.c` when `a` is
  `None`; whole-chain short-circuit (Swift) is simpler to use.
* **Allowing `Result ?? default`:** discards errors silently; left
  **Unresolved**.

## Consequences

* Lexer: `??` and `?.` tokens (implemented, with tests). `T??` is split by the
  parser in type position.
* Parser: `?.` joins the postfix loop; leading `?.` and `??` continue lines
  (ADR-0014).
* Types: rules for `Option` flattening and laziness. PIR: branches on the
  discriminant; no runtime support needed.
