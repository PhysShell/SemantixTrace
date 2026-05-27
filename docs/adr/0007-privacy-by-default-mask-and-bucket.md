# ADR 0007: Privacy by default — mask strings, bucket numerics, raw export is opt-in

Date: 2026-05-27
Status: Accepted

## Context

SemantxTrace records interactions in enterprise desktop applications:
customs declarations, banking forms, healthcare workflows. The recording
buffer is exactly the kind of artefact that, leaked, ruins both the
operator and the customer. GDPR Article 5(1)(c) (data minimization)
makes the trade-off legally explicit: store the minimum necessary for
the purpose. Datadog's Session Replay defaults to masking inputs when
the privacy setting is not specified, and that default is correct.

Three categories of values appear in a trace:

1. Free-text user inputs (names, addresses, comments).
2. Identifiers with structure (emails, phones, IBANs, IINs, BINs, credit
   cards).
3. Numerics whose magnitude matters for analysis but whose exact value
   does not (quantities, amounts).

A useful workflow miner only needs categorical equivalence, not raw
values. The exception is the **diagnostic package** an operator
deliberately exports for a specific support case; that path must be
guarded, audited, and impossible to take by mistake.

## Decision

The library models privacy at the type level via `ValuePolicy`:

```rust
pub enum ValuePolicy {
    Raw(Value),
    Masked(String),
    Bucketed { bucket: String },
    Hashed { hash: String, algo: String },
    Removed,
}
```

Defaults:

- String fields are recorded as `ValuePolicy::Masked("***")` unless an
  explicit per-field policy promotes them.
- Numeric fields are recorded as `ValuePolicy::Bucketed { bucket: … }`
  using the bucket table in `glossary.md` §4.
- Identifiers matching one of the known PII regexes (email, E.164
  phone, IBAN with mod-97, Luhn-valid credit card, 12-digit IIN/BIN
  with checksum) are recorded as `ValuePolicy::Hashed { algo: "blake3"
  }` so equivalence-on-identity is still mineable.
- An adapter that wants a different policy for a specific field must
  declare it explicitly via `[TraceField(policy = Policy.Raw)]` (or the
  equivalent in non-WPF adapters); the policy is part of the
  application's source and reviewable.

Raw export is an explicit two-step process:

1. The CLI command `trace export --raw <session_id>` shows an
   interactive consent prompt naming the session(s), the destination,
   and the policy override.
2. Acceptance is recorded in `audit.log` with `who`, `when`, `what`,
   `where`. A `--yes-i-have-consent` flag is available for CI use, but
   it still writes the audit entry.

ML-based PII detection (named-entity recognition, ONNX models) is
**explicitly out of scope** for v1.0. Regex + structural checks are the
only mechanisms.

## Consequences

- Default-mode recording is GDPR-defensible without per-deployment
  legal review.
- Operators who genuinely need raw values still have a clean path, with
  an audit trail strong enough to satisfy compliance.
- Workflow mining quality degrades only for analyses that would have
  depended on raw values; bucketed numerics and hashed identifiers
  preserve equivalence classes, which is what the miner actually uses.
- An adapter that fails to declare per-field policy and leaks PII via
  free-text fields is a known failure mode; the WPF adapter's
  documentation enumerates the patterns to follow, and the audit-log
  format makes after-the-fact discovery possible.
- The cost of the policy machinery is borne by every adapter: each must
  thread `ValuePolicy` through field-change events. This is intentional
  — the cheaper alternative (apply policy in the core) loses the
  fidelity needed for per-field overrides.

## See also

- [`../privacy.md`](../privacy.md) — operational policy reference.
- [`../glossary.md`](../glossary.md) §8.
