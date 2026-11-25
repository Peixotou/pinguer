# 0021. Fundamental library functions

- Status: Accepted (feature status: Planned)
- Date: 2026-11-25

## Decision

The standard library will provide these functions, specified in
[STANDARD_LIBRARY.md §3](../../specifications/STANDARD_LIBRARY.md#3-fundamental-functions-planned):

`assert`, `panic`, `exit`, `min`, `max`, `abs`, `len`, `clone`, `swap`,
`input`, `format`, `sizeOf<T>()`, `alignOf<T>()`.

Target versions follow the features each function depends on: `min`, `max`
and `abs` start as intrinsics over primitive types (v0.2) and become generic
with interfaces (v0.4); `clone` and `sizeOf`/`alignOf` need generics (v0.4).

`print` and `println` already exist in v0.1 (`std.io`). Whether these
functions are available without an import depends on the pending prelude
decision (C9).

### Naming convention (pending confirmation)

The approved names (`sizeOf`, `alignOf`) and the project's examples use
**lowerCamelCase** for functions and methods. The specification examples
follow it (lowerCamelCase for functions, methods and variables,
UpperCamelCase for types, interfaces and enum variants,
SCREAMING_SNAKE_CASE for constants), but the convention itself is listed as a
pending decision in
[FEATURE_MATRIX.md §5](../../specifications/FEATURE_MATRIX.md#5-pending-decisions)
until it is explicitly confirmed.

### Not adopted (pending decision)

`range()` (redundant with `0..n`), `round()` (better as a method
`x.round()`), and `typeof()` (runtime type inspection conflicts with the
no-runtime-reflection rule; a compile-time `typeName<T>()` is possible).

## Consequences

* `assert`, `panic`, `format` and `sizeOf`/`alignOf` are **compiler
  intrinsics**: they need source locations, compile-time templates or type
  layout information.
* `swap` depends on mutable-borrow syntax; `clone` on interfaces; `input` and
  `format` on heap strings. Target versions follow those dependencies.
