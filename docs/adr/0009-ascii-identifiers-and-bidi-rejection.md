# 0009. ASCII identifiers and rejection of bidirectional controls

- Status: Accepted
- Date: 2026-11-24

## Context

Unicode in source code enables two classes of attack:

* **Trojan Source** (CVE-2021-42574): bidirectional control characters make
  code display in a different order from how it is compiled, hiding logic
  from reviewers.
* **Homoglyphs**: identifiers that look identical but differ (Latin `a`,
  Cyrillic `а`), allowing a malicious contribution to shadow or impersonate a
  name.

Supporting Unicode identifiers safely requires normalization (NFC),
identifier rules (UAX #31) and confusable detection (UTS #39), plus Unicode
data tables that the compiler would have to carry or depend on.

## Decision

* Bidirectional formatting characters (U+061C, U+200E, U+200F,
  U+202A–U+202E, U+2066–U+2069) are rejected **everywhere** in source,
  including comments and literals. Literals can still contain them through
  `\u{…}` escapes.
* v0.1 identifiers are ASCII only. Non-ASCII text remains allowed in comments
  and literals.
* The diagnostics renderer escapes control and bidirectional characters when
  displaying source lines.

## Alternatives considered

* **Warn instead of error** (rustc's approach is a deny-by-default lint).
  Pinguer has no lint configuration yet; an error is the safe default and can
  be relaxed later without breaking programs.
* **Unicode identifiers in v0.1.** Deferred until confusable detection is
  specified and implemented together with them.

## Consequences

* Programs written in non-English languages cannot use native-script
  identifiers in v0.1. This is a known limitation, listed as a proposal in
  [LEXICAL_STRUCTURE.md](../../specifications/LEXICAL_STRUCTURE.md).
* Relaxing these rules later is backward compatible; tightening would not be.
