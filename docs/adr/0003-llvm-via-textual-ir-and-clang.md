# 0003. LLVM integration through textual IR and clang

- Status: Accepted (for v0.1)
- Date: 2026-11-24

## Context

Pinguer targets native code through LLVM. The usual ways to drive LLVM from
Rust are the `llvm-sys` bindings (often through `inkwell`). They require a
full LLVM development installation, including `llvm-config` and static
libraries, of a version matching the bindings.

The Phase 01 toolchain audit found, on the primary development machine
(Windows 11, x86-64):

* the official LLVM 23.1.6 Windows installer provides `clang`, `lld` and
  `LLVM-C.dll`/`LLVM-C.lib`, but **no `llvm-config` and no static LLVM
  libraries**, so `llvm-sys` cannot be built without building LLVM from
  source;
* `clang` successfully compiled a hand-written textual LLVM IR module (opaque
  pointers, `llvm.sadd.with.overflow.i64`, a `printf` call) into a native
  executable linked with the MSVC toolchain.

## Decision

For v0.1, the code generator emits **textual LLVM IR** (`.ll`) and invokes
**`clang`** as an external process to optimize, assemble and link it.

* The code generator is written in safe Rust without LLVM bindings.
* `clang` is located through the `PATH`, overridable by an environment
  variable or CLI option (to be specified with the `build` command).
* The minimum supported LLVM version will be determined by CI testing before
  v0.1.0; the emitted IR uses only long-standing features (opaque pointers
  require LLVM 15 or newer).

## Alternatives considered

| Alternative | Why not now |
|---|---|
| `llvm-sys` / `inkwell` | Requires a source-built LLVM on Windows; version coupling; large build. |
| Dynamic loading of `LLVM-C.dll` | Requires `unsafe` FFI and manual bindings for a large API; platform differences. |
| Cranelift | Pure Rust and fast, but the project's stated backend is LLVM and Cranelift's optimization is weaker for release builds. It remains an option for fast debug builds later. |
| Emitting C | Simplest, but C semantics (undefined behaviour on signed overflow, aliasing rules) would leak into Pinguer's semantics unless worked around everywhere. |

## Consequences

* Users need `clang` installed to build executables (it is already needed for
  linking on most platforms).
* Text generation and parsing add compile-time overhead, acceptable for v0.1.
* The textual IR is easy to inspect and to snapshot-test.
* The code generator depends only on PIR, so replacing the text path with the
  LLVM C API later does not affect earlier stages.
* **Open question:** how the small runtime support code (trap reporting,
  number formatting) is provided: generated as IR by the compiler, or written
  in another language and shipped as a library. To be decided in the code
  generation phase.
