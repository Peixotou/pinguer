# 0030. Parser resource limits

- Status: Accepted
- Date: 2026-11-26

## Context

Source code is untrusted input
([SECURITY_MODEL.md §2](../../specifications/SECURITY_MODEL.md#2-compiler-robustness-normative-v01-requirements-for-the-implementation)).
A recursive descent parser recurses once per nesting level, and so does every
later pass over the tree and the destruction of the tree itself. Without a
bound, ten thousand nested parentheses, which the existing lexer robustness
corpus already contains, overflow the stack. The main thread of a Windows
process has 1 MiB of stack; secondary threads elsewhere often have less.

## Decision

* **Nesting limit: 256.** The parser counts the constructs it is currently
  inside whose parsing recurses: parenthesized expressions, argument lists,
  prefix operators and blocks (including the blocks of `if`, `while` and
  functions). Opening the 257th is an error (a dedicated `P2xxx` code). The
  parser then skips the offending construct, counting delimiters
  iteratively without recursion, replaces it with an error node and
  continues. 256 matches Clang's default limit on nested brackets; real
  programs stay far below it.
* **Stack guarantee.** Parsing a construct nested at the limit, rendering the
  resulting tree and dropping it must succeed on a thread with 1 MiB of stack
  in an unoptimized build. A test checks this.
* **Everything else is iterative.** Sequences of items, statements,
  arguments, member accesses and left-associative binary operators are
  parsed by loops and have no limit beyond the file size. `else if` chains
  are a list in the tree ([ADR-0028](0028-ast-representation.md)), not a
  nesting.
* **Node IDs.** IDs are `u32`. Allocation is checked; if a file ever needed
  more IDs than exist, the parser reports a resource-limit error and stops
  instead of reusing or wrapping IDs.
* **Diagnostics.** The parser reports into the caller's `Diagnostics`
  collection, so the CLI's limit of 100 reported diagnostics applies to
  lexer and parser diagnostics together.
* **Memory.** The tree is linear in the number of tokens; the parser keeps
  no other structure that grows with the input apart from the filtered token
  vector.

## Alternatives considered

* **No limit, larger stacks** (spawning the compiler on a thread with a large
  stack, or growing the stack on demand as rustc does with `stacker`): moves
  the failure point instead of removing it, and growing the stack needs a
  dependency with `unsafe` code.
* **A lower limit such as 64:** safer margins, but generated code and deeply
  nested arithmetic can legitimately exceed it.
* **Separate limits per construct:** more knobs without a clear benefit; one
  limit bounds the recursion depth of every pass.

## Consequences

* Later recursive passes (name resolution, type checking, lowering) inherit
  the bound on tree depth and need no limit of their own for v0.1.
* Programs nested deeper than 256 levels are rejected with a diagnostic that
  names the limit.
