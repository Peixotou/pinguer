# 0028. AST representation

- Status: Accepted
- Date: 2026-11-26
- Refines: [ADR-0004](0004-intermediate-representations.md) (AST with node IDs and side tables)

## Context

ADR-0004 makes the AST the input of name resolution and type checking, which
annotate it through side tables keyed by node IDs instead of rewriting it.
The tree must therefore have stable identifiers, precise spans, a shape that
later phases can traverse simply, and room for the error recovery of
[ADR-0029](0029-parser-error-recovery.md). The formatter and the language
server will reuse it ([COMPILER_ARCHITECTURE.md §3.6](../architecture/COMPILER_ARCHITECTURE.md#36-shared-with-tooling)).

## Decision

### Storage

* An owned tree: `Box` for single children, `Vec` for lists. Nesting depth
  is bounded by [ADR-0030](0030-parser-resource-limits.md), so recursive
  construction, traversal and destruction are bounded too.
* Node kinds are enums over the v0.1 constructs only: items (`import`,
  function, constant), statements, expressions and types. Each kind exists
  because the grammar has it; nothing is added for planned features.

### Identity and location

* Every item, parameter, block, statement, expression and type has a
  `NodeId`, a `u32` that is unique within one file's tree. IDs are dense and
  assigned in pre-order (a node before its children, children left to
  right), so the same input always produces the same IDs and a side table
  can be a vector indexed by ID. The tree records how many IDs it uses.
* Every node has the `Span` of the tokens it was parsed from. A name is an
  `Ident`: the text and its span, without an ID of its own; the node that
  contains it is what later phases annotate.
* Names are copied into the tree as `Box<str>`, so the tree does not borrow
  the source text. Interning is left for when profiling shows a need.

### Shape

* Parentheses are kept as a node. They have no meaning of their own, but the
  negative-literal rule ([TYPE_SYSTEM.md §5.1](../../specifications/TYPE_SYSTEM.md#51-literals))
  and diagnostics need to know whether `-` applies directly to a literal.
* An `if` with `else if` branches is one node with a list of
  condition-block branches and an optional final `else` block. The grammar
  nests them, but a list keeps the depth of the tree independent of the
  length of the chain.
* Bindings (`let`, `var`, `const`) share one structure with a kind; the
  top-level `const` item uses the same structure.
* A binary expression records the span of its operator, which type errors
  point at.

### Literals

| Literal | Stored as | Why |
|---|---|---|
| Integer | the value as `Option<u128>` and the base; `None` when the value needs more than 128 bits | The type checker chooses the type from the context and checks the range; 128 bits cover every planned integer type, and `None` keeps a too-large literal reportable instead of truncated. |
| Floating-point | the literal's digits with separators removed, for example `6.02e23` | Rounding is done once, at the expected type, by the standard library's correctly rounded and locale-independent parser; converting to `f64` first would round `f32` literals twice. |
| String | the decoded text | Escapes are resolved once. |
| Character | the decoded `char` | |
| Boolean | `bool` | |

`-128` is a prefix `-` applied to the literal `128`; the type checker applies
the negative-literal rule.

### Errors

* Each node category has an `Error` kind, used where recovery replaced
  invalid input. Every error node has been reported, by the lexer or the
  parser; later phases skip error nodes without further diagnostics.
* A literal that the lexer reported as malformed (an invalid escape, digit or
  suffix) becomes an error expression, without a second diagnostic.

### What the tree does not contain

* No types, resolved names or other semantic information.
* No comments or whitespace. Whether the formatter gets a lossless syntax
  tree or recovers comments from the source between token spans is decided
  with the formatter (COMPILER_ARCHITECTURE.md §3.6).
* No support for incremental reparsing: IDs are dense per file and change
  when the file changes.

### Rendering

`pinguer_ast` renders a tree as indented, deterministic text with IDs, spans
and literal values, for `pinguer debug ast` and for tests. The format is an
internal interface and may change.

## Alternatives considered

* **Arena with typed indices** (nodes in vectors, children as indices): better
  locality, cheap destruction, no `Box` per node; but every access needs the
  arena and the code is harder to read. Can be adopted behind the same node
  IDs if profiling shows a need.
* **Lossless concrete syntax tree** (rowan, as in rust-analyzer): ideal for the
  formatter and incremental reparsing, but it is a different data model that
  every later phase would traverse through typed views; v0.1 needs neither
  feature yet.
* **Integers as `i64` and floats as `f64`:** loses the information needed for
  `u64` values above `i64::MAX` and double-rounds `f32`.
* **Spans only, no copied names:** saves allocations but makes every consumer
  depend on the source text.

## Consequences

* Semantic side tables can be `Vec<Option<T>>` of length `node_count`.
* The negative-literal and literal-range checks belong to the type checker.
* Adding a construct means adding a kind and its rendering; IDs and spans
  need no change.
