# 0001. Record architecture decisions

- Status: Accepted
- Date: 2026-11-24

## Context

Pinguer is a long-lived language project. Design choices made early (syntax,
memory model, backend) constrain everything that follows, and contributors
joining later need to know not only what was decided but why, and which
alternatives were rejected.

## Decision

Significant decisions are recorded as ADRs in `docs/adr/`, numbered
sequentially, using the template in [README.md](README.md). A decision is
significant if it affects language semantics, compiler architecture, public
interfaces (CLI, diagnostics format), dependencies or security.

Language-level decisions are also reflected in the normative specification in
`specifications/`. The specification states *what* the language is; ADRs
record *why*.

## Alternatives considered

* **Decisions only in the specification.** Loses the rationale and the
  rejected alternatives.
* **Decisions in issue trackers.** Not versioned with the code and not
  available offline.

## Consequences

* Changing an accepted decision requires a new ADR that supersedes it.
* Reviews of changes that contradict an ADR can point to it.
