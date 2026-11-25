# Functional Programming

Status: none of these features are in v0.1. Each section is marked:
**Proposal**, **Unresolved**, **Experimental** (§4.2, queries) or **Planned**
(§5, pipelines).
Implementation: not started.

Pinguer supports a functional style where it improves clarity: immutable
bindings by default (`let`), algebraic data types, exhaustive pattern
matching, first-class functions and composable iteration. It is not a pure
functional language: side effects are allowed and not tracked by the type
system.

## 1. Functions as values (Proposal)

```pinguer
func apply(f: func(int) -> int, x: int) -> int {
    return f(x)
}
```

Function types are written `func(ParameterTypes) -> ReturnType`. Named
functions can be passed where a function type is expected.

## 2. Closures (Unresolved)

Closures capture variables from their environment. How they capture depends
on the ownership model:

* **Non-escaping closures** (passed to a function that only calls them during
  the call) can borrow captured variables. This covers iteration helpers such
  as `map` and `filter` without allocation.
* **Escaping closures** (stored or returned) must copy or move their captures.

**Unresolved (C7), with recommendation:** closure syntax. Candidates:

| Syntax | Used by | Assessment |
|---|---|---|
| `x => x * 2`, `(a, b) => a + b`, `x => { … }` | C#, JavaScript, TypeScript, Scala | **Recommended.** Familiar to the largest audience; reads well in pipelines and queries. |
| `{ x -> x * 2 }` | Kotlin, Swift-like | Conflicts with the rule that `{` must stay on the line of its construct, and looks like a block. |
| <code>&#124;x&#124; x * 2</code> | Rust | Compact; visually noisy next to <code>&#124;</code>, <code>&#124;&#124;</code> and <code>&#124;&gt;</code>. |

Consequences of the recommended syntax:

* `=>` is already a token (match arms). In a match arm whose body is a
  closure (`Some(f) => x => x + 1`) the meaning is unambiguous but hard to
  read; a style lint will suggest parentheses.
* `(a, b) => …` starts like a parenthesized expression; the parser looks ahead
  to the matching `)` and checks for `=>` (bounded lookahead, as in C# and
  TypeScript).
* Parameter types are optional when inferable: `(x: int) => x * 2`.

## 3. Algebraic data types and pattern matching (Proposal)

```pinguer
func area(shape: Shape) -> f64 {
    return match shape {
        Circle(radius) => 3.141592653589793 * radius * radius
        Rectangle(width, height) => width * height
        Empty => 0.0
    }
}
```

* `match` is exhaustive: a missing case is a compile-time error that names an
  uncovered value.
* Patterns: literals, bindings, `_`, enum variants with nested patterns,
  tuples, struct fields, and ranges.
* Guards: `Circle(r) if r > 0.0 => …`.
* Unreachable arms produce a warning.

Match arms are separated by line breaks, like statements.

## Expression-oriented control flow

**Proposal:** `if` and `match` become expressions that produce a value when
every branch produces one:

```pinguer
let label = if count == 1 { "item" } else { "items" }
```

This is deferred from v0.1 to keep the first grammar and type checker simple.
**Unresolved:** whether blocks in general become expressions whose value is
their last expression (Rust), which interacts with the newline-terminated
statement rules.

## 4. Iteration and queries

### 4.1 Iterator combinators (Proposal)

The standard library will provide lazy iterator adapters (`map`, `filter`,
`take`, `fold`, `zip`, `groupBy`, `orderBy`, …) that compile to loops without
allocation:

```pinguer
let names = users
    .filter(u => u.active)
    .map(u => u.name)
    .collect()
```

Lines starting with `.` continue the previous line (Normative v0.1,
[ADR-0014](../docs/adr/0014-leading-dot-continuation.md)). The closure syntax
shown is the recommendation for C7 and is not yet decided.

### 4.2 Query expressions (Experimental)

Decision: [ADR-0015](../docs/adr/0015-contextual-query-keywords.md). Grammar:
`QueryExpr` in [GRAMMAR.ebnf](GRAMMAR.ebnf).

```pinguer
let names = query {
    from user in users
    where user.active
    order by user.name ascending
    select user.name
}
```

**Semantics.** A query is pure syntax: it is translated into iterator
combinators before type checking, and has no meaning beyond that translation:

| Clause | Translation (on the sequence built so far) |
|---|---|
| `from x in source` | start from `source.iter()`; `x` names each element |
| `where cond` | `.filter(x => cond)` |
| `order by k1, k2 descending` | `.orderBy(x => k1).thenByDescending(x => k2)` |
| `select e` | `.map(x => e)` |
| `select e1, e2` | `.map(x => (e1, e2))` (a tuple) |

The result is a lazy sequence; nothing is evaluated until it is iterated or
collected.

**Restrictions.**

* Exactly one `from`, first; then any number of `where` and at most one
  `order by`; exactly one `select`, last. Each clause starts on its own line.
* `source` must be iterable; `order by` keys must be comparable.
* The clause words are contextual keywords
  ([LEXICAL_STRUCTURE.md §5.2.1](LEXICAL_STRUCTURE.md#521-contextual-keywords)):
  a variable named `order` or `select` remains usable everywhere, including
  inside clause expressions (`where order.total > 10`).
* `query {` directly in the condition of `if`/`while` or the scrutinee of
  `match` is not a query (it is an identifier followed by a block); wrap the
  query in parentheses there.

**Exceptional behaviour.** Exactly that of the translated combinators.
**Unresolved:** whether `try` inside a clause propagates from the enclosing
function (it would require the translation to use non-escaping closures that
can return from their caller).

**Requires:** closures, iterators, interfaces for comparison, tuples. Target
v0.5 or later.

## 5. Composition

**Planned:** the pipeline operator, `x |> f(a)` meaning `f(x, a)`
([OPERATORS.md §6](OPERATORS.md#6-pipeline--planned),
[ADR-0019](../docs/adr/0019-pipeline-operator.md)). Methods are used for
operations associated with a type; pipelines compose free functions from any
module:

```pinguer
let report = readLines(path)
    |> parseRecords
    |> summarize(byMonth)
```
