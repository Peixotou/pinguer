<p align="center">
  <img src="assets/logo.png" alt="Pinguer logo" width="160">
</p>

<h1 align="center">Pinguer</h1>

Pinguer is a statically typed, general-purpose programming language designed
for ahead-of-time compilation to native code through LLVM. The compiler is
written in Rust.

The v0.1 language is specified with local type inference, no implicit
conversions, no null, checked integer arithmetic that traps on overflow, and
newline-terminated statements. See the
[language specification](specifications/LANGUAGE_SPECIFICATION.md) and the
[v0.1 scope](specifications/V0_1_SCOPE.md).

## Status

**0.1.0-dev, pre-alpha. Pinguer cannot compile or run programs yet.**

| Component | State |
|---|---|
| Specification and grammar | Draft |
| Source management (`compiler/source`) | Implemented |
| Diagnostics, human and JSON (`compiler/diagnostics`) | Implemented |
| Lexer (`compiler/lexer`) | Implemented |
| Syntax tree and parser (`compiler/ast`, `compiler/parser`) | Implemented |
| Semantic analysis, code generation | Not started |
| CLI (`cli`) | `help`, `version`, `debug tokens` and `debug ast` only |

The phases toward v0.1.0 are described in [ROADMAP.md](ROADMAP.md).

## Installation

There are no releases or prebuilt binaries. Build from source as described
below.

## Building

Requires stable Rust and a platform C toolchain; see
[docs/BUILDING.md](docs/BUILDING.md).

```sh
cargo build --workspace
cargo test --workspace
```

The executable is `target/debug/pinguer` (`pinguer.exe` on Windows).

## Example

[examples/hello.pgr](examples/hello.pgr):

```pinguer
// The smallest Pinguer program.
import std.io

func main() {
    io.println("Hello, Pinguer!")
}
```

The commands that process source code today print the token stream and the
syntax tree. Their output formats are unstable and meant for compiler
development:

```console
$ cargo run -p pinguer_cli -- debug tokens examples/hello.pgr
2:1	Keyword(Import)	"import"
2:8	Ident	"std"
2:11	Dot	"."
2:12	Ident	"io"
2:14	Newline	"\n"
4:1	Keyword(Func)	"func"
...
7:1	Eof	""
$ cargo run -p pinguer_cli -- debug ast examples/hello.pgr
File @1:1-7:1 nodes=8
  Import #0 @2:1-2:14 std.io
  Func #1 @4:1-6:2 main
    Block #2 @4:13-6:2
...
```

Lexical errors are reported with stable codes. The diagnostics are written to
standard error after the token listing; for a file containing
`let s = "unclosed`, standard error is:

```text
error[P1002]: unterminated string literal
 --> bad.pgr:1:9
  |
1 | let s = "unclosed
  |         ^^^^^^^^^ missing closing `"`
  |
  = note: string literals cannot contain line breaks; use `\n` instead

error: aborting due to 1 previous error
```

Pass `--diagnostic-format json` for JSON Lines output.

## Documentation

* [Language specification](specifications/LANGUAGE_SPECIFICATION.md),
  [lexical structure](specifications/LEXICAL_STRUCTURE.md) and
  [grammar](specifications/GRAMMAR.ebnf)
* [Compiler architecture](docs/architecture/COMPILER_ARCHITECTURE.md)
* [Diagnostics](docs/architecture/DIAGNOSTICS.md) and [CLI](docs/architecture/CLI.md)
* [Architecture Decision Records](docs/adr/README.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Report security issues as described in
[SECURITY.md](SECURITY.md).

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
