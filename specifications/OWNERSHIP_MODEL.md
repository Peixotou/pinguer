# Ownership Model

Status: **Proposal.** Nothing in this document is part of v0.1. v0.1 uses the
restricted value-only model in [MEMORY_MODEL.md §2](MEMORY_MODEL.md#2-the-v01-memory-model-normative-v01).
Implementation: not started.

## 1. Problem

Once Pinguer has heap-allocated values (growable strings, lists, maps), it
needs rules that decide who frees memory and when, and that prevent
use-after-free, double free and data races, without a garbage collector and
without burdening every signature with annotations.

Rust demonstrates that ownership and borrowing achieve this statically, but
explicit lifetime parameters are a frequent source of difficulty. Swift and
Hylo (formerly Val) demonstrate that *mutable value semantics* with
second-class references cover most code with far fewer annotations.

## 2. Proposed model

### 2.1 Copy and move types

* **Copy types** are plain data: all v0.1 primitive types, and structs, enums,
  tuples and arrays composed only of copy types. Assignment copies them.
* **Move types** own resources (heap memory, file handles). Assignment and
  passing by value *move* them; the source binding can no longer be used.
  Using a moved-from binding is a compile-time error.

```pinguer
let a = List<int>()
let b = a          // `a` is moved into `b`
print(a.len())     // error: `a` was moved
```

Copying a move type is explicit: `let b = a.clone()`.

### 2.2 Parameter conventions

A parameter declares how it accesses its argument. The default is the most
common and cheapest safe option.

| Convention | Syntax (tentative) | Meaning |
|---|---|---|
| borrow (default) | `func f(x: List<int>)` | Read-only access for the duration of the call. The caller keeps ownership. |
| mutable borrow | `func f(x: inout List<int>)` | Exclusive read-write access for the duration of the call. Call site writes `f(&x)` **(Unresolved spelling)**. |
| take | `func f(x: take List<int>)` | Ownership moves into the callee. |

For copy types, borrowing and copying are indistinguishable to the program,
and the compiler chooses the cheaper one.

### 2.3 Second-class references

References created by borrowing cannot be stored in structs, captured by
escaping closures, or returned. Because a reference never outlives the call
that created it, the compiler can check borrowing without lifetime
annotations: it only has to verify **exclusivity** at each call:

* while a value is mutably borrowed, no other access to it may occur;
* while a value is immutably borrowed, it may not be mutated or moved.

This is the "law of exclusivity" of Swift and the core of Hylo's model.

### 2.4 Projections

Accessing part of a value (`user.name`, `list[i]`) yields a borrow of that
part, under the same exclusivity rules, so the compiler can track disjoint
field borrows.

### 2.5 Destruction

A value is destroyed when its owner goes out of scope, in reverse declaration
order, unless it was moved. Types may define a destructor (spelling
**Unresolved**).

## 3. Escape hatches

Cases that need longer-lived sharing use explicit library types:

* `Rc<T>` / `Arc<T>`: shared ownership with reference counting (non-atomic and
  atomic). Cycles must be broken with weak references.
* Arenas: many objects freed together, with handles (indices) instead of
  references.
* Raw pointers inside `unsafe` code.

## 4. Alternatives

| Alternative | Why not the default (for now) |
|---|---|
| First-class references with explicit lifetimes (Rust) | Most expressive, but lifetime annotations contradict the low-annotation goal. Remains the fallback if second-class references prove insufficient. |
| First-class references with whole-program lifetime inference | Breaks separate compilation and local reasoning; errors appear far from their cause. |
| ARC for all heap values (Swift classes) | Hidden reference-count traffic, cycles, and less predictable deallocation. Offered as a library type instead. |
| Tracing GC | See [MEMORY_MODEL.md §3](MEMORY_MODEL.md#3-alternatives-evaluated-for-general-memory-management). |

## 5. Unresolved questions

1. Are second-class references expressive enough for iterators over
   collections? Hylo uses *subscripts* and *projections*; Pinguer must
   evaluate them on real library code before committing.
2. Should there be an opt-in form of stored references with lifetime
   parameters for advanced code?
3. Spelling of the mutable-borrow call-site marker and of `take`.
4. How move semantics interact with pattern matching on enums.
5. How closures capture: by copy, by move, or by borrow for non-escaping
   closures.
6. Integration with concurrency: which types may be sent between tasks (see
   [CONCURRENCY_MODEL.md](CONCURRENCY_MODEL.md)).

## 6. Validation plan

Before this proposal becomes normative:

1. Write the standard collections, string type and iterator protocol against
   the model on paper and in a prototype.
2. Port a set of benchmark programs and real-world-shaped samples and record
   every place that needs an escape hatch.
3. Specify the checking algorithm precisely, including the diagnostics it
   produces.
4. Record the decision in an ADR.
