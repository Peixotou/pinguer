# 0029. Parser error recovery

- Status: Accepted
- Date: 2026-11-26

## Context

Each stage reports as many independent errors as possible
([COMPILER_ARCHITECTURE.md §3.2](../architecture/COMPILER_ARCHITECTURE.md#32-error-recovery)),
and the language server needs a tree for code that is being edited. Recovery
that is too eager produces cascades of diagnostics about one mistake;
recovery that is too timid stops at the first error; and any recovery loop
risks never terminating on unexpected input.

## Decision

### Diagnostics

* One problem, one diagnostic. After reporting an error in a statement or
  item, the parser does not report further errors until it has
  resynchronized.
* Two parser diagnostics never blame the same token, with one exception:
  unclosed delimiters, each of which names a different opening delimiter.
  A second diagnostic at the token of the previous one is a consequence of
  it (for example a `while` without its block after a `)` missing before
  `}`) and is dropped.
* `Unknown` tokens were reported by the lexer
  ([LEXICAL_STRUCTURE.md §7](../../specifications/LEXICAL_STRUCTURE.md#7-error-recovery))
  and are skipped without a diagnostic.
* Invalid syntax is never accepted silently: every error node in the tree
  corresponds to a reported diagnostic.

### Synchronization

* **Statements.** When a statement cannot be parsed, the parser skips to the
  end of the statement: the next separator (`NEWLINE` or `;`) at the
  statement's own nesting level, the `}` that closes the enclosing block
  (left for the block to consume) or the end of the file. Skipping counts
  `(`, `[` and `{` and their closers, so a nested block or argument list is
  skipped as a whole. The statement becomes an error statement.
* **Items.** At the top level the parser skips to the next token that can
  start an item (`func`, `import`, `const`) at the start of a line, or to the
  end of the file, again skipping balanced delimiters, so that an error in
  one function does not hide the functions that follow.
* **Delimiters.** A missing closing delimiter is reported once, with a
  secondary label at the opening one. A closing delimiter without an opening
  one is reported and skipped.
* **Missing tokens.** When a token is missing but the construct can be
  completed (for example a `)` missing at the end of an argument list), the
  parser reports it and continues as if it had been there, instead of
  discarding what follows. This applies where only the delimiter can
  complete the construct: at a `}`, which is left for the block it closes,
  and at the end of the file, including directly after `(` or `,`.

### Progress

* Every loop of the parser consumes at least one token per iteration or
  exits. A function that reports an error without consuming anything returns
  to a caller that is at a synchronization point, and every synchronization
  either consumes a token or stops at a token that an enclosing construct
  consumes. The end-of-file token is never consumed and stops every loop.
* Robustness tests check termination and the absence of panics on random
  and truncated inputs.

## Alternatives considered

* **Stop at the first error:** simple, but useless for the language server
  and tedious for users.
* **Token insertion and deletion search** (as in some generated parsers):
  better guesses in some cases, but harder to predict and to bound.

## Consequences

* Later phases receive trees with error nodes and must treat them as
  already diagnosed.
* The number of parser diagnostics is bounded by the number of statements
  and items plus the number of delimiter errors, and the existing diagnostic
  limit of the CLI applies.
