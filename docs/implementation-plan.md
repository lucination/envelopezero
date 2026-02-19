# EnvelopeZero — General Implementation Plan

This plan operationalizes the design doc into a pragmatic execution track.

## Project Objective

Ship a product people can use daily now, while preserving a clean path to stronger budgeting semantics later.

Guiding constraints:
- Fast iteration
- Stable canonical data
- Evolvable computed layer
- No irreversible shortcuts

---

## Stage 1 — Data Contract Stabilization (Foundation) ✅

Detailed scope/evidence: `docs/stage-1-data-contract.md`

### Purpose
Protect canonical user-entered facts as the long-term source of truth.

### Scope
- Final audit of canonical write models:
  - transaction headers
  - transaction details
  - assignments (when present)
  - auth/session events
- Validate transaction/detail invariants:
  - every transaction has >= 1 detail
  - no orphan details
  - no cross-user/cross-budget references
- Ensure PillID-first external API contract consistency.

### Why this stage matters
If this layer is clean, derived math and UX can evolve safely.

### Exit Criteria
- No endpoint bypasses canonical model
- Invariants enforced at service and DB layers where practical
- PillIDs stable across all read/write APIs

---

## Stage 2 — Core Workflow Reliability (MVP Usability Core) ✅

Detailed scope/evidence: `docs/stage-2-core-workflow.md`

### Purpose
Make the daily workflow dependable before visual polish.

### Scope
- Stabilize critical user loops:
  - sign in
  - accounts/categories management
  - transaction + detail create/edit/delete
- Harden empty/error/loading behavior with actionable messages
- Validate mobile baseline usability (forms, taps, layout flow)

### Why this stage matters
Behavioral trust is required before deeper features.

### Exit Criteria
- Core loops pass consistently in browser E2E
- No dead-end UX states in primary flows
- Manual smoke from login to transaction entry succeeds without workaround

---

## Stage 3 — Derived Projection Hardening (Evolvable, not rigid) ✅

Detailed scope/evidence: `docs/stage-3-derived-projection.md`

### Purpose
Provide reliable computed values now without overcommitting to final accounting semantics.

### Scope
- Standardize computed read paths:
  - dashboard totals
  - category activity/availability projections
- Isolate projection logic so fixes happen in one place
- Add projection-focused regression coverage

### Why this stage matters
Delivers value now while preserving freedom to improve formulas later.

### Exit Criteria
- Deterministic outputs for identical canonical inputs
- Projection bugs are fixable in a single logic layer
- No persisted derived columns treated as authoritative truth

---

## Stage 4 — UX Coherence Pass (Product Feel) ✅

Detailed scope/evidence: `docs/stage-4-ux-coherence.md`

### Purpose
Move from “functional prototype” to “clean MVP experience”.

### Scope
- Establish UI consistency baseline:
  - spacing
  - hierarchy
  - control consistency
- Improve key screen ergonomics:
  - dashboard clarity
  - transaction readability
  - category/account management flow
- Improve confidence cues:
  - confirmations
  - status toasts
  - clearer empty/error states

### Why this stage matters
UX coherence is required for adoption and retention.

### Exit Criteria
- Cohesive screenshots across desktop/mobile
- Core tasks complete without user guesswork
- Usability baseline is “clean MVP” (not “rough prototype”)

---

## Stage 5 — QA Discipline and Release Safety ✅

Detailed scope/evidence: `docs/stage-5-qa-release-safety.md`

### Purpose
Sustain velocity while preventing regressions.

### Scope
- Keep deterministic release gates:
  - smoke script
  - browser E2E
  - existing CI/test hooks
- Maintain lightweight release checklist:
  - stack startup
  - auth flow
  - transaction CRUD
  - projection sanity
  - mobile sanity

### Why this stage matters
Makes fast iteration safe and repeatable.

### Exit Criteria
- High push confidence
- Regressions caught locally/early
- Test workflow is routine and documented

---

## Stage 6 — Next-Layer Capability (Signal-driven) ✅ (governance complete; feature expansion deferred by signal)

Detailed scope/evidence: `docs/stage-6-next-layer-capability.md`

### Purpose
Add depth based on usage signal, not speculative complexity.

### Candidate scope
- richer month budgeting semantics
- assignment/move audit UX
- reconciliation/import workflows
- passkey auth polish
- multi-budget graduation

### Why this stage matters
Ensures roadmap follows product reality.

### Exit Criteria
- Triggered by observed user pain/usage signal
- Additions preserve canonical-input + computed-projection philosophy

---

## Stage Interlock Summary

- Stage 1 protects data integrity
- Stage 2 secures day-to-day usability
- Stage 3 makes derived math reliable and fixable
- Stage 4 improves product quality and confidence
- Stage 5 keeps delivery sustainable
- Stage 6 expands only when justified

This sequence avoids both extremes:
- reckless breadth with hidden instability
- over-engineered depth with stalled product progress
