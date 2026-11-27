# Feature Matrix

Status: maintained alongside the specification. Classifications follow
[ADR-0022](../docs/adr/0022-feature-classification.md). Target versions after
v0.1 are **tentative** and ordered by dependency.

Implementation columns: **Yes** implemented and tested · **·** not started ·
**—** not applicable to this stage.

## 1. Version plan (tentative)

| Version | Theme | Depends on |
|---|---|---|
| v0.1 | Primitive types, functions, control flow, checked arithmetic, native compilation | — |
| v0.2 | Data types: structs, enums, `match`, tuples, arrays; `i128`/`u128`/`byte`; `**`; first attributes; `assert`/`panic`/`exit` | v0.1 |
| v0.3 | Ownership and heap data; owned strings and collections; `Option`, `Result`, `T?`, `try`, `??`, `?.`; closures | v0.2, ownership ADR |
| v0.4 | Generics and interfaces; modules and packages; `public`/`private`; `\|>`; `@derive`, `@export`, `@entry`, `@target` | v0.3 |
| v0.5+ | Concurrency, `unsafe` and C interoperability, layout attributes, compile-time execution, queries (experimental) | v0.4, concurrency ADR |

## 2. Core language

| Feature | Class | Target | Spec | ADR | Lexer | Parser | Types | PIR |
|---|---|---|---|---|---|---|---|---|
| UTF-8 source, comments, identifiers, literals | Normative | v0.1 | [LEXICAL_STRUCTURE](LEXICAL_STRUCTURE.md) | 0009 | Yes | — | — | — |
| Newline-terminated statements | Normative | v0.1 | [LANGUAGE_SPECIFICATION §4](LANGUAGE_SPECIFICATION.md#4-statements-and-termination-normative-v01) | 0005 | Yes | Yes | — | — |
| Leading `.` continues a line | Normative | v0.1 | [OPERATORS §8](OPERATORS.md#8-line-continuation) | 0014 | Yes | Yes | — | — |
| `let`, `var`, `const`, functions, `if`, `while`, `break`, `continue`, `return` | Normative | v0.1 | [LANGUAGE_SPECIFICATION](LANGUAGE_SPECIFICATION.md) | — | Yes | Yes | · | · |
| Primitive types `bool`, `i8`–`i64`, `u8`–`u64`, `int`, `f32`, `f64`, `char`, `string` | Normative | v0.1 | [TYPE_SYSTEM §2](TYPE_SYSTEM.md#2-primitive-types) | — | — | — | · | · |
| v0.1 operators and `as` | Normative | v0.1 | [OPERATORS](OPERATORS.md) | 0007 | Yes | Yes | · | · |
| Traps | Normative | v0.1 | [ERROR_HANDLING §2](ERROR_HANDLING.md#2-traps-normative-v01) | 0007 | — | — | — | · |
| `import std.io`; `io.print`, `io.println` | Normative | v0.1 | [STANDARD_LIBRARY §2](STANDARD_LIBRARY.md#2-the-v01-standard-library-normative-v01) | — | Yes | Yes | · | · |
| `i128`, `u128`, `byte` | Planned | v0.2 | [TYPE_SYSTEM §9](TYPE_SYSTEM.md#9-planned-type-constructors-and-types) | 0016 | — | — | · | · |
| `**`, `**=` | Planned | v0.2 | [OPERATORS §3](OPERATORS.md#3-power--and--planned) | 0017 | Yes | · | · | · |
| Structs, enums, methods | Planned | v0.2 | [OBJECT_MODEL](OBJECT_MODEL.md) | 0011 | Yes | · | · | · |
| `match`, tuples, arrays | Proposal | v0.2 | [FUNCTIONAL_PROGRAMMING](FUNCTIONAL_PROGRAMMING.md), [TYPE_SYSTEM §9](TYPE_SYSTEM.md#9-planned-type-constructors-and-types) | pending | Yes | · | · | · |
| Ownership, borrowing, heap types | Proposal | v0.3 | [OWNERSHIP_MODEL](OWNERSHIP_MODEL.md) | pending | — | · | · | · |
| `Option<T>`, `Result<T, E>`, `T?` (built-in generic types until v0.4) | Planned | v0.3 | [ERROR_HANDLING §1](ERROR_HANDLING.md#1-error-values-planned) | 0013, 0018 | Yes | · | · | · |
| `try` | Planned | v0.3 | [ERROR_HANDLING §1.1](ERROR_HANDLING.md#11-propagation-with-try) | 0013 | Yes | · | · | · |
| `??`, `?.` | Planned | v0.3 | [OPERATORS §4–5](OPERATORS.md#4-optional-member-access--planned) | 0018 | Yes | · | · | · |
| Generics with interface constraints | Proposal | v0.4 | [GENERICS](GENERICS.md) | pending | Yes | · | · | · |
| Interfaces, composition, default methods | Planned | v0.4 | [OBJECT_MODEL](OBJECT_MODEL.md) | 0011 | Yes | · | · | · |
| `public`, `private` | Planned | v0.4 | [MODULE_SYSTEM §2.3](MODULE_SYSTEM.md#23-visibility-planned) | 0012 | Yes | · | · | — |
| Multi-file modules, packages, lockfile | Proposal | v0.4 | [MODULE_SYSTEM](MODULE_SYSTEM.md) | pending | — | · | · | — |
| <code>&#124;&gt;</code> | Planned | v0.4 | [OPERATORS §6](OPERATORS.md#6-pipeline--planned) | 0019 | Yes | · | · | — |
| Queries with contextual keywords | Experimental | v0.5+ | [FUNCTIONAL_PROGRAMMING §4](FUNCTIONAL_PROGRAMMING.md#4-iteration-and-queries) | 0015 | Yes | · | · | — |
| `async`, `await`, tasks, actors | Proposal | v0.5+ | [CONCURRENCY_MODEL](CONCURRENCY_MODEL.md) | pending | Yes | · | · | · |
| `unsafe`, `extern "C"` | Proposal | v0.5+ | [INTEROPERABILITY](INTEROPERABILITY.md) | pending | Yes | · | · | · |
| Compile-time execution (`comptime`) | Proposal | v0.5+ | [METAPROGRAMMING §2](METAPROGRAMMING.md#2-compile-time-execution-proposal) | pending | Yes | · | · | · |

"Yes" in the Lexer column of a future feature means that its keywords and
tokens are already recognised (reserved), not that the feature works. The
parser reports such tokens as syntax that a later version adds (`P2010`), as
it does generic parameter and type argument lists in declarations, and the
reserved keywords as reserved (`P2017`).

## 3. Attributes and library

| Feature | Class | Target | Spec | ADR |
|---|---|---|---|---|
| `@test`, `@deprecated`, `@inline` | Planned | v0.2 | [METAPROGRAMMING §3](METAPROGRAMMING.md#3-attributes-planned) | 0020 |
| `@bench` | Planned | v0.3 | same | 0020 |
| `@derive`, `@export`, `@entry`, `@target` | Planned | v0.4 | same | 0020 |
| `@packed`, `@align` | Planned | v0.5+ | same | 0020 |
| `assert`, `panic`, `exit`; `min`, `max`, `abs` on primitive types (intrinsics) | Planned | v0.2 | [STANDARD_LIBRARY §3](STANDARD_LIBRARY.md#3-fundamental-functions-planned) | 0021 |
| `len`, `swap`, `input`, `format` | Planned | v0.3 | same | 0021 |
| `clone`; generic `min`/`max`; `sizeOf<T>()`, `alignOf<T>()` | Planned | v0.4 (with generics, interfaces and explicit type arguments) | same | 0021 |

## 4. Rejected

| Feature | Reason | ADR |
|---|---|---|
| Class inheritance (`extends`, `virtual`, `override`, `super`) | Fragile base classes, forced reference semantics | 0011 |
| `protected` | Meaningful only with inheritance | 0011, 0012 |
| `pub` keyword | Replaced by `public` | 0012 |
| Postfix `?` for error propagation | Replaced by `try` | 0013 |
| Global reservation of query words | Cost to every program; contextual instead | 0015 |
| Multiple dispatch in the core language | Conflicts with static dispatch and separate compilation | — ([OBJECT_MODEL](OBJECT_MODEL.md#multiple-dispatch)) |
| Runtime reflection | Conflicts with dead-code elimination and predictable size | — ([METAPROGRAMMING §5](METAPROGRAMMING.md#5-reflection-proposal)) |
| `decimal` and arbitrary-precision integers as primitives | No hardware support; library types instead | 0016 |
| Glob imports | Unclear name origins; fragile library evolution | — ([MODULE_SYSTEM §2.4](MODULE_SYSTEM.md#24-import-forms)) |

## 5. Pending decisions

Each item has documented options and a recommendation; none is adopted yet.

| # | Question | Recommendation | Where documented |
|---|---|---|---|
| C4 | Spell the unit type `void`? | Yes: `void` as the prelude name of `()` | [TYPE_SYSTEM §12](TYPE_SYSTEM.md#12-unresolved-questions) |
| C5 | `float` as an alias of `f64`? | Yes, like `int` | [TYPE_SYSTEM §12](TYPE_SYSTEM.md#12-unresolved-questions) |
| C6 | String interpolation syntax | `"${expr}"`; must be decided **before v0.1.0** | [LEXICAL_STRUCTURE §5.5](LEXICAL_STRUCTURE.md#55-string-literals), [ADR-0025](../docs/adr/0025-string-interpolation-syntax.md) |
| C7 | Closure syntax `x => expr` | Yes | [FUNCTIONAL_PROGRAMMING §2](FUNCTIONAL_PROGRAMMING.md#2-closures-unresolved) |
| C9 | Prelude with `print`, `println`, `assert`, `panic` | Yes, from v0.1 | [STANDARD_LIBRARY §1](STANDARD_LIBRARY.md#1-layering-and-prelude) |
| C11 | `module` declaration and `export` keyword | Neither: module = file, visibility = `public` | [MODULE_SYSTEM §2.2](MODULE_SYSTEM.md#22-file-based-modules) |
| C12 | `init`, `borrow`, `parallel`, `supervisor` keywords | None reserved now | [OBJECT_MODEL](OBJECT_MODEL.md), [CONCURRENCY_MODEL](CONCURRENCY_MODEL.md) |
| — | `repeat N { }` | Reject (redundant with `for _ in 0..N`) | — |
| — | `none` literal | Use the `None` variant | [TYPE_SYSTEM](TYPE_SYSTEM.md#10-null-safety) |
| — | `range()`, `round()`, `typeof()` | Reject / method / compile-time `typeName<T>()` | [STANDARD_LIBRARY §3](STANDARD_LIBRARY.md#3-fundamental-functions-planned) |
| — | `internal`, `readonly` | Undecided | [MODULE_SYSTEM §2.3](MODULE_SYSTEM.md#23-visibility-planned) |
| — | Naming convention: lowerCamelCase functions, UpperCamelCase types | Confirm (implied by the approved names `sizeOf`, `alignOf`) | [ADR-0021](../docs/adr/0021-fundamental-library-functions.md) |
| — | `await` (prefix or postfix) | Decide with the concurrency model | [CONCURRENCY_MODEL §3](CONCURRENCY_MODEL.md#3-the-main-design-question-unresolved) |
