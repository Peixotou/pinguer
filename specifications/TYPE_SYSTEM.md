# Type System

Status: sections 1–8 are **Normative (v0.1)**; section 9 onwards contains
**Planned**, **Proposal** and **Unresolved** items, each marked.
Implementation: not started.

Pinguer is statically typed: every expression has a type known at compile
time, and programs with type errors are rejected before code generation.

## 1. Principles

1. **No implicit conversions.** A value of one type is never silently turned
   into a value of another type. Widening conversions are explicit too.
2. **Inference is local.** Types of local bindings are inferred from their
   initializers; function signatures are always written out. This keeps
   signatures readable as documentation and keeps error messages local.
3. **No null.** No type includes a null value. Absence is expressed with
   `Option<T>` (Planned, see [Null safety](#null-safety)).
4. **Defined behaviour for every operation.** Every operator either produces
   a result, is rejected at compile time, or traps at run time.

## 2. Primitive types

| Type | Values | Size |
|---|---|---|
| `bool` | `true`, `false` | 1 byte |
| `i8`, `i16`, `i32`, `i64` | two's-complement signed integers | 1, 2, 4, 8 bytes |
| `u8`, `u16`, `u32`, `u64` | unsigned integers | 1, 2, 4, 8 bytes |
| `int` | alias for `i64` | 8 bytes |
| `f32`, `f64` | IEEE 754 binary32 and binary64 | 4, 8 bytes |
| `char` | a Unicode scalar value (U+0000–U+D7FF, U+E000–U+10FFFF) | 4 bytes |
| `string` | an immutable sequence of UTF-8 encoded text | see [ABI.md](ABI.md) |
| `()` | the unit type, with the single value `()` | 0 bytes |

* `int` is a type alias, not a distinct type: `int` and `i64` are
  interchangeable. `int` is 64 bits on every platform, independent of pointer
  width, so that program behaviour does not depend on the target.
* Pointer-sized integers (`isize`, `usize`) are a **Proposal**; they become
  necessary with arrays and indexing.
* There is currently no `float` alias; floating-point code names its
  precision. Adding `float` as an alias of `f64` is pending (C5, §12).

### Strings

In v0.1 a `string` value is an immutable reference to UTF-8 text with static
storage duration, created only by string literals. Strings can be bound,
passed, returned, compared with `==` and `!=`, and printed. Concatenation,
slicing, indexing and formatting require heap allocation and are deferred to
the ownership model ([MEMORY_MODEL.md](MEMORY_MODEL.md)).

String equality compares content (the UTF-8 bytes), not identity. No Unicode
normalization is applied.

### The never type

Expressions that never produce a value (such as a call to a function that
always traps) have the *never* type. In v0.1 the never type exists only
inside the compiler, for control-flow analysis; it has no surface syntax.
**Proposal:** spell it `never` (`func fail(message: string) -> never`).

## 3. Type equivalence

Two types are equal if they are the same primitive type after resolving
aliases. No subtyping exists in v0.1.

## 4. Mutability

Mutability is a property of bindings, not of types. A `var` binding may be
reassigned; a `let` binding, parameter or constant may not. In v0.1 all values
are copied on assignment and when passed to functions, so mutating a variable
never affects any other binding.

## 5. Type inference

* The type of a `let`, `var` or `const` without an annotation is the type of
  its initializer.
* With an annotation, the initializer is checked against the annotated type.

### 5.1 Literals

* An integer literal has the type expected by its context if that type is an
  integer type. Without such an expectation it has type `int`. It is an error
  if the value does not fit the type. An integer literal is never given a
  floating-point type: `let x: f64 = 1` is an error; write `1.0`.
* A unary `-` applied directly to an integer literal forms a single negative
  literal for this check, so `let x: i8 = -128` is valid. This applies only
  when the literal is the whole operand of `-` (see `**` in
  [OPERATORS.md §3](OPERATORS.md#3-power--and--planned)).
* A floating-point literal has the type expected by its context if it is
  `f32` or `f64`, and `f64` otherwise. It is rounded to the nearest
  representable value (ties to even). A literal whose value rounds to infinity
  is an error.
* String literals have type `string`; character literals have type `char`;
  `true` and `false` have type `bool`.

### 5.2 Expected types

Expected types flow from annotations, parameter types, return types, the
other operand of a binary operator, and the target of an assignment. For
example, in `let small: u8 = 200`, the literal is checked as `u8`; in
`x + 1` where `x: u16`, the literal `1` has type `u16`.

## 6. Operators

### 6.1 Arithmetic

`+`, `-`, `*`, `/` and `%` require two operands of the same numeric type and
produce that type. Unary `-` requires a signed integer or floating-point
operand.

Integer arithmetic is **checked**. These conditions trap at run time and are
compile-time errors when both operands are constants:

| Condition | Example |
|---|---|
| Result outside the range of the type | `i8` `127 + 1`; `u8` `0 - 1` |
| Division or remainder by zero | `x / 0` |
| Signed minimum divided by −1 (`/` or `%`) | `i64` `MIN / -1` |
| Negation of the signed minimum | `-x` where `x` is `i32` `MIN` |

* Integer division truncates toward zero. The result of `%` has the sign of
  the dividend: `-7 / 2 == -3` and `-7 % 2 == -1`.
* Wrapping, saturating and widening arithmetic are a **Proposal**, as named
  library functions (for example `a.wrappingAdd(b)`).

Floating-point arithmetic follows IEEE 754 with round-to-nearest-ties-to-even.
It never traps: division by zero produces an infinity or NaN. `%` on floats
computes the remainder of truncated division, with the sign of the dividend
(as C `fmod`).

### 6.2 Bitwise and shift operators

* `&`, `|`, `^` require two operands of the same integer type. `~` inverts all
  bits of an integer operand. Bitwise operators are not defined on `bool`; use
  `&&`, `||` and `!`.
* `a << n` and `a >> n` require `a` and `n` to have the same integer type.
  The shift amount must be at least 0 and smaller than the bit width of `T`;
  otherwise the operation traps.
  `>>` is an arithmetic shift for signed types and a logical shift for unsigned
  types. `<<` does not trap when bits are shifted out.

### 6.3 Comparison

* `==` and `!=` are defined for two operands of the same type among `bool`,
  integer, floating-point, `char` and `string`.
* `<`, `<=`, `>`, `>=` are defined for two operands of the same integer,
  floating-point or `char` type. `char` compares scalar values.
* Floating-point comparison follows IEEE 754: NaN is unequal to every value,
  including itself, and every ordered comparison with NaN is `false`.
* Comparison produces `bool`.

### 6.4 Logical operators

`&&`, `||` and `!` require `bool` operands. `&&` and `||` evaluate their right
operand only when needed.

### 6.5 Assignment

`x = e` requires `e` to have the type of `x`. `x op= e` is equivalent to
`x = x op e` with `x` evaluated once, and requires `op` to be defined for the
types involved.

## 7. Conversions

All conversions are explicit, written `expr as T`. The `as` operator is
**checked**: unlike in Rust, it never silently truncates.

| From → To | Behaviour |
|---|---|
| integer → integer | Value preserved. Traps if the value is not representable in `T`. |
| integer → float | Rounded to the nearest representable value (ties to even). |
| float → integer | Truncated toward zero. Traps if the value is NaN or the truncated value is not representable in `T`. |
| `f32` → `f64` | Exact. |
| `f64` → `f32` | Rounded to nearest (ties to even); values beyond the `f32` range become infinities. |
| `char` → integer | The scalar value. Traps if it does not fit in `T` (for example `'é' as u8` is valid; `'🐧' as u8` traps). |
| `T` → `T` | Identity. |

All other conversions are compile-time errors, including:

* integer → `char`, because not every integer is a scalar value
  (**Proposal:** `char.fromU32(n) -> Option<char>`);
* `bool` → integer and integer → `bool`;
* any conversion to or from `string`.

When the operand is a constant expression, a conversion that would trap is a
compile-time error.

## 8. Function types and calls

In v0.1 functions are not values: a function name may only be called. A call
must supply exactly as many arguments as there are parameters, and each
argument must have the parameter's type.

## 9. Planned type constructors and types

These follow the incremental plan in [ROADMAP.md](../ROADMAP.md).

| Feature | Sketch | Notes |
|---|---|---|
| Tuples | `(int, string)` | Structural, value semantics. |
| Arrays | `[int; 4]` | Fixed length, value semantics. |
| Slices | `[int]` (view) | Requires references; see [OWNERSHIP_MODEL.md](OWNERSHIP_MODEL.md). |
| Structures | `struct User { id: int }` | See [OBJECT_MODEL.md](OBJECT_MODEL.md). |
| Enumerations / ADTs | `enum Shape { Circle(f64), Square(f64) }` | Payloads; exhaustive `match`. |
| Generic types | `List<T>`, `Map<K, V>` | See [GENERICS.md](GENERICS.md). |
| Function types | `func(int) -> int` | Closures and capture rules depend on ownership. |
| Interfaces | `interface Display { … }` | Used as generic constraints. |
| Type aliases | `type UserId = u64` | Transparent aliases, like `int`. **Unresolved:** whether to add distinct "newtypes" as well. |
| `i128`, `u128` | 128-bit integers | **Planned** (v0.2, [ADR-0016](../docs/adr/0016-additional-integer-types.md)). Same operator and trap rules as other integers. |
| `byte` | alias for `u8` | **Planned** (v0.2, ADR-0016). Interchangeable with `u8`, as `int` is with `i64`. |

## Null safety

No Pinguer type contains a null value. Optional values use the library enum
`Option<T>` with variants `Some(T)` and `None`. Planned conveniences:

* **Planned:** `T?` as sugar for `Option<T>` (Kotlin, Swift). `T??` is
  `Option<Option<T>>`; the lexer reads `??` as one token and the parser splits
  it in type position.
* **Planned:** `??` (coalescing) and `?.` (optional chaining), specified in
  [OPERATORS.md §4–5](OPERATORS.md#4-optional-member-access--planned).
* **Planned:** `try` for early return of `None` or `Err`
  ([ERROR_HANDLING.md §1.1](ERROR_HANDLING.md#11-propagation-with-try)).
  Because propagation uses `try`, the `?` symbol has no expression meaning and
  is free for these optional forms ([ADR-0013](../docs/adr/0013-prefix-try.md)).
* Exhaustive `match` and `if let`-style narrowing (Proposal).
* The absent value is written `None` (the enum variant); a lowercase `none`
  literal is not planned, to avoid two spellings of the same value.

## 11. Exhaustive pattern matching (Proposal)

`match` must cover every possible value of its scrutinee; a missing case is a
compile-time error naming an example of an uncovered value. The algorithm will
follow the usefulness-based approach used by Rust and OCaml. See
[FUNCTIONAL_PROGRAMMING.md](FUNCTIONAL_PROGRAMMING.md).

## 12. Unresolved questions

1. Should shadowing produce a warning?
2. Should integer literals without context default to `int` everywhere, or
   should ambiguity be an error in some positions?
3. Should `string` ordering (`<`) be defined, and if so by bytes or by scalar
   values? (For UTF-8 these orders coincide.)
4. Naming and spelling of the never type.
5. Whether to add implicit widening for literals only (already covered) or
   also for values in specific safe cases (currently: no).
6. **C4 — `void`.** Should the unit type `()` be spelled `void`?
   *Recommendation:* yes. `void` becomes the prelude name of the unit type
   (an identifier, like `int`, not a keyword), and `()` remains only as the
   value in the rare generic case (`Ok(())`). Unlike C's `void`, it is a real
   zero-sized type usable in generics (`Result<void, Error>`), as Swift's
   `Void`. *Consequence:* v0.1 programs rarely write the unit type
   (`-> ()` is valid but redundant); adding the name `void` does not change
   the meaning of any existing program.
7. **C5 — `float`.** Should `float` be an alias of `f64`?
   *Recommendation:* yes, for consistency with `int` = `i64`. The cost is that
   code using `float` does not state its precision; `f32` remains available
   where precision matters. *Consequence:* a prelude name; no lexer change.
