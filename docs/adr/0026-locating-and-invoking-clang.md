# 0026. Locating and invoking clang

- Status: Proposed. Requirements for the code generation phase (Phase 04);
  refines [ADR-0003](0003-llvm-via-textual-ir-and-clang.md). No driver code
  exists yet.
- Date: 2026-11-25

## Context

ADR-0003 has the compiler emit textual LLVM IR and run `clang` to optimize,
assemble and link it, with `clang` "located through the `PATH`, overridable
by an environment variable or CLI option". Facts observed on the primary
development machine (Windows 11 x86-64):

* The official LLVM 23.1.6 installer put `clang.exe` in
  `C:\Program Files\LLVM\bin` and did not add it to `PATH`: neither Git Bash
  nor PowerShell finds `clang`. A `PATH`-only search fails there.
* Windows Smart App Control blocks some freshly built unsigned executables
  (`os error 4551`), including executables that `clang` produces.
* The Phase 01 audit compiled a hand-written `.ll` file into an executable
  with this `clang` and the MSVC linker.

Linux distributions often install versioned binaries (`clang-18`), and on
macOS the Xcode Command Line Tools provide an Apple build of clang whose
version numbers differ from upstream LLVM.

## Requirements

### Discovery

1. Candidates are tried in a fixed, documented order, and the first that
   passes validation is used:
   1. an explicit command-line option of `pinguer build` (name decided with
      that command);
   2. the environment variable `PINGUER_CLANG`;
   3. `clang` found through `PATH` (`clang.exe` on Windows);
   4. platform defaults: `%ProgramFiles%\LLVM\bin\clang.exe` on Windows;
      `xcrun --find clang` and the Homebrew LLVM prefixes on macOS;
      versioned names (`clang-N`, newest first) on Linux.
2. An explicit option or environment variable that names an unusable
   program is an error; the search does not silently fall back to another
   candidate.
3. Discovery never uses a shell, never downloads anything and keeps no
   state between runs.
4. Paths are handled as operating-system strings, so that paths with spaces
   or that are not valid Unicode work.

### Validation

5. The candidate's `--version` output is parsed; a version below the
   minimum supported version (to be established by CI, ADR-0003) is
   rejected with both versions in the diagnostic. Apple clang versions are
   mapped or validated separately.
6. The candidate must support the requested target triple.

### Invocation

7. `clang` is run with an explicit argument vector, an explicit target
   triple and data layout in the emitted IR, and temporary files in a
   unique temporary directory that is removed afterwards.
8. Builds aim to be reproducible: the arguments do not depend on the
   working directory, and the linker is asked for deterministic output
   where it supports it (for example `/Brepro` with the MSVC linker).

### Diagnostics

9. Each failure has its own code in the `P6001`–`P6999` range and an
   English message, with operating-system and `clang` text quoted
   unchanged in notes, following
   [DIAGNOSTICS.md](../architecture/DIAGNOSTICS.md):
   * no usable `clang`, listing every location that was searched and how to
     configure one;
   * unsupported `clang` version or target;
   * `clang` could not be started, distinguishing a program blocked by
     application control (`os error 4551` on Windows) from a missing file;
   * `clang` or the linker failed, with its exit status and standard error.

### Testing

10. Discovery is a function of the options, the environment, the `PATH`
    entries and file-system probes, written so that it can be unit-tested
    without a real `clang`.
11. End-to-end tests run the real `clang` on every CI platform. Tests that
    start stub executables must expect application control to block them
    on some Windows machines and report that as an environment problem.

## Consequences

* A default installation of LLVM on Windows works without changing `PATH`.
* The search order and its error messages become part of the CLI contract
  and are documented in `docs/architecture/CLI.md` when `build` exists.
* This ADR is accepted, possibly amended, when the driver is implemented in
  Phase 04.
