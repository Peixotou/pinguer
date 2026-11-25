# 0022. Feature classification and the version matrix

- Status: Accepted. Extends the labels of
  [LANGUAGE_SPECIFICATION.md §1.1](../../specifications/LANGUAGE_SPECIFICATION.md#11-classification-of-content).
- Date: 2026-11-25

## Context

The specification used four labels (Normative, Experimental, Proposal,
Unresolved). A design review approved many features for later versions and
rejected others; neither state had a label, and there was no single place
listing every feature with its target version.

## Decision

Every feature carries exactly one classification:

| Label | Meaning |
|---|---|
| **Normative (vX)** | Part of language version X; conforming compilers implement it. |
| **Experimental** | Specified; available only behind an explicit opt-in; may change. |
| **Planned** | Approved and specified (by an ADR), targeted at a future version, not yet normative. |
| **Proposal** | Under consideration; not approved. |
| **Unresolved** | An open decision with documented options and a recommendation. |
| **Rejected** | Considered and declined, with the reason recorded; may only return through a new ADR. |

[FEATURE_MATRIX.md](../../specifications/FEATURE_MATRIX.md) lists every
feature with its classification, tentative target version, specification,
ADR and implementation status per compiler stage. Target versions after v0.1
are **tentative** and ordered by dependency.

## Consequences

* A feature moves from Planned to Normative only when its version is
  released, its tests exist and its documentation describes it as
  implemented.
* Rejected features are listed explicitly, so that the same debate does not
  restart without new information.
