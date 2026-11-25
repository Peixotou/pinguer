# 0012. Visibility with `public` and `private`

- Status: Accepted (feature status: Planned, with the module system)
- Date: 2026-11-25

## Context

The specification used `pub` (Rust) with private-by-default. A proposal under
review suggested C#'s five modifiers (`public`, `private`, `protected`,
`internal`, `readonly`).

## Decision

* Declarations are **private to their module by default**.
* `public` makes a declaration visible to every module and package that can
  import it. It replaces `pub`, which is no longer a keyword.
* `private` may be written explicitly; it has the same meaning as the
  default and produces no warning.
* `protected` is not added: it only has meaning with inheritance, which
  ADR-0011 rejects.
* A package-internal level (`internal`) and read-only fields (`readonly`) are
  **Unresolved** and not reserved.
* Visibility applies to top-level items, struct fields, methods and interface
  declarations. Enum variants and interface members share the visibility of
  their enclosing declaration.
* A `public` declaration may not expose a private type in its signature
  (error), so that importers can always name the types they receive.

## Alternatives considered

* **`pub`:** shorter, but less readable for newcomers; rejected in favour of
  the full word.
* **Public by default (Go uses capitalization, Python uses conventions):**
  makes accidental API surface the default; rejected.

## Consequences

* Lexer: `public` and `private` are reserved keywords; `pub` is an ordinary
  identifier (implemented, with tests).
* v0.1 programs are single files without modules, so visibility modifiers
  have no meaning there; the parser reports them as reserved for future use.
* Name resolution will check visibility at every path access, and type
  checking will check signatures of public items for private types.
