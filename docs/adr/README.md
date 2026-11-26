# Architecture Decision Records

An Architecture Decision Record (ADR) captures one significant decision: its
context, the decision itself, the alternatives that were considered and the
consequences. ADRs are never rewritten after acceptance; a later ADR
supersedes an earlier one.

| ADR | Title | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-workspace-layout-and-engineering-policies.md) | Workspace layout and engineering policies | Accepted, partially superseded by 0023 |
| [0003](0003-llvm-via-textual-ir-and-clang.md) | LLVM integration through textual IR and clang | Accepted |
| [0004](0004-intermediate-representations.md) | Intermediate representations for v0.1 | Accepted |
| [0005](0005-newline-terminated-statements.md) | Newline-terminated statements | Accepted, amended by 0014 |
| [0006](0006-v0-1-memory-model.md) | v0.1 memory model: value semantics without allocation | Accepted |
| [0007](0007-checked-arithmetic-and-explicit-conversions.md) | Checked arithmetic and explicit conversions | Accepted |
| [0008](0008-diagnostic-codes-and-formats.md) | Diagnostic codes and output formats | Accepted |
| [0009](0009-ascii-identifiers-and-bidi-rejection.md) | ASCII identifiers and rejection of bidirectional controls | Accepted, extended by 0024 |
| [0010](0010-cli-exposes-only-implemented-commands.md) | The CLI exposes only implemented commands | Accepted |
| [0011](0011-composition-and-interfaces-without-class-inheritance.md) | Composition and interfaces without class inheritance | Accepted |
| [0012](0012-visibility-public-and-private.md) | Visibility with `public` and `private` | Accepted |
| [0013](0013-prefix-try.md) | Prefix `try` for error propagation | Accepted |
| [0014](0014-leading-dot-continuation.md) | Lines starting with `.` continue the previous line | Accepted |
| [0015](0015-contextual-query-keywords.md) | Contextual keywords for declarative queries | Accepted |
| [0016](0016-additional-integer-types.md) | Additional integer types: `i128`, `u128`, `byte` | Accepted |
| [0017](0017-power-operator.md) | Power operator `**` and `**=` | Accepted |
| [0018](0018-optional-operators.md) | Optional operators `??` and `?.` | Accepted |
| [0019](0019-pipeline-operator.md) | Pipeline operator <code>&#124;&gt;</code> | Accepted |
| [0020](0020-compiler-attributes.md) | Compiler attributes | Accepted |
| [0021](0021-fundamental-library-functions.md) | Fundamental library functions | Accepted |
| [0022](0022-feature-classification.md) | Feature classification and the version matrix | Accepted |
| [0023](0023-no-placeholder-directories.md) | No placeholder directories | Accepted |
| [0024](0024-control-characters-and-line-separators.md) | Rejection of control characters and Unicode line separators | Accepted |
| [0025](0025-string-interpolation-syntax.md) | String interpolation syntax | Proposed |
| [0026](0026-locating-and-invoking-clang.md) | Locating and invoking clang | Proposed |
| [0027](0027-parser-architecture.md) | Parser architecture | Accepted |
| [0028](0028-ast-representation.md) | AST representation | Accepted |

## Template

```markdown
# NNNN. Title

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context
## Decision
## Alternatives considered
## Consequences
```
