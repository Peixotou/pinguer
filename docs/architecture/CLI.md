# Command-Line Interface

Implementation: `cli/` (crate `pinguer_cli`, binary `pinguer`).
Decision record: [ADR-0010](../adr/0010-cli-exposes-only-implemented-commands.md).

## 1. Available in 0.1.0-dev

| Command | Description |
|---|---|
| `pinguer help`, `pinguer --help`, `pinguer -h`, `pinguer` | Print usage. |
| `pinguer version`, `pinguer --version`, `pinguer -V` | Print `pinguer 0.1.0-dev`. |
| `pinguer debug tokens <file> [--diagnostic-format human\|json]` | Print the token stream of a file, one token per line, followed by any diagnostics. For compiler development; the output format is **not stable**. |

`debug tokens` output, one line per token, tab-separated:

```text
1:1	Keyword(Import)	"import"
1:8	Ident	"std"
```

The columns are the start position (`line:column`), the token kind and the
token text as a quoted, escaped string.

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
* A closed standard output (for example when piping into `head`) is not
  reported as an error.
* `help` and `version` take no arguments; extra arguments are a usage error.
* Every argument that starts with `-` is treated as an option. Reading source
  from standard input (`-`) is not supported, and a file whose name starts
  with `-` must be written with a path prefix (`./-name.pgr`). There is no
  `--` end-of-options marker yet.

## 4. Exit status

| Status | Meaning |
|---|---|
| 0 | Success. |
| 1 | The input contains errors; diagnostics were printed. |
| 2 | The command line is invalid. |
| 3 | The command is planned but not available in this version. |
