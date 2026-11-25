# Examples

Pinguer 0.1.0-dev cannot compile programs yet. These files show the v0.1
syntax described in the [specification](../specifications/LANGUAGE_SPECIFICATION.md)
and can be inspected with the lexer:

```sh
cargo run -p pinguer_cli -- debug tokens examples/hello.pgr
```

| File | Shows |
|---|---|
| [hello.pgr](hello.pgr) | Imports and the entry point |
| [fibonacci.pgr](fibonacci.pgr) | Functions, recursion, loops, mutable variables |
