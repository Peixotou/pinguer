# 0007. Checked arithmetic and explicit conversions

- Status: Accepted
- Date: 2026-11-24

## Context

Languages differ on integer overflow: C makes signed overflow undefined
behaviour, Rust traps in debug builds and wraps in release builds, Swift traps
in all builds, Go and Java wrap. Implicit numeric conversions (C, C++) are a
classic source of bugs, and Rust's `as` silently truncates.

## Decision

* Integer overflow, division by zero, signed `MIN / -1`, out-of-range shifts
  and invalid conversions **trap in every build mode**.
* There are **no implicit numeric conversions**, including widening.
* `as` is **checked**: integer conversions trap when the value does not fit;
  float-to-integer conversions truncate toward zero and trap on NaN or
  out-of-range values.
* Wrapping and saturating arithmetic will be available as explicit library
  functions.

Details: [TYPE_SYSTEM.md §6–7](../../specifications/TYPE_SYSTEM.md#6-operators).

## Alternatives considered

| Alternative | Assessment |
|---|---|
| Wrap silently (Go, Java) | Hides bugs; contradicts safety goal. |
| Trap in debug, wrap in release (Rust) | Behaviour depends on build mode; release builds hide bugs that tests did not hit. |
| Undefined behaviour (C) | Unacceptable for a safe language. |
| Arbitrary-precision integers by default (Python) | Unpredictable performance and allocation. |

## Consequences

* Arithmetic has a small run-time cost (overflow checks), which LLVM removes
  when it can prove the absence of overflow. The cost must be measured with
  real benchmarks before any claim about performance is made.
* The `as` operator behaves differently from Rust's; documentation and
  diagnostics must point this out to Rust programmers.
* Optimizations must preserve traps.
