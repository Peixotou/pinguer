# 0004. Intermediate representations for v0.1

- Status: Accepted
- Date: 2026-11-24

## Context

A conventional multi-IR design places an AST, a high-level IR, a mid-level IR
and a low-level IR before LLVM. Each layer costs a data model,
a lowering pass, tests and maintenance. Production compilers introduce such
layers when specific features need them: rustc's HIR exists for desugaring and
its MIR for borrow checking and optimization.

## Decision

v0.1 uses two representations of its own:

1. **AST** produced by the parser. Every node has a stable `NodeId`. Name
   resolution and type checking do not rewrite the AST; they produce **side
   tables** keyed by `NodeId` (resolved definitions, types of expressions).
   The pair *AST + side tables* is the "typed AST".
2. **PIR (Pinguer Intermediate Representation)**: a typed control-flow graph
   of basic blocks per function. Locals are explicit, values are not in SSA
   form (LLVM's `mem2reg` constructs SSA), and every operation has fully
   explicit semantics: checked arithmetic becomes explicit overflow checks
   with trap edges, short-circuit operators become branches.

PIR is target-independent and contains no LLVM concepts, so other backends
and a compile-time interpreter can consume it.

A separate HIR will be introduced when desugaring becomes substantial
(methods, pattern matching, generics). A low-level IR is not planned; LLVM
IR plays that role.

## Alternatives considered

* **Full HIR/MIR/LIR stack now.** Rejected: empty abstractions with nothing
  to desugar in v0.1.
* **AST directly to LLVM IR.** Rejected: it would mix semantic decisions
  (traps, evaluation order) with LLVM details, and future backends and
  compile-time execution could not reuse the work.
* **SSA-form PIR.** Rejected for v0.1: SSA construction is unnecessary when
  LLVM performs it, and non-SSA locals are simpler to generate and to
  interpret. Can be revisited if PIR-level optimization needs it.

## Consequences

* Optimization in v0.1 is done by LLVM. `compiler/optimizer` stays empty
  until a PIR-level transformation is needed (for example for compile-time
  evaluation or borrow-checking support).
* PIR is the stable contract between the front end and all backends.
