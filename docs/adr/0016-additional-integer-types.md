# 0016. Additional integer types: `i128`, `u128` and `byte`

- Status: Accepted (feature status: Planned, target v0.2)
- Date: 2026-11-25

## Decision

* `i128` and `u128`: 128-bit two's-complement signed and unsigned integers,
  with the same operator and trap rules as the other integer types.
* `byte`: a **type alias** for `u8`, exactly as `int` is an alias for `i64`.
  `byte` and `u8` are interchangeable.

These names are identifiers resolved by name resolution, not keywords.

## Constraints and exceptional behaviour

* Integer literals may be typed as `i128`/`u128`; the compiler must store
  literal values with at least 128 bits. A literal above the maximum `u128` value is an
  error.
* Size 16 bytes. Alignment must match the platform ABI for 128-bit integers
  (16 bytes on current x86-64 and ARM64 ABIs); this must be verified on every
  CI target before the feature becomes normative.
* 128-bit division, remainder and some conversions lower to compiler runtime
  routines (for example `__divti3`). Their availability with `clang` on the
  Windows MSVC target must be verified; this is a known risk.
* Printing needs runtime support for 128-bit decimal formatting.

## Rejected alternatives

* **`decimal` as a primitive:** no hardware support; it will be a library
  type.
* **Arbitrary-precision integers as primitives:** unpredictable allocation;
  library type.

## Consequences

Lexer: none. Types: three new names in the prelude. PIR/LLVM: `i128` maps to
LLVM `i128`. Runtime: formatting and possibly compiler-rt builtins.
