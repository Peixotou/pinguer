# Concurrency Model

Status: **Proposal** and **Unresolved.** v0.1 programs are single-threaded and
have no concurrency features. `async`, `await`, `spawn` and `actor` are
reserved keywords.
Implementation: not started.

## 1. Goals

1. **Data-race freedom** in safe code, checked at compile time.
2. **Structured concurrency:** concurrent tasks have a lexical scope; a scope
   does not end until its tasks have finished or been cancelled, so tasks are
   never leaked and errors are never lost.
3. **Scalable I/O** without one operating-system thread per connection.
4. **No mandatory runtime** for programs that do not use concurrency.

## 2. Data-race freedom

Data-race freedom will follow from the ownership model
([OWNERSHIP_MODEL.md](OWNERSHIP_MODEL.md)): a value may be accessed by another
task only if it is moved there, or if its type is marked as safe to share
(analogous to Rust's `Send` and `Sync`, or Swift's `Sendable`). Mutable shared
state requires synchronization types (mutexes, atomics, channels) from the
standard library.

## 3. The main design question (Unresolved)

| Option | Description | Benefits | Costs |
|---|---|---|---|
| **A. async/await with stackless coroutines** (Rust, C#, Swift, Kotlin) | `async func` returns a suspended computation; `await` suspends. | Zero-cost when unused; works without a runtime; fits LLVM well; precise control. | "Function colouring": async and sync code do not compose freely; library duplication. |
| **B. Stackful lightweight threads** (Go, Java virtual threads, Erlang) | Every function can block; a scheduler multiplexes tasks onto OS threads. | No colouring; simple mental model. | Requires a runtime and scheduler; segmented or growable stacks; FFI calls block a carrier thread; harder for embedded targets. |
| **C. Effect-based** (OCaml 5 effects, Koka) | Suspension is an effect handled by a scheduler. | Avoids colouring with static typing. | Research-grade in compiled systems languages; high implementation risk. |

Option A would look like this:

```pinguer
async func fetchData(url: string) -> Result<Response, Error> {
    return await http.get(url)
}
```

With prefix `try` ([ADR-0013](../docs/adr/0013-prefix-try.md)), a failing
asynchronous call is written `try await http.get(url)`, which means
`try (await http.get(url))`: wait for the result, then propagate an error.

**Information needed to decide:** a prototype comparing binary size, runtime
requirements, and the ergonomics of a realistic server and CLI under each
option; the cost of colouring for the standard library; and the needs of the
embedded and WebAssembly targets. The decision will be recorded in an ADR.

Whichever option is chosen, `await` will be a prefix keyword as sketched only
if it does not create ambiguity with the newline rules; a postfix form
(`http.get(url).await`) remains under consideration.

## 4. Structured concurrency (Proposal)

```pinguer
task.scope { s ->
    let a = s.spawn(loadUser(id))
    let b = s.spawn(loadOrders(id))
    render(a.join(), b.join())
}
```

The closure syntax above is illustrative only; closure syntax is
**Unresolved** ([FUNCTIONAL_PROGRAMMING.md](FUNCTIONAL_PROGRAMMING.md#2-closures-unresolved)).

* Child tasks cannot outlive the scope.
* If a child fails, siblings are cancelled and the error propagates to the
  scope owner.
* Cancellation is cooperative and observed at suspension points.

## 5. Actors and supervision (Proposal)

Erlang-style actors (isolated state, message passing) and supervision trees
(restart strategies for failed tasks) are planned as a **library** built on
the task model, not as core syntax, unless experience shows that language
support is needed for isolation guarantees. `actor` is reserved to keep that
option open.

**Unresolved (C12), with recommendation:** `parallel` (a block that runs its
statements concurrently) and `supervisor` were proposed as keywords. The
recommendation is to reserve neither now: their meaning depends on the
undecided model in section 3. If a `parallel { … }` block is adopted later, it
can be introduced as a **contextual** keyword (like `query`,
[ADR-0015](../docs/adr/0015-contextual-query-keywords.md)) without breaking
programs that use `parallel` as a name.

## 6. Memory ordering

Atomics will follow the C++/LLVM memory model (sequentially consistent,
acquire, release, relaxed), since that is what the backend implements.
