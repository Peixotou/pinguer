# Error Handling

Status: section 2 (traps) is **Normative (v0.1)**. Section 1 is **Planned**
(target v0.3, [ADR-0013](../docs/adr/0013-prefix-try.md),
[ADR-0018](../docs/adr/0018-optional-operators.md)) except where marked
**Unresolved**.
Implementation: not started.

Pinguer distinguishes two kinds of failure:

| Kind | Cause | Mechanism |
|---|---|---|
| **Bugs** | A violated precondition: overflow, division by zero, out-of-bounds index, failed assertion | **Trap**: the program stops. |
| **Expected failures** | Conditions a correct program must handle: missing file, invalid input, network errors | **Values**: `Result<T, E>` and `Option<T>`. |

There are no exceptions. Exceptions make control flow invisible at call
sites, complicate the ownership model (unwinding through partially moved
values) and have costs that are hard to predict. Errors as values keep every
possible failure visible in function signatures.

## 1. Error values (Planned)

```pinguer
enum Result<T, E> {
    Ok(T)
    Err(E)
}

enum Option<T> {
    Some(T)
    None
}
```

In v0.3, `Option<T>` and `Result<T, E>` are **built into the compiler** as the
only generic types, as Go provided built-in generic maps and slices before it
had user-defined generics. Once user-defined generics exist (v0.4), they
become ordinary library enums (see [OBJECT_MODEL.md](OBJECT_MODEL.md)) with
the same behaviour.

```pinguer
func divide(a: f64, b: f64) -> Result<f64, Error> {
    if b == 0.0 {
        return Err(Error("division by zero"))
    }
    return Ok(a / b)
}

match divide(1.0, x) {
    Ok(value) => io.println(value)
    Err(error) => io.println(error.message)
}
```

Note that the comparison is written `b == 0.0`: `b == 0` would be a type
error, because integer literals are never given floating-point types
([TYPE_SYSTEM.md §5.1](TYPE_SYSTEM.md#51-literals)).

### 1.1 Propagation with `try`

```pinguer
func readConfig(path: string) -> Result<Config, Error> {
    let text = try fs.read(path)
    return parse(text)
}
```

**Semantics.** `try e` evaluates `e`, then:

| Operand `e` | Outcome when successful | Outcome on failure |
|---|---|---|
| `Result<T, E>` | `Ok(v)`: the expression evaluates to `v: T` | `Err(x)`: the enclosing function returns `Err(x)` |
| `Option<T>` | `Some(v)`: the expression evaluates to `v: T` | `None`: the enclosing function returns `None` |

**Precedence.** `try` is a prefix operator at level 3 of
[OPERATORS.md](OPERATORS.md#1-precedence-and-associativity), like `-` and `!`.
Like every prefix operator, it applies to the unary expression that
follows it, which includes postfix chains and `**`:

| Expression | Means |
|---|---|
| `try load() + 1` | `(try load()) + 1` |
| `try f() ** 2` | `try (f() ** 2)`: `**` is applied to the `Result`, a type error; write `(try f()) ** 2` |
| `try a.b().c()` | `try (a.b().c())`: `.c()` is called on the `Result` |
| `(try a.b()).c()` | `.c()` is called on the unwrapped value |
| `try await fetch()` | `try (await fetch())` |
| `try (x \|> parse)` | propagate the result of a pipeline |

**Restrictions.**

* The operand must have type `Result<T, E>` or `Option<T>`. Any other type is
  an error.
* `try` may appear only inside a function body (not in constant
  initializers). **Unresolved:** its meaning inside a closure depends on the
  closure design (C7); the expected rule is that it returns from the closure,
  which is also why `try` inside a query clause is unresolved
  ([FUNCTIONAL_PROGRAMMING.md §4.2](FUNCTIONAL_PROGRAMMING.md#42-query-expressions-experimental)).
* With a `Result` operand, the enclosing function must return `Result<U, F>`.
  Until error conversion is decided (below), `E` must equal `F`.
* With an `Option` operand, the enclosing function must return `Option<U>`.
  Mixing is an error: an `Option` in a `Result` function must be converted
  explicitly (for example `try value.okOr(MissingValue)`), and a `Result` in an
  `Option` function with `.ok()`.
* `main` may return `Result<(), E>` (**Unresolved:** spelling of the unit type,
  see C4). An `Err` returned from `main` is printed to standard error and the
  process exits with a non-zero status.

**Exceptional behaviour.** `try` never traps. On failure it performs an
ordinary early return: local values are destroyed as on any `return`, and no
unwinding mechanism exists.

**Interactions and known costs.**

* Because postfix operators bind more tightly than `try`, calling a method on
  the unwrapped value requires parentheses. The type checker detects the
  common mistake (a method missing on `Result` but present on its `Ok` type)
  and suggests the parenthesized form.
* `await try f()` is only valid if the unwrapped value can itself be awaited;
  the usual order is `try await f()`.
* Postfix `?` is **not** an operator in Pinguer; writing `f()?` is a syntax
  error whose diagnostic suggests `try f()`.

**Unresolved:** how errors of different types are converted during
propagation. Options: (a) require identical types and explicit `mapErr`;
(b) an interface-based conversion like Rust's `From`; (c) a single standard
`Error` type with dynamic payload. Option (b) is favoured but depends on
interfaces.

### 1.2 The standard `Error` type

**Unresolved.** A standard error type with a message, an optional cause chain
and optional source location would make simple programs easy; library authors
would still define precise error enums.

### 1.3 Ignoring results

A `Result` that is neither used nor explicitly discarded will produce a
warning, as in Rust's `#[must_use]`.

## 2. Traps (Normative v0.1)

A trap is raised when a program violates a language-level precondition. In
v0.1 the trapping conditions are:

* integer overflow in arithmetic, negation and conversions;
* integer division or remainder by zero, and signed minimum divided by −1;
* shifts by an amount that is negative or not smaller than the bit width of the type;
* float-to-integer conversion of NaN or out-of-range values;
* char-to-integer conversion of a value that does not fit.

When a trap occurs:

1. a message identifying the kind of trap is written to standard error;
2. the process terminates with a non-zero exit status;
3. no further Pinguer code runs: there is no unwinding and no recovery.

A trap never results in undefined behaviour, and the compiler must not
optimize on the assumption that a trapping condition cannot occur in a way
that changes observable behaviour before the trap.

**Unresolved:**

* the exact exit status (candidates: 101 like Rust panics, or 134 / abort);
* whether the message includes the source location (desirable; requires
  embedding location tables in the executable);
* whether stack traces are printed in debug builds.

### 2.1 Future trapping operations (Proposal)

Out-of-bounds indexing, `unwrap()` on `None`/`Err`, failed `assert`, and an
explicit `panic("message")` function will also trap.

**Unresolved:** whether traps may ever be caught (for example at task or
thread boundaries, to let servers survive a failing request). If allowed, this
will be restricted to task boundaries in the concurrency model and require
unwinding support.
