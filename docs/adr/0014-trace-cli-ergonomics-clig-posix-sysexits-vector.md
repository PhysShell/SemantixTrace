# ADR 0014: `trace-cli` ergonomics follow clig.dev + POSIX/GNU + sysexits.h + Vector precedent

Date: 2026-05-27
Status: Accepted

## Context

`trace-cli` is the project's user-facing binary. It is the surface
DevOps engineers see in CI, support engineers run on customer-supplied
diagnostic packages, developers use during demos, and product /
UX folk run to read analytics reports. CLI ergonomics is therefore
a *product* concern, not a stylistic one: a CLI that violates the
conventions of its category fails adoption regardless of how good
the underlying engine is.

ADR-0012 covers `trace-cli`'s **library** surface (the `pub` items
inside the crate). This ADR covers its **binary** surface — the
subcommand grammar, flags, output shapes, exit codes, and stream
discipline a user encounters at the shell.

There is a small set of canonical references and a clear set of
direct precedents:

- [Command Line Interface Guidelines (clig.dev)](https://clig.dev/)
  — the modern philosophy reference: discoverability, conversation,
  robustness, helpfulness, output, errors, configuration.
- [POSIX Utility Conventions, XBD §12](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html)
  — the formal stream / argument conventions.
- [GNU Coding Standards — CLI](https://www.gnu.org/prep/standards/html_node/Command_002dLine-Interfaces.html)
  — `--help`, `--version`, long options.
- [`sysexits.h`](https://man.freebsd.org/cgi/man.cgi?query=sysexits&sektion=3)
  — standard exit codes; Rust crates [`sysexits`](https://github.com/sorairolake/sysexits-rs)
  and `sysexit`.
- [Rain's Rust CLI recommendations](https://rust-cli-recommendations.sunshowers.io/)
  — practical Rust-specific patterns (clap derive, subcommand
  structure, exit code idioms), authored by the maintainer of
  `cargo-nextest`.
- [Rust CLI book](https://rust-cli.github.io/book/) (rust-cli WG)
  — exit codes chapter cross-checked.

Direct architectural precedents (Rust + observability + user-facing
CLI):

- **Vector** (Datadog). Same shape as us: Rust core, observability,
  user-facing CLI. Uses noun-verb subcommands (`vector tap`,
  `vector validate`, `vector graph`, `vector generate`,
  `vector test`, `vector top`), exits with **78** on config-
  validation failure (= `EX_CONFIG`), exports the action graph as
  **DOT**, ingests **newline-delimited JSON** for VRL. Our defaults
  should match.
- **kubectl** — `kubectl <verb> <noun>` (`get pods`, `describe
  deployment`); `-o {json,yaml,wide}` is the de-facto standard for
  output format.
- **gh** (GitHub CLI) — `gh <noun> <verb>` (`gh pr create`,
  `gh issue list`); `--json <fields>` for machine output.
- **cargo** — Rust-ecosystem common idioms (`--workspace`,
  `--package`, `--manifest-path`, `--quiet/-q`, `--verbose/-v`,
  `--offline`, `--frozen`).

## Decision

### 1. Scope

Applies to the `trace` binary built from `crates/trace-cli`. The
library API surface of the same crate is bound by ADR-0012
(API Guidelines) and is out of scope here.

### 2. Reference documents

clig.dev for philosophy and gaps; POSIX XBD §12 + GNU CLI standards
for formal stream/argument rules; `sysexits.h` for exit codes;
Rain's Rust CLI recommendations for the Rust-specific patterns; the
Vector / kubectl / gh / cargo CLIs as worked examples. This ADR
enumerates project-specific commitments on top of those.

### 3. Subcommand grammar

**Noun-verb** in the kubectl / gh / docker / Vector style. Top-level
verbs are reserved for primary single-noun operations; multi-noun
operations live under a noun:

```
trace analyze            <file>
trace normalize          <file> [-o <out>]
trace graph              <file> [--format mermaid|dot]
trace ingest             --from {jsonl,sqlite,parquet}
                         --to   {jsonl,sqlite,parquet}
trace compact            <file>
trace oracle  run        <file> [--rules <pack>]
trace oracle  list
trace plan    generate   <scenario-id> --mode {strict,relaxed}
trace plan    explore    --strategy {coverage,weighted,quick}
trace plan    mutate     --kinds <list>
trace report  workflows  <file> [--top N] [--rare-failing]
trace report  html       <file>
trace export  diagnostic <session-id>
trace completions        {bash,zsh,fish,powershell}
trace version
trace help               [<command>]
```

Never invent a third-level depth without an ADR (`trace plan
explore strategy add weighted` is a smell).

### 4. Global options

| Flag | Behaviour | Precedent |
|---|---|---|
| `-o`, `--output <FORMAT>` | `text` \| `json` \| `wide` (`text` default) | kubectl `-o`, gh `--json` |
| `-q`, `--quiet` | Reduce diagnostic noise | cargo, kubectl |
| `-v`, `--verbose...` | Repeatable; `-v`, `-vv`, `-vvv` map to log levels | cargo, ripgrep |
| `--no-color` | Disable ANSI colour | clig.dev §output |
| `--color {auto,always,never}` | Override autodetect | cargo |
| `--manifest-path <PATH>` | Path to optional `trace.toml` config | cargo |
| `--offline` | Refuse network operations | cargo |
| `-h`, `--help` | Print help | GNU |
| `-V`, `--version` | Print binary + schema version (JSON in `--output json`) | GNU |

Environment variables honoured: `NO_COLOR` (any non-empty value
disables colour, per <https://no-color.org/>); `TRACE_LOG`
(structured-log level for the binary's own observability); `RUST_LOG`
(fallback if `TRACE_LOG` unset, per Rust convention).

### 5. Output discipline

Per clig.dev §output and POSIX:

- **Data on `stdout`, diagnostics on `stderr`.** A user piping
  `trace analyze … | jq` must get JSON on stdout uncontaminated by
  progress bars.
- **`-` means stdin/stdout** wherever a path is accepted.
- **`--output json` emits one JSON document per invocation**, or a
  newline-delimited stream when the command is naturally streaming
  (`trace analyze --output json --stream`).
- **Default text output is stable and snapshot-friendly**: no
  timestamps, no random ids, no map-iteration-order leaks. Snapshot
  tests under `crates/trace-cli/tests/` bless the output and
  regress accidental changes.
- **Colour is autodetect by default** (`isatty(stdout)` + no
  `NO_COLOR`); always preserve plain text in pipes.
- **Progress** (long-running commands like `trace ingest`,
  `trace plan explore`): progress goes to stderr, suppressible
  with `-q`, machine-readable structured progress in
  `--output json`.

### 6. Exit codes (sysexits.h)

| Code | Symbolic | Used for |
|---|---|---|
| `0` | OK | Success |
| `1` | — | Generic failure (reserved for "an oracle returned `Error` severity") |
| `2` | — | Reserved for "an oracle returned `Critical`" |
| `64` | `EX_USAGE` | Bad CLI arguments, unknown subcommand, missing required flag |
| `65` | `EX_DATAERR` | Input file malformed (parse failure, schema mismatch, upcaster chain rejection) |
| `66` | `EX_NOINPUT` | Input file not found / not readable |
| `70` | `EX_SOFTWARE` | Internal bug (impossible state hit) |
| `73` | `EX_CANTCREAT` | Output destination cannot be created |
| `74` | `EX_IOERR` | Read/write I/O failure |
| `78` | `EX_CONFIG` | Configuration invalid (matches Vector's `validate` precedent) |

The crate `sysexits` is used at the binary entry point for typed
exit codes. Rust's automatic `101` exit on panic stays as a
last-resort signal that a bug escaped (`70`/`EX_SOFTWARE` is
preferred where the bug was caught).

A command's `--help` lists its non-zero exit codes in a dedicated
`EXIT CODES:` section.

### 7. Help text

- `trace --help` shows top-level synopsis, subcommands grouped by
  noun (`analyze`, `oracle …`, `plan …`, `report …`, `export …`,
  utility), global options, exit-code legend, link to mdBook docs.
- `trace <cmd> --help` shows the command's synopsis, args, flags,
  examples (≥ 2), and the relevant subset of exit codes.
- Examples in help are real, runnable, and asserted in CI via
  `trycmd` (snapshot-test crate) — they cannot drift from reality.

### 8. Shell completions

`clap_complete` powers `trace completions {bash,zsh,fish,powershell}`,
emitting to stdout (idiom from `kubectl completion`, `gh completion`,
`fly completions`). The mdBook installation chapter documents the
shell-specific install paths.

### 9. Configuration

A `trace.toml` file is **opt-in** (no auto-discovery surprises):
only loaded if `--manifest-path` is given or `TRACE_MANIFEST` env
is set. It carries defaults for global options, named oracle packs,
default storage backend feature. The schema is versioned exactly
like the wire schema (ADR-0006).

### 10. Error reporting (clig.dev §errors)

Errors emitted with the structure:

```
error: <one-line summary>

  <details — what the user did, what we expected, where>

help:  <what to try next>
docs:  <relative URL into the mdBook>
```

`miette` is the chosen crate for spans-into-input-text style
diagnostics (Rust-ecosystem standard, used by `oxc`, `tauri`,
`cargo-llvm-cov`, etc.). `thiserror` for typed errors stays per
ADR-0012; `miette` wraps them at the binary boundary.

### 11. JSON mode contract

When the user passes `--output json`:

- All structured data goes to stdout as a single JSON document
  (or NDJSON if `--stream` is also set).
- Diagnostics go to stderr as **plain text** unless the user adds
  `--log-format json` (then stderr is NDJSON too).
- No ANSI colour anywhere.
- No interactive prompts (anywhere `--yes` / `--non-interactive`
  is needed for the operation, it must be supplied or the binary
  exits `64`).
- The JSON schema for each command's `--output json` payload is
  versioned with its own upcaster chain (ADR-0006 applies). A `trace
  output-schemas` subcommand emits the JSON schemas for every
  subcommand's structured-output shape.

### 12. Acceptance tests

The CLI's behavioural contract is anchored in CI by:

- `trycmd` snapshots for every subcommand (success + each error
  class).
- A "noun-verb consistency" test that walks the help tree and
  asserts the grammar from §3.
- An exit-code smoke test that drives every error class and
  asserts the table from §6.
- A "no contextless output" test that runs every command with
  `--output json` and validates against the per-command schema
  published by `trace output-schemas`.

## Consequences

- Users from the kubectl / gh / cargo / Vector world meet a CLI
  with the conventions they expect; no relearning.
- Scripts can rely on stable exit codes and on `--output json`
  schemas being versioned and breakable only via explicit upcaster
  bumps.
- snapshot-tested help text means docs cannot rot relative to
  actual behaviour — at the cost of more CI churn when help text
  legitimately changes.
- `miette` adds a non-trivial dependency to `trace-cli`. Acceptable:
  it never reaches `trace-core`; it lives only in the binary crate.
- The disciplined output-stream split (stdout/stderr/json) forces
  careful design for any command that does *both* a useful thing
  and prints progress. Worth it — pipelines are the primary use
  case in CI.
- `--output json` schemas have their own upcaster chain. Extra
  surface to maintain, but the alternative is breaking downstream
  scripts on every minor release.

## See also

- External philosophy / spec: [clig.dev](https://clig.dev/),
  [POSIX XBD §12](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html),
  [GNU CLI Standards](https://www.gnu.org/prep/standards/html_node/Command_002dLine-Interfaces.html),
  [`sysexits.h`](https://man.freebsd.org/cgi/man.cgi?query=sysexits&sektion=3),
  [no-color.org](https://no-color.org/).
- External Rust-specific: [Rain's Rust CLI recommendations](https://rust-cli-recommendations.sunshowers.io/),
  [Rust CLI book — Exit codes](https://rust-cli.github.io/book/in-depth/exit-code.html),
  [`clap`](https://docs.rs/clap), [`clap_complete`](https://docs.rs/clap_complete),
  [`miette`](https://docs.rs/miette), [`trycmd`](https://docs.rs/trycmd),
  [`sysexits`](https://github.com/sorairolake/sysexits-rs).
- External worked examples: [Vector CLI](https://vector.dev/docs/reference/cli/),
  [kubectl conventions](https://kubernetes.io/docs/reference/kubectl/conventions/),
  [GitHub CLI manual](https://cli.github.com/manual/), Cargo.
- Internal: ADR-0006 (upcaster chain — applied to `--output json`
  shapes), ADR-0007 (consent prompt for `trace export --raw`),
  ADR-0011 (projection table — drives which subcommands exist),
  ADR-0012 (library-side companion).
- Stages with CLI surface that this ADR governs:
  [S2](../stages/S2-jsonl-storage-and-cli-skeleton.md),
  [S4](../stages/S4-action-graph-and-heuristics-miner.md),
  [S5](../stages/S5-oracle-engine-and-builtin-rules.md),
  [S7](../stages/S7-demo-app-and-mvp-pipeline.md),
  [S8](../stages/S8-sqlite-backend-and-inductive-miner.md),
  [S9](../stages/S9-parquet-archive-tier.md),
  [S11](../stages/S11-replay-planner-semantic-monkey-and-trace-mutation.md),
  [S12](../stages/S12-v1-0-stable-release.md).
