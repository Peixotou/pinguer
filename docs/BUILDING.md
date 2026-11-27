# Building Pinguer

## Requirements

| Tool | Required for | Notes |
|---|---|---|
| Rust (stable) with `rustfmt` and `clippy` | Building the compiler | Edition 2024 requires Rust 1.85 or newer, and the code uses `let` chains, stable since Rust 1.88. Development and CI use the latest stable release; Phase 01 was verified with Rust 1.100.0. A minimum supported version is not yet tested or declared. |
| Platform C toolchain | Linking the compiler (Rust requirement) | Windows: Visual Studio Build Tools with the "Desktop development with C++" workload. Linux: `cc` (GCC or Clang). macOS: Xcode Command Line Tools. |
| LLVM `clang` | Building Pinguer programs (from Phase 04) | **Not needed yet.** Phase 01 validated LLVM/clang 23.1.6 on Windows. The minimum version will be established by CI. The official Windows installer does not add `clang` to `PATH`; how the compiler will find it is specified in [ADR-0026](adr/0026-locating-and-invoking-clang.md) (proposed). |

The compiler has no third-party Rust dependencies, so no network access is
needed after the toolchain is installed.

## Build and test

```sh
cargo build --workspace            # debug build
cargo build --workspace --release  # optimized build
cargo test --workspace             # all tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

The `pinguer` executable is written to `target/debug/pinguer` (or
`target/release/pinguer`; `.exe` on Windows).

```sh
cargo run -p pinguer_cli -- --version
cargo run -p pinguer_cli -- debug tokens examples/hello.pgr
cargo run -p pinguer_cli -- debug ast examples/hello.pgr
```

## Platform notes

### Windows

* Use the `x86_64-pc-windows-msvc` (or `aarch64-pc-windows-msvc`) Rust
  toolchain, which requires the MSVC build tools.
* The repository's `.gitattributes` keeps text files with LF line endings in
  the working tree on every platform; tests compare exact output and rely on
  it.
* Windows Defender Application Control or Smart App Control can block newly
  built, unsigned executables, including `pinguer.exe`, Cargo's test
  executables and executables produced by `clang` (`os error 4551`). This is
  an environment issue, not a compiler or test failure; build inside a
  trusted directory or adjust the policy.

### Linux and macOS

No additional steps.
