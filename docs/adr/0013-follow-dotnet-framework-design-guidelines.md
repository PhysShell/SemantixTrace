# ADR 0013: Follow .NET Framework Design Guidelines and library best practices on adapter NuGets

Date: 2026-05-27
Status: Accepted

## Context

SemantxTrace ships .NET libraries to NuGet that customers reference
from production WPF / Avalonia / MAUI applications. Once published,
these packages carry binary-compatibility commitments the same way
the crates.io Rust crates do (ADR-0012) — but to a different audience
with different conventions and tooling. Inconsistency with the
established .NET library style (`Sentry`, `OpenTelemetry`,
`Microsoft.Extensions.*`, `Serilog`, `MassTransit`) is read as
amateur hour and blocks adoption regardless of what the Rust core
does.

The .NET ecosystem has a deep formal standard for this — the
[Framework Design Guidelines](https://learn.microsoft.com/en-us/dotnet/standard/design-guidelines/)
(Cwalina, Abrams; Microsoft Learn) — plus a practical companion
([.NET Library Guidance](https://learn.microsoft.com/en-us/dotnet/standard/library-guidance/))
that maps the abstract rules to concrete `.csproj` knobs, and an
enforcement layer (the built-in
[.NET code-analysis rules](https://learn.microsoft.com/en-us/dotnet/fundamentals/code-analysis/quality-rules/),
~400+ `CA*` analyzers).

Direct architectural precedents:

- **Sentry .NET SDK** ships as `Sentry` (core) plus per-framework
  packages (`Sentry.AspNetCore`, `Sentry.Extensions.Logging`,
  `Sentry.Serilog`, `Sentry.NLog`). Multi-targeted (NetStd 2.0 +
  .NET 8.0 + .NET Framework 4.6.2). Same shape we need for
  `Trace.Abstractions` + `Trace.Wpf` + `Trace.Avalonia`.
- **OpenTelemetry .NET** splits `OpenTelemetry.Api` (the contract,
  semver-stable) from `OpenTelemetry` (the SDK, faster cadence).
  API and SDK packages are versioned **independently**. Same pattern
  applies to our split: `Trace.Abstractions` is the ABI-frozen
  contract, the per-framework adapters carry implementations that
  can evolve.

## Decision

1. **Scope.** Every .NET package SemantxTrace publishes to NuGet is
   bound by this ADR. v1.0 publishing list:
   - `Trace.Abstractions` (interfaces, attributes, `ValuePolicy`,
     `ITraceContext`; .NET Standard 2.0);
   - `Trace.Wpf` (WPF-specific; .NET Framework 4.7.2 + .NET 8.0-windows);
   - `Trace.Avalonia` (cross-platform; .NET 8.0); ships at S10;
   - `Trace.Maui` (experimental; .NET 8.0); post-v1.0 cadence.

   Demo apps (`examples/DeclarationApp.Demo*`) are out of scope.

2. **Reference documents.** The full Microsoft Framework Design
   Guidelines (Naming, Type Design, Member Design, Designing for
   Extensibility, Exceptions, Usage Guidelines, Common Design
   Patterns) plus
   [.NET Library Guidance](https://learn.microsoft.com/en-us/dotnet/standard/library-guidance/)
   plus
   [NuGet package authoring best practices](https://learn.microsoft.com/en-us/nuget/create-packages/package-authoring-best-practices)
   apply by default. This ADR enumerates the project-specific
   commitments on top of them.

3. **Mandatory `.csproj` properties** (Directory.Build.props at
   `adapters/`):

   ```xml
   <PropertyGroup>
     <LangVersion>latest</LangVersion>
     <Nullable>enable</Nullable>
     <ImplicitUsings>enable</ImplicitUsings>
     <EnableNETAnalyzers>true</EnableNETAnalyzers>
     <AnalysisMode>AllEnabledByDefault</AnalysisMode>
     <AnalysisLevel>latest</AnalysisLevel>
     <TreatWarningsAsErrors>true</TreatWarningsAsErrors>
     <WarningsNotAsErrors></WarningsNotAsErrors>
     <EnforceCodeStyleInBuild>true</EnforceCodeStyleInBuild>
     <GenerateDocumentationFile>true</GenerateDocumentationFile>
     <Deterministic>true</Deterministic>
     <ContinuousIntegrationBuild Condition="'$(CI)' == 'true'">true</ContinuousIntegrationBuild>
     <PublishRepositoryUrl>true</PublishRepositoryUrl>
     <EmbedUntrackedSources>true</EmbedUntrackedSources>
     <IncludeSymbols>true</IncludeSymbols>
     <SymbolPackageFormat>snupkg</SymbolPackageFormat>
     <EnablePackageValidation>true</EnablePackageValidation>
     <GenerateCompatibilitySuppressionFile>true</GenerateCompatibilitySuppressionFile>
   </PropertyGroup>
   ```

   `AnalysisMode=AllEnabledByDefault` opts the package into the full
   CA-rule set as warnings; `TreatWarningsAsErrors=true` promotes
   them to merge-blockers. This is the direct .NET analog of the
   Rust workspace's `clippy::pedantic -D warnings` from ADR-0012.
   `EnablePackageValidation` activates Microsoft's built-in
   binary-compatibility check against the previous published
   version — the .NET analog of `cargo-public-api` / `cargo-
   semver-checks` from ADR-0012.

4. **PublicApi analyzers.** Every published package references
   [`Microsoft.CodeAnalysis.PublicApiAnalyzers`](https://github.com/dotnet/roslyn-analyzers)
   and commits `PublicAPI.Shipped.txt` + `PublicAPI.Unshipped.txt`
   to the repository. Any change to the public surface fails CI
   until the unshipped file is updated in the PR — the .NET equivalent
   of the `cargo-public-api` baseline.

5. **Package structure** (Sentry-style split):

   ```
   adapters/
   ├── Directory.Build.props          # the property block above
   ├── Trace.Abstractions/            # contract; NetStd 2.0; ABI-frozen v1.0
   │   ├── ITraceContext.cs
   │   ├── ValuePolicy.cs
   │   ├── TraceCommandAttribute.cs
   │   ├── ScreenIdAttribute.cs
   │   └── TraceFieldAttribute.cs
   ├── Trace.Wpf/                     # NetFx 4.7.2 + net8.0-windows
   ├── Trace.Avalonia/                # net8.0; S10
   └── Trace.Maui/                    # net8.0; post-v1.0, experimental
   ```

   `Trace.Abstractions` has *zero* third-party dependencies and is
   ABI-frozen after v1.0 — the .NET equivalent of the Rust
   `trace-core` discipline (ADR-0002, ADR-0012). Implementations
   live in the framework-specific packages.

6. **Multi-targeting.**
   - `Trace.Abstractions`: `netstandard2.0` (broadest reach for the
     interface contract).
   - `Trace.Wpf`: `net472;net8.0-windows` (legacy WPF still runs on
     .NET Framework in production; both must work).
   - `Trace.Avalonia`: `net8.0` (cross-platform from day one).
   - `Trace.Maui`: `net8.0` (current MAUI baseline).

7. **Package metadata.** Every `.csproj` sets `PackageId`,
   `Description`, `Authors`, `Company`, `Copyright`,
   `RepositoryUrl`, `RepositoryType`, `PackageLicenseExpression`
   (MIT for v1.0), `PackageProjectUrl`, `PackageTags`,
   `PackageReadmeFile` (per-package README packed into the
   `.nupkg`), `PackageIcon`. Inherited via
   `Directory.Build.props` where possible.

8. **API design** (the parts of FDG most relevant for us):
   - **Naming**: PascalCase types/members, camelCase parameters,
     `I`-prefix for interfaces, `Async` suffix for async methods,
     `Attribute` suffix for attributes (FDG `Naming`).
   - **Type design**: prefer interfaces for ports; sealed for
     concrete classes unless explicit extensibility is wanted
     (FDG `Class vs Interface`; CA1052, CA1724).
   - **Member design**: virtual only when designed to be overridden
     (FDG `Member Design`); properties for state, methods for
     behaviour; no `out` params except `Try*` pattern (CA1021).
   - **Exceptions**: throw `ArgumentNullException` /
     `ArgumentException` on validation; never `Exception`
     directly (CA1031, CA2200); preserve stack on rethrow.
   - **Disposability**: implement `IDisposable` / `IAsyncDisposable`
     for any class owning unmanaged resources or background workers
     (`FileJsonlTraceContext` will); standard dispose pattern
     (CA1063, CA1816).
   - **`Equals` / `GetHashCode`**: implement together; for value
     types prefer `record struct` (CA1067).
   - **`Span<T>` / `Memory<T>` for hot paths** (event-emission loop
     in `FileJsonlTraceContext`).

9. **Strong naming** is **not** required for v1.0. Microsoft's own
   guidance has shifted to "strong-name only if you have a specific
   reason"; Sentry / OpenTelemetry follow suit. We can add later if
   a downstream demands it.

10. **CI enforcement.** GitHub Actions matrix builds on Windows
    (for `Trace.Wpf` UI tests) and Linux (for `Trace.Abstractions` +
    `Trace.Avalonia` + library compile-checks of `Trace.Wpf` against
    .NET 8.0-windows reference assemblies):
    - `dotnet build -c Release -warnaserror`,
    - `dotnet test -c Release` (Windows-only for WPF UI suites),
    - `dotnet pack -c Release` (verifies PackageValidation),
    - `dotnet format --verify-no-changes`,
    - `Microsoft.CodeAnalysis.PublicApiAnalyzers` baseline check
      (fails on uncommitted public-surface deltas).

11. **Documentation contract.** Every public type / member carries
    an XML doc comment (`<summary>`, `<param>`, `<returns>`,
    `<exception>` where applicable). `GenerateDocumentationFile=true`
    causes the .NET analyzer `CS1591` ("missing XML doc") to fire on
    any undocumented public surface, treated as error by
    `TreatWarningsAsErrors`. Same level of discipline as the Rust
    `missing_docs` lint (ADR-0012).

12. **README per package.** Each `.csproj` packs a per-package
    `README.md` into the `.nupkg`. The README contains: a Quick
    start that copy-pastes into a working WPF / Avalonia /
    MAUI sample; the conformant adoption checklist (the
    "minimum traceability checklist" from glossary §10); a link
    to the canonical project docs.

13. **Sample compilability.** Each package's Quick start sample
    lives under `examples/`-projects and is built in CI to ensure
    docs never lie.

## Consequences

- The .NET surface looks and behaves like an idiomatic modern
  library (Sentry-shaped). Customers can adopt it without learning
  SemantxTrace-specific quirks.
- `AnalysisMode=AllEnabledByDefault` + `TreatWarningsAsErrors=true`
  forces a high upfront discipline level. Some CA-rules will be
  legitimately wrong for our case (e.g. CA1014 `CLSCompliantAttribute`
  on the assembly — not relevant for a 4.7.2 + .NET 8 library); we
  suppress them in `.editorconfig` with a comment, not silently.
- `PackageValidation` blocks accidental binary-compat breaks at
  build time, before they reach NuGet.
- `Trace.Abstractions` discipline (no third-party deps,
  semver-frozen) costs us flexibility but pays the same dividend as
  `trace-core` discipline (ADR-0002): downstream code can reference
  the contract package without dragging in framework-specific
  baggage.
- PublicAPI baseline files become part of every PR that touches
  public types — extra reviewer load, but accidental SemVer breaks
  become impossible.
- Strong-naming-off means downstream apps cannot use the GAC, but
  no modern .NET deployment uses the GAC anyway.

## See also

- External canonical:
  - [Framework Design Guidelines (Microsoft Learn)](https://learn.microsoft.com/en-us/dotnet/standard/design-guidelines/)
  - [.NET Library Guidance](https://learn.microsoft.com/en-us/dotnet/standard/library-guidance/)
  - [NuGet package authoring best practices](https://learn.microsoft.com/en-us/nuget/create-packages/package-authoring-best-practices)
  - [Code analysis rules (Microsoft Learn)](https://learn.microsoft.com/en-us/dotnet/fundamentals/code-analysis/quality-rules/)
  - [Configure code analysis (Microsoft Learn)](https://learn.microsoft.com/en-us/dotnet/fundamentals/code-analysis/configuration-options)
  - [`dotnet/roslyn-analyzers`](https://github.com/dotnet/roslyn-analyzers)
    (in particular `Microsoft.CodeAnalysis.PublicApiAnalyzers`).
- External worked examples:
  - [Sentry .NET SDK](https://github.com/getsentry/sentry-dotnet)
    (package split, multi-targeting).
  - [OpenTelemetry .NET](https://github.com/open-telemetry/opentelemetry-dotnet)
    (API/SDK independent versioning).
- Internal:
  - ADR-0002 (hexagonal — `Trace.Abstractions` is the .NET-side
    ports surface).
  - ADR-0005 (semantic action map — drives the attribute design).
  - ADR-0006 (wire schema is the cross-language contract).
  - ADR-0007 (privacy default — drives `[TraceField]` attribute).
  - ADR-0012 (the Rust-side counterpart).
  - [`../stages/S6-wpf-adapter.md`](../stages/S6-wpf-adapter.md),
    [`../stages/S10-avalonia-adapter.md`](../stages/S10-avalonia-adapter.md),
    [`../stages/S12-v1-0-stable-release.md`](../stages/S12-v1-0-stable-release.md).
