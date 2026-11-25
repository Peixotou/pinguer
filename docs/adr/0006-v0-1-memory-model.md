# 0006. v0.1 memory model: value semantics without allocation

- Status: Accepted
- Date: 2026-11-24

## Context

Memory safety by default is a core goal, and the ownership design that will
achieve it for heap data is still a proposal
([OWNERSHIP_MODEL.md](../../specifications/OWNERSHIP_MODEL.md)). Shipping v0.1
with heap allocation would either require finishing that design first or
introducing a temporary model (such as reference counting or a GC) that later
changes semantics.

## Decision

v0.1 has value semantics only, no references or pointers, no user-visible
heap allocation, immutable static strings, mandatory initialization, no
global mutable state and a single thread
([MEMORY_MODEL.md §2](../../specifications/MEMORY_MODEL.md#2-the-v01-memory-model-normative-v01)).

## Alternatives considered

* **Reference counting for strings in v0.1.** Would allow concatenation, but
  introduces runtime support and semantics (sharing, destruction) that the
  ownership model would later have to absorb.
* **Leak-only allocation** (allocate, never free). Simple, but it normalizes
  a behaviour that would have to be removed later.
* **Deferring v0.1 until ownership is designed.** Delays validating the whole
  pipeline end to end.

## Consequences

* v0.1 cannot concatenate or format strings; output of non-string values uses
  the `io.print`/`io.println` intrinsics.
* v0.1 programs are memory safe by construction, conditional on compiler
  correctness, which keeps the first implementation verifiable.
* The ownership model can be introduced without breaking v0.1 programs,
  because every v0.1 type will be a copy type.
