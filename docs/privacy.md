# privacy.md — mask-by-default policy

> Status: operational reference for the policy fixed in ADR-0007.

## Posture

SemantxTrace records interactions in enterprise desktop applications
(customs, banking, healthcare, internal back-office tools). The default
posture is **suspicion**: assume any unannotated value is sensitive
until the application's developer marks it otherwise. This is the
posture GDPR Article 5(1)(c) (data minimization) operationalises, and
the posture Datadog's Session Replay defaults to ("`mask` is enabled by
default" when no privacy level is specified).

## Five-armed `ValuePolicy`

```rust
pub enum ValuePolicy {
    Raw(serde_json::Value),    // only with explicit opt-in
    Masked(String),            // "***"
    Bucketed { bucket: String },
    Hashed   { hash: String, algo: String },
    Removed,
}
```

| Variant | Default for | Mineable as |
|---|---|---|
| `Raw` | nothing | the raw value (audit-log required) |
| `Masked` | free-text strings | the field's presence only |
| `Bucketed` | numerics | the bucket |
| `Hashed` | structural identifiers (email, phone, IBAN, IIN/BIN, card) | the hash (equivalence preserved) |
| `Removed` | fields the application explicitly opts out of | nothing |

## Defaults

- **Strings** → `ValuePolicy::Masked("***")`. The `*` count does not
  encode length (length itself is sensitive in some cases).
- **Numerics** → `ValuePolicy::Bucketed` using the table from
  [`glossary.md`](glossary.md) §4: `0`, `1`, `2–10`, `11–100`,
  `101–1000`, `1000+`. Negative numbers map symmetrically.
- **Dates / timestamps** → `ValuePolicy::Bucketed` with relative
  buckets: `past_week`, `past_month`, `past_year`, `future`, `epoch`.
- **Booleans** → `ValuePolicy::Raw` (booleans have negligible
  re-identification risk by themselves).
- **Identifiers matching a known PII pattern** → `ValuePolicy::Hashed`
  with `algo: "blake3"`. Matching is regex / structural:
  - email (RFC-ish);
  - E.164 phone numbers;
  - IBANs with mod-97 check;
  - credit-card numbers passing Luhn;
  - Kazakhstan IIN / BIN (12 digits + checksum).
- **Anything explicitly tagged `[TraceField(policy = Policy.Removed)]`**
  → `ValuePolicy::Removed`.

Adapters must promote values *down* the table (more permissive) only via
explicit per-field configuration; they must never silently promote.
Adapters may promote *up* the table (more restrictive) for any reason
they choose.

## Per-field opt-out / opt-in

The WPF adapter (analogously: Avalonia, MAUI, Web) exposes
`[TraceField(policy = ...)]` for case-by-case overrides:

```csharp
[TraceField(policy = Policy.Raw)]
public string CategoryCode { get; set; } // domain-relevant code, not PII

[TraceField(policy = Policy.Removed)]
public string InternalDebugComment { get; set; } // never of interest

[TraceField(policy = Policy.Hashed, algorithm = "blake3")]
public string CustomerCode { get; set; } // identifier, not text
```

A reviewable per-field policy is the only sanctioned way to step away
from the defaults. The audit posture rests on the assumption that
opt-outs are visible in source-controlled code.

## Diagnostic packages

When raw values are unavoidable (support escalation, reproducing a
specific data-shape bug), the operator runs:

```
trace export --raw <session_id> -o <dir>
```

The command:

1. Prints an **interactive consent prompt** listing each session id,
   the destination directory, and the policies that will be overridden.
2. Refuses to proceed unless the operator types the exact session ids
   back (no `y/N` shortcut for this; ambient consent is the failure
   mode the consent prompt exists to prevent).
3. Writes a line to `./audit.log` of shape:

   ```
   2026-05-27T12:13:14Z  who=alice@host  what=trace-export-raw
   sessions=[01HXG…]  to=/tmp/dump  policies=[promote-all-to-raw]
   ```

4. Writes the exported `*.jsonl` with `ValuePolicy::Raw` in place where
   the original was a defaulted policy.

CI use is supported via `--yes-i-have-consent`; the audit-log entry is
written regardless.

## Where the policy is enforced

- **Adapter side** (`trace-wpf`, `trace-avalonia`, …): policies apply
  at the moment the value is captured for emission. The adapter sees
  the raw value; the trace sink does not.
- **Core side** (`trace-core`, `trace-schema`): values arrive already
  policy-wrapped. The core never sees a raw value unless the envelope
  carries `ValuePolicy::Raw`.
- **Normalizer side** (`trace-normalizer`): folds bucketed values into
  equivalence classes for mining without ever attempting to re-derive
  the raw value.
- **Reporter side** (`trace-oracle` HTML reporter, `trace report`):
  renders `Masked` as `***`, `Bucketed` as the bucket label, `Hashed`
  as a short prefix, `Raw` as the value, `Removed` as a placeholder.

## Anti-patterns

1. **"Mask in post-processing."** Wrong. By the time the value has been
   written by the adapter as `Raw`, leakage has already happened on
   disk. The policy enforces at capture.
2. **"Just hash everything to be safe."** Wrong. Hashed strings lose
   the *bucket* signal that mining depends on; hashed numerics lose
   ordering. Use the right `ValuePolicy` per type.
3. **"Log the length so we can grep later."** Wrong. Length is often
   identifying (a comment of exactly 47 characters in a 100-record
   table is unique). The default `Masked` shape intentionally drops
   length.
4. **"Use the operator's `--raw` workflow for routine export."** Wrong.
   `--raw` is for explicit, audited diagnostic packages. The default
   export shape carries the masked / bucketed / hashed values.

## ML-based PII detection

**Out of scope for v1.0.** Regex + structural checks have known
behaviour; ONNX-shipped NER models have unknown behaviour on real input
and add a non-trivial dependency closure. Re-evaluate post-v1.0 with a
concrete failure case in hand.

## See also

- [`adr/0007-privacy-by-default-mask-and-bucket.md`](adr/0007-privacy-by-default-mask-and-bucket.md)
- [`SPEC.md`](SPEC.md) §5
- [`glossary.md`](glossary.md) §8
- External: GDPR Article 5(1)(c); ISO/IEC 29100 privacy framework
  (consent, purpose limitation, transparency); Datadog Session Replay
  Privacy Options.
