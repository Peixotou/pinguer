# Lexical Structure

Status: **Normative (v0.1)** unless a section says otherwise.
Implementation: `compiler/lexer` (crate `pinguer_lexer`) implements this
document completely. The rules are covered by tests in
`compiler/lexer/tests/`, except that UTF-8 validation and byte order mark
removal are tested in `compiler/source` and `cli/tests`. The file size limit
(`P0003`) has no automated test, because triggering it requires a 4 GiB
input.

## 1. Source text

* A source file is a sequence of bytes that must be valid UTF-8. Invalid
  UTF-8 is rejected before lexing (diagnostic `P0002`).
* A leading UTF-8 byte order mark (U+FEFF) is ignored. Byte offsets reported
  by the compiler are relative to the text after the byte order mark.
* A source file may be at most 4 GiB − 1 byte long (`P0003`).
* Source files use the extension `.pgr`.

## 2. Line breaks

A line break is either a line feed (U+000A) or a carriage return followed by a
line feed (U+000D U+000A). A carriage return that is not immediately followed
by a line feed is an error anywhere in the file, including inside comments and
literals (`P1010`).

Lines and columns in diagnostics are 1-based. Columns count Unicode scalar
values.

## 3. Forbidden characters

The following characters are rejected anywhere in a source file, including
inside comments, string literals and character literals (`P1009`):

| Code points | Name |
|---|---|
| U+061C | Arabic letter mark |
| U+200E, U+200F | Left-to-right mark, right-to-left mark |
| U+202A – U+202E | Bidirectional embeddings, overrides and pop |
| U+2066 – U+2069 | Bidirectional isolates and pop |

These characters can make the displayed order of source code differ from the
order the compiler reads, which has been used to disguise malicious code
(CVE-2021-42574, "Trojan Source"). A program that needs one of them in a
string writes it as an escape sequence, for example `"\u{202E}"`.

## 4. Whitespace and comments

* Whitespace is space (U+0020), horizontal tab (U+0009) and line breaks. Any
  other character, including other Unicode whitespace such as U+00A0, is an
  error when it appears outside a comment or literal (`P1001`).
* A line comment starts with `//` and extends to the end of the line. `///`
  starts an ordinary line comment in v0.1; documentation comments are a
  **Proposal**.
* A block comment starts with `/*` and ends with the matching `*/`. Block
  comments nest: `/* a /* b */ c */` is one comment. An unterminated block
  comment is an error reported at its opening `/*` (`P1003`).

Whitespace and comments separate tokens and are otherwise ignored, except that
line breaks may produce `NEWLINE` tokens (section 6).

## 5. Tokens

The lexer produces tokens of the following kinds, always followed by a single
end-of-file token.

### 5.1 Identifiers

```ebnf
identifier = ( letter | "_" ) , { letter | digit | "_" } ;
letter     = "A" … "Z" | "a" … "z" ;
digit      = "0" … "9" ;
```

* Identifiers are case-sensitive.
* A lone `_` is not an identifier; it is the *underscore* token, reserved for
  wildcard patterns and discarded bindings.
* Identifiers containing non-ASCII letters are an error (`P1011`). The lexer
  still reads the whole word as one identifier token, so a single diagnostic
  is reported.
* Names of built-in types such as `int`, `string` and `bool` are identifiers,
  not keywords; they are resolved by name resolution.

**Proposal:** Unicode identifiers following Unicode Standard Annex #31, with
NFC normalization and confusable detection based on Unicode Technical
Standard #39. They are excluded from v0.1 because normalization and
confusable handling must be specified together to avoid spoofing.

### 5.2 Keywords

Keywords cannot be used as identifiers.

Keywords with meaning in v0.1:

```text
as  break  const  continue  else  false  func  if  import  let  return  true  var  while
```

Keywords reserved for future use. They are recognised by the lexer so that
introducing them later does not break existing programs; the parser reports
a dedicated error when it meets one:

```text
actor  async  await  class  comptime  defer  enum  extern  for  impl  in
interface  loop  match  move  private  public  self  Self  spawn  static
struct  try  type  unsafe  where  yield
```

`public`, `private` ([ADR-0012](../docs/adr/0012-visibility-public-and-private.md)),
`try` ([ADR-0013](../docs/adr/0013-prefix-try.md)) and `class`
([ADR-0011](../docs/adr/0011-composition-and-interfaces-without-class-inheritance.md))
were added on 2026-11-25; `pub` was removed and is an ordinary identifier.
Words such as `extends`, `override`, `protected`, `module`, `export`, `none`,
`void` and `float` are **not** keywords.

### 5.2.1 Contextual keywords

Some words have a special meaning only in a specific syntactic position and
are ordinary identifiers everywhere else. The lexer always produces `Ident`
tokens for them; the parser recognises them by position.

| Words | Special only | Status |
|---|---|---|
| `query` | directly before `{` where an expression is expected | Experimental ([ADR-0015](../docs/adr/0015-contextual-query-keywords.md)) |
| `from`, `order`, `select` | at the start of a clause inside a query | Experimental |
| `by` | directly after `order` in a query | Experimental |
| `ascending`, `descending` | after an `order by` key | Experimental |

Contextual keywords exist so that rarely used syntax does not take common
names (`order`, `select`) away from every program.

### 5.3 Integer literals

```ebnf
integer_literal = decimal_literal | hex_literal | octal_literal | binary_literal ;
decimal_literal = "0" | nonzero_digit , { [ "_" ] , digit } ;
hex_literal     = "0x" , hex_digit , { [ "_" ] , hex_digit } ;
octal_literal   = "0o" , octal_digit , { [ "_" ] , octal_digit } ;
binary_literal  = "0b" , binary_digit , { [ "_" ] , binary_digit } ;
```

* Prefixes are lowercase. Hexadecimal digits may be upper- or lowercase.
* `_` is a digit separator and may only appear between two digits of the
  literal's base (`P1006`).
* A decimal literal with more than one digit cannot start with `0`
  (`P1007`). This avoids the C convention in which `017` is octal.
* Literal suffixes such as `10u8` are not supported (`P1006`); the type of a
  literal comes from its context ([TYPE_SYSTEM.md §5](TYPE_SYSTEM.md#5-type-inference)).
* A minus sign is never part of a literal: `-1` is the unary `-` operator
  applied to `1`.
* The lexer does not check that the value fits a type; the type checker does.

### 5.4 Floating-point literals

```ebnf
float_literal = decimal_part , "." , digit_run , [ exponent ]
              | decimal_part , exponent ;
exponent      = ( "e" | "E" ) , [ "+" | "-" ] , digit_run ;
digit_run     = digit , { [ "_" ] , digit } ;
```

where `decimal_part` follows the rules of `decimal_literal`.

* Digits are required on both sides of the `.`: `1.` and `.5` are not
  floating-point literals. Consequently `1.foo` is a member access on `1`
  and `1..2` is `1`, `..`, `2`.
* An exponent marker must be followed by at least one digit (`P1006`).
* There are no hexadecimal floating-point literals.

### 5.5 String literals

```ebnf
string_literal = '"' , { string_char | escape } , '"' ;
escape         = "\" , ( "n" | "r" | "t" | "\" | "0" | '"' | "'" )
               | "\u{" , hex_digit , [ hex_digit ] , [ hex_digit ] ,
                 [ hex_digit ] , [ hex_digit ] , [ hex_digit ] , "}" ;
```

`string_char` is any Unicode scalar value except `"`, `\` and line breaks.

| Escape | Meaning |
|---|---|
| `\n` | line feed U+000A |
| `\r` | carriage return U+000D |
| `\t` | horizontal tab U+0009 |
| `\\` | backslash |
| `\0` | null U+0000 |
| `\"` | quotation mark |
| `\'` | apostrophe |
| `\u{X}` | the Unicode scalar value X, one to six hexadecimal digits |

* A string literal cannot contain a raw line break. A string that is not
  closed before the end of the line is an error (`P1002`); the lexer resumes
  at the next line.
* Any other escape, an escape with no or more than six digits, a missing
  brace, and a value that is a surrogate (U+D800–U+DFFF) or above U+10FFFF
  are errors (`P1005`).

**Proposal:** multi-line strings and raw strings.

**Unresolved (must be decided before v0.1.0):** string interpolation syntax,
`"Hello, ${name}"` (recommended) or `"Hello, {name}"`. Whichever is chosen
changes the meaning of characters that are ordinary text in v0.1 string
literals, so the chosen sequence must be rejected (or required to be escaped)
in v0.1 literals before v0.1.0 is released; otherwise adding interpolation
later would silently change the meaning of existing programs.

### 5.6 Character literals

```ebnf
char_literal = "'" , ( char_char | escape ) , "'" ;
```

`char_char` is any Unicode scalar value except `'`, `\` and line breaks.

* A character literal contains exactly one Unicode scalar value (`P1008`).
  A user-perceived character made of several scalar values, such as `e`
  followed by U+0301 COMBINING ACUTE ACCENT, is not a valid character
  literal.
* An unterminated character literal is an error (`P1004`).

**Proposal:** byte literals (`b'a'`) and byte string literals.

### 5.7 Operators and punctuation

```text
(  )  {  }  [  ]  ,  .  ..  ..=  :  ;  ->  =>  ?  ?.  ??  @
+  -  *  /  %  **  &  |  ^  ~  !  <<  >>  &&  ||  |>
=  ==  !=  <  <=  >  >=
+=  -=  *=  /=  %=  **=  &=  |=  ^=  <<=  >>=
```

Operators are read greedily: the longest sequence that forms a token is used.
`>>` is therefore a single token; the parser splits `>>`, `>=` and `>>=`
where a single `>` closes a generic argument list (see
[GENERICS.md](GENERICS.md)), and `??` in a type such as `T??`. `a * *b` is
two `*` tokens, while `a**b` contains the power token `**`.

These tokens have no meaning in v0.1 and are reserved for planned features
(see [OPERATORS.md](OPERATORS.md)): `..` and `..=` (ranges), `=>` (match arms),
`?` (optional types `T?`), `?.` and `??` (optional operators), `**` and `**=`
(power), `|>` (pipeline), `@` (attributes), and `[`/`]` (indexing).

Any other character is an error (`P1001`).

## 6. Line breaks

Pinguer statements are terminated by line breaks (see
[LANGUAGE_SPECIFICATION.md §4](LANGUAGE_SPECIFICATION.md#4-statements-and-termination-normative-v01)).
The lexer emits a `NEWLINE` token for a line break when **all** of the
following hold:

1. The innermost open delimiter is `{`, or no delimiter is open. Line breaks
   inside `( … )` and `[ … ]` are insignificant.
2. A token other than `NEWLINE` has already been produced. No `NEWLINE` token
   is produced at the start of a file.
3. No `NEWLINE` token has been produced since the last other token. A run of
   line breaks, together with the whitespace and comments between them,
   produces a single `NEWLINE` token.

A block comment that contains a line break counts as a line break.

Delimiter tracking for this rule is tolerant of mismatches: a closing
delimiter closes the innermost matching open delimiter and discards any
unclosed delimiters nested inside it; a closing delimiter without a matching
open delimiter is ignored. Mismatched delimiters are reported by the parser.

The parser ignores `NEWLINE` tokens in the positions listed in
[GRAMMAR.ebnf](GRAMMAR.ebnf).

## 7. Error recovery

For every lexical error, the lexer reports a diagnostic and still produces
tokens covering the offending text, so that parsing can continue:

| Error | Recovery |
|---|---|
| Unknown character | One `Unknown` token per character. |
| Malformed numeric literal | One literal token covering the whole malformed literal, including any suffix. |
| Unterminated string or character literal | A literal token up to the end of the line. |
| Invalid escape | The literal continues after the escape. |
| Unterminated block comment | The comment extends to the end of the file. |
| Non-ASCII identifier | One identifier token. |

Diagnostics are reported in source order. When the compilation limits the
number of reported diagnostics, the lexer reports the ones that come first in
the source and counts the rest; recovery and the token stream are the same
with or without a limit.

## 8. Diagnostic codes

| Code | Meaning |
|---|---|
| P0002 | Source file is not valid UTF-8 |
| P0003 | Source file is too large |
| P1001 | Unknown character |
| P1002 | Unterminated string literal |
| P1003 | Unterminated block comment |
| P1004 | Unterminated character literal |
| P1005 | Invalid escape sequence |
| P1006 | Malformed numeric literal |
| P1007 | Decimal literal with a leading zero |
| P1008 | Character literal without exactly one character |
| P1009 | Bidirectional formatting character |
| P1010 | Lone carriage return |
| P1011 | Non-ASCII identifier |
