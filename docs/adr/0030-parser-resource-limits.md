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

* **Depth limit: 256.** What must be bounded is the depth of the tree,
  because every recursive pass and the destruction of the tree recurse once
  per level of it, wherever the parser itself iterates. The parser counts
  the levels of the tree it is building: blocks (including those of `if`,
  `while` and functions), parenthesized expressions, prefix operators,
  calls, member accesses, `as` conversions and operator chains. Going past
  256 is an error (`P2019`); the parser skips the rest of the offending
  construct, counting delimiters without recursion, replaces it with an
  error node and continues. 256 matches Clang's default limit on nested
  brackets; real programs stay far below it.
* **Counting nodes that enclose earlier ones.** A call, member access,
  conversion or operator chain encloses the nodes built before it in the
  same chain: in `f(x)()()`, each later call adds a level above `x`. The
  parser therefore measures the deepest level reached in a chain and adds
  one to it for each such node, instead of counting only the constructs
  that enclose the current position. Counting the position alone would
  accept arguments nested 255 levels deep followed by 255 calls, at every
  level of nesting, and so trees about 256²/2 levels deep.
* **Operator chains are one level.** Binary operators of one precedence
  level form a single node with a list of operands
  ([ADR-0028](0028-ast-representation.md)), so `a + b + … + z` adds one
  level to the tree whatever its length, and `a + b * c - d` adds two.
  Long sums and long `&&`/`||` conditions therefore need no limit, while a
  binary tree would be as deep as the chain is long.
* **Stack guarantee.** Parsing a construct nested at the limit, rendering the
  resulting tree and dropping it must succeed on a thread with 1 MiB of stack
  in an unoptimized build. A test checks this.
* **Lists are unlimited.** Items, statements, parameters and arguments are
  lists in the tree and are parsed by loops; `else if` chains are a list as
  well ([ADR-0028](0028-ast-representation.md)). They have no limit beyond
  the file size.
* **Node IDs.** IDs are `u32`. Allocation is checked; if a file ever needed
  more IDs than exist, the parser reports a resource-limit error and stops
  instead of reusing or wrapping IDs.
* **Diagnostics.** The parser reports into the caller's `Diagnostics`
  collection, so the CLI's limit of 100 reported diagnostics applies to
  lexer and parser diagnostics together.
* **Memory.** The tree is linear in the number of tokens; the parser keeps
  no other structure that grows with the input apart from the filtered token
  vector.

Measurements on Windows with an unoptimized build and a 1 MiB thread,
before this limit existed: a left-deep tree of 2,000 binary operators could
be numbered and dropped but not rendered, 4,000 could not be dropped, and
fewer than 500 nested parentheses already exhausted the stack while
parsing.

An unoptimized build gives every temporary of a function its own stack
slot, so code that only runs on errors, or after the recursive call
returns, still enlarges every recursive frame. The recursive functions
therefore leave diagnostics, node construction and non-recursive cases to
helpers. Measured on the same thread by the smallest stack that parses,
renders and drops each construct, the stack used per level of nesting fell
from 7.0 KiB to 1.6 KiB for parentheses, from 3.2 KiB to 1.6 KiB for blocks
and from 4.5 KiB to 2.9 KiB for `if` statements; calls and prefix operators
use about 1 KiB. The deepest case, 256 nested `if` statements, needs about
770 KiB of the 1 MiB, and the stack guarantee test fails if a change
removes that margin.

## Alternatives considered

* **No limit, larger stacks** (spawning the compiler on a thread with a large
  stack, or growing the stack on demand as rustc does with `stacker`): moves
  the failure point instead of removing it, and growing the stack needs a
  dependency with `unsafe` code.
* **A lower limit such as 64:** safer margins, but generated code and deeply
  nested arithmetic can legitimately exceed it.
* **Separate limits per construct:** more knobs without a clear benefit; one
  limit bounds the recursion depth of every pass.
* **Counting chain length towards the limit, with binary nodes:** a sum of
  more than 256 terms, plausible in generated code, would be rejected.
* **Iterative destruction and traversal everywhere:** possible for the
  parser's own passes, but every later phase would have to avoid recursion
  over expressions.

## Consequences

* Later recursive passes (name resolution, type checking, lowering) inherit
  the bound on tree depth and need no limit of their own for v0.1.
* Programs nested deeper than 256 levels are rejected with a diagnostic that
  names the limit.
