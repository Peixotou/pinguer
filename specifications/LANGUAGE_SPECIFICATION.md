# Pinguer Language Specification

| | |
|---|---|
| Language version | 0.1 (draft) |
| Compiler version | 0.1.0-dev |
| Status | Draft. Normative sections are binding for the v0.1 implementation. |

This document is the entry point to the Pinguer specification. It defines the
conventions used across all specification documents and the core semantics of
the v0.1 language. Topic-specific rules live in the documents listed in
[Document map](#document-map).

## 1. Conventions

### 1.1 Classification of content

Every feature, rule and proposal in the specification carries exactly one of
these labels:

| Label | Meaning |
|---|---|
| **Normative (v0.1)** | Required for the Pinguer v0.1 language. A conforming v0.1 compiler implements it exactly as written. |
| **Experimental** | Specified well enough to implement behind an explicit opt-in. May change incompatibly. Not part of v0.1 conformance. |
| **Planned** | Approved by an ADR and specified, targeted at a future version. Not yet normative. |
| **Proposal** | A direction under consideration. Not approved, not binding, and may be rejected. |
| **Unresolved** | An open design question. Lists the options, a recommendation and the information needed to decide. |
| **Rejected** | Considered and declined; the reason is recorded. It may only return through a new ADR. |

The labels are defined by [ADR-0022](../docs/adr/0022-feature-classification.md).
Every feature, its classification and its tentative target version are listed
in [FEATURE_MATRIX.md](FEATURE_MATRIX.md).

A specified feature is not necessarily an implemented feature. The current
implementation status is tracked separately in
[V0_1_SCOPE.md](V0_1_SCOPE.md) and in [ROADMAP.md](../ROADMAP.md). As of
0.1.0-dev, only lexical analysis is implemented.

The words **must**, **must not**, **should** and **may** in normative text are
used as in RFC 2119.

### 1.2 Code examples

Code blocks marked `pinguer` are Pinguer source. Unless a section is labelled
Normative (v0.1), examples illustrate proposals and are not guaranteed to
compile with any version of the compiler.

## 2. Design principles

These principles are used to evaluate every feature. They are explained in
[VISION.md](VISION.md).

1. **Complexity belongs in the compiler and tooling**, not in application code.
2. **Explicit over implicit** when the implicit behaviour could surprise the
   reader: no implicit numeric conversions, no truthiness, no hidden control
   flow.
3. **Safe by default, powerful on request.** Unsafe operations are possible
   only inside explicitly marked code.
4. **Predictable performance.** The cost of a construct should be visible
   from its syntax.
5. **Every feature must earn its place** by solving a real problem without
   conflicting with existing semantics.

## 3. Program structure (Normative v0.1)

A v0.1 program is a single source file with the extension `.pgr`, encoded in
UTF-8 (see [LEXICAL_STRUCTURE.md](LEXICAL_STRUCTURE.md)). The file contains a
sequence of top-level items:

* `import` declarations ([MODULE_SYSTEM.md](MODULE_SYSTEM.md)),
* function declarations (`func`),
* constant declarations (`const`).

Top-level mutable state is not permitted: `let` and `var` are only valid
inside function bodies.

The program's entry point is a function named `main` that takes no
parameters and returns no value:

```pinguer
import std.io

func main() {
    io.println("Hello, Pinguer!")
}
```

A program without a valid `main` function is rejected. When `main` returns
normally, the process exits with status 0.

The full syntax is defined in [GRAMMAR.ebnf](GRAMMAR.ebnf).

## 4. Statements and termination (Normative v0.1)

Statements are terminated by a line break or by `;`. The lexer turns line
breaks into `NEWLINE` tokens only where they can be significant (see
[LEXICAL_STRUCTURE.md §6](LEXICAL_STRUCTURE.md#6-line-breaks)); the parser
additionally ignores a `NEWLINE` that directly follows a binary operator,
an assignment operator, `as`, `->` or `,`, and a `NEWLINE` that is directly
followed by `.` ([ADR-0014](../docs/adr/0014-leading-dot-continuation.md)).
Therefore:

```pinguer
let total = first +
    second              // continues the expression

io
    .println("Hi")     // a line starting with `.` continues the previous line

let ok = check(
    a,
    b,
)                       // line breaks inside parentheses are ignored
```

Two further layout rules keep the grammar unambiguous:

* An opening brace `{` must be on the same line as the construct it belongs
  to (`func`, `if`, `else`, `while`).
* `else` must be on the same line as the closing brace of the preceding block.

Only function calls may be used as expression statements. An expression
statement whose value would be discarded without effect, such as `a + b` or
`-x`, is an error. This also prevents a line that starts with `-` from being
silently parsed as a separate statement.

**Planned** (with the corresponding operators): an expression statement may
also be `try` applied to a call (`try file.write(data)`), `await` applied to a
call (once `await` exists), or a pipeline (`data |> save`), because each of
these performs a call. A discarded `Result` produces a warning
([ERROR_HANDLING.md §1.3](ERROR_HANDLING.md#13-ignoring-results)).

Assignment is a statement, not an expression, so `if x = 1 { }` is a syntax
error rather than a bug.

Rationale and alternatives: [ADR-0005](../docs/adr/0005-newline-terminated-statements.md).

## 5. Declarations and bindings (Normative v0.1)

| Form | Meaning |
|---|---|
| `let name [: Type] = expr` | Immutable binding. |
| `var name [: Type] = expr` | Mutable binding. |
| `const NAME [: Type] = expr` | Compile-time constant. |

Rules:

* Every `let` and `var` declaration must have an initializer. Declaring a
  variable without a value is a future proposal that requires definite
  initialization analysis.
* If the type annotation is omitted, the type is inferred from the
  initializer ([TYPE_SYSTEM.md §5](TYPE_SYSTEM.md#5-type-inference)).
* A `let` binding cannot be assigned after its declaration. A `var` binding
  can be assigned with `=` and the compound assignment operators.
* A `const` initializer must be a *constant expression*: literals, other
  constants, and the arithmetic, bitwise, logical, comparison and `as`
  operators applied to constant expressions. It is evaluated at compile time;
  arithmetic errors during evaluation (overflow, division by zero) are
  compile-time errors.
* Function parameters are immutable bindings.

### 5.1 Scopes

* All top-level items are visible throughout the file, regardless of order.
  Functions may call functions declared later and may be recursive or
  mutually recursive.
* A local binding is visible from the end of its declaration to the end of the
  enclosing block. A binding is not visible in its own initializer.
* Declaring a name that is already declared **in the same scope** is an
  error. A function's parameters and the top level of its body form a single
  scope.
* A declaration in an inner block may shadow a binding of an outer block.
  Whether this should produce a warning is **Unresolved**.
* Cyclic constant definitions are an error.
* Type names and value names live in separate namespaces.

## 6. Functions (Normative v0.1)

```pinguer
func sum(a: int, b: int) -> int {
    return a + b
}
```

* Parameter types are always written explicitly. Return types are written
  explicitly; omitting `-> Type` means the function returns the unit type
  `()`.
* Arguments are passed by value and evaluated left to right.
* A function with a non-unit return type must return a value on every path;
  the compiler rejects functions where control can reach the end of the body.
  A unit function may use `return` without a value or simply end.
* There is no overloading, no default arguments and no named arguments in
  v0.1. Named arguments are a [proposal](OBJECT_MODEL.md#construction).
* Recursion is supported. Exceeding the stack limit terminates the process;
  it never causes undefined behaviour (see [MEMORY_MODEL.md](MEMORY_MODEL.md#stack-overflow)).

## 7. Control flow (Normative v0.1)

```pinguer
if count > 10 {
    io.println("large")
} else if count > 0 {
    io.println("small")
} else {
    io.println("empty")
}

var i = 0
while i < 10 {
    i += 1
    if i % 2 == 0 {
        continue
    }
    if i > 7 {
        break
    }
}
```

* The condition of `if` and `while` must have type `bool`. There is no
  truthiness.
* No parentheses around conditions; braces are mandatory.
* `break` and `continue` apply to the innermost `while` loop and are errors
  outside a loop.
* In v0.1, `if` and blocks are statements, not expressions. Expression-oriented
  control flow (`if` and `match` producing values) is a
  [proposal](FUNCTIONAL_PROGRAMMING.md#expression-oriented-control-flow).
* `for` loops over ranges and iterators are a proposal; `for`, `in` and `loop`
  are reserved keywords.

## 8. Expressions (Normative v0.1)

### 8.1 Operator precedence

From highest to lowest. Operators on the same row have equal precedence.
This table lists the v0.1 operators only. The complete table, including the
planned operators `**`, `?.`, `|>`, `??` and `try`, and the proposed
`await`, is in
[OPERATORS.md](OPERATORS.md); the planned operators are inserted without
changing the relative order of the v0.1 operators.

| Precedence | Operators | Associativity |
|---|---|---|
| 1 | call `f(x)`, member access `a.b` | left |
| 2 | unary `-` `!` `~` | prefix |
| 3 | `as` | left |
| 4 | `*` `/` `%` | left |
| 5 | `+` `-` | left |
| 6 | `<<` `>>` | left |
| 7 | `&` | left |
| 8 | `^` | left |
| 9 | <code>&#124;</code> | left |
| 10 | `==` `!=` `<` `<=` `>` `>=` | none |
| 11 | `&&` | left |
| 12 | <code>&#124;&#124;</code> | left |

Unlike C, the bitwise operators bind more tightly than comparisons, so
`flags & MASK == 0` means `(flags & MASK) == 0`. Comparison operators are
non-associative: `a < b < c` is a syntax error.

### 8.2 Operator semantics

The operand rules, overflow behaviour and conversion rules are defined in
[TYPE_SYSTEM.md](TYPE_SYSTEM.md). In summary:

* Arithmetic, bitwise and comparison operators require both operands to have
  the same type. There are no implicit conversions.
* Integer overflow, division by zero and out-of-range shifts **trap**: the
  program stops with an error report. They never wrap silently and are never
  undefined behaviour.
* `&&` and `||` take `bool` operands and short-circuit.
* `x as T` performs an explicit, checked numeric conversion.

### 8.3 Evaluation order

Evaluation order is fully defined: operands are evaluated left to right before
the operator is applied (except for the short-circuit right operand of `&&`
and `||`, and, when they exist, of `??` and the rest of a `?.` chain; see
[OPERATORS.md](OPERATORS.md)), and call arguments are evaluated left to right after the callee.
In a compound assignment `x op= e`, `e` is evaluated before the operation.

## 9. Runtime errors (Normative v0.1)

A *trap* is an unrecoverable runtime error caused by a bug in the program,
such as integer overflow. When a trap occurs the program writes a description
to standard error and terminates with a non-zero exit status. A trap never
continues execution and never results in undefined behaviour. The exact exit
status and message format are **Unresolved**
([ERROR_HANDLING.md](ERROR_HANDLING.md#2-traps-normative-v01)).

## 10. Unsafe code

The `unsafe` keyword is reserved. v0.1 has no unsafe operations and therefore
no `unsafe` blocks or functions. The intended semantics are described in
[MEMORY_MODEL.md](MEMORY_MODEL.md#unsafe-code) and
[SECURITY_MODEL.md](SECURITY_MODEL.md).

## Document map

| Document | Topic | Contains normative v0.1 rules |
|---|---|---|
| [VISION.md](VISION.md) | Goals, audience, principles, influences | No |
| [V0_1_SCOPE.md](V0_1_SCOPE.md) | v0.1 feature boundaries and implementation status | Yes |
| [FEATURE_MATRIX.md](FEATURE_MATRIX.md) | Every feature with its classification and target version | No |
| [LEXICAL_STRUCTURE.md](LEXICAL_STRUCTURE.md) | Source encoding, tokens, comments, literals | Yes |
| [GRAMMAR.ebnf](GRAMMAR.ebnf) | Formal syntax | Yes |
| [TYPE_SYSTEM.md](TYPE_SYSTEM.md) | Types, inference, arithmetic, conversions | Yes |
| [OPERATORS.md](OPERATORS.md) | Precedence, associativity and semantics of every operator | Yes |
| [MEMORY_MODEL.md](MEMORY_MODEL.md) | Storage, safety guarantees, alternatives evaluated | Yes |
| [OWNERSHIP_MODEL.md](OWNERSHIP_MODEL.md) | Ownership and borrowing proposal | No |
| [CONCURRENCY_MODEL.md](CONCURRENCY_MODEL.md) | Tasks, async, actors | No |
| [ERROR_HANDLING.md](ERROR_HANDLING.md) | Traps, `Result`, `Option` | Traps only |
| [MODULE_SYSTEM.md](MODULE_SYSTEM.md) | Imports, modules, packages | `import std.*` only |
| [GENERICS.md](GENERICS.md) | Generic functions and types | No |
| [OBJECT_MODEL.md](OBJECT_MODEL.md) | Structs, enums, methods, interfaces | No |
| [FUNCTIONAL_PROGRAMMING.md](FUNCTIONAL_PROGRAMMING.md) | Closures, ADTs, pattern matching | No |
| [METAPROGRAMMING.md](METAPROGRAMMING.md) | Constant evaluation, compile-time execution, attributes | Constants only |
| [INTEROPERABILITY.md](INTEROPERABILITY.md) | C interoperability, embedding | No |
| [STANDARD_LIBRARY.md](STANDARD_LIBRARY.md) | Library layering and v0.1 API | Yes |
| [COMPILATION_MODEL.md](COMPILATION_MODEL.md) | Pipeline, targets, linking | Yes |
| [ABI.md](ABI.md) | Data layout, calling conventions, symbols | Partially |
| [SECURITY_MODEL.md](SECURITY_MODEL.md) | Threat model and guarantees | Yes |
