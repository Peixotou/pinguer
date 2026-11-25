# Metaprogramming

Status: section 1 is **Normative (v0.1)**; section 3 is **Planned**
([ADR-0020](../docs/adr/0020-compiler-attributes.md)); the rest is **Proposal**.
Implementation: not started.

## 1. Constant expressions (Normative v0.1)

`const` declarations are evaluated at compile time:

```pinguer
const KIB = 1024
const MIB = KIB * 1024
const LIMIT: u32 = 10 * MIB as u32
```

A constant expression may contain literals, references to other constants,
and the arithmetic, bitwise, logical, comparison and `as` operators. Function
calls are not allowed in v0.1. Evaluation follows exactly the run-time
semantics of the same expression; a condition that would trap at run time
(overflow, division by zero, failing conversion) is a compile-time error.

The compiler may also fold constant subexpressions in ordinary code, but this
never changes program meaning: a trapping operation in reachable code is
reported as a compile-time error only when it is certain to execute, and
otherwise still traps at run time.

## 2. Compile-time execution (Proposal)

Inspired by Zig's `comptime`, Pinguer may allow ordinary functions to run at
compile time when all their inputs are known:

```pinguer
const TABLE = comptime buildCrcTable()
```

Requirements before adoption:

* A compile-time interpreter that implements exactly the run-time semantics
  (likely by interpreting PIR), including traps.
* Determinism: compile-time code cannot perform I/O, read the clock or access
  the network, so builds are reproducible.
* Resource limits (steps, memory) with clear diagnostics.

Compile-time execution would later enable const generics and generating
lookup tables without build scripts. `comptime` is a reserved keyword.

## 3. Attributes (Planned)

Attributes attach metadata to declarations, using `@`:

```pinguer
@inline
func fast() {
    // ...
}

@test
func additionWorks() {
    // ...
}

@deprecated("use newApi instead")
func oldApi() {
    // ...
}
```

Decision: [ADR-0020](../docs/adr/0020-compiler-attributes.md).

**Syntax.** `@name` or `@name(arguments)` on the lines before a declaration.
A `NEWLINE` after an attribute is ignored. Several attributes may be stacked.
Attributes apply to declarations only, not to statements or expressions.

**Closed set.** Only the attributes below exist; an unknown attribute, an
attribute on a declaration it does not apply to, or invalid arguments are
compile-time errors. User-defined attributes depend on the macro design.

| Attribute | Applies to | Semantics | Restrictions and conflicts | Target |
|---|---|---|---|---|
| `@test` | functions | A test, run by `pinguer test` and excluded from normal builds. | No parameters; returns nothing or a `Result` whose `Err` fails the test. | v0.2 |
| `@bench` | functions | A benchmark, run by a planned `pinguer bench`. | No parameters. Results are measurements, never claims, unless the methodology is documented. | v0.3 |
| `@inline`, `@inline(always)`, `@inline(never)` | functions | Inlining hint; `always` and `never` are requirements. | `@inline(always)` on a recursive function is an error. Never changes behaviour. | v0.2 |
| `@deprecated(message)` | any declaration | Every use produces a warning with `message`. | Message must be a string literal. | v0.2 |
| `@derive(I, …)` | structs, enums | The compiler generates implementations of the listed interfaces. | Only interfaces declared derivable by the standard library; every field must implement them. | v0.4 |
| `@target(os: "…", arch: "…")` | any item | The item exists only when compiling for a matching target. | Excluded items must still parse but are removed before name resolution; tools must analyse every configuration. | v0.4 |
| `@entry` | one function | That function is the program entry point instead of `main`. | At most one per program; same signature rules as `main`; with `@entry` present, `main` is an ordinary function. | v0.4 |
| `@export(symbol)` | functions | Exported with the C calling convention under the exact `symbol` name. | Signature must use C-compatible types ([INTEROPERABILITY.md](INTEROPERABILITY.md)); duplicate symbols are errors. Distinct from visibility (`public`). | v0.4 |
| `@packed` | structs | No padding between fields. | Fields may be unaligned: borrowing a field is forbidden (reads copy the value), a known pitfall in other languages. | v0.5+ |
| `@align(n)` | structs | Minimum alignment `n` bytes. | `n` must be a power of two not smaller than the natural alignment; conflicts with `@packed` are errors. | v0.5+ |

## 4. Macros (Proposal, low priority)

Macros are powerful but harm readability, tooling and compile times. Pinguer
will try to cover the common uses with other features first:

| Need | Intended feature |
|---|---|
| Formatted output | String interpolation checked by the compiler (syntax pending, C6) |
| Derived implementations (equality, hashing, printing) | `@derive(...)` attributes implemented by the compiler |
| Domain-specific code generation | Compile-time execution |

If macros are added, they must be hygienic and operate on syntax trees, not
on text.

## 5. Reflection (Proposal)

Compile-time reflection (inspecting a type's fields during compile-time
execution) may be provided. Run-time reflection is not planned because it
conflicts with dead-code elimination and predictable binary size.
