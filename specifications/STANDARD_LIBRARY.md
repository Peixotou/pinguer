# Standard Library

Status: section 2 is **Normative (v0.1)**; section 3 is **Planned**
([ADR-0021](../docs/adr/0021-fundamental-library-functions.md)); the rest is
**Proposal** or **Unresolved** as marked.
Implementation: not started.

## 1. Layering and prelude

| Layer | Requires | Contents |
|---|---|---|
| `core` | Nothing | Primitive type operations, `Option`, `Result`, iterators, formatting traits, checked/wrapping arithmetic |
| `alloc` | An allocator | Owned strings, lists, maps, `Rc`/`Arc` |
| `std` | An operating system | I/O, files, processes, environment, time, threads, networking |

Programs for embedded targets use `core` (and optionally `alloc`). The layers
are re-exported through `std` so that ordinary programs only see `std`.

### The prelude (Unresolved, C9)

A *prelude* is a set of names available in every file without an import.

**Recommendation:** a small prelude containing the primitive type names,
`Option`/`Some`/`None`, `Result`/`Ok`/`Err` (when they exist) and the
functions `print`, `println`, `assert` and `panic`, so that

```pinguer
func main() {
    println("Hello, Pinguer!")
}
```

is a complete program. Rules that would accompany it:

* the prelude is the outermost scope: any declaration or import in the file
  shadows a prelude name, so adding a name to the prelude never breaks a
  program;
* `std.io` keeps the full I/O API; `println` in the prelude is the same
  function as `io.println`.

If adopted before v0.1.0, the v0.1 entry-point example no longer needs
`import std.io`. Decision pending.

## 2. The v0.1 standard library (Normative v0.1)

v0.1 provides a single module.

### `std.io`

| Function | Behaviour |
|---|---|
| `print(value)` | Writes the textual representation of `value` to standard output. |
| `println(value)` | Like `print`, followed by a line feed (U+000A) on every platform. |

* `value` may have any v0.1 primitive type except `()`: `bool`, any integer
  type, `f32`, `f64`, `char` or `string`. These two functions are compiler
  intrinsics; this is a transitional design until interfaces allow a
  user-extensible formatting interface (such as `Display`). They are the only
  v0.1 functions that accept more than one argument type.
* Representations:
  * `bool`: `true` or `false`;
  * integers: decimal, with a leading `-` for negative values;
  * `char` and `string`: the UTF-8 encoding of the text, unmodified;
  * floating-point: the shortest decimal representation that converts back to
    the same value, using `inf`, `-inf` and `NaN` for special values, and
    always containing a `.` or exponent for finite values (`1.0`, not `1`).
    **Unresolved:** exact exponent formatting thresholds.
* Output is buffered; buffers are flushed when the program ends normally and
  before a trap message is written.
* A write error (for example a closed pipe) terminates the program with a
  non-zero exit status. **Unresolved:** whether this is a trap or a distinct
  error path.

## 3. Fundamental functions (Planned)

Decision: [ADR-0021](../docs/adr/0021-fundamental-library-functions.md).
Names follow lowerCamelCase. Functions marked *intrinsic* are implemented by
the compiler because they need information that ordinary functions do not
have (source locations, compile-time templates, type layout).

| Function | Signature (tentative) | Semantics | Exceptional behaviour | Target |
|---|---|---|---|---|
| `assert` | `assert(condition: bool)`, `assert(condition: bool, message: string)` (intrinsic) | Does nothing if `condition` is `true`. | Traps if `false`; the message includes the source location and the text of the condition. Enabled in every build mode. | v0.2 |
| `panic` | `panic(message: string) -> never` (intrinsic) | Stops the program: an explicit trap. | Always traps. | v0.2 |
| `exit` | `exit(code: i32) -> never` | Flushes the standard streams and terminates the process with `code`. | Values still alive are **not** destroyed, as with C `exit`. | v0.2 |
| `min`, `max` | v0.2: intrinsics over each primitive numeric type and `char`; v0.4: `min<T: Comparable>(a: T, b: T) -> T` | The smaller/larger argument; for equal integers or characters, `a`. | Floating-point follows IEEE 754-2019 `minimum`/`maximum`: NaN if either argument is NaN, and `-0.0` is less than `+0.0`. | v0.2 / v0.4 |
| `abs` | `abs(x: T) -> T` for signed integers and floats | Absolute value. | Signed integers: traps on the minimum value. Floats: clears the sign (`abs(-0.0) == 0.0`); NaN stays NaN. Unsigned operands are a compile-time error. | v0.2 |
| `len` | `len(x) -> int` | Number of elements; for `string`, the number of **bytes** of its UTF-8 encoding. | None. | v0.3 |
| `clone` | `clone<T: Clone>(x: T) -> T` | An independent copy, including owned heap data. | Allocation failure (policy undecided). | v0.4 |
| `swap` | `swap(a, b)` with both arguments mutably borrowed | Exchanges the two values. | Error if `a` and `b` are the same place (exclusivity). | v0.3 |
| `input` | `input(prompt: string) -> Result<string, InputError>` | Writes `prompt`, reads one line from standard input, removes the line terminator. | End of input and I/O failures are `Err` values, never traps. | v0.3 |
| `format` | `format(template, args…) -> string` (intrinsic) | Builds a string; the template must be a string literal checked at compile time. | A mismatch between placeholders and arguments is a compile-time error. Placeholder syntax follows C6. | v0.3 |
| `sizeOf`, `alignOf` | `sizeOf<T>() -> int`, `alignOf<T>() -> int` (intrinsic) | Size and alignment of `T` in bytes. Constant expressions. | None. Depends on the explicit type-argument syntax ([GENERICS.md §2.1](GENERICS.md#21-explicit-type-arguments-unresolved)). | v0.4 |

Not adopted (pending decision): `range()` (redundant with `0..n`;
recommendation: reject), `round()` (recommendation: a method, `x.round()`),
`typeof()` (runtime type inspection conflicts with the no-runtime-reflection
rule; recommendation: a compile-time `typeName<T>()`).

## 4. Planned modules (Proposal)

| Module | Contents |
|---|---|
| `std.io` | Readers, writers, buffered I/O, standard streams |
| `std.fs` | Files and directories |
| `std.env` | Arguments and environment variables |
| `std.process` | Exit, spawning processes |
| `std.string` | Owned strings, builders, Unicode utilities |
| `std.collections` | `List`, `Map`, `Set`, `Deque` |
| `std.math` | Elementary functions, constants, checked/wrapping/saturating arithmetic |
| `std.time` | Instants, durations, clocks |
| `std.task` | Structured concurrency ([CONCURRENCY_MODEL.md](CONCURRENCY_MODEL.md)) |
| `std.sync` | Mutexes, atomics, channels |
| `std.net` | TCP, UDP, DNS |
| `std.test` | Test assertions and runner support |

Numerical computing (Julia's domain) is served by `std.math` plus external
packages for arrays and linear algebra; a dedicated numeric array type may be
proposed once generics exist.

## 5. Design guidelines

1. Fallible operations return `Result`; they do not trap.
2. Allocation is visible: functions that allocate are documented, and
   allocator-parameterized variants exist for performance-critical code.
3. No global mutable state except the standard streams.
4. Every public item is documented with an example.
