# Security Model

Status: sections marked **Normative (v0.1)** are binding; the rest records
**Proposals**. For reporting vulnerabilities in the Pinguer project itself,
see [SECURITY.md](../SECURITY.md).

## 1. Threat model

| Asset | Threat | Defence |
|---|---|---|
| The developer's machine | Malicious or malformed source code fed to the compiler | Compiler robustness (section 2) |
| Code review integrity | Source that displays differently from how it compiles | Lexical restrictions (section 3) |
| Programs built with Pinguer | Memory-safety bugs exploitable by untrusted input | Language guarantees (section 5) |
| The build | Compromised dependencies | Supply-chain controls (section 4) |

Out of scope: a compromised compiler binary or operating system, and side
channels such as timing attacks (constant-time programming is a library
concern).

## 2. Compiler robustness (Normative v0.1 requirements for the implementation)

* The compiler must not crash, hang or exhibit undefined behaviour on any
  input. Malformed input produces diagnostics.
* The compiler is written in safe Rust. `unsafe` Rust is forbidden by a
  workspace-wide lint; any exception requires documented invariants and review
  ([ADR-0002](../docs/adr/0002-workspace-layout-and-engineering-policies.md)).
* The compiler core has no third-party dependencies in Phase 01, minimizing
  supply-chain exposure; every future dependency needs a justification.
* Diagnostics never write raw control or bidirectional characters to the
  terminal: the human renderer escapes them and the JSON renderer escapes
  control characters ([docs/architecture/DIAGNOSTICS.md](../docs/architecture/DIAGNOSTICS.md)).
* Robustness is tested with property-based tests today and will be tested with
  coverage-guided fuzzing for every front-end stage before v0.1.0.

## 3. Source-level protections (Normative v0.1)

* **Bidirectional formatting characters** are rejected everywhere in source
  files ([LEXICAL_STRUCTURE.md §3](LEXICAL_STRUCTURE.md#3-forbidden-characters)),
  preventing "Trojan Source" attacks (CVE-2021-42574).
* **Identifiers are ASCII-only** in v0.1, which rules out homoglyph attacks
  between identifiers (for example Latin `a` and Cyrillic `а`). Unicode
  identifiers will only be introduced together with confusable detection
  (Unicode TS #39).
* **Lone carriage returns** are rejected, so that all tools agree on line
  boundaries.
* **Control characters and Unicode line separators** (all controls except
  tab and line breaks, including vertical tab, form feed and next line, plus
  U+2028 and U+2029) are rejected everywhere in source files
  ([LEXICAL_STRUCTURE.md §3.2](LEXICAL_STRUCTURE.md#32-control-characters-and-unicode-line-separators),
  [ADR-0024](../docs/adr/0024-control-characters-and-line-separators.md)):
  other tools may show them as line breaks or interpret them, which would let
  displayed code differ from compiled code.
* Other invisible characters inside comments and string literals remain
  allowed. **Proposal:** a lint for zero-width and other invisible
  characters in literals, suggesting escapes.

## 4. Supply chain

* The lockfile records a cryptographic hash of every dependency; builds fail
  on mismatch.
* Packages cannot execute code at build time by default. If build-time code
  becomes necessary, it will run with restricted capabilities (no network, no
  writes outside the build directory) and require explicit opt-in by the
  depending project.
* The package registry will support namespaced packages, mandatory two-factor
  authentication for publishers, and yanking.

## 5. Guarantees for compiled programs

### 5.1 v0.1 (Normative v0.1)

* Every operation has defined behaviour: arithmetic errors trap
  ([ERROR_HANDLING.md §2](ERROR_HANDLING.md#2-traps-normative-v01)), and the
  memory model has no references, pointers, allocation or threads
  ([MEMORY_MODEL.md §2](MEMORY_MODEL.md#2-the-v01-memory-model-normative-v01)).
* Stack exhaustion terminates the process and never corrupts memory.

These are requirements on the compiler. They will be validated by tests that
compile and run programs exercising each trapping condition on every CI
platform. Until that exists, Pinguer makes **no claim** that its compiled
programs are memory safe.

### 5.2 Long term (Proposal)

* Safe code cannot cause undefined behaviour. Unsafe code is explicit,
  greppable and linted.
* Data-race freedom through the ownership model.
* Hardening flags enabled by default where the platform supports them
  (position-independent executables, stack protection, control-flow
  integrity where cheap).

## 6. Process

* Security-relevant design changes require an ADR.
* Vulnerabilities in the compiler or standard library are handled according
  to [SECURITY.md](../SECURITY.md).
