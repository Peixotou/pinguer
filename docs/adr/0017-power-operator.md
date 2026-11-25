# 0017. Power operator `**` and `**=`

- Status: Accepted (feature status: Planned, target v0.2)
- Date: 2026-11-25

## Decision

Add `a ** b` (power) and the assignment `a **= b`, specified in
[OPERATORS.md §3](../../specifications/OPERATORS.md#3-power--and--planned):

* precedence level 2 (above prefix operators), right-associative:
  `-x ** 2 == -(x ** 2)`, `2 ** 3 ** 2 == 2 ** 9`;
* both operands have the same type (integer or floating-point);
* integer overflow and negative integer exponents trap (compile-time error
  for constants); `0 ** 0 == 1`;
* floating-point power follows IEEE 754 / C `pow`.

## Alternatives considered

* **`^` (mathematical notation):** already bitwise XOR.
* **Library function only (`pow(a, b)`):** an operator was requested;
  numerical code reads better with one.
* **Lower precedence than prefix minus (Excel):** `-2 ** 2 == 4` contradicts
  mathematics and Python; rejected, and a lint will flag the unparenthesized
  form.

## Consequences

* Lexer: `**` and `**=` tokens (implemented, with tests). `a * *b` still
  lexes as two multiplications.
* If a prefix dereference `*` is ever added, the parser must split `**` in
  prefix position.
* PIR: integer power lowers to a checked loop or exponentiation by squaring
  with overflow checks; float power lowers to the `llvm.pow` intrinsic.
