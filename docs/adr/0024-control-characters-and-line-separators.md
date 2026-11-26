# 0024. Rejection of control characters and Unicode line separators

- Status: Accepted. Extends [ADR-0009](0009-ascii-identifiers-and-bidi-rejection.md).
- Date: 2026-11-25

## Context

Pinguer has exactly two line breaks: line feed and carriage return followed
by line feed ([LEXICAL_STRUCTURE.md §2](../../specifications/LEXICAL_STRUCTURE.md#2-line-breaks)).
Outside comments and literals every other character that is not a token is
already an error. Inside comments and literals, however, v0.1 accepted any
character except the bidirectional controls of ADR-0009 and a lone carriage
return.

Several characters are line breaks for other software but not for Pinguer:
vertical tab (U+000B), form feed (U+000C), next line (U+0085), line separator
(U+2028) and paragraph separator (U+2029). Python's `str.splitlines`, the
Unicode line breaking algorithm, JavaScript (for U+2028 and U+2029) and many
editors and terminals end a line at some or all of them. A line comment such
as

```text
// check the caller's permissions<U+2028>return grant(user)
```

is a single comment for the compiler, but a reviewer whose tool breaks the
line sees `return grant(user)` as live code. The same trick hides code inside
a string literal. This is the "Trojan Source" class of attack
(CVE-2021-42574) that ADR-0009 addresses for bidirectional controls.

The remaining control characters (C0 controls other than tab and the line
breaks, DEL and the C1 controls) carry a related risk: they are invisible in
most editors, and terminals interpret some of them, most importantly ESC
(U+001B), which starts escape sequences that move the cursor and erase text.
Displaying a source file with `cat` or in a diff can therefore show text that
differs from what the compiler reads. Rejecting only the five line-break
look-alikes would leave ESC and NUL allowed next to a rejected NEL, which is
itself a C1 control.

## Decision

* The following characters are rejected **anywhere** in a source file,
  including comments, string literals and character literals, with the new
  diagnostic `P1012`:
  * every control character (Unicode general category Cc, U+0000–U+001F and
    U+007F–U+009F) except horizontal tab (U+0009), line feed (U+000A) and
    carriage return (U+000D); this includes vertical tab, form feed and
    next line;
  * U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR.
* Carriage return keeps its own rule: it is allowed only before a line feed
  (`P1010`).
* The restriction applies to characters written directly in the file. Escape
  sequences such as `"\u{2028}"`, `"\u{1B}"` or `"\0"` remain valid, because
  an escape is visible text and produces the character only in the value of
  the literal.
* Outside comments and literals, such a character is reported once, with
  `P1012` rather than `P1001`, and produces one `Unknown` token.

## Alternatives considered

* **Reject only VT, FF, NEL, LS and PS.** Closes the line-break ambiguity but
  keeps ESC and the other invisible controls, which enable a similar attack
  through terminals; the rule would also be harder to state.
* **Treat these characters as line breaks.** Then the compiler would agree
  with some tools and disagree with others (a tool that does not break at
  U+2028 would show the opposite), and line numbers would depend on the
  tool. One unambiguous set of line breaks is simpler.
* **Warn instead of error.** As in ADR-0009, there is no lint configuration
  yet; an error is the safe default and can be relaxed later without
  breaking programs, while tightening later would break them.

## Consequences

* Valid v0.1 programs can no longer contain these characters directly. No
  meaningful program needs them: the escapes cover every legitimate use in
  literals.
* Form feed as a page separator in source files, a convention of some C code
  bases, is not supported.
* The rule is a precondition for a precise "one line in the compiler is one
  line on screen" guarantee together with the rejection of lone carriage
  returns.
* Zero-width and other invisible formatting characters that are not
  controls (for example U+200B) remain allowed inside comments and literals;
  the diagnostics renderer escapes them, and a lint is a proposal in
  [SECURITY_MODEL.md §3](../../specifications/SECURITY_MODEL.md#3-source-level-protections-normative-v01).
