# glossary.md — SemantxTrace project glossary

> Purpose: single source of truth for terms used by humans and LLM agents
> working on `SemantxTrace`.
>
> Rule of use: if a term appears in the spec, an ADR, a stage doc, code, or
> an agent task, check this file first. Do not invent competing definitions.
> If a term is missing or ambiguous, extend this file rather than creating
> synonyms in code.

## 0. Project terms

### SemantxTrace
The behavioral observability project for desktop UI apps: a system that
records named semantic actions with scenario context, fans them out into
seven projections (analytics, diagnostic, regression test, UX, product,
support replay, exploration — see §0/`behavioral observability` and
ADR-0011), and never depends on physical UI selectors at the schema
layer. Work-title; the GitHub repository is `PhysShell/SemantixTrace`
for legacy reasons.

### behavioral observability
The frame the project occupies. Observability for *how users actually
behave* inside a desktop UI — not server-side tracing (OpenTelemetry's
domain) and not contextless counters ("button clicked 123 times"). One
canonical artefact (the semantic trace) feeds seven projections; see
the projection table below and ADR-0011.

### scenario observability
Synonym for `behavioral observability` used in user-facing copy where
"behavioral" sounds too clinical. Both terms refer to the same thing.

### semantic metric
A measurement derived from semantic traces that preserves scenario
context: not just *how many*, but *in which scenario*, *after which
steps*, *with which outcome*. Opposite of `contextless counter`.
Examples: "Top-N workflows by frequency", "Average steps to
`Graph47.Recalculate` post-redesign vs pre-redesign", "Sessions
ending in `ErrorModal` after `Export`".

### contextless counter
The anti-pattern SemantxTrace replaces. A count of an action with no
record of the scenario it sat in (`btnExport.clicked: 1200`). Useful
for capacity planning, useless for understanding behaviour. Banned as
a recording shape inside the trace; aggregate counts may be derived
from traces post-hoc by the analytics projection.

### projection (of a trace)
A named consumer of the canonical semantic trace, fixed by ADR-0011.
Seven exist at v1.0:

| Projection | Question it answers |
|---|---|
| `analytics` | Which scenarios run in production, and how often? |
| `diagnostic` | What did the user do before this crash? |
| `regression-test` | Can we replay this scenario and assert what should happen? |
| `ux` | Where do users back out, retry, or get stuck? |
| `product` | Which features are dead? Which got worse after the redesign? |
| `support-replay` | Reproduce the customer's bug step-by-step. |
| `exploration` | What neighbouring scenarios might also be broken? |

A projection is *not* a new schema; every projection reads `Current`
through `read_event` (ADR-0006).

### semantic action map
The model of an application as a set of named domain actions
(`Graph47.Recalculate`) and their relationships, independent of any
physical UI representation. The defendable angle of the project (ADR-0005).

### physical UI map
The model of an application as a tree of visual elements with positions,
bounds, and selectors (AutomationId, XPath). Changes on every redesign.
Lives only in adapter code; never in the trace schema.

### production-informed testing
Generating regression tests from real user sessions captured in production,
rather than hand-writing tests against expected interactions. The category
Meticulous occupies for web; SemantxTrace targets it for desktop.

### desktop UI
Applications that run as native processes on Windows / macOS / Linux,
notably WPF, WinForms, Avalonia, MAUI desktop, Qt. Web and mobile are out
of scope for v1.0.

### oracle
A function over a trace (or a window of a trace) that decides whether the
session satisfies a domain invariant. See §5.

### replay plan
A portable JSON description of a scenario that an adapter can execute
against a built UI. Operates on semantic IDs (`Graph47.Recalculate`),
resolved per-adapter to physical interactions. See §6.

### MVP
Minimum Viable Product. In `SemantxTrace`: an end-to-end pipeline from
recording (WPF demo) through JSONL ingest, normalization, action graph,
oracle evaluation, and HTML report. Closes at S7. Not "a throwaway".

### vertical slice
An end-to-end piece of functionality through every layer of the system.

### roadmap stage
A development stage `S0`, `S1`, … Each stage has a goal, tasks, acceptance
criteria, tests, and constraints.

### S0
Baseline workspace stage. Establish the Cargo workspace, lint policy, CI,
`unsafe_code = "forbid"`, ADR/stage process. No domain code yet.

### S1
`trace-core` + `trace-schema` v1 stage. Define `TraceEvent` and friends,
publish the v1 JSON Schema, wire up the upcaster machinery with the
identity upcaster from `V1` to `Current`.

### S2
JSONL storage + CLI skeleton stage. Implement `JsonlBackend`, the
`StorageBackend` trait, and the `trace` CLI with `analyze` / `version`
subcommands.

### S3
`trace-normalizer` stage. Value abstraction (numeric buckets, string
classes), temporal abstraction (`BurstAction`, `SessionPause`),
equivalence classes, scenario folding.

### S4
Action graph + Heuristics miner stage. `ActionGraph` over petgraph 0.8.x,
Heuristics-miner implementation, Mermaid + DOT export, anomaly detection.

### S5
Oracle engine + built-in rules stage. `OracleRule` trait,
`OracleSchedule`, composition (`AndRule`, `OrRule`, `WithinWindowRule`),
five built-in rules (see §5).

### S6
WPF adapter stage. `Trace.Wpf` NuGet: `[TraceCommand]`,
`[ScreenId]`, `AutoAutomationId` attached behavior, `TracedRelayCommand`
decorator, JSONL sink, weak-event subscriptions.

### S7
Demo app + MVP pipeline stage. `DeclarationApp.Demo` (4–6 screens, 3
intentional bugs), full pipeline `record → analyze → graph → oracle →
report`, GIF and demo video, mdBook docs site. Closes v1.0-MVP / PH-launch.

### S8
SQLite analysis backend + Inductive miner stage. `SqliteBackend` (feature
`sqlite`), `inductive-miner` IMDF implementation, scenario-folding
improvements driven by real corpus feedback. Closes v0.2.

### S9
Parquet archive tier stage. `ParquetBackend` (feature `parquet`) via
`arrow` + `parquet`, zstd compression, DuckDB-compatible layout. Closes
part of v0.3.

### S10
Avalonia adapter stage. `Trace.Avalonia` library: same `[TraceCommand]` /
`[ScreenId]` story over Avalonia's `ICommand` + `AutomationProperties`.
Closes v0.3.

### S11
Replay planner + semantic-monkey + trace mutation stage.
`trace-replay-planner`: scenario → `ReplayPlan` JSON with two explicit
modes (`strict` for bug reproduction and regression assertions,
`relaxed` / `normalized` for analytics and test-candidate selection);
semantic-monkey traversal over the action graph for coverage-guided
exploration; domain-aware trace mutation as a generation path. Closes
v0.4.

### S12
v1.0 stable release stage. API freeze on `trace-core` and `trace-schema`,
at least two upcaster chains in production, fuzz corpora green for 30+
consecutive nightlies, semver guarantees published, mdBook deployed,
crates.io release.

## 1. Architecture and data model

### hexagonal architecture
The architectural style adopted by the project (ADR-0002). Domain logic
in `trace-core` and the inner crates depends only on traits (ports);
concrete I/O lives in adapters that depend on the ports, not the other
way round.

### port
A trait declared in `trace-core` (or a near-core crate) that decouples
domain logic from a specific implementation. Examples: `StorageBackend`,
`EventSource`, `OracleRule`, `ReplayAdapter`, `Reporter`.

### adapter
A concrete implementation of a port, typically gated behind a feature flag
or living in a separate crate. Examples: `JsonlBackend`, `SqliteBackend`,
`ParquetBackend`, `WpfReplayAdapter`, `HtmlReporter`.

### `trace-core`
The kernel crate. Holds `TraceEvent`, `Session`, `Scenario`, value-object
newtypes, and the port traits. Dependencies: `serde`, `serde_json`,
`chrono`, `uuid` — and nothing else.

### `trace-schema`
Crate owning the versioned wire schema. Hosts the per-version event types
(`v1::TraceEvent`, `v2::TraceEvent`, …), the `Upcaster` trait, the
`From`-based upcaster chain, the `read_event` dispatch, and the published
JSON Schema files.

### `trace-storage`
Crate hosting `StorageBackend` plus `JsonlBackend` (always on), and
feature-gated `SqliteBackend`, `ParquetBackend`. Reads always go through
the upcaster chain so callers only ever see `Current`.

### `trace-normalizer`
Crate that turns a raw `Session` into a normalized one: value abstraction,
temporal abstraction, equivalence classes, scenario folding. Pure and
deterministic.

### `trace-graph`
Crate that builds an `ActionGraph` from normalized scenarios, runs miners
(Heuristics in MVP, Inductive from v0.2), exports Mermaid / DOT.

### `trace-oracle`
Crate hosting the `OracleRule` trait, the `OracleContext`, scheduling,
composition, and the built-in rules.

### `trace-replay-planner`
Crate (v0.4) that turns a scenario into a `ReplayPlan` JSON document
(with explicit `strict` / `relaxed` modes), provides semantic-monkey
exploration over the action graph, and hosts the trace-mutation
generators (S11).

### `trace-cli`
Binary `trace`. Subcommands: `analyze`, `normalize`, `graph`,
`oracle run`, `plan generate`, `report`. `clap` 4.x with derive macros.

### `trace-viewer`
TUI viewer (`ratatui`); deferred to post-v1.0. Not on the v1.0 critical
path.

### `trace-wpf`
.NET / NuGet adapter for WPF. Defines `[TraceCommand]`, `[ScreenId]`,
`AutoAutomationId`, `TracedRelayCommand`, `ITraceContext`, the JSONL sink.

### `trace-avalonia`
.NET adapter for Avalonia (v0.3). Same pattern as `trace-wpf`.

### `trace-maui`
.NET adapter for MAUI. Marked **experimental** in v1.0; MAUI automation
peers are immature.

### bounded context
A subset of the domain with its own ubiquitous language. SemantxTrace
recognises five: trace ingestion, normalization, graph analysis, oracle
evaluation, replay planning.

### DDD-lite
Domain-Driven Design used pragmatically: value objects, aggregates,
ubiquitous language, bounded contexts; no CQRS, no event-sourcing-as-DB,
no enterprise-grade saga orchestration.

### type-driven design
Encoding invariants in the type system: newtype wrappers (`SessionId`,
`CommandId`, `ScreenId`, `FieldId`), phantom-typed lifecycle states
(`Trace<Raw>` / `Trace<Normalized>` / `Trace<Analyzed>`), sealed traits
for closed sets such as schema versions.

## 2. Trace events

### TraceEnvelope
The on-wire wrapper around a trace event. Carries `schemaVersion` so the
upcaster chain can dispatch correctly. The wire form is a single line of
JSON per event (JSONL).

### schemaVersion
Integer version tag attached to every envelope, e.g. `"schemaVersion":1`.
Lives in the envelope, not on individual events. Bumped only when a
breaking change is unavoidable (ADR-0006).

### TraceEvent
The domain event seen by all code outside `trace-schema`. Always
`trace_schema::Current`, the highest schema version known to the binary
at build time.

### TraceEventV1
The first concrete version of the event type, defined in
`trace_schema::v1`. Frozen forever after v1.0.

### EventSeq
A monotonic per-session counter. Typed as a newtype around `u64`. Gaps
indicate dropped events.

### SessionId
A newtype around `uuid::Uuid`. Identifies a recording session.

### CorrelationId
Optional newtype around `uuid::Uuid` linking related events (e.g. the
start and end of an async operation, a command and its post-modal).

### TraceEventKind
The discriminated union of concrete event shapes. v1 set:

- `ScreenOpened { screen_id, params }`
- `CommandExecuted { command_id, args, duration_ms, outcome }`
- `FieldChanged { field_id, old, new }`
- `ExceptionThrown { exception_type, message, stack }`
- `NavigationOccurred { from, to }`
- `ValidationFailed { validator, field_id, reason }`
- `AsyncOperationCompleted { operation_id, duration_ms, outcome }`

### CommandId
Newtype around `String` for the semantic name of a command
(`Graph47.Recalculate`).

### ScreenId
Newtype around `String` for the semantic name of a screen
(`DeclarationEditor`).

### FieldId
Newtype around `String` for the semantic name of a field (`Quantity`,
`Customer.Iin`).

### Outcome
Result of a command/async operation: `Success`, `Failure(String)`,
`Cancelled`, `TimedOut`. Domain-meaningful, not HTTP-style.

### ValuePolicy
The privacy-aware wrapper around a value (see §8): `Raw`, `Masked`,
`Bucketed`, `Hashed`, `Removed`.

### domain event
A bizdomain-level event recorded by the adapter (`Declaration.Validated`,
`Payments.Computed`) — as opposed to a low-level UI event
(`Button.Click`). High signal-to-noise. Preferred shape for oracle rules.

### UI event
A physical-layer event such as a click or keystroke. Discouraged as a
first-class trace shape; only used in adapters when no domain event
exists.

## 3. Sessions and scenarios

### Session
An ordered, non-empty sequence of `TraceEvent`s with a single `SessionId`.
Aggregate root for the recording bounded context.

### Scenario
The normalized, domain-meaningful unit derived from a session: a sequence
of canonical actions with deterministic data abstraction. Aggregate root
for the normalization bounded context.

### canonical action
A `(screen_id, command_id, abstract_args)` triple, where
`abstract_args` is value-abstracted (see §4). Two events fold into the
same canonical action iff their triples are equal.

### scenario folding
The transformation `Session → Scenario`: bucket values, abstract
temporals, drop noise, collapse equivalent events. Idempotent under the
property test `normalize(normalize(t)) == normalize(t)`.

### canonical action sequence
The ordered list of canonical actions inside a scenario. The mining
substrate.

## 4. Workflow mining and action graph

### value abstraction
Replacing concrete values with stable buckets so equivalent flows fold:

- numerics → buckets `0`, `1`, `2–10`, `11–100`, `101–1000`, `1000+`;
- strings → `{length_bucket, format_class}` (email-like, GUID-like,
  numeric, free-text);
- dates → relative buckets (`past_week`, `past_month`, `future`).

### temporal abstraction
Replacing micro-timing with stable shapes:

- consecutive events with `Δt < 50 ms` collapse into a single
  `BurstAction`;
- idle gaps `> 5 s` produce a `SessionPause` marker.

### equivalence class
The set of events that produce the same canonical action. Driven by value
+ temporal abstraction.

### ActionGraph
A directed graph (`petgraph::graph::DiGraph<ActionNode, Transition>`)
built from canonical action sequences across all scenarios in a corpus.

### ActionNode
A graph node: `{ screen_id, action_id }`. Identity is the pair, not the
graph index.

### Transition
A directed edge between two action nodes carrying `frequency: u64`,
`avg_duration_ms: f64`, and `error_rate: f32`.

### Heuristics miner
The MVP workflow-mining algorithm (S4). Frequency-driven, threshold-based,
tolerant of noise. Borrowed from PM4Py / ProM, reimplemented in Rust.

### Inductive miner
The v0.2 workflow-mining algorithm (S8). IMDF variant. Top-down, returns a
sound process tree. The industry default.

### Alpha miner
A classic process-mining algorithm. **Not used**: brittle on loops and
noise (see issue rationale §2.F).

### PrefixSpan
A sequence-mining algorithm. Implemented in `trace-graph` for motif
discovery. Roughly 200 lines of Rust; useful as a showcase implementation.

### sequence mining
The class of algorithms that find frequent subsequences in a set of
sequences. PrefixSpan, SPADE, GSP. PrefixSpan in MVP; others
opportunistic.

### anomaly detection
The process of flagging transitions or nodes that are unusually rare or
absent from the normalized graph. Driven by frequency thresholds (MVP),
clustering (post-v1.0).

### workflow clustering
Grouping scenarios by similarity over canonical action sequences
(Levenshtein on bigrams, Jaccard on action sets) and applying k-medoids.
Optional; not on the v1.0 critical path.

### error-prone subgraph
A subgraph identified by reverse BFS from `ExceptionThrown` nodes,
weighted by contribution to errors. Surface in `trace report`.

### most-frequent path
A path through the action graph chosen by Dijkstra with inverse-frequency
edge weights. Used in `trace graph` exports.

### Mermaid export
Graph rendering as a Mermaid `flowchart` block for README inclusion.

### DOT export
Graphviz `dot` source for higher-fidelity SVG rendering.

## 5. Oracles

### OracleRule
A trait implemented by anything that judges whether a trace satisfies an
invariant. Methods: `name`, `evaluate`, `schedule`.

### OracleSchedule
When an oracle runs: `PerEvent`, `PerScenario`, `EndOfSession`,
`WindowBased { window_ms }`.

### OracleResult
Pass/fail with `severity` (`Info` / `Warning` / `Error` / `Critical`),
`message`, and `evidence` (a vector of event references).

### OracleContext
Mutable state passed across events for a stateful oracle (e.g., the
`WithinWindowRule` keeps a sliding window).

### built-in oracle
One of the rules shipped in `trace-oracle` (S5):

- `NoUnhandledException` — no `ExceptionThrown` in the session.
- `NoErrorModalAfterCommand` — no error-modal screen appears within N
  events after a successful command.
- `CommandMustSucceed` — a named command returns `Outcome::Success`.
- `ScreenMustNavigateForward` — after a named command, navigation occurs
  to an expected screen.
- `ValidationsPassBeforeSubmit` — no `ValidationFailed` after the latest
  submit-equivalent command.

### domain oracle
A user-defined `OracleRule` registered by the application using the
library. Domain oracles live in the application's code, not in
`trace-oracle`. The MVP rejects in-trace embedded rule scripts; rules are
compiled Rust.

### rule composition
Combinators for oracles: `AndRule`, `OrRule`, `WithinWindowRule(inner,
ms)`. Allow building complex invariants out of primitives.

### severity
`Info` / `Warning` / `Error` / `Critical`. Drives reporter behavior
(coloring, exit codes, alerting).

### evidence
The list of event references attached to an `OracleResult` so a reviewer
can navigate from a failure to its underlying events.

## 6. Replay plans

### ReplayPlan
A JSON document describing a portable scenario for replay. Owns
`scenario`, `expectedOutcome`, `steps`, `preconditions`,
`dataDependencies`. Serialized via `trace-schema`'s own upcaster chain
(plans have their own version line).

### Step
One operation in a `ReplayPlan`: `OpenScreen`, `ExecuteCommand`,
`SetField`, `Wait`, `Assert`. Each step may carry attached oracles.

### precondition
A condition that must hold before a plan runs (e.g. seed data present,
specific feature flag on). Adapters check preconditions; failure aborts
the run.

### data dependency
A declared dependency on a field's abstract value. Adapters bind real
values from a fixture pool that matches the abstract bucket.

### ReplayAdapter
Trait implemented per UI framework. Resolves semantic IDs to physical
interactions, drives the app, and records the resulting trace for
comparison.

### tolerance
The allowed deviation for non-deterministic timings (`expectedDuration ±
tolerance_pct`) and for expected modals (`expectedModals: [...]`).

### replay mode
A `ReplayPlan` carries a `mode: ReplayMode` field. Two values exist:

- **`strict`** — reproduce the recorded scenario step-by-step in the
  recorded order. Used for bug reproduction, regression assertions,
  and customer-support replay. Failure to reach the expected state is
  the test result.
- **`relaxed`** (a.k.a. **`normalized`**) — collapse irrelevant
  orderings (e.g. fill order of independent fields) according to the
  normalizer's equivalence classes. Used for analytics, clustering,
  and test-candidate selection — operations that care about the
  *shape* of the scenario, not its byte-exact reproduction.

The two modes are not the same code path with a flag flipped; they are
distinct operations with distinct determinism contracts. SPEC hard
rule 16 forbids merging them.

### strict replay
See `replay mode`. The mode for "reproduce *this* recorded session".

### relaxed replay
See `replay mode`. The mode for "execute *a* scenario equivalent to
this recorded class of sessions". Also called `normalized replay`.

### semantic-monkey exploration
v0.4 feature: coverage-guided walks over the `ActionGraph`, optionally
constrained by oracles, producing fresh scenarios that exercise
rarely-visited transitions. "Semantic" because the walks operate on
canonical actions, not on physical UI selectors, and respect the
domain-meaning of each transition. (The name `smart-monkey` is banned
per §19; `semantic-monkey` is the canonical term.)

### trace mutation
A first-class generation path (S11): take a recorded scenario, apply
a domain-aware mutation, run the result through the same oracle engine
that judged the original. Mutations are typed and named — `swap_field
_order`, `replace_with_boundary_value`, `skip_optional_step`,
`repeat_command`, `navigate_back`, `replace_document_type`,
`shuffle_independent_blocks` — and they live in the planner's own
versioned schema (its own upcaster chain). Distinguished from
`semantic-monkey` exploration: the monkey *walks the graph* generating
fresh sequences; mutation *transforms an existing sequence* preserving
the bulk of its structure.

### domain-aware mutation testing
The broader name for what `trace mutation` does: mutate inputs in ways
that make sense in the domain (not random byte-flipping), then check
whether oracle rules still hold. Borrowed terminology from mutation
testing of code (Stryker, PIT, cargo-mutants), but the artefact being
mutated is a *scenario*, not a source-code AST.

### diagnostic package
The bundle a user, support engineer, or operator exports for offline
analysis: a trace file plus app version, schema version, dependency
versions, and any attached `OracleResult` evidence. The
support-projection equivalent of a `ReplayPlan`. Always built through
the same `--raw` consent / audit-log path documented in
[`privacy.md`](privacy.md).

## 7. Storage tiering

### StorageBackend
Port in `trace-core`. Methods: `append`, `iter`. Implementations may add
backend-specific query methods behind extension traits.

### JsonlBackend
Default backend: one JSON object per line, append-only, no indexes. Read
path always goes through the upcaster chain. Compression via external
zstd (`*.jsonl.zst`).

### SqliteBackend
v0.2 backend: rows in a single `events` table keyed by
`(session_id, seq)`, with `schema_version` as an indexed column for cheap
filtering. `rusqlite` 0.31+ with the `bundled` feature.

### ParquetBackend
v0.3 backend: Parquet files for archival and analytics, partitioned by
day. `arrow` + `parquet`. zstd / snappy compression.

### Apache Arrow
Unifying columnar layer (`arrow-rs`) used to convert JSONL → RecordBatch
→ Parquet. Read paths for SQLite and Parquet build typed query
projections on top.

### DuckDB
Mentioned only because it reads Parquet out of the box; SemantxTrace does
not depend on DuckDB. Operators may use it externally for ad-hoc
analysis.

### compression
zstd for JSONL archives (level 3, balance); Parquet uses snappy or zstd.
No compression for in-flight JSONL (latency over size).

### at-rest encryption
Out of scope for v1.0. When added (post-v1.0), preferred crate: `age`
(X25519 + ChaCha20-Poly1305).

### diagnostic package
A `--raw` export bundling events with `ValuePolicy::Raw`. Requires an
explicit consent prompt and writes an audit-log entry naming the
exporter, the session ids, and the timestamp.

## 8. Privacy and masking

### ValuePolicy
Five-armed enum: `Raw(Value)`, `Masked(String)`, `Bucketed { bucket }`,
`Hashed { hash, algo }`, `Removed`.

### mask-by-default
Hard rule (SPEC §5): all string values are recorded as `Masked` unless
explicit per-field policy promotes them to a less-sensitive variant.

### bucket-by-default
Hard rule: numeric values default to `Bucketed` using the policy table in
§4 (value abstraction).

### PII
Personally identifiable information. Detected via regex (emails, phones
E.164, IBANs with mod-97 check, credit cards with Luhn, Kazakhstan IIN /
BIN 12-digit + checksum). ML-based detection is **not** in v1.0
(glossary §17.4).

### IIN
Kazakhstan Individual Identification Number; 12 digits with a checksum.
Treated as PII by default.

### BIN
Kazakhstan Business Identification Number; same shape as IIN.

### consent prompt
The interactive confirmation shown by the CLI before a `--raw` diagnostic
export proceeds. Suppressing it requires `--yes-i-have-consent` plus an
audit log entry.

### audit log
An append-only record of who exported what raw data and when. Lives in
`./audit.log` by default; configurable.

### GDPR data minimization
GDPR Article 5(1)(c). The mask-by-default policy operationalises this for
SemantxTrace.

## 9. UI adapters

### ReplayAdapter
See §6.

### adapter capability matrix
A table per adapter listing supported event kinds, supported replay
steps, and known gaps. Lives in each adapter's README. WPF capability is
the v1.0 baseline; other adapters declare deviations explicitly.

### WPF adapter
`trace-wpf` (`Trace.Wpf` NuGet). The reference adapter. See §10.

### Avalonia adapter
`trace-avalonia`. v0.3. Same MVVM-and-attributes story as WPF, executed
against Avalonia's `ICommand` and `AutomationProperties`.

### MAUI adapter
`trace-maui`. Marked **experimental** in v1.0: MAUI's cross-platform
automation peers are still maturing.

### Web adapter
Planned post-v1.0. Bound to Playwright via `data-testid` and ARIA roles.
Out of scope for the 14-week MVP.

### FlaUI
.NET wrapper around UIA2/UIA3. Used by the WPF replay adapter for
physical interactions. Active, MIT-style license.

### WinAppDriver
Microsoft's Appium driver for Windows. Future is unclear (no recent
releases). FlaUI is the chosen path.

## 10. WPF specifics

### MVVM
Model-View-ViewModel. The mandatory pattern for SemantxTrace-friendly WPF
apps: every domain action is an `ICommand` on a ViewModel.

### ICommand
WPF's interface for invokable commands. The unit of semantic action
recording.

### `[TraceCommand]`
Attribute applied to ICommand properties on a ViewModel:
`[TraceCommand("Graph47.Recalculate")]`. Source-generator wraps the
command with `TracedRelayCommand`.

### `[ScreenId]`
Attribute applied to View classes:
`[ScreenId("DeclarationEditor")]`. On `Loaded`, the adapter emits
`ScreenOpened { screen_id: "DeclarationEditor", … }`.

### AutoAutomationId
Attached behavior that walks the visual tree on `Loaded` and assigns
`AutomationProperties.AutomationId = "{ScreenId}.{x:Name}"` to every
named control. Provides the physical fallback when no semantic command
applies.

### TracedRelayCommand
Decorator over `ICommand` that emits `CommandExecuted` events with
`CorrelationId`, captures duration, and turns exceptions into
`ExceptionThrown` while preserving rethrow semantics.

### ITraceContext
The DI-injected interface ViewModels call to emit events (`StartCommand`,
`EndCommand`, `NotifyFieldChanged`, …). The release-build implementation
is a no-op.

### CommandManager
WPF's static class coordinating routed commands. Not used as an
interception point; `TracedRelayCommand` decorates explicitly.

### WeakEventManager
WPF's mechanism for weak event subscriptions. Mandatory for trace
listeners on long-lived UI objects to prevent memory leaks.

### AutomationId
UIA's machine-stable identifier. Per Microsoft's documentation, must be
the same across locales and unique among sibling elements. SemantxTrace
generates them via `AutoAutomationId`.

### AutomationProperties.Name
Localised, human-readable name (accessibility). Distinct from
`AutomationId`.

### Stateless (library)
`github.com/dotnet-state-machine/stateless`. Recommended for modelling
workflow state machines. Each transition fires `NotifyTransition`, which
becomes a high-signal trace event.

### CommunityToolkit.Mvvm source generators
Used to back the `[TraceCommand]` attribute with generated wrapping
without runtime reflection. Optional; manual `TracedRelayCommand` is
also supported.

### FluentValidation
Recommended (but not mandatory) validation library. Each named validator
becomes a `ValidationFailed` event on failure.

## 11. .NET interop / FFI

### domain-injected oracle
An oracle whose logic lives in the consumer's `.NET` code and is invoked
from Rust via FFI. **Not in v1.0.** Built-in oracles only.

### netcorehost
Rust crate hosting CoreCLR. Candidate for v1.0+ when domain-injected
oracles land.

### LibraryImport
.NET 7+ replacement for `DllImport`. Used by `trace-wpf` if Rust DLL
calls are needed.

### gRPC / Named Pipes
Alternative to in-process FFI. Higher latency, cleaner separation.
Considered for offline analysis later; out of scope for MVP.

### Framework Design Guidelines (FDG)
Microsoft's canonical conventions for .NET library authors: Naming,
Type Design, Member Design, Designing for Extensibility, Exceptions,
Usage Guidelines, Common Design Patterns. Online at
<https://learn.microsoft.com/en-us/dotnet/standard/design-guidelines/>.
Mandatory baseline for every NuGet SemantxTrace publishes (ADR-0013).

### .NET Library Guidance
Microsoft's practical companion to FDG, specifically for NuGet
authors: multi-targeting, versioning, breaking changes, source
linking, package metadata, deterministic builds. Online at
<https://learn.microsoft.com/en-us/dotnet/standard/library-guidance/>.

### CA-rule
A single rule emitted by the built-in .NET code analyzers. The set
is enabled wholesale via `AnalysisMode=AllEnabledByDefault` per
ADR-0013. Examples: `CA1062` (validate args of public methods),
`CA1063` (implement IDisposable correctly), `CA1303` (no string
literals as localizable), `CA2007` (use `ConfigureAwait`).

### `AnalysisMode=AllEnabledByDefault`
The `.csproj` property that turns on every CA-rule as a build
warning. Combined with `TreatWarningsAsErrors=true` it becomes the
.NET analog of the Rust workspace's `clippy::pedantic -D warnings`
(ADR-0012 + ADR-0013).

### `EnablePackageValidation`
Microsoft's built-in binary-compatibility checker. Compares the
about-to-be-packed assembly against the previously published version
and fails the build on accidental ABI/API breaks. The .NET analog
of `cargo-public-api` / `cargo-semver-checks` from ADR-0012.

### `Microsoft.CodeAnalysis.PublicApiAnalyzers`
Roslyn analyzer that tracks the public surface of an assembly via
two checked-in text files: `PublicAPI.Shipped.txt` (released surface)
and `PublicAPI.Unshipped.txt` (next-release deltas). Any change to a
`public` member fails CI until the unshipped file is updated.
Mandatory per ADR-0013 §4.

### `Trace.Abstractions`
The contract-only NuGet package. Holds `ITraceContext`,
`ValuePolicy`, `[TraceCommand]`, `[ScreenId]`, `[TraceField]`.
Zero third-party dependencies, ABI-frozen after v1.0 — the .NET
analog of `trace-core` discipline (ADR-0002, ADR-0013).

### Sentry-style package split
The package layout pattern: a small contract package plus
per-framework / per-sink implementation packages
(`Sentry` + `Sentry.AspNetCore` + `Sentry.Serilog`;
`OpenTelemetry.Api` + `OpenTelemetry`). Adopted for `Trace.*`
per ADR-0013 §5.

## 12. Rust and workspace

### Rust workspace
A group of crates in one repo. SemantxTrace's workspace (after S0):
`trace-core`, `trace-schema`, `trace-storage`, `trace-normalizer`,
`trace-graph`, `trace-oracle`, `trace-replay-planner`, `trace-cli`,
`trace-viewer`.

### crate
A Rust package/library/binary unit.

### `Cargo.toml`
Rust workspace / crate manifest: dependencies, package metadata,
profiles, lints.

### `rust-toolchain.toml`
Pins the toolchain and components: stable, rustfmt, clippy.

### MSRV
Minimum Supported Rust Version. SemantxTrace targets stable Rust 1.80+
(set at S0; do not bump without a decision).

### clippy
The Rust linter. Configured workspace-wide with `-D warnings`.

### rustfmt
The Rust code formatter.

### `unsafe_code = "forbid"`
Workspace-wide lint enforced by `Cargo.toml [workspace.lints.rust]`. The
isolated `fuzz/` crate is the only exception (ADR-0010).

### serde
Serialization framework. The only allowed serialization mechanism in
`trace-schema`.

### serde_json
JSON backend for serde. Used for JSONL.

### chrono
Date/time crate. Used for `TraceEvent.ts: DateTime<Utc>`.

### uuid
UUID crate. Backs `SessionId` and `CorrelationId`.

### petgraph
Graph crate. Pinned to 0.8.x (ADR-0008) until 0.9 stabilises.

### `clap`
CLI argument parser. v4.x with derive macros, in `trace-cli` only.

### tracing
Structured logging crate. The library uses it for its own observability;
**do not confuse** with the trace-event-recording domain.

### tracing-opentelemetry
Bridge to OTLP. Used in `trace-cli` for export to an OTLP collector when
configured.

### `thiserror`
Ergonomic error types. Used throughout.

### `Result`
Rust success/error type. All fallible operations return errors, not
panic.

### `Option`
Rust optional-value type (e.g. `CorrelationId`).

### newtype
A wrapper around a primitive: `SessionId(uuid::Uuid)`, `EventSeq(u64)`,
`CommandId(String)`. Provides type safety.

### sealed trait
A trait whose private `Sealed` marker prevents downstream impls. Used to
keep the set of schema versions closed.

### phantom type
A zero-sized type used as a type parameter to encode state (e.g.
`Trace<Raw>` vs `Trace<Normalized>` vs `Trace<Analyzed>`).

### Rust API Guidelines
The Rust project's official checklist of conventions for crate
authors, hosted at <https://rust-lang.github.io/api-guidelines/>.
Mandatory for every `pub` item in a SemantxTrace crate published to
crates.io (ADR-0012). Each rule has the form `C-<SHORTNAME>` (e.g.
`C-CONV`, `C-SEALED`, `C-NON-EXHAUSTIVE`).

### C-* check
A single rule in the API Guidelines. Pre-v1.0 audit walks the full
checklist crate-by-crate.

### `C-COMMON-TRAITS`
"Eagerly implement common traits": every public type derives `Debug`,
`Clone`, `PartialEq`, `Eq`, `Hash` where the data permits, `Default`
where meaningful, `Display` for stringly-rendered newtypes. Project
baseline.

### `C-GOOD-ERR`
"Error types are meaningful and well-behaved": error types implement
`std::error::Error`, are `Send + Sync + 'static`, expose `source` via
`#[from]` / `#[source]`. We satisfy via `thiserror`.

### `C-SEALED`
"Sealed traits protect against downstream implementations." Mandatory
for `trace_schema::Upcaster` (downstream impls would break the
version-dispatch invariant); recommended for any closed-set trait.
The pattern: a `pub` trait with a `pub(crate)` `Sealed` supertrait.

### `C-NON-EXHAUSTIVE`
"Data structures do not duplicate derived trait bounds." For us the
practical rule: public enums that may grow post-v1.0 carry
`#[non_exhaustive]` (`Outcome`, `OracleSchedule`, `Severity`,
`MutationKind`, `ReplayMode`). Per-version event enums
(`v1::TraceEventKind`, …) do **not** carry it — they are frozen
forever by ADR-0006 and the attribute would impose a downstream
default-arm cost for no benefit.

### `C-NEWTYPE`
"Newtypes encapsulate implementation details." Already baseline for
domain identifiers (`SessionId`, `CommandId`, `ScreenId`, `FieldId`,
`EventSeq`).

### `C-VALIDATE`
"Functions validate their arguments." Wire-boundary functions
(`read_event`, `JsonlBackend::iter`, `OracleRule::evaluate`,
`plan_from`) validate and return typed errors; no `unwrap()` on
user-controlled data outside `fuzz/`.

### `C-DEBUG` / `C-DEBUG-NONEMPTY`
"All public types implement `Debug`" and "`Debug` representation is
never empty." Both enforced by the `missing_debug_implementations`
lint plus review.

### `C-DOC` / `C-EXAMPLE` / `C-FAILURE` / `C-PANIC-DOC`
"All items have a rustdoc example" / "Function docs include error /
panic / safety considerations" / "Hyperlinks point to other rustdoc
items." Enforced by `missing_docs` lint and `cargo doc -D warnings`.

### `C-METADATA`
"Cargo.toml includes all common metadata." Inherited via
`[workspace.package]` from S0; per-crate manifests override only
`description`.

### cargo-public-api
External tool that diffs the public API of a crate between two builds
or git revisions. Used in CI from S12 onwards to catch accidental
semver-breaking changes (ADR-0012 §4).

### cargo-semver-checks
External tool that lints intended version bumps for semver
compatibility against the previous release. Used at S12 release
ritual.

### missing_docs
`rustc` lint that warns when a `pub` item has no doc comment. Enabled
workspace-wide at S0 per ADR-0012.

### pedantic clippy
The `clippy::pedantic` lint group. Enabled workspace-wide as `warn`
(promoted to `deny` in CI via `-D warnings`). Per-item `#[allow(...)]`
requires a comment.

### clig.dev
The Command Line Interface Guidelines (<https://clig.dev/>). The
canonical modern philosophy reference for CLI design. Authoritative
for `trace-cli` (ADR-0014).

### POSIX Utility Conventions
The formal standard for CLI argument parsing and stream behaviour
(XBD §12, <https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html>).
What `-x`, `--`, `-`, short-flag clustering mean.

### GNU CLI Standards
Long-options style (`--help`, `--version`), `--help` output format,
mandatory information items
(<https://www.gnu.org/prep/standards/html_node/Command_002dLine-Interfaces.html>).

### `sysexits.h`
The standard exit-code table from BSD (`EX_USAGE=64`,
`EX_DATAERR=65`, `EX_NOINPUT=66`, `EX_SOFTWARE=70`,
`EX_CONFIG=78`, …). Adopted by SemantxTrace per ADR-0014 §6.
Vector uses `78` for `vector validate` failures, which is the
direct precedent for our `trace` command exit shape.

### Rain's Rust CLI recommendations
Practical Rust-specific CLI gradient document by Rain Sunshowers
(maintainer of `cargo-nextest`), at
<https://rust-cli-recommendations.sunshowers.io/>. Covers handling
arguments / subcommands, exit codes, versioning, configuration.

### noun-verb subcommand
The subcommand structure pattern adopted from kubectl / gh / docker
/ Vector: `trace <noun> <verb>` (`trace plan generate`,
`trace oracle run`, `trace report workflows`). ADR-0014 §3.

### `NO_COLOR`
The de-facto environment-variable standard (<https://no-color.org/>)
for disabling ANSI colour in terminal output. Honoured by
`trace-cli` per ADR-0014 §4.

### `trycmd`
Rust crate for snapshot-testing CLI behaviour from `*.md` /
`*.toml` files. Used in `crates/trace-cli/tests/` to bless help
text, success / error outputs, and example invocations
(ADR-0014 §12).

### `miette`
Rust crate for rich, span-aware diagnostics (errors that point at a
position in source/input text). Used at the `trace-cli` binary
boundary to wrap typed `thiserror` errors per ADR-0014 §10.

### `clap_complete`
Companion crate to `clap` that generates shell-completion scripts
(`bash`, `zsh`, `fish`, `powershell`). Powers
`trace completions <shell>` per ADR-0014 §8.

## 13. TDD, property tests, fuzzing

### TDD
Test-Driven Development: red → green → refactor. Mandatory for
`trace-core`, `trace-schema`, `trace-normalizer`, `trace-oracle`.

### red
A test fails because the feature is not implemented or a bug is
confirmed.

### green
A minimal implementation makes the test pass.

### refactor
Improving structure without changing behavior.

### characterization test
A test that pins existing/legacy behavior. Required before refactoring
anywhere; not required at first writing of a new module.

### golden test
A test against a reference file/output. Snapshots for CLI output and
normalized scenarios.

### snapshot test
A test comparing output to a stored snapshot. Hand-rolled re-blessing
via `TRACE_BLESS=1` (no `insta` dependency, mirroring griff's
`decisions.log` entry).

### unit test
A test of a small function/module.

### integration test
A test of several components together: record → ingest → normalize, CLI
→ output, WPF demo → trace file.

### property-based test
Testing properties over generated inputs. `proptest` 1.x. Required for
core invariants — schema roundtrip, normalizer idempotency, upcaster
chain.

### proptest
Rust property-based testing crate.

### upcaster property test
`forall v_n_event, upcast_to_current(parse(serialize(v_n_event)))` must
succeed and equal `upcast_to_current(v_n_event)`. Required for every
schema version.

### normalizer idempotency
`normalize(normalize(t)) == normalize(t)`. Required property.

### oracle commutativity
For stateless oracles, evaluation order does not change the set of
results. Required property where applicable.

### insta
Snapshot-testing crate. **Not used**; we hand-roll snapshots for the same
reason griff did (small dep graph, strict `cargo-deny`).

### cargo-nextest
Fast test runner. Optional; not on the MSRV path.

### cargo-mutants
Mutation-testing tool. Recommended post-v1.0 for measuring test strength;
not a v1.0 gate.

### fuzz testing
Feeding automatically generated inputs to a target to find panics, hangs,
unbounded allocations, or invalid internal models. Mandatory for the
JSONL parser, the upcaster chain, and selected normalizer transforms
(ADR-0010, `fuzzing.md`).

### fuzz target
A small entry point a fuzzer drives, with an oracle. In SemantxTrace:
`fuzz/fuzz_targets/{jsonl_parse, upcaster_v1_to_current, normalize_fold,
…}.rs`.

### cargo-fuzz
Standard Rust fuzzing harness around libFuzzer. SemantxTrace's chosen
tool; lives in the isolated nightly `fuzz/` crate.

### libfuzzer-sys
Rust bindings exposing the libFuzzer engine; the `fuzz_target!` macro
crate.

### arbitrary
Rust crate turning raw bytes into typed structures. Used for
structure-aware fuzzing of normalizer transforms and upcaster inputs.

### structure-aware fuzzing
Fuzzing typed inputs (via `arbitrary`) rather than raw byte slices. Used
for `normalize_fold`, `upcaster_v1_to_current`, `oracle_replay`.

### fuzz corpus
The committed input set for a target: `fuzz/corpus/<target>/`.

### seed corpus
Hand-picked starting inputs (minimal valid traces) that bootstrap a
target.

### regression corpus
Minimised crash/hang inputs committed permanently so a fixed bug stays
fixed.

### fuzz oracle
The pass/fail contract for a target: no panic / no hang / no unbounded
alloc / typed-error xor success / target-specific invariants.

### hang
Input that makes the target not return. Caught by libFuzzer `-timeout`.

### uncontrolled allocation
Input that drives unbounded memory growth (zip-bomb-style). Bounded by
libFuzzer `-rss_limit_mb` / `-malloc_limit_mb`.

### CI gate
A condition required to accept a PR: tests green, clippy green, format
green, snapshots reviewed, bounded smoke fuzz + regression replay green.

## 14. Documentation and agent development

### AGENTS.md
Instruction file for AI agents in the repo. Short, concrete, links to
docs.

### CLAUDE.md
Agent instruction file for Claude-style tooling. Bridges to `AGENTS.md`.

### SPEC.md
The main project spec: what SemantxTrace does, does not do, and which
architectural rules are mandatory.

### ADR
Architecture Decision Record: a short doc fixing an important decision —
context, decision, consequences (Nygard format, ADR-0009).

### stage doc
A per-stage doc: goal, inputs/outputs, approach, acceptance criteria,
open questions, see also.

### upcasters.md
The pattern doc for schema evolution. Authoritative reference for how
new schema versions are introduced.

### privacy.md
The privacy / masking policy doc.

### fuzzing.md
The fuzz-testing policy doc.

### decisions.log.md
Append-only Y-statement log for small decisions that don't warrant a
full ADR.

### Definition of Done
Task completion criteria. For SemantxTrace: tests green, docs updated,
property tests cover any new invariant, schema bumps documented in
`upcasters.md`, no hidden behavior changes, ADR or `decisions.log.md`
entry when applicable.

### prompt rot
When long instructions go stale and start to harm. Cured by short docs,
ADRs, stage files.

### context window
The limited text an LLM can hold in a request. Hence a glossary, not an
endless scroll.

### knowledge base
The set of docs, glossary, ADRs, specs, and stage files an agent relies
on.

### mdBook
The chosen docs-site generator. Plain Markdown, single binary, rendered
output deployed at v1.0 (S12).

## 15. Quality, risks, constraints

### technical debt
Debt in code/architecture that speeds up now and slows down later.

### architectural risk
Risk that the chosen model will not survive future use cases (e.g.
adding domain-injected oracles forces an FFI surface that hexagonal
boundaries did not anticipate).

### format risk
Risk that an external format (Apache Arrow, Parquet) introduces breaking
API changes in a dependency.

### dependency churn
The pace at which third-party crates introduce breaking changes.
SemantxTrace mitigates by pinning major versions (`petgraph` 0.8.x).

### semantic loss
Loss of meaning during conversion (e.g. masking a free-text field for
privacy means the diagnostic value is reduced).

### backward compatibility
Preserving readability of old recordings via the upcaster chain.
**Required forever** after v1.0 for `trace-schema`.

### migration path
The path between schema versions. Always realised via upcasters; never as
in-place storage rewrites.

### feature flag
A Cargo feature gating optional functionality (`sqlite`, `parquet`,
`avalonia`). Required for any backend that brings a heavy dependency.

### experimental support
A feature is present, but API/behavior stability is not promised
(`trace-maui`).

### stable support
A feature covered by tests, docs, fixtures, fuzz targets where
applicable, and acceptance criteria (`trace-wpf`).

### fail-fast
Fail quickly and explicitly on unsupported input instead of silently
producing nonsense.

### graceful degradation
Partial support with a warning when full support is impossible (e.g. an
event references an unknown screen — the report flags it but does not
crash).

### observability
The ability to understand what happened inside SemantxTrace itself:
debug dumps, summaries, ingest warnings, normalizer reports.

### normalized dump
A dump without unstable details (paths, random ids, map iteration
order). Required for snapshot tests.

## 16. Quick map: term → where it lives

- `TraceEvent`, `Session`, `Scenario`, `EventSeq`, `SessionId` →
  `trace-core` value objects.
- `TraceEnvelope`, `schemaVersion`, `Upcaster`, `read_event` →
  `trace-schema` (versioned wire format).
- `StorageBackend`, `JsonlBackend`, `SqliteBackend`, `ParquetBackend` →
  `trace-storage`.
- `value abstraction`, `temporal abstraction`, `scenario folding` →
  `trace-normalizer`.
- `ActionGraph`, `ActionNode`, `Transition`, `Heuristics miner`,
  `Inductive miner`, `PrefixSpan` → `trace-graph`.
- `OracleRule`, `OracleResult`, `OracleSchedule`, built-in rules →
  `trace-oracle`.
- `ReplayPlan`, `Step`, `precondition`, `data dependency`,
  `replay mode`, `strict replay`, `relaxed replay`,
  `semantic-monkey exploration`, `trace mutation`,
  `domain-aware mutation testing` → `trace-replay-planner`.
- `behavioral observability`, `scenario observability`,
  `semantic metric`, `contextless counter`, `projection (of a trace)`
  → cross-cutting; defined in §0, fixed by ADR-0011.
- `diagnostic package` → `trace-cli` (`trace export`); semantics fixed
  in [`../privacy.md`](privacy.md).
- `Rust API Guidelines`, `C-* check`, `C-SEALED`, `C-NON-EXHAUSTIVE`,
  `C-COMMON-TRAITS`, `C-GOOD-ERR`, `C-VALIDATE`, `cargo-public-api`,
  `cargo-semver-checks`, `missing_docs`, `pedantic clippy` →
  cross-cutting; defined in §12, fixed by ADR-0012; enforced from S0,
  audited at S12.
- `Framework Design Guidelines`, `.NET Library Guidance`, `CA-rule`,
  `AnalysisMode=AllEnabledByDefault`, `EnablePackageValidation`,
  `Microsoft.CodeAnalysis.PublicApiAnalyzers`, `Trace.Abstractions`,
  `Sentry-style package split` → .NET adapter surface; defined in
  §11, fixed by ADR-0013; enforced from S6 (`Trace.Wpf`), audited
  at S12.
- `clig.dev`, `POSIX Utility Conventions`, `GNU CLI Standards`,
  `sysexits.h`, `Rain's Rust CLI recommendations`,
  `noun-verb subcommand`, `NO_COLOR`, `trycmd`, `miette`,
  `clap_complete` → `trace-cli` binary surface; defined in §12,
  fixed by ADR-0014; enforced from S2 (CLI skeleton), audited at
  S12.
- `CLI subcommands`, `trace analyze`, `trace graph`, `trace oracle run`
  → `trace-cli`.
- `[TraceCommand]`, `[ScreenId]`, `AutoAutomationId`,
  `TracedRelayCommand`, `ITraceContext`, `WeakEventManager` →
  `trace-wpf`.
- `ValuePolicy`, `mask-by-default`, `IIN`, `BIN`, `audit log` →
  `trace-core` + privacy policy (`privacy.md`).
- `Heuristics miner`, `Inductive miner`, `PrefixSpan` → `trace-graph`
  (algorithms, all hand-implemented in Rust).
- `TDD`, `snapshot`, `property test`, `mutation testing`,
  `cargo-fuzz`, `fuzz oracle` → development process (`AGENTS.md`,
  `fuzzing.md`).
- `ADR`, `SPEC.md`, `AGENTS.md`, `upcasters.md` →
  documentation/process.

## 17. Rules for the LLM agent

1. **Never put physical UI selectors into the trace schema.** AutomationId
   and XPath belong in the adapter. The schema, the oracle rules, and the
   replay plans speak in `CommandId`, `ScreenId`, `FieldId`.
2. **Never branch on `schema_version` outside `trace-schema::upcasters`.**
   Domain code reads `Current`. The upcaster chain is the single place
   where versions are visible.
3. **Never rewrite stored JSONL to upgrade its schema version.** Add an
   upcaster step, point `Current` at the new version, leave the old
   files untouched.
4. **Never record raw string values without explicit policy.** Default is
   `ValuePolicy::Masked`.
5. **Never add dependencies to `trace-core` beyond `serde`, `serde_json`,
   `chrono`, `uuid`.** That crate's dependency closure is part of the
   contract.
6. **Never improvise a `trace-replay-planner` step shape.** Add the new
   step to the planner's own versioned schema and update the upcaster
   chain.
7. **Never add an oracle that depends on `unsafe`, FFI, or threads beyond
   `Send + Sync`.** Oracles must be portable and deterministic.
8. **Never use `eprintln!` in library crates.** Use `tracing::{warn, info,
   debug}` for the library's own observability.
9. **Never name a new event kind after a UI control type.**
   `ButtonClicked` is a code smell; introduce a `CommandId` instead.
10. **Never start implementation on a stage whose docs are not green.**
    Update `docs/stages/SN-*.md` first; commit it; then write the code.
11. **Never add a `pub` item to a published crate without a doc
    comment** — `missing_docs` will fail CI. If the item is genuinely
    internal, mark it `pub(crate)`.
12. **Never un-seal `Upcaster` or remove `#[non_exhaustive]`** from a
    published enum without an ADR. Both are semver-relevant per
    ADR-0012 (`C-SEALED`, `C-NON-EXHAUSTIVE`).
13. **Never silence a clippy `pedantic` lint with a bare `#[allow]`.**
    Add a comment explaining why; reviewers may push back.
14. **Never put a non-`Send + Sync` type on a `pub` trait method
    signature** in a port crate (`trace-core`, `trace-oracle`,
    `trace-storage`) without explicit doc justification. The
    project's downstream consumers expect to fan oracles / backends /
    upcasters across threads.

## 18. Preferred naming

Use these as defaults until an ADR decides otherwise:

`TraceEvent`, `TraceEnvelope`, `TraceEventKind`, `Session`, `SessionId`,
`Scenario`, `EventSeq`, `CommandId`, `ScreenId`, `FieldId`,
`CorrelationId`, `Outcome`, `ValuePolicy`, `Upcaster`, `Current`,
`StorageBackend`, `JsonlBackend`, `ActionGraph`, `ActionNode`,
`Transition`, `OracleRule`, `OracleSchedule`, `OracleResult`,
`OracleContext`, `ReplayPlan`, `ReplayMode`, `Step`, `ReplayAdapter`,
`TraceMutation`, `SemanticMonkey`, `Projection`,
`TracedRelayCommand`, `AutoAutomationId`, `ITraceContext`.

## 19. Terms to avoid or use carefully

### "AI" / "smart"
Banned in marketing copy unless concretely backed. The historical
working name `smart-monkey` is **renamed to `semantic-monkey`**
(decision log 2026-05-27) — there is no exception any longer.

### "smart-monkey"
**Renamed**. Use **`semantic-monkey`** everywhere. Old occurrences in
the literature (if any) refer to the same construct.

### "works fine"
Banned as self-soothing. If it works fine, show tests, fixtures,
property invariants, and acceptance criteria.

### "just a quick hack"
A danger sign. Spikes are allowed but must be marked as spikes and not
dragged into production core without cleanup.

### "MIDI articulation"
Not relevant to this project. Listed here only to short-circuit
copy-paste from griff docs.

### "session replay"
Used in the literature primarily for web/mobile (rrweb, Datadog Session
Replay). SemantxTrace's `Session` is structurally similar but the
recording layer differs (semantic actions, not DOM mutations). Prefer
**scenario** when talking about post-normalization artefacts;
**session replay** is acceptable for the `support-replay` and
`diagnostic` projections (ADR-0011) when the audience expects the
phrase.

### "process mining"
Used in literature for business-analyst dashboards (Celonis, PM4Py,
ProM). SemantxTrace borrows the algorithms (Heuristics miner,
Inductive miner) and the goal (understanding real workflows) but its
artefacts are technical (action graphs, replay plans, oracle reports)
rather than spreadsheet-shaped KPI panels. Prefer **workflow mining**
in user-facing copy when the audience is developer-centric;
**process mining** is fine when the audience expects it.

### "model-based testing"
Academically loaded. Use **scenario replay** or **plan-driven testing**
in user-facing copy. ADRs and stage docs may use the precise term.

### "contextless counter"
The anti-pattern term, see §0. Use it pejoratively in user-facing
copy: it is the foil for SemantxTrace's `semantic metric` framing.
Never describe a SemantxTrace artefact *as* a contextless counter;
that would be a self-own.

### "click tracking" / "button-press metrics"
Avoid. These phrases place SemantxTrace in the category it explicitly
rejects (counters without scenarios). Use **scenario-aware metrics**,
**workflow metrics**, **semantic metrics** instead.

### "schema migration"
Banned as a phrase when describing how SemantxTrace evolves the wire
format. Use **upcaster chain** (see `upcasters.md`). Migrations imply
in-place rewrites, which the architecture explicitly rejects.

### "real-time" / "live monitoring"
Out of scope for v1.0. Recording is local-buffer-then-flush; analysis is
batch. Do not promise sub-second feedback.

### "deterministic replay"
Qualify carefully. SemantxTrace replay is **semantically** deterministic
(same `CommandId` sequence), not **pixel-accurately** deterministic
(timing, layout, modal behaviour can drift).

## 20. Definition of Done for glossary changes

A `glossary.md` change is done if:

- the new term has a short definition;
- it states how the term is used in `SemantxTrace`;
- if ambiguous, a warning is added;
- if it replaces an old name, the preferred name is stated;
- agent/stage docs do not use contradicting definitions;
- if it introduces or renames a public type, an ADR backs the change.
