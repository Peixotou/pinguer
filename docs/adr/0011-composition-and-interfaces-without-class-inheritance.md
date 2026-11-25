# 0011. Composition and interfaces without class inheritance

- Status: Accepted
- Date: 2026-11-25

## Context

A proposal under review suggested Java/C#-style classes: `class`, `extends`,
`virtual`, `override`, `super` and `protected`. The specification already
favoured structs, interfaces and composition
([OBJECT_MODEL.md](../../specifications/OBJECT_MODEL.md)).

Implementation inheritance couples subclasses to the internal behaviour of
their base classes (the fragile base class problem), makes virtual dispatch
the default cost of method calls, and interacts badly with value semantics
and ownership: a subclass value cannot be stored in a base-class-sized slot,
so inheritance effectively forces reference semantics and heap allocation.

## Decision

* Pinguer organises behaviour with **structs, enums, methods and
  interfaces**. Reuse is achieved by **composition** and by **default
  methods** in interfaces.
* **Implementation inheritance is rejected**: `extends`, `virtual`,
  `override`, `super` and `protected` are not part of the language and are not
  reserved.
* To keep the object model able to evolve, `class` is **reserved**. A future
  ADR may introduce `class` as a *reference type* (shared identity, as in
  Swift) without inheritance, if real programs need it.
* Interface conformance stays explicit and nominal.

## Alternatives considered

| Alternative | Assessment |
|---|---|
| Single inheritance (Java, C#, Kotlin) | Familiar; brings fragile base classes, `protected`, constructor chaining and virtual-by-default costs. |
| Multiple inheritance (C++) | Diamond problem; rejected. |
| Go-style struct embedding | Promotes fields and methods implicitly; a candidate for reducing composition boilerplate, but implicit promotion hides where behaviour comes from. |
| Kotlin-style interface delegation (`… by impl`) | Recorded as a **Proposal** for reducing composition boilerplate explicitly. |

## Consequences

* The lexer reserves `class` (implemented, with tests); `extends`, `super`,
  `virtual`, `override` and `protected` remain usable identifiers.
* No vtables are needed for ordinary method calls; dynamic dispatch exists
  only for interface-typed values (`any Interface`, spelling unresolved).
* Programmers from Java/C# need guidance; the documentation will include a
  "from inheritance to composition" chapter.
