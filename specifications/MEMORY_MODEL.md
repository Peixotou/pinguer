# Memory Model

Status: section 2 is **Normative (v0.1)**. Sections 3–6 evaluate alternatives
and record **Proposals** and **Unresolved** questions.
Implementation: not started.

## 1. Long-term goals

1. **Memory safety by default.** Without `unsafe`, a program cannot read
   uninitialized memory, access memory after it is freed, free memory twice,
   access out of bounds, or race on data between threads.
2. **Predictable performance.** No mandatory tracing garbage collector and no
   hidden allocations. Deallocation happens at points that can be predicted
   from the source.
3. **Low annotation burden.** Most code should need no lifetime or ownership
   annotations.
4. **Explicit control when needed.** Custom allocators, arenas and raw memory
   access are available, with the unsafe parts clearly marked.

These are goals. None of them is a claim about the current compiler; they
must be validated by the implementation, tests, fuzzing and, eventually,
formal reasoning.

## 2. The v0.1 memory model (Normative v0.1)

v0.1 deliberately uses a restricted model that is memory safe by
construction, so the first release does not depend on an unfinished ownership
design.

1. **Value semantics only.** Every v0.1 type is a value type. Assignment,
   argument passing and return copy the value. There are no references,
   pointers or shared mutable state.
2. **No user-visible heap allocation.** Programs cannot allocate. Local
   bindings live in the function's stack frame (or in registers).
3. **Strings are static.** A `string` refers to literal text stored in the
   read-only data of the executable for the whole program run. Strings are
   immutable.
4. **Definite initialization.** Every binding is initialized at its
   declaration, so uninitialized memory cannot be observed.
5. **No global mutable state.** Top-level items are functions, imports and
   constants.
6. **Single-threaded.** v0.1 programs have one thread of execution.

Under these rules no v0.1 operation can create a dangling reference, a double
free, an out-of-bounds access or a data race, because the language has no
references, frees, indexing or threads. The remaining failure modes are
defined: arithmetic errors trap ([TYPE_SYSTEM.md §6](TYPE_SYSTEM.md#6-operators)),
and stack exhaustion is handled as below. The guarantee is conditional on the
correctness of the compiler, the runtime and the platform, which is what the
test strategy must establish.

### Stack overflow

Deep recursion can exhaust the stack. The compiler and runtime must ensure
that stack exhaustion terminates the process rather than corrupting memory,
relying on the guard page that every supported platform places below the
stack, and on stack probes for frames larger than a page (LLVM's
`probe-stack` support). **Unresolved:** whether the runtime installs a handler
that prints a Pinguer-specific message before terminating, as Rust does.

## 3. Alternatives evaluated for general memory management

| Approach | Safety | Performance profile | Annotation burden | Main costs |
|---|---|---|---|---|
| **Ownership and borrowing** (Rust) | Static; no runtime cost | Predictable; no GC pauses | High with explicit lifetimes; moderate with inference | Learning curve; lifetime errors; borrow checker complexity |
| **Affine types** | Static; values used at most once | Predictable | Low for moves, needs a story for sharing | Alone, insufficient for shared access |
| **Automatic reference counting** (Swift) | Dynamic; leaks on cycles unless weak references are used | Count updates on every copy; atomic counts across threads | Low | Hidden costs, cycle leaks, unpredictable deallocation points under sharing |
| **Tracing garbage collection** (Go, Java) | Dynamic | Throughput good; pause and memory overhead; nondeterministic finalization | Lowest | Runtime size, pauses, poor fit for real-time, embedded and FFI |
| **Region-based allocation** | Static with region inference | Very fast bulk deallocation | Low with inference, high without | Region inference is hard to make predictable; leaks within long-lived regions |
| **Arena allocation** | Safe when tied to lifetimes | Very fast; bulk free | Low as a library | Needs lifetimes or scoping to stay safe |
| **Explicit allocation** (Zig, C) | Unsafe unless checked | Fully controllable | High | Use-after-free and leaks are the programmer's problem |
| **Lifetime inference** | Static | No runtime cost | Low inside functions; signatures still need information | Inference across function boundaries harms local reasoning and error messages |
| **Escape analysis** | Optimization, not a safety model | Turns heap into stack allocations | None | Results are not visible in the source; cannot carry correctness |

### Compatibility of combinations

* Ownership + arenas + explicit allocators are compatible: arenas and
  allocators become library types whose borrows are checked by the ownership
  model (as in Rust's ecosystem and Zig's allocator interface).
* Ownership + reference counting are compatible when reference counting is an
  explicit library type (`Rc<T>`, `Arc<T>`), not the default for all values.
* Tracing GC + ownership as two default models for the same values are **not**
  compatible without a rigorous design: destructors, FFI pointers and
  borrow guarantees conflict with a collector moving or delaying objects.
  Pinguer will not combine them.
* Escape analysis is an optimization the compiler may apply under any model,
  but it never changes program meaning.

## 4. Proposed direction (Proposal)

The current proposal, detailed in [OWNERSHIP_MODEL.md](OWNERSHIP_MODEL.md):

1. **Single ownership with move semantics** for types that own resources;
   implicit copies for plain data types.
2. **Second-class references** as the default way to share: parameters can be
   borrowed for the duration of a call (read-only or mutable), but references
   cannot be stored in data structures or returned. This removes the need for
   lifetime annotations in the common case.
3. **Deterministic destruction** at the end of the owner's scope.
4. **Explicit library types** for the cases the default does not cover:
   reference-counted `Rc<T>`/`Arc<T>`, arenas, and allocator-parameterized
   collections.
5. **`unsafe`** for raw pointers and manual allocation.

## Unsafe code

**Proposal.** Code inside `unsafe { … }` or an `unsafe func` may perform
operations the compiler cannot verify: dereferencing raw pointers, calling
foreign functions, and manual allocation. The programmer must uphold the
documented invariants of each operation. A safe function that contains an
`unsafe` block is responsible for ensuring that no safe caller can cause
undefined behaviour through it. The compiler will provide a lint that requires
a `// SAFETY:` comment on every `unsafe` block.

## 6. Unresolved questions

1. Whether second-class references are sufficient for idiomatic collection
   and iterator APIs, or whether first-class references with inferred
   lifetimes are needed (see [OWNERSHIP_MODEL.md](OWNERSHIP_MODEL.md)).
2. How strings and collections are represented once allocation exists
   (owned `String` vs. a single copy-on-write `string`).
3. Whether destructors may fail, and how that interacts with traps.
4. The exact stack-overflow behaviour and message.
5. The memory model for concurrent access (atomics and ordering), to be
   aligned with C++/LLVM's model when threads are introduced.
