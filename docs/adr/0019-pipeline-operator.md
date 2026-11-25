# 0019. Pipeline operator `|>`

- Status: Accepted (feature status: Planned)
- Date: 2026-11-25

## Decision

`x |> f(a)` means `f(x, a)`: the left operand becomes the **first argument**
of the call on the right (Elixir style). Specified in
[OPERATORS.md §6](../../specifications/OPERATORS.md#6-pipeline--planned):

* level 11, left-associative: `a + b |> f` is `f(a + b)`;
  `x |> f == y` is `f(x) == y`;
* the right operand must be a function name, a path to a function, or a call
  of one;
* left-to-right evaluation is preserved.

## Alternatives considered

| Alternative | Assessment |
|---|---|
| F#-style (`x \|> f` calls a function *value*) | Requires functions as values and partial application for `f(a)`; Pinguer has neither in the near term. |
| Hack-style with a placeholder (`x \|> f(a, $)`) | Most flexible; adds a placeholder token and a new scoping rule. Possible later extension. |
| Method chaining only | Works only for methods defined on the type; pipelines compose free functions from any module. |

## Consequences

* Lexer: `|>` token (implemented, with tests).
* Parser: a binary level and a check on the right operand's shape; leading
  `|>` continues a line (ADR-0014).
* Types/PIR: pure desugaring into a call before type checking; no new
  runtime behaviour.
* Overlap with method chaining is intended: methods for type-associated
  operations, pipelines for free functions.
