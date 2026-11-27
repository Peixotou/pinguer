# Diagnostics

Implementation: `compiler/diagnostics` (crate `pinguer_diagnostics`).
Decision record: [ADR-0008](../adr/0008-diagnostic-codes-and-formats.md).

## 1. Data model

```rust
pub struct Diagnostic {
    pub severity: Severity,        // Error | Warning | Note
    pub code: Option<Code>,        // e.g. P1002
    pub message: String,           // one-line summary
    pub labels: Vec<Label>,        // spans with Primary or Secondary style
    pub notes: Vec<String>,        // "= note: ..."
    pub help: Vec<String>,         // "= help: ..."
}
```

Stages push diagnostics into a `Diagnostics` collection and continue. The
first primary label determines the location in the header.

A collection may have a limit (`Diagnostics::with_limit`). It then retains
only the first diagnostics up to the limit; later ones are counted, so error
and warning counts stay exact, but not stored. When anything was suppressed,
`Diagnostics::suppression_notice` returns a `P0004` note giving the number of
suppressed diagnostics, which renderers print after the retained ones.

## 2. Writing good diagnostics

* **Message:** lowercase, no trailing period, states the problem
  (`unterminated string literal`), not the fix. Messages are the compiler's
  own English text. Text produced by the operating system, which may be
  localized, is never part of a message: it is quoted unchanged, with its
  error code, in a note (`the operating system reported: ... (os error 5)`).
* **Primary label:** points at the exact offending text and says what is wrong
  *there* (`missing closing "`).
* **Secondary labels:** point at related code that explains the problem
  (for example, the declaration a type comes from).
* **Notes:** background or rules (`string literals cannot contain line
  breaks`).
* **Help:** an action the user can take. A help message may claim that a
  change is correct only when the compiler knows it is; otherwise phrase it as
  a possibility.
* Every diagnostic about the program being compiled (its files and source
  text) has a code and a test. `P0003` (file too large) is tested through
  the source loader with a reduced limit; an end-to-end test would need a
  4 GiB input. Command-line usage errors
  and "command not available" messages concern the invocation, not the
  program, and have no code.

## 3. Human format

```text
error[P1002]: unterminated string literal
 --> lexical_errors.pgr:2:19
  |
2 |     let message = "unterminated
  |                   ^^^^^^^^^^^^^ missing closing `"`
  |
  = note: string literals cannot contain line breaks; use `\n` instead
```

Rules implemented by the renderer:

* Lines and columns are 1-based; columns count Unicode scalar values.
* Labels on the same line are shown in column order, one marker row each.
* Labels in other files get their own ` ::: file:line:col` section.
* A label spanning several lines is underlined to the end of its first line.
* An excerpt shows at most 120 cells of a source line. A longer line is cut
  around its labels, keeping up to 24 cells of context before the first one,
  and each cut is marked with `...`; labels too far apart to share one
  excerpt get one excerpt each. This bounds the size of every diagnostic
  independently of line length: without it, many diagnostics on one long
  line would each repeat the whole line.
* Tabs are expanded to four spaces. In source lines, messages and file names,
  the following characters are shown as `<U+XXXX>`, so that diagnostics
  cannot reorder or hide text in a terminal:
  * every control character (C0, DEL and C1, including U+0085);
  * Unicode's default-ignorable code points and the other invisible
    formatting characters: U+00AD, U+034F, U+061C, U+115F–U+1160,
    U+17B4–U+17B5, U+180B–U+180F, U+200B–U+200F, U+2028–U+202E (line and
    paragraph separators, bidirectional embeddings and overrides),
    U+2060–U+206F, U+3164, U+FE00–U+FE0F (variation selectors), U+FEFF,
    U+FFA0, U+FFF0–U+FFFB (including the interlinear annotation
    characters), U+1BCA0–U+1BCA3, U+1D173–U+1D17A and U+E0000–U+E0FFF
    (tags and supplementary variation selectors).

  The list lives in `pinguer_diagnostics::human` (`INVISIBLE`), whose tests
  check every range. Spaces such as U+00A0 are visible as spaces and are
  shown unchanged.
* Output is never colored (yet).

Known limitation: wide characters (CJK, most emoji) are assumed to occupy one
terminal cell, so markers after them can be misaligned.

## 4. JSON format

Selected with `--diagnostic-format json`. Each diagnostic is one JSON object
on its own line, written to standard error.

```json
{"format_version":1,"severity":"error","code":"P1002","message":"unterminated string literal","labels":[{"file":"main.pgr","byte_start":14,"byte_end":20,"line_start":1,"column_start":15,"line_end":1,"column_end":21,"style":"primary","message":"missing closing `\"`"}],"notes":["string literals cannot contain line breaks; use `\\n` instead"],"help":[]}
```

| Field | Type | Meaning |
|---|---|---|
| `format_version` | integer | Schema version, currently `1`. |
| `severity` | `"error"` \| `"warning"` \| `"note"` | |
| `code` | string or `null` | Diagnostic code. |
| `message` | string | Summary. |
| `labels[].file` | string | File name as given to the compiler. |
| `labels[].byte_start`, `byte_end` | integer | Byte offsets (end exclusive). |
| `labels[].line_start`, `column_start` | integer | 1-based start position. |
| `labels[].line_end`, `column_end` | integer | 1-based position just past the end. |
| `labels[].style` | `"primary"` \| `"secondary"` | |
| `labels[].message` | string | Label text, possibly empty. |
| `notes`, `help` | array of strings | |

Within `format_version` 1, fields may be added but never removed or changed.
Strings escape the same characters as the human format, as `\uXXXX`;
characters outside the Basic Multilingual Plane are escaped as a UTF-16
surrogate pair (`\udb40\udc41` for U+E0041), as RFC 8259 requires.

## 5. Code registry

| Range | Stage | Allocated codes |
|---|---|---|
| P0001–P0999 | Driver, files | P0001–P0004 |
| P1001–P1999 | Lexical analysis | P1001–P1012 |
| P2001–P2999 | Syntax | P2001–P2013 |
| P3001–P3999 | Name resolution | — |
| P4001–P4999 | Types | — |
| P5001–P5999 | Memory and ownership | — |
| P6001–P6999 | Code generation, linking | — |
| P9001–P9999 | Internal compiler errors | — |

The authoritative list, with summaries, is `pinguer_diagnostics::codes::REGISTRY`.

## 6. Planned

* `pinguer explain P1002` printing a long-form explanation for each code.
* Warning levels and lint configuration (`allow`, `warn`, `deny`).
* Colored output when the terminal supports it.
* Deduplication of cascading errors.
