# Operators

Status: the operators marked **Normative (v0.1)** are binding for v0.1. The
others are **Planned**: approved and specified here, but not part of v0.1
(see [FEATURE_MATRIX.md](FEATURE_MATRIX.md)).
Implementation: all operator tokens are recognised by the lexer, and the v0.1
operators are parsed with the precedence and associativity of section 1; the
planned ones are reported as not supported (`P2010`). Semantics are not
started.

This document is the single source of truth for operator precedence,
associativity and semantics. Other documents refer to it.

## 1. Precedence and associativity

From highest (binds tightest) to lowest.

| Level | Operators | Associativity | Status |
|---|---|---|---|
| 1 | call `f(x)`, member `a.b` | left | Normative (v0.1) |
| 1 | optional member `a?.b`, index `a[i]` | left | Planned |
| 2 | power `**` | right | Planned |
| 3 | prefix `-` `!` `~` | prefix | Normative (v0.1) |
| 3 | prefix `try` | prefix | Planned |
| 3 | prefix `await` | prefix | Proposal (depends on the concurrency model) |
| 4 | `as` | left | Normative (v0.1) |
| 5 | `*` `/` `%` | left | Normative (v0.1) |
| 6 | `+` `-` | left | Normative (v0.1) |
| 7 | `<<` `>>` | left | Normative (v0.1) |
| 8 | `&` | left | Normative (v0.1) |
| 9 | `^` | left | Normative (v0.1) |
| 10 | <code>&#124;</code> | left | Normative (v0.1) |
| 11 | pipeline <code>&#124;&gt;</code> | left | Planned |
| 12 | coalescing `??` | right | Planned |
| 13 | `==` `!=` `<` `<=` `>` `>=` | none | Normative (v0.1) |
| 14 | `&&` | left | Normative (v0.1) |
| 15 | <code>&#124;&#124;</code> | left | Normative (v0.1) |

Assignment (`=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`,
and the planned `**=`) is a **statement**, not an expression, so it has no
precedence level.

The planned operators were inserted without changing the relative order of
any v0.1 operator, so adding them does not change the meaning of any v0.1
program.

Examples of the resulting grouping:

| Expression | Means |
|---|---|
| `-2 ** 2` | `-(2 ** 2)`, which is `-4` |
| `2 ** 3 ** 2` | `2 ** (3 ** 2)`, which is `512` |
| `x as f64 ** 2.0` | a syntax error: `as` must be followed by a type, and parsing stops at `**`; write `(x as f64) ** 2.0` |
| `a + b \|> f` | `f(a + b)` |
| `x \|> f ?? d` | `(x \|> f) ?? d` |
| `count ?? 0 > 5` | `(count ?? 0) > 5` |
| `a ?? b ?? c` | `a ?? (b ?? c)` |
| `flags & MASK == 0` | `(flags & MASK) == 0` |
| `try load() + 1` | `(try load()) + 1` |
| `try await fetch()` | `try (await fetch())` |

## 2. v0.1 operators (Normative v0.1)

The semantics of the arithmetic, bitwise, shift, comparison, logical and
conversion operators are defined in
[TYPE_SYSTEM.md §6–7](TYPE_SYSTEM.md#6-operators). In summary: operands of
binary operators have the same type, integer errors trap, and `as` is checked.

## 3. Power: `**` and `**=` (Planned)

Decision: [ADR-0017](../docs/adr/0017-power-operator.md).

**Semantics.**

* Integer operands: `a ** b` multiplies `a` by itself `b` times. Both operands
  must have the same integer type, and so does the result. `x ** 0` is `1` for
  every `x`, including `0 ** 0`.
* Floating-point operands: both operands must have the same floating-point
  type. The result follows C's `pow` and IEEE 754 `pow`, including its special
  cases (`x ** 0.0 == 1.0` even for NaN; `0.0 ** -1.0` is infinity).
* `a **= b` is `a = a ** b` with `a` evaluated once.

**Precedence and associativity.** Level 2, above the prefix operators and
right-associative, as in Python and mathematics: `-x ** 2` is `-(x ** 2)` and
`a ** b ** c` is `a ** (b ** c)`. The right operand may itself start with a
prefix operator: `2.0 ** -1.0`.

**Negative literals.** The rule that `-` directly applied to an integer
literal forms one negative literal ([TYPE_SYSTEM.md §5.1](TYPE_SYSTEM.md#51-literals))
applies only when the literal is the whole operand of `-`. Because `**` binds
more tightly, `-128 ** 1` is `-(128 ** 1)`; with type `i8`, `128` is out of
range and the expression is a compile-time error. Write `(-128) ** 1`.

**Restrictions.**

* No mixed types: `x ** 2` with `x: f64` is an error, because integer
  literals are never floating-point; the diagnostic suggests `x ** 2.0`.
* Not defined for `bool`, `char` or `string`.

**Exceptional behaviour.**

* Integer overflow traps, as for all integer arithmetic.
* A negative exponent on an integer type traps; if the exponent is a
  constant, it is a compile-time error.
* Floating-point `**` never traps.

**Conflicts and interactions.**

* The lexer reads `**` greedily. If a prefix dereference operator spelled `*`
  is ever introduced, the parser must split `**` in prefix position, as it
  splits `>>` in generic arguments.
* A planned lint warns about `-x ** 2` without parentheses, because readers
  coming from spreadsheets expect `(-x) ** 2`.

## 4. Optional member access: `?.` (Planned)

Decision: [ADR-0018](../docs/adr/0018-optional-operators.md). Requires
`Option<T>` ([TYPE_SYSTEM.md](TYPE_SYSTEM.md#10-null-safety)).

**Semantics.** For `a?.b`, where `a: Option<T>`: if `a` is `None`, the result
is `None`; if `a` is `Some(v)`, the result is `v.b` wrapped in `Some`. If
`v.b` is itself an `Option<U>`, the result is flattened to `Option<U>`.

The short-circuit covers the **rest of the postfix chain**, as in Swift:
in `user?.address.city.len()`, if `user` is `None`, none of `.address`,
`.city` or `.len()` is evaluated, and the whole expression is `None`. Call
arguments inside a skipped chain are not evaluated.

**Precedence.** Level 1, like `.`.

**Restrictions.**

* The receiver must have type `Option<T>`; using `?.` on any other type is an
  error, and using `.` on an `Option<T>` is an error with a suggestion of `?.`
  or `??`.
* `?.` does not apply to `Result`. Convert first (for example with `.ok()`).
* A chain containing `?.` has an `Option` type, so it cannot be used directly
  where a `T` is expected: combine it with `??`, `try` or `match`.

**Exceptional behaviour.** None: `?.` never traps.

**Conflicts.** `?.` is one token. `a ? .b` (with a space) is not optional
access. Because `?.` cannot start a statement, a line starting with `?.`
continues the previous line ([§8](#8-line-continuation)).

## 5. Coalescing: `??` (Planned)

Decision: [ADR-0018](../docs/adr/0018-optional-operators.md).

**Semantics.** `a ?? b`, where `a: Option<T>`:

* if `b: T`, the result has type `T`: the value inside `a`, or `b` if `a` is
  `None`;
* if `b: Option<T>`, the result has type `Option<T>`: `a` if it is `Some`,
  otherwise `b`.

`b` is evaluated only when `a` is `None` (short-circuit).

**Precedence and associativity.** Level 12: below the arithmetic, bitwise and
pipeline operators and above comparisons; right-associative, so
`a ?? b ?? c` tries `a`, then `b`, then `c`.

**Restrictions.**

* The left operand must be an `Option`. `??` on a non-optional value is an
  error.
* `??` does not accept `Result` on the left. **Unresolved:** whether
  `result ?? default` should be allowed; it would silently discard the error.

**Exceptional behaviour.** None.

**Conflicts.** The lexer reads `??` greedily, so the type `T??` (optional
optional) produces one `??` token that the parser splits in type position.

## 6. Pipeline: `|>` (Planned)

Decision: [ADR-0019](../docs/adr/0019-pipeline-operator.md).

**Semantics.** The left operand becomes the **first argument** of the call on
the right:

| Expression | Means |
|---|---|
| `x \|> f` | `f(x)` |
| `x \|> f(a, b)` | `f(x, a, b)` |
| `x \|> io.println` | `io.println(x)` |
| `x \|> f \|> g(y)` | `g(f(x), y)` |

`x` is evaluated before the arguments `a, b`, which preserves left-to-right
evaluation order.

**Precedence and associativity.** Level 11, left-associative: below
arithmetic and bitwise operators (`a + b |> f` is `f(a + b)`) and above `??`
and comparisons (`x |> f == y` is `f(x) == y`).

**Restrictions.**

* The right operand must be a function name, a path to a function
  (`io.println`), or a call of one. Arbitrary expressions are not allowed.
  **Planned extension:** a parenthesized expression of function type, once
  functions are values.
* `try` binds more tightly than `|>`: to propagate an error from a pipeline,
  write `try (x |> parse)`.

**Exceptional behaviour.** Whatever the called function does; `|>` adds none.

**Conflicts.** `|>` is one token; `| >` is a bitwise or followed by `>`.
A line starting with `|>` continues the previous line ([§8](#8-line-continuation)).

## 7. Error propagation: `try` (Planned)

Defined in [ERROR_HANDLING.md §1.1](ERROR_HANDLING.md#11-propagation-with-try)
([ADR-0013](../docs/adr/0013-prefix-try.md)). In this table it is a prefix
operator at level 3.

## 8. Line continuation

Normative rules for where a line break does **not** end a statement
([ADR-0005](../docs/adr/0005-newline-terminated-statements.md),
[ADR-0014](../docs/adr/0014-leading-dot-continuation.md)):

1. **Trailing operator (Normative v0.1):** a `NEWLINE` directly after a binary
   operator, an assignment operator, `as`, `->` or `,` is ignored.
2. **Leading member access (Normative v0.1):** a `NEWLINE` directly followed
   by `.` is ignored, so method chains may start lines with `.`.
3. **Leading planned operators (Planned):** a `NEWLINE` directly followed by
   `?.`, `|>` or `??` is ignored once these operators exist.

Rule 2 and 3 are unambiguous because none of these tokens can begin a
statement or an expression in Pinguer. A leading `-`, `*` or `&` does **not**
continue a line, because those tokens can begin an expression.
