# Command-Line Interface

Implementation: `cli/` (crate `pinguer_cli`, binary `pinguer`).
Decision record: [ADR-0010](../adr/0010-cli-exposes-only-implemented-commands.md).

## 1. Available in 0.1.0-dev

| Command | Description |
|---|---|
| `pinguer help`, `pinguer --help`, `pinguer -h`, `pinguer` | Print usage. |
| `pinguer version`, `pinguer --version`, `pinguer -V` | Print `pinguer 0.1.0-dev`. |
| `pinguer debug tokens <file> [--diagnostic-format human\|json]` | Print the token stream of a file, one token per line, followed by any diagnostics. For compiler development; the output format is **not stable**. |
| `pinguer debug ast <file> [--diagnostic-format human\|json]` | Print the syntax tree of a file, one node per line, followed by any diagnostics. For compiler development; the output format is **not stable**. |

`debug tokens` output, one line per token, tab-separated:

```text
1:1	Keyword(Import)	"import"
1:8	Ident	"std"
```

The columns are the start position (`line:column`), the token kind and the
token text as a quoted, escaped string.

`debug ast` output, one line per node, children indented by two spaces:

```text
File @1:1-6:1 nodes=8
  Import #0 @1:1-1:14 std.io
  Func #1 @3:1-5:2 main
    Block #2 @3:13-5:2
```

Each line holds the node kind, its ID, its span as
`@line:column-line:column` (the end just past the node) and its data. String
and character values are quoted and escaped. Some lines are parts of a
node rather than nodes, and have no ID: `Op` lines, the operators of a
chain, between its operands, and the `Branch` and `Else` lines of an `if`
statement. Error nodes are `ErrorItem`, `ErrorStmt` and `ErrorExpr`
(`ErrorType` is reserved; see ADR-0028). The tree is printed
even when the file has errors, so that error recovery can be inspected, and
the exit status is then 1.

## 2. Planned commands

Each of these is recognised today and exits with status 3 and an explanation.

| Command | Purpose | Planned for |
|---|---|---|
| `check <file>` | Report all errors without generating code | Phase 03 |
| `build <file>` | Compile to a native executable | Phase 04 |
| `run <file>` | Build and run | Phase 04 |
| `new <name>`, `init` | Create a package | After packages exist |
| `test` | Run `@test` functions | After attributes exist |
| `fmt` | Format source code | Tooling milestone |
| `lint` | Additional static checks | Tooling milestone |
| `doc` | Generate documentation | Tooling milestone |
| `add`, `remove` | Manage dependencies | Package manager milestone |

Further planned tooling: a language server (LSP), a VS Code extension,
Debug Adapter Protocol support, a lockfile and
dependency resolver, build caching and cross-compilation
(`--target <triple>`).

## 3. Conventions

* Results go to standard output; diagnostics and errors go to standard error.
* Command-line errors (usage errors and "not available" messages) are
  diagnostics too and follow `--diagnostic-format` whenever the format can be
  determined: the last well-formed `--diagnostic-format` option anywhere on
  the command line, before the first malformed one, applies even to a
  command that does not accept the option. Otherwise they are printed in the
  human format. Exit statuses do not depend on the format.
* One invocation reports at most 100 diagnostics, those that come first in
  the source. If there are more, they are counted and summarized by a final
  `P0004` note ("N more diagnostics not shown"), in both diagnostic formats.
  The closing
  `error: aborting due to N previous errors` line and the exit status count
  every error, including suppressed ones. The limit bounds the output and the
  memory spent on diagnostics for inputs with millions of errors.
* A closed standard output (for example when piping into `head`) is not
  reported as an error.
* `help` and `version` take no arguments; extra arguments are a usage error.
* Every argument that starts with `-` is treated as an option. Reading source
  from standard input (`-`) is not supported, and a file whose name starts
  with `-` must be written with a path prefix (`./-name.pgr`). There is no
  `--` end-of-options marker yet.
* Arguments are read as operating-system strings. A file name that is not
  valid Unicode (bytes that are not UTF-8 on Linux, an unpaired surrogate on
  Windows) is passed to the operating system unchanged. Anywhere a command,
  option or option value is expected, such an argument is a usage error
  (status 2), and messages show its invalid parts as U+FFFD.

## 4. Exit status

| Status | Meaning |
|---|---|
| 0 | Success. |
| 1 | The input contains errors; diagnostics were printed. |
| 2 | The command line is invalid. |
| 3 | The command is planned but not available in this version. |
