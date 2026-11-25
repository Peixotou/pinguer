# 0020. Compiler attributes

- Status: Accepted (feature status: Planned)
- Date: 2026-11-25

## Decision

Attributes are written `@name` or `@name(arguments)` on the lines before a
declaration. The set is **closed**: only attributes defined by the language
are accepted; unknown attributes are errors. The approved set, specified in
[METAPROGRAMMING.md §3](../../specifications/METAPROGRAMMING.md#3-attributes-planned):

`@test`, `@bench`, `@inline`, `@deprecated`, `@derive`, `@target`, `@packed`,
`@align`, `@export`, `@entry`.

## Rationale

A closed set keeps the meaning of every attribute specified, checked and
documented, and avoids a macro system before one is designed.

## Consequences

* Lexer: none (`@` is already a token).
* Parser: attributes attach to the following item; a `NEWLINE` after an
  attribute is ignored.
* Each attribute has its own checks (target of application, argument types).
* `@target` removes items before name resolution, so excluded code must still
  parse but is not type-checked; tooling (language server) must handle both
  configurations.
* `@packed` creates fields that may be unaligned; taking a reference to such
  a field must be forbidden or copied, a known soundness pitfall in other
  languages.
* `@export` and `@entry` interact with the ABI and the entry-point rule; see
  the specification.
