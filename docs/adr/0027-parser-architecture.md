# 0027. Parser architecture

- Status: Accepted
- Date: 2026-11-26

## Context

Phase 02 turns the lexer's token stream into an abstract syntax tree for the
v0.1 grammar ([GRAMMAR.ebnf](../../specifications/GRAMMAR.ebnf)). The grammar
is small, mostly LL(1), and has two features that shape the parser:

* statements end at line breaks, with token-based rules for where a `NEWLINE`
  is ignored ([ADR-0005](0005-newline-terminated-statements.md),
  [ADR-0014](0014-leading-dot-continuation.md));
* the expression grammar has twelve precedence levels, written in
  GRAMMAR.ebnf as one rule per level, with non-associative comparisons and
  `as` taking a type as its right operand.

The parser will be reused by the language server and the formatter, so it
must produce a tree for incomplete code and precise diagnostics, and the
data model must be usable by later phases without the parser.

## Decision

* **Two new crates.** `compiler/ast` (`pinguer_ast`) holds the syntax tree
  and depends only on `pinguer_source`. `compiler/parser` (`pinguer_parser`)
  depends on source, diagnostics, lexer and ast. Name resolution, type
  checking, lowering and tools consume `pinguer_ast` without depending on
  the parser's internals or on token types.
* **Hand-written recursive descent** for items, statements, blocks and
  types: one function per production of GRAMMAR.ebnf, with at most two
  tokens of lookahead.
* **Precedence climbing for binary expressions.** One loop handles all binary
  operators using the levels of
  [OPERATORS.md §1](../../specifications/OPERATORS.md#1-precedence-and-associativity);
  left-associative chains are iterative, so `a + b + … + z` does not recurse.
  Comparisons are parsed once per level and a second comparison operator is
  reported, which implements their non-associativity. `as` binds at its own
  level and takes a type, not an expression. Prefix operators recurse;
  calls and member accesses are parsed by a postfix loop.
* **Newline filtering before parsing.** A linear pass over the tokens removes
  every `NEWLINE` that the grammar ignores: one that directly follows a
  binary operator, an assignment operator, `as`, `->` or `,`, and one that
  is directly followed by `.`. The rule is token-based, exactly as written in
  GRAMMAR.ebnf, so it also applies after a prefix `-`. The remaining
  `NEWLINE` tokens are separators, and the grammar's layout rules (`{` and
  `else` on the same line as what precedes them) follow from it.
* **Input and output.** `parse` takes the file's text and its complete token
  vector and returns the tree; diagnostics go to the same `Diagnostics`
  collection as the lexer's. Tokens are read in place; only identifier names
  and decoded literal values are copied into the tree.
* **Literal values are decoded by the parser** (escapes, digits, separators),
  as the token documentation announces; their representation is part of
  [ADR-0028](0028-ast-representation.md).
* **Syntax only.** The parser enforces the grammar and nothing else.
  Restrictions the specification assigns to later phases stay there, for
  example that an expression statement must be a call, that member access
  applies only to modules, that `break` is inside a loop, or that a `const`
  initializer is constant.
* **No dependencies.** The parser uses only the standard library.

## Alternatives considered

| Alternative | Why not |
|---|---|
| Parser generator (LALR, PEG) | Adds a build-time dependency, gives less control over error recovery and messages, and handles the token-based newline rules awkwardly. |
| Parser combinator library | A runtime dependency; recovery and performance depend on the library. |
| One function per precedence level, as GRAMMAR.ebnf is written | Correct, but every primary expression would pass through twelve nested calls, which costs stack depth and time. Precedence climbing parses the same language. |
| Handling ignored newlines inside each parse function | Spreads one rule over the whole parser; the pre-pass implements it once and is testable on its own. |
| Lossless syntax tree as the parser's output (rowan style) | See ADR-0028. |

## Consequences

* The precedence table must agree with OPERATORS.md; a test checks every pair
  of adjacent levels.
* Adding the planned operators (`**`, `??`, `|>`, `?.`) means adding rows to
  the precedence table and tokens to the newline filter, without changing the
  structure of the parser.
* Later phases may report errors for constructs the parser accepts, such as
  `x` alone as a statement, which is syntactically an expression statement.
