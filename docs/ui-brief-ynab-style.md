# EnvelopeZero UI Brief — YNAB-Style Direction (MVP)

## 1) Purpose and Scope

Define a practical UI direction for EnvelopeZero that feels inspired by YNAB’s budgeting clarity while matching EnvelopeZero’s current domain model and MVP constraints.

This document covers:
- Visual principles
- Layout rules
- Component behavior
- Acceptance criteria

Out of scope:
- Visual perfection work
- New budgeting features not yet supported by backend
- Code-level implementation details

---

## 2) Product UX Intent (MVP)

Users should be able to answer, at a glance:
1. How much money is ready to assign?
2. Where is my money assigned?
3. Which categories are overspent or at risk?
4. What happened recently (transaction activity)?

Primary UX promise: **fast financial clarity with low cognitive load**.

---

## 3) Visual Principles

### 3.1 Clarity over decoration
- High-contrast data hierarchy.
- Minimal ornamental UI.
- Use color as status signal, not branding wallpaper.

### 3.2 Table-first budgeting workspace
- Budget screen is a structured workspace (category rows + amount columns).
- Keep rows scannable and stable while editing.
- Preserve column alignment across loading and data refresh.

### 3.3 Progressive disclosure
- Default view shows only data needed for daily budgeting.
- Secondary details (notes, historical context, advanced actions) appear on demand.

### 3.4 Calm, explicit state signaling
- Positive, warning, and error states must be visually distinct and text-labeled.
- No “mystery red numbers”; always show why and what to do next.

### 3.5 Predictable interaction patterns
- Similar actions behave similarly across screens.
- Inline editing and modal/panel editing use consistent controls and validation language.

---

## 4) Layout Rules

## 4.1 Global shell

### Desktop (>= 1024px)
- Left rail navigation (fixed).
- Top context bar (budget selector/month controls/global quick add).
- Main content area with max readable width and internal scrolling where appropriate.

### Tablet (768–1023px)
- Collapsible left rail.
- Top bar remains; actions prioritized by frequency.

### Mobile (< 768px)
- Bottom tab nav for primary sections.
- Contextual top header with page title + key action.
- Full-screen forms/panels preferred over dense inline controls.

## 4.2 Spatial rhythm
- Base spacing unit: 8px.
- Vertical sections use 16/24px gaps.
- Minimum touch target: 44x44 on mobile.
- Sticky critical controls only when they reduce repeated scrolling.

## 4.3 Data density policy
- Budget table: medium density by default; compact mode deferred.
- Avoid truncating money values; truncate labels first with tooltip/expand support.
- Column order remains fixed for learned muscle memory.

---

## 5) Screen-Level Behavior

## 5.1 Budget screen (primary)

Required structure:
- Header summary strip (Ready to Assign + month context + quick actions).
- Category group sections.
- Category rows with columns:
  - Assigned
  - Activity
  - Available

Row behavior:
- Click/tap row selects it (shows quick actions).
- Assigned cell supports inline edit (desktop) and focused edit panel (mobile).
- Available value uses semantic styling:
  - positive/neutral: standard
  - near zero: warning tone
  - negative: overspent/error tone
- Overspent categories include action affordance (e.g., “Cover overspending”).

Feedback behavior:
- On save: optimistic update where safe, with rollback on failure.
- Inline validation for invalid amounts.
- Toast for success/failure with plain-language message.

## 5.2 Transactions screen

Required structure:
- Search/filter bar (date range, account, category, payee).
- Transaction list (most recent first).
- Clear visual distinction for split vs non-split transactions.

Interaction behavior:
- New transaction CTA is always visible.
- Create/edit opens panel/modal (desktop) or full-screen form (mobile).
- Split editing supports multiple category lines with running total clarity.
- Save requires balanced/valid line details per current domain rules.

## 5.3 Accounts screen

Required structure:
- Account list with balances.
- Account detail view with transaction context.

Behavior:
- Fast create/edit/delete flows with confirmation on destructive actions.
- Empty states explain how accounts affect budget workflow.

## 5.4 Settings/Auth screen

Required structure:
- Auth method management.
- Session/sign-out controls.

Behavior:
- Clear warnings for risky auth changes.
- Prevent removing final active auth method (as domain rule requires).

---

## 6) Component Behavior Standards

## 6.1 Money input
- Accept numeric typing with optional decimal.
- Normalize on blur/submit to currency format.
- Preserve minus sign behavior for outflow contexts where applicable.
- Validation messages must state allowed format and constraint.

## 6.2 Table row + cell editing
- Row hover/selection state visible on desktop.
- Keyboard navigation between editable cells (deferred if high effort, but structure should not block it).
- Loading placeholder preserves row height to avoid layout jump.

## 6.3 Status chips/badges
- Use concise labels: “Overspent”, “Needs Assignment”, “Cleared” (if applicable).
- Color + text pairing required for accessibility.

## 6.4 Toasts and inline alerts
- Success toasts auto-dismiss.
- Error toasts persist long enough for comprehension and include next step.
- Form-level errors appear near submit control and field-level where possible.

## 6.5 Empty/loading/error states
- Every primary screen has:
  - Initial loading skeleton
  - Empty state with one primary CTA
  - Recoverable error state with retry option

---

## 7) Accessibility and Quality Baseline

- Text contrast meets WCAG AA minimum.
- Interactive controls reachable by keyboard on desktop for core actions.
- Visible focus states on all actionable controls.
- Semantic labels for screen reader clarity on financial values and actions.

---

## 8) Copy and Tone Rules

- Use plain, direct language.
- Prefer action-oriented labels (“Assign money”, “Add transaction”).
- Avoid accounting jargon unless unavoidable; when used, keep helper text nearby.

---

## 9) Acceptance Criteria (Definition of Done for UI direction)

This brief is considered successfully implemented when:

1. **Budget workspace clarity**
   - Users can identify Ready to Assign, category availability, and overspending within 5 seconds of loading Budget screen.

2. **Consistent shell and navigation**
   - Desktop uses left rail + top context bar.
   - Mobile uses bottom tabs + contextual header.
   - Primary destinations are reachable in <= 2 taps/clicks from root.

3. **Reliable financial interactions**
   - Assign/edit transaction flows provide explicit validation and success/failure feedback.
   - No silent failures on save.

4. **State completeness**
   - Budget, Transactions, Accounts, Settings each have loading/empty/error states.

5. **Accessibility baseline**
   - Visible focus, readable contrast, and labeled controls across core workflows.

6. **MVP restraint maintained**
   - No net-new advanced feature scope introduced to satisfy UI overhaul.

---

## 10) Non-Goals (for this phase)

- Advanced reports/analytics dashboards
- Long-term goals/targets UX
- Complex reconciliation workflows
- Multi-budget UX expansion

These may be layered later after core UI coherence is complete.