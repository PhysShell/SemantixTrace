# ADR 0009: Use the Nygard ADR format, stored in-repo, append-only

Date: 2026-05-27
Status: Accepted

## Context

The project needs a lightweight, durable way to record architectural
decisions so future contributors (human and LLM) can reconstruct why
the system is the way it is. Heavyweight RFC processes are wrong for a
solo-dev / small-team cadence; ad-hoc wiki pages rot and lose
authorship.

## Decision

We use the **Nygard ADR format** (Michael Nygard, "Documenting
Architecture Decisions", 2011). Sections: Context, Decision,
Consequences. Status enum: `Proposed | Accepted | Deprecated |
Superseded by ADR-NNNN`. ADRs live in `docs/adr/` with monotonic
zero-padded numbering (`NNNN-short-title.md`). The template is
[`0000-template.md`](0000-template.md).

ADRs are **append-only**: after `Accepted`, the body is immutable. A
later decision that overrides an earlier one is recorded as a new ADR
that marks the old one `Superseded by ADR-NNNN`; the original text is
preserved. The index lives in [`README.md`](README.md).

Small decisions that do not warrant a full ADR go to
[`../decisions.log.md`](../decisions.log.md) as Y-statements.

## Consequences

- Decision history is preserved verbatim, including the reasoning that
  later turned out to be wrong.
- Reviewers can use ADR numbers as stable references across the
  glossary, SPEC, and stage docs.
- The append-only rule forbids editorial cleanup of accepted ADRs; small
  typos may be fixed but substantive changes require a new ADR.
