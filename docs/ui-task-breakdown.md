# EnvelopeZero UI Task Breakdown (YNAB-Style Direction, MVP)

## 1) Purpose

Translate UI planning into implementation-ready tasks with clear sequencing and rough effort sizing.

Sizing key:
- **S** = small (same-day to ~1 day)
- **M** = medium (~1–3 days)
- **L** = large (multi-day, cross-screen/cross-component)

---

## 2) Dependency-Ordered Task List

## Phase 0 — Alignment & Baseline

### T0.1 Confirm UI scope against current backend capabilities (S)
- Validate planned interactions do not require non-existent APIs.
- Document any temporary UX constraints.
- **Depends on:** none

### T0.2 Inventory current screens/components and identify reusable pieces (S)
- Capture what can be adapted vs rebuilt.
- **Depends on:** T0.1

---

## Phase 1 — App Shell and Navigation

### T1.1 Implement responsive app shell foundation (L)
- Desktop: left rail + top context bar.
- Mobile: bottom tabs + contextual header.
- Shared layout primitives (container, content region, sticky zones).
- **Depends on:** T0.2

### T1.2 Define canonical nav destinations and route wiring updates (M)
- Ensure Budget/Transactions/Accounts/Settings are primary routes.
- Add or normalize detail routes for create/edit flows.
- **Depends on:** T1.1

### T1.3 Add global quick action entry point (“Add Transaction”) (S)
- Accessible from top context bar/header.
- **Depends on:** T1.1

---

## Phase 2 — Budget Workspace (Primary Surface)

### T2.1 Build budget summary strip (M)
- Ready to Assign display + month context + key actions.
- **Depends on:** T1.1

### T2.2 Refactor category group + row presentation for table-first clarity (L)
- Stable row heights and column alignment.
- Assigned / Activity / Available columns.
- **Depends on:** T2.1

### T2.3 Implement assigned-value editing interaction (M)
- Desktop inline edit; mobile focused edit flow.
- Validation and save feedback.
- **Depends on:** T2.2

### T2.4 Add overspending status treatment + corrective action affordance (M)
- Visual status + actionable path.
- **Depends on:** T2.2

### T2.5 Budget loading/empty/error states (S)
- Skeleton, empty CTA, retry-capable error state.
- **Depends on:** T2.1

---

## Phase 3 — Transactions UX Coherence

### T3.1 Rework transactions list hierarchy and scanability (M)
- Clear row information order (date/payee/account/category/amount).
- Split indicator clarity.
- **Depends on:** T1.2

### T3.2 Standardize transaction create/edit container behavior (M)
- Modal/side panel on desktop; full-screen form on mobile.
- **Depends on:** T1.1, T1.2

### T3.3 Implement split transaction line editor improvements (L)
- Multi-line category allocations with running total clarity.
- Explicit validation messaging.
- **Depends on:** T3.2

### T3.4 Transactions loading/empty/error states (S)
- Consistent with budget state patterns.
- **Depends on:** T3.1

---

## Phase 4 — Accounts and Settings Coherence

### T4.1 Accounts list visual and interaction pass (M)
- Balance emphasis + reliable row actions.
- **Depends on:** T1.2

### T4.2 Account create/edit/delete UX consistency pass (M)
- Form structure, confirmation patterns, feedback behavior.
- **Depends on:** T4.1

### T4.3 Accounts loading/empty/error states (S)
- **Depends on:** T4.1

### T4.4 Settings/auth management clarity pass (M)
- Explicit warnings for risky auth changes.
- Prevent invalid final-auth-method removal in UX messaging.
- **Depends on:** T1.2

---

## Phase 5 — Shared Component and UX Quality Layer

### T5.1 Standardize money input component behavior (M)
- Parsing, formatting, negative handling, field errors.
- **Depends on:** T2.3, T3.3

### T5.2 Standardize feedback patterns (toasts + inline alerts) (S)
- Shared copy and lifecycle behavior.
- **Depends on:** T2.3, T3.2

### T5.3 Introduce status chip/badge system (S)
- Overspent/needs-attention style tokens and labels.
- **Depends on:** T2.4

### T5.4 Accessibility baseline pass (M)
- Focus visibility, keyboard path for core flows, contrast checks.
- **Depends on:** T1.1, T2.2, T3.1, T4.1

---

## Phase 6 — QA and Acceptance Validation

### T6.1 Define UI acceptance checklist mapped to brief criteria (S)
- Convert brief criteria into testable checks.
- **Depends on:** phases 1–5 complete

### T6.2 Execute desktop + mobile smoke validation for core loop (M)
- Budget review → transaction edit/add → budget verification.
- **Depends on:** T6.1

### T6.3 Fix-blocking polish pass before release (M)
- Address defects found in smoke checks.
- **Depends on:** T6.2

---

## 3) Suggested Delivery Slices (Pragmatic)

### Slice A (must-have foundation)
- T1.1, T1.2, T2.1, T2.2, T2.5

### Slice B (core interaction trust)
- T2.3, T2.4, T3.2, T3.3, T3.4

### Slice C (cross-screen coherence)
- T3.1, T4.1, T4.2, T4.3, T4.4

### Slice D (quality hardening)
- T5.1, T5.2, T5.3, T5.4, T6.1, T6.2, T6.3

---

## 4) Notes for Junior Implementers

- Follow IA map for route and navigation decisions; avoid inventing additional top-level screens.
- Follow UI brief for behavior and state handling; if uncertain, prioritize explicit user feedback over clever UI.
- Keep changes incremental per phase to reduce regressions.
- Do not mix feature expansion with UI coherence tasks in the same PR.

---

## 5) Done Criteria for This Plan

Plan execution is considered complete when:
1. Primary shell/navigation model is implemented for desktop + mobile.
2. Budget and transaction core workflows meet acceptance criteria from `ui-brief-ynab-style.md`.
3. Accounts/settings achieve the same state/feedback consistency standard.
4. Accessibility and smoke validation pass for MVP core loop.