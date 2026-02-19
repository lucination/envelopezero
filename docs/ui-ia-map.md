# EnvelopeZero UI IA Map (MVP)

## 1) IA Objectives

- Keep primary budgeting actions shallow and obvious.
- Match navigation to daily user frequency:
  1. Budget
  2. Transactions
  3. Accounts
  4. Settings
- Support both desktop and mobile without separate mental models.

---

## 2) Route Map (Proposed)

## 2.1 Top-level routes
- `/login`
- `/budget`
- `/transactions`
- `/accounts`
- `/settings`

## 2.2 Secondary/detail routes
- `/budget/:month` (optional explicit month addressing)
- `/transactions/new`
- `/transactions/:transactionId/edit`
- `/accounts/new`
- `/accounts/:accountId`
- `/accounts/:accountId/edit`
- `/settings/auth`
- `/settings/profile` (optional if profile settings exist; otherwise keep under `/settings`)

Note: route naming can be adapted to existing router conventions; hierarchy should remain equivalent.

---

## 3) Screen Hierarchy

## 3.1 Auth branch
- Login
  - Magic link flow
  - Error recovery (expired link, invalid token)

## 3.2 App branch (authenticated)

### Budget (primary hub)
- Budget month view
  - Summary strip (Ready to Assign)
  - Category group list
  - Category row edit interactions
  - Overspending resolution entry point

### Transactions
- Transactions list
  - Filters/search
  - Transaction create
  - Transaction edit
  - Split line management

### Accounts
- Accounts list
  - Account create
  - Account detail
  - Account edit/delete

### Settings
- Settings home
  - Auth methods
  - Session management/sign out

---

## 4) Navigation Model

## 4.1 Desktop navigation

Primary pattern:
- Left rail (persistent): Budget, Transactions, Accounts, Settings
- Top context bar:
  - Current month switcher (on budget contexts)
  - Global “Add Transaction” action
  - User/profile menu

Secondary pattern:
- In-view tabs only when needed (avoid deep nested sidebars).
- Detail views open in modal/side panel when feasible to preserve context.

## 4.2 Mobile navigation

Primary pattern:
- Bottom tabs: Budget, Transactions, Accounts, Settings
- Top header per screen with page title + highest-value action.

Secondary pattern:
- Full-screen sheets/pages for create/edit flows.
- Back navigation must return to the exact prior context (filters/month preserved where practical).

---

## 5) Information Grouping by Screen

## 5.1 Budget
- Header context:
  - Month label + month navigation
  - Ready to Assign amount
  - Quick add/assign action
- Body:
  - Supercategory sections
  - Category rows with Assigned / Activity / Available

## 5.2 Transactions
- Header context:
  - Filters + new transaction action
- Body:
  - Transaction list rows
  - Split indicator + amount summary

## 5.3 Accounts
- Header context:
  - New account action
- Body:
  - Account rows with balance summary
  - Tap/click to open account detail

## 5.4 Settings
- Header context:
  - Account/security label
- Body:
  - Auth methods
  - Session controls
  - Other lightweight preferences (if present)

---

## 6) State and Flow Mapping

## 6.1 Entry flow
1. User lands on login (if not authenticated).
2. Successful auth routes to `/budget`.
3. Budget month defaults to current month.

## 6.2 Core daily loop
1. Review Budget availability.
2. Add or edit Transaction.
3. Return to Budget and verify updated availability.

## 6.3 Exception loop (overspending)
1. Overspent category flagged in Budget.
2. User opens correction action (assign or move funds as supported).
3. Category returns to non-overspent state.

---

## 7) IA Guardrails (MVP)

- Keep global nav to 4 primary destinations.
- Avoid introducing Reports/Goals tabs in this phase.
- Keep route depth generally <= 3 segments for common actions.
- Prefer contextual panels over route explosion for edit forms.

---

## 8) Acceptance Criteria

1. A first-time user can find Budget, Transactions, Accounts, and Settings without instruction.
2. Transaction create/edit is reachable from both Budget context and Transactions screen.
3. Mobile and desktop share the same top-level destination labels.
4. No dead-end screens: every detail/edit view has clear cancel/back path.
5. Auth failure and empty-state paths are represented in IA (not left implicit).