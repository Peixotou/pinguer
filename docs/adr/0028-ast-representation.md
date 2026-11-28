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

* An owned tree: `Box` for single children, `Box<[T]>` for lists. Nesting
  depth is bounded by [ADR-0030](0030-parser-resource-limits.md), so
  recursive construction, traversal and destruction are bounded too.
* Lists are boxed slices because the tree is not modified after parsing: a
  `Vec` keeps the capacity it grew to, at least four elements, which the
  many one-operand chains and short argument lists leave mostly unused. A
  boxed slice costs one shrinking reallocation per list.
* Expressions are at most 64 bytes on 64-bit targets. An integer literal
  stores its value as two `u64` halves (`IntValue`), because a `u128` field
  would align every expression to 16 bytes, and a conversion boxes its
  target type, which is rare. A test checks the bound.
* Statements are at most 136 bytes. A binding boxes its type annotation,
  which local bindings usually omit, so a binding without one is smaller
  and one with it costs an allocation. Boxing the whole binding instead
  would cost an allocation for every binding.
* The benchmark (`cargo bench -p pinguer_cli --bench scalability`) prints
  the heap of the tree per node, counted by walking the finished tree: each
  `Box`, and each boxed slice or string that is not empty, is one
  allocation of exactly its contents' size. Allocator overhead and the
  temporary allocations made while parsing are not counted. On its 10 MB
  inputs that parse into trees, a long expression, many statements and many
  functions, the tree holds 62–75 bytes and 1.0 allocation per node on a
  64-bit target.
* Node kinds are enums over the v0.1 constructs only: items (`import`,
  function, constant), statements, expressions and types. Each kind exists
  because the grammar has it; nothing is added for planned features.

### Identity and location

* Every item, parameter, block, statement, expression and type has a
  `NodeId`, a `u32` that is unique within one file's tree. IDs are assigned
  after parsing, by one pre-order walk of the finished tree (a node before
  its children, children left to right). They are therefore dense: every
  ID below the tree's node count belongs to exactly one node, also after
  error recovery, because discarded nodes never reach the tree. The same
  input always produces the same IDs, so a side table can be a vector
  indexed by ID. Numbering after parsing also makes the IDs independent of
  the order in which the parser builds nodes: precedence climbing builds
  the left operand of a binary expression before the expression.
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
* Binary operators of one precedence level form one node: the first
  operand and a list of operator-operand pairs, applied from left to right,
  so `a - b + c` is one node meaning `(a - b) + c`. The tree's depth then
  does not grow with the length of a chain ([ADR-0030](0030-parser-resource-limits.md)).
  Each pair records the span of its operator, which type errors point
  at.
* A chain means that left fold and nothing else: operands are evaluated in
  order, `&&` and `||` short-circuit from the left, and no later phase may
  reassociate or reorder a chain, since checked overflow, `-`, `/`, `%`,
  shifts and floating-point arithmetic depend on the grouping. The
  intermediate results of a chain have no node ID. Every v0.1 binary
  operator takes two operands of the same type, and the operators whose
  result has another type, the comparisons, cannot be chained; so an
  intermediate result has the type of the operands (`bool` for `&&` and
  `||`). A side table that needs one per operator,
  for example for the location of a trap, keys it by the chain's ID and
  the operator's index, and reports at the operator's span.

### Literals

| Literal | Stored as | Why |
|---|---|---|
| Integer | the value as `Option<IntValue>`, up to 128 bits, and the base; `None` when the value needs more than 128 bits | The type checker chooses the type from the context and checks the range; 128 bits cover every planned integer type, and `None` keeps a too-large literal reportable instead of truncated. |
| Floating-point | the literal's digits with separators removed, for example `6.02e23` | Rounding is done once, at the expected type, by the standard library's correctly rounded and locale-independent parser; converting to `f64` first would round `f32` literals twice. |
| String | the decoded text | Escapes are resolved once. |
| Character | the decoded `char` | |
| Boolean | `bool` | |

`-128` is a prefix `-` applied to the literal `128`; the type checker applies
the negative-literal rule.

### Errors

* Items, statements and expressions have an `Error` kind, used where
  recovery replaced invalid input. Every error node has been reported, by
  the lexer or the parser; later phases skip error nodes without further
  diagnostics. Types have none: a type that cannot be parsed makes the
  enclosing declaration, statement or expression an error, so an error
  type would never be produced.
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
