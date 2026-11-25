# Generics

Status: **Proposal.** Generics are not part of v0.1.
Implementation: not started.

## 1. Goals

* Write algorithms and data structures once for many types.
* Check generic code once, at its definition, with errors that refer to the
  generic code rather than to each instantiation (unlike C++ templates before
  concepts).
* Zero runtime cost by default.

## 2. Syntax

```pinguer
func identity<T>(value: T) -> T {
    return value
}

func largest<T: Comparable>(a: T, b: T) -> T {
    if a > b {
        return a
    }
    return b
}

struct Pair<A, B> {
    first: A
    second: B
}
```

* Type parameters are declared in `< >` after the name.
* Constraints use interfaces: `T: Comparable`, combined with `+`:
  `T: Hash + Equatable`. Complex constraints use a `where` clause.
* Type arguments are usually inferred from the call's arguments and expected
  type.

### 2.1 Explicit type arguments (Unresolved)

`identity<int>(5)` is ambiguous with comparisons (`a < b > (c)`) when parsed
without type information. Options:

| Option | Example | Notes |
|---|---|---|
| Turbofish | `identity::<int>(5)` | Unambiguous; widely disliked; requires `::`. |
| Square brackets for type arguments | `identity[int](5)` | Go and Scala; conflicts with indexing syntax. |
| Speculative parsing | `identity<int>(5)` | C# and TypeScript: try to parse `<…>` as type arguments, accept only if followed by `(`, `)`, `.`, `,` and similar tokens. Familiar, but adds parser complexity and rare surprises. |

Speculative parsing is favoured, because explicit type arguments are rare
when inference is good. The parser must also split `>>` into two `>` tokens
when closing nested argument lists (`List<List<int>>`).

## 3. Semantics

### 3.1 Checking

A generic function body is type checked once, using only the operations its
constraints provide. Calling `a > b` on an unconstrained `T` is an error at
the definition.

### 3.2 Implementation strategy

| Strategy | Used by | Benefits | Costs |
|---|---|---|---|
| **Monomorphization** | C++, Rust | Fastest code; inlining; no boxing | Larger binaries; longer compile times |
| **Dictionary passing** | Swift (witness tables), Haskell | Smaller code; separate compilation of generics | Indirect calls; harder optimization |
| **Type erasure with boxing** | Java | Simple | Boxing and casts; poor performance for value types |

Pinguer will **monomorphize** by default, consistent with zero-cost
abstractions and LLVM optimization. Dictionary passing remains available
for interface-typed values (dynamic dispatch, see
[OBJECT_MODEL.md](OBJECT_MODEL.md#interfaces)). Code-size mitigation
(sharing identical instantiations, polymorphization) is an optimization, not
a language rule.

### 3.3 Not planned for the first generics release

* Specialization (overlapping implementations).
* Higher-kinded types.
* Variadic generics.
* Generic associated types.

**Proposal for later:** const generics (`Array<T, N: int>`), driven by the
compile-time execution design ([METAPROGRAMMING.md](METAPROGRAMMING.md)).

## 4. Unresolved questions

1. Explicit type argument syntax (section 2.1).
2. Coherence rules for interface implementations (orphan rule as in Rust, or a
   relaxed variant).
3. Variance of generic types once references exist.
4. Whether interfaces can have associated types in the first release.
