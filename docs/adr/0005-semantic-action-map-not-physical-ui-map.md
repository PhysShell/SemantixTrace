# ADR 0005: The trace lives at the semantic action map, not the physical UI map

Date: 2026-05-27
Status: Accepted

## Context

Every existing desktop test automation tool we surveyed (TestComplete,
Ranorex, Squish, WinAppDriver, FlaUI, Sikuli) couples tests to physical
UI selectors: AutomationId, XPath, visual-tree paths, image hashes. The
consequence is that any UI redesign — a button moved into a submenu, a
panel rebuilt as a tab, a dialog promoted to a screen — breaks every
test that touches the affected region. This is exactly why every
record-and-replay tool in this category needs a "self-healing locators"
feature, and self-healing is exactly as reliable as it sounds.

A trace that records `Window.Grid.StackPanel[1].Button[3].Click` cannot
survive a redesign by definition: the path encodes layout, not intent.
A trace that records `Graph47.Recalculate` survives any layout change,
because the command's domain meaning is unchanged.

The same observation drives ADR-0006: schema evolution must keep
historical traces readable because the domain meaning of recorded
sessions outlives the wire format.

## Decision

The trace schema, the oracle rules, and the replay plans operate on
**semantic IDs only**: `CommandId`, `ScreenId`, `FieldId`. Physical
selectors (`AutomationId`, XPath, bounds, image hashes) live only in
adapter code (`trace-wpf`, `trace-avalonia`, …) where they are needed to
resolve semantic IDs to real interactions during replay.

The WPF adapter enforces this by convention: every domain action is an
`ICommand` decorated with `[TraceCommand("Graph47.Recalculate")]`; every
view is annotated with `[ScreenId("DeclarationEditor")]`; physical
AutomationIds are auto-generated via an `AutoAutomationId` attached
behavior so the developer never names them by hand. Where no
domain-meaningful command exists for an interaction, the trace records
no event — the silence is a signal that the application is not
trace-ready.

We intentionally accept that some interactions cannot be recorded
semantically (pure visual hover, drag-resize of a panel) and treat that
as out-of-scope rather than as a reason to fall back to physical paths.

## Consequences

- Tests, oracles, and replay plans survive UI redesigns. This is the
  product's central marketing claim and the single defendable angle vs
  TestComplete / Meticulous-for-desktop competitors.
- Application authors must adopt MVVM with named commands and annotated
  views to benefit. Code-behind-heavy legacy apps are not viable
  customers without restructuring. The `DeclarationApp.Demo` shows the
  conformant pattern.
- A "minimum traceability checklist" (in [`../glossary.md`](../glossary.md)
  §10 and the WPF adapter README) becomes the customer-facing contract.
- Tools that would auto-generate semantic IDs from physical interactions
  (a tempting feature) are explicitly **not built**: they would
  reintroduce the brittleness they claim to solve.
