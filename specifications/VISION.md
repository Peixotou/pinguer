# Pinguer Vision

> *Complexity simplified.*

Status: Draft. This document is not normative; it explains the goals that the
normative specification serves.

## Purpose

Pinguer is an open-source, general-purpose, statically typed programming
language that compiles ahead of time to native code. It aims to give
professional developers the control and performance of systems languages with
the readability and productivity of high-level languages.

The guiding idea is to place complexity where it is cheapest to pay: in the
compiler and the tooling, which are written once, rather than in every
program, which is written many times.

## Goals

1. **Memory safety by default.** Programs that do not use `unsafe` must not
   exhibit undefined behaviour. This is a design goal that must be validated
   by the implementation, tests and, eventually, formal reasoning; it is not a
   claim about the current compiler.
2. **Predictable performance.** Native compilation through LLVM, no mandatory
   garbage collector, and language constructs whose cost is visible.
3. **Low annotation burden.** Local type inference, inferred ownership where
   possible, and defaults that are right for most code.
4. **Readable code.** A small, regular syntax without semicolons at line ends,
   without parentheses around conditions, and with diagnostics that explain
   problems in plain English.
5. **First-class tooling.** One command-line tool for building, running,
   testing, formatting, linting, documenting and managing packages, plus a
   language server.
6. **Cross-platform from day one.** Windows, Linux and macOS on x86-64 and
   ARM64.

## Non-goals

* Being a superset of, or source-compatible with, any existing language.
* Combining every popular feature. Features from other languages are adopted
  only when they fit Pinguer's semantics (see below).
* Replacing scripting languages for one-off scripts. Fast compile times
  matter, but Pinguer is a compiled language first.
* Runtime reflection-heavy or dynamically typed programming styles.

## Audience

* Application and service developers who want native performance without
  manual memory management.
* Systems programmers who want safer defaults with an explicit escape hatch.
* Teams that value consistent tooling and readable, reviewable code.

## Influences and how they are evaluated

Pinguer studies many languages. A concept is adopted only after answering:
*what problem does it solve, how does it interact with the rest of Pinguer,
and what does it cost to implement and to learn?*

| Language | Concept considered | Current position |
|---|---|---|
| C | Native interoperability, low-level control | Planned: C ABI interop behind `unsafe` ([INTEROPERABILITY.md](INTEROPERABILITY.md)) |
| C++ | Zero-cost abstractions, generic programming | Planned: monomorphized generics ([GENERICS.md](GENERICS.md)); templates' duck typing rejected in favour of checked constraints |
| Rust | Ownership, borrowing, data-race freedom | Under evaluation: ownership with second-class references to reduce annotations ([OWNERSHIP_MODEL.md](OWNERSHIP_MODEL.md)) |
| Java | Interfaces, modular organisation | Planned: interfaces and packages; class inheritance **rejected** ([ADR-0011](../docs/adr/0011-composition-and-interfaces-without-class-inheritance.md)); `public`/`private` adopted ([ADR-0012](../docs/adr/0012-visibility-public-and-private.md)) |
| C# | Properties, async, declarative queries, `??`, `?.` | Async under evaluation; `??` and `?.` planned ([ADR-0018](../docs/adr/0018-optional-operators.md)); LINQ-style queries experimental with contextual keywords ([ADR-0015](../docs/adr/0015-contextual-query-keywords.md)) |
| Go | Simple tooling, concurrency | Adopted: single CLI, formatter as standard; concurrency model unresolved |
| Python | Readability, `**` | Adopted: minimal punctuation, newline-terminated statements; `**` with Python's precedence planned ([ADR-0017](../docs/adr/0017-power-operator.md)) |
| TypeScript | Type inference | Adopted: local inference in v0.1; richer inference later |
| Kotlin | Null safety | Adopted: no null; absence is `Option<T>` ([TYPE_SYSTEM.md](TYPE_SYSTEM.md#null-safety)) |
| Swift | Protocols, value semantics, enums, `try` | Adopted: value semantics in v0.1; enums with payloads planned; prefix `try` planned ([ADR-0013](../docs/adr/0013-prefix-try.md)) |
| F#, Elixir | Pipelines | Planned: <code>&#124;&gt;</code> inserting the first argument ([ADR-0019](../docs/adr/0019-pipeline-operator.md)) |
| Zig | Explicit allocation, compile-time execution | Under evaluation: allocator-aware library, compile-time execution |
| Haskell | ADTs, pattern matching, composition | Planned: enums with payloads and exhaustive `match` |
| Erlang | Actors, supervision | Proposal: library-level actors on top of the task model |
| Lua | Embeddability | Proposal: embedding API through the C ABI |
| Julia | Numerical computing, multiple dispatch | Numerics planned in the library; multiple dispatch not planned (conflicts with static dispatch and separate compilation, see [OBJECT_MODEL.md](OBJECT_MODEL.md#multiple-dispatch)) |
| SQL | Declarative queries | Experimental: `query { from … where … select … }` with contextual keywords, translated to library combinators ([ADR-0015](../docs/adr/0015-contextual-query-keywords.md)) |

## Feature admission criteria

A feature is accepted into the normative specification when:

1. it has a written motivation with concrete use cases;
2. its interaction with the type system, memory model and error handling is
   specified, including the edge cases;
3. its implementation cost and its effect on compile time are understood;
4. diagnostics for its misuse have been designed;
5. it has an Architecture Decision Record if it changes a previous decision.
