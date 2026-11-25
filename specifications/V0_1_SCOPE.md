# Pinguer v0.1 Scope

Status: Normative. This document defines the boundary of the first release.
It also tracks implementation status, which is updated as work progresses.

The first release favours correctness over breadth: a small language that is
fully specified, fully implemented and thoroughly tested.

## 1. In scope for v0.1 (Normative v0.1)

| Feature | Specification | Implementation status |
|---|---|---|
| UTF-8 source files, `.pgr` extension | [LEXICAL_STRUCTURE.md](LEXICAL_STRUCTURE.md) | **Implemented** (loading and validation) |
| Lexical analysis | [LEXICAL_STRUCTURE.md](LEXICAL_STRUCTURE.md) | **Implemented** |
| Source locations and spans | [COMPILATION_MODEL.md](COMPILATION_MODEL.md) | **Implemented** |
| Diagnostics (human and JSON) | [docs/architecture/DIAGNOSTICS.md](../docs/architecture/DIAGNOSTICS.md) | **Implemented** (infrastructure; lexical diagnostics only) |
| Parsing to an AST with error recovery | [GRAMMAR.ebnf](GRAMMAR.ebnf) | Not started |
| Name resolution | [LANGUAGE_SPECIFICATION.md §5](LANGUAGE_SPECIFICATION.md#5-declarations-and-bindings-normative-v01) | Not started |
| Static type checking with local inference | [TYPE_SYSTEM.md](TYPE_SYSTEM.md) | Not started |
| Primitive types: `bool`, `i8`–`i64`, `u8`–`u64`, `int`, `f32`, `f64`, `char`, `string`, `()` | [TYPE_SYSTEM.md](TYPE_SYSTEM.md) | Not started |
| `let`, `var`, `const` | [LANGUAGE_SPECIFICATION.md §5](LANGUAGE_SPECIFICATION.md#5-declarations-and-bindings-normative-v01) | Not started |
| Functions, calls, recursion | [LANGUAGE_SPECIFICATION.md §6](LANGUAGE_SPECIFICATION.md#6-functions-normative-v01) | Not started |
| Arithmetic, bitwise, comparison and logical expressions | [TYPE_SYSTEM.md §6](TYPE_SYSTEM.md#6-operators) | Not started |
| Checked conversions with `as` | [TYPE_SYSTEM.md §7](TYPE_SYSTEM.md#7-conversions) | Not started |
| `if` / `else`, `while`, `break`, `continue`, `return` | [LANGUAGE_SPECIFICATION.md §7](LANGUAGE_SPECIFICATION.md#7-control-flow-normative-v01) | Not started |
| Immutable string literals, string equality | [TYPE_SYSTEM.md](TYPE_SYSTEM.md#strings) | Not started |
| Traps for overflow, division by zero, invalid conversions | [ERROR_HANDLING.md](ERROR_HANDLING.md#2-traps-normative-v01) | Not started |
| `import std.io`; `io.print`, `io.println` | [STANDARD_LIBRARY.md](STANDARD_LIBRARY.md) | Not started |
| Typed intermediate representation (PIR) | [COMPILATION_MODEL.md](COMPILATION_MODEL.md) | Not started |
| Native executables through LLVM | [COMPILATION_MODEL.md](COMPILATION_MODEL.md) | Not started (toolchain path validated, see ADR-0003) |
| CLI: `version`, `help`, `check`, `build`, `run` | [docs/architecture/CLI.md](../docs/architecture/CLI.md) | `version`, `help` and `debug tokens` implemented |
| Automated tests and cross-platform CI | [docs/architecture/TESTING_STRATEGY.md](../docs/architecture/TESTING_STRATEGY.md) | Implemented for existing components |
| Build documentation | [docs/BUILDING.md](../docs/BUILDING.md) | Implemented |

## 2. Explicitly out of scope for v0.1

These features are specified only as **Planned**, **Experimental**, **Proposal**
or **Unresolved** and must
not be presented as available:

* structs, enums, algebraic data types and `match`;
* generics and interfaces;
* `Option`, `Result`, `T?`, `try`, `??` and `?.`;
* `**`, `|>`, `i128`, `u128`, `byte`, attributes and the fundamental
  library functions (`assert`, `panic`, …), which are Planned for later
  versions ([FEATURE_MATRIX.md](FEATURE_MATRIX.md));
* arrays, slices, tuples and collections;
* heap allocation, ownership transfer and references;
* closures and function types as values;
* string concatenation, formatting and interpolation;
* multi-file programs, user modules and packages;
* `async`/`await`, threads, actors;
* `unsafe` code and foreign function interfaces;
* compile-time execution beyond constant expressions, macros and attributes;
* the JIT, WebAssembly, bytecode, embedded and GPU backends;
* the formatter, linter, documentation generator, language server, debugger
  support and package manager.

## 3. Release criteria for v0.1.0

v0.1.0 is released when all of the following hold:

1. Every row in section 1 is implemented.
2. Every normative rule in the specification has at least one positive and,
   where applicable, one negative test.
3. Every diagnostic code has a test that triggers it.
4. The compiler does not panic on any input in the test corpus or during a
   fuzzing campaign of at least 24 CPU-hours on the lexer and parser.
5. The CI matrix (Windows, Linux, macOS; x86-64 and ARM64 where runners are
   available) is green.
6. Compiled test programs produce the expected output and exit status on every
   CI platform.
7. The documentation describes only behaviour that exists.
