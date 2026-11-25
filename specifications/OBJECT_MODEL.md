# Object Model

Status: the absence of class inheritance is **decided**
([ADR-0011](../docs/adr/0011-composition-and-interfaces-without-class-inheritance.md)).
Structs, enums and interfaces are **Planned** (v0.2 and v0.4); details marked
**Unresolved** are open. No user-defined types exist in v0.1.
Implementation: not started.

Pinguer organises data with structs and enums, behaviour with methods, and
abstraction with interfaces. It has no classes and no implementation
inheritance: composition and interfaces cover the same needs without the
fragile-base-class problem and without the cost of virtual dispatch on every
call.

### Decision summary (ADR-0011)

| Feature | Status |
|---|---|
| Structs, enums, methods | Planned (v0.2) |
| Interfaces with default methods, explicit conformance | Planned (v0.4) |
| Composition | The reuse mechanism |
| `extends`, `virtual`, `override`, `super`, `protected` | **Rejected**; not reserved |
| `class` | Reserved, so that a future ADR may add reference types without inheritance |
| Interface delegation (Kotlin's `by`), to reduce composition boilerplate | Proposal |
| `init` constructors | Unresolved (C12); recommended against, see [Construction](#construction) |

## 1. Structs

```pinguer
struct User {
    id: int
    name: string
    active: bool
}
```

* Fields are separated by line breaks, consistent with statements.
* Structs have value semantics. Whether a struct is a copy type or a move type
  follows from its fields ([OWNERSHIP_MODEL.md](OWNERSHIP_MODEL.md)).
* Field mutability follows the binding: fields of a `var` struct can be
  assigned; fields of a `let` struct cannot.

### Construction

```pinguer
let user = User(id: 1, name: "Ana", active: true)
```

Construction uses call syntax with named arguments rather than a brace
literal (`User { id: 1 }`). Brace literals are ambiguous in conditions such
as `if user == User { … } { … }`, a known problem in Rust's grammar; call
syntax avoids it. **Unresolved:** whether named arguments become available for
ordinary functions at the same time (favoured).

## 2. Enums

```pinguer
enum Shape {
    Circle(radius: f64)
    Rectangle(width: f64, height: f64)
    Empty
}
```

Enums are algebraic data types: each variant may carry a payload. They are
consumed with exhaustive `match` ([FUNCTIONAL_PROGRAMMING.md](FUNCTIONAL_PROGRAMMING.md)).

## 3. Methods

**Unresolved:** where methods are declared.

| Option | Example | Notes |
|---|---|---|
| Inside the type | `struct User { … func greet() { … } }` | Swift, Kotlin, C#. Everything in one place. |
| Separate blocks | `impl User { func greet() { … } }` | Rust. Separates data from behaviour; allows implementing interfaces in separate blocks. |

Both forms are likely needed (inside for inherent methods, separate blocks for
interface conformance and extensions). The receiver is written `self` with the
same access conventions as parameters (borrow by default).

### Properties

**Proposal:** computed properties (C#, Swift, Kotlin), declared like
parameterless methods and accessed without parentheses. They must not hide
expensive work; the standard library's style guide will require properties to
be cheap and side-effect free.

## Interfaces

```pinguer
interface Display {
    func display(self) -> String
}
```

* Interfaces are **nominal**: a type conforms only when it says so. Go-style
  structural conformance is rejected because accidental conformance and
  unclear coherence make large codebases harder to reason about.
* Interfaces may provide default method implementations.
* Interfaces are used as generic constraints (static dispatch, the default)
  and as existential types for dynamic dispatch, spelled explicitly
  (tentatively `dyn Display` or `any Display`, **Unresolved**), so the cost is
  visible.

## Multiple dispatch

Julia's multiple dispatch selects a method at run time based on the dynamic
types of all arguments. It is **not planned** for Pinguer's core language:

* it conflicts with static, monomorphized dispatch and separate compilation;
* method resolution becomes a whole-program property, which harms
  predictability and compile times;
* most uses in numerical code are served by static overloading through
  interfaces plus generics.

Static function overloading itself is **Unresolved**; it interacts poorly with
type inference and is excluded from v0.1.

## Not planned

* Implementation inheritance and `protected` (**Rejected**, ADR-0011).
* Constructors with arbitrary code; factory functions are ordinary functions.
  **Unresolved (C12):** an `init` keyword was proposed. Recommendation: do
  not reserve it. Memberwise construction (`User(id: 1, …)`) covers plain
  data, and a validating factory function can return `Result`, which a
  constructor cannot do cleanly.
* Implicit conversions defined by user types.
