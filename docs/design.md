# EnvelopeZero — Core Design Document (v1)

## 1) Product Intent

EnvelopeZero is a zero-based budgeting app where users assign dollars they already have, categorize spending, and always know how much is available per category.

The system prioritizes:
- Correctness of money math
- Auditability of user inputs
- Simple, trustworthy UX

## 2) Foundational Design Principles

1. **Canonical inputs only**
   - Persist only facts users enter (transactions, details, assignments, etc.).
2. **Derived values computed on read**
   - `available`, `activity`, `ready_to_assign`, etc. are never source-of-truth columns.
3. **Deterministic recomputation**
   - Rebuilding balances from canonical inputs always produces the same result.
4. **External IDs are PillIDs**
   - API contracts use PillIDs; UUIDs remain internal implementation detail.
5. **User-scoped ownership everywhere**
   - Every budget object belongs to a user and is enforced in queries/constraints.
6. **Transaction lines are the financial truth**
   - A transaction header is metadata; transaction details hold category allocation amounts.

## 3) Domain Model

### 3.1 Identity and Ownership
- `User` owns all budgeting data.
- `Budget` belongs to one `User`.
- Current stance: one default budget per user (multi-budget can be feature-gated).

### 3.2 Budget Structure
- `Supercategory` belongs to User + Budget.
- `Category` belongs to User + Budget + Supercategory.
- `Account` belongs to User + Budget.
- `Payee` belongs to User + Budget.

### 3.3 Transaction Structure
- `Transaction` (header): metadata only (date, account, payee, memo, etc.)
- `TransactionDetail` (lines): category allocation with inflow/outflow (+ optional memo)
- Every transaction must have **at least one** detail line.
- Detail count = 1 means non-split; detail count > 1 means split transaction.

### 3.4 Budgeting Inputs
- `CategoryAssignment`: category, budget month, assigned amount
- Optional `MoneyMove` event: from_category, to_category, month, amount, reason

### 3.5 Auth Domain
- Passwordless methods (magic-link now, passkeys optional/flagged)
- Multiple methods per user supported
- Constraint: user must retain at least one active auth method
- Sessions via bearer token for MVP

## 4) Invariants (Must Always Hold)

1. **Transaction detail cardinality**
   - Every transaction has `>= 1` detail row.
2. **Ownership consistency**
   - Parent/child rows must match user and budget scope.
3. **No orphaned financial rows**
   - A transaction detail cannot exist without its parent transaction.
4. **Transaction net derived from details**
   - Header does not carry authoritative amount.
5. **Category availability is derived**
   - Never manually edited/stored as source-of-truth.
6. **PillID uniqueness**
   - PillID is unique per table and stable for external references.

## 5) Financial Computation Model

For a given `budget` and `month`:

- **Assigned(category, month)**
  - Sum of assignment inputs for that category/month

- **Activity(category, month)**
  - Sum of transaction detail movement in that category/month

- **Available(category, month)**
  - `Available(category, previous_month) + Assigned(category, month) - Activity(category, month)`

- **Ready To Assign(month)**
  - Budget cash basis available to assign minus total assigned for that month

All computations are query-time projections from canonical inputs.

## 6) API Design Contract

1. **External identifiers**
   - All domain APIs use PillIDs (not UUIDs).
2. **Write semantics**
   - Transaction writes accept detail arrays.
   - Server validates detail constraints and ownership.
3. **Read semantics**
   - Budget views return computed values (`assigned/activity/available`) per category.
4. **Auth**
   - Bearer session token for MVP.
5. **Error behavior**
   - Domain invariants return explicit conflict/validation responses.

## 7) UX Model

### 7.1 Mental Model
- Users assign money to categories.
- Spending reduces category availability.
- Overspending is resolved by reassignment/moves.

### 7.2 Key UX Surfaces
- **Budget view**: category groups with assigned/activity/available
- **Transactions**: fast entry/edit, split-first support
- **Accounts**: account management and balance context
- **Auth/settings**: method management and session controls

### 7.3 UX Requirements
- Split entry is first-class.
- Budget math updates immediately after transaction/assignment changes.
- Empty/error states clearly explain unblock actions.

## 8) Data Integrity & Auditability

- Every user-editable financial fact must be traceable.
- Recomputing balances from canonical inputs must be possible at any time.
- Any future performance cache/materialization is disposable and rebuildable.

## 9) Extensibility Boundaries

Designed to support later:
- Multi-budget mode
- Passkey-first auth
- Reconciliation flows
- CSV import/bank sync
- Category goals/targets
- Reporting/analytics

These additions should not change the canonical-input, computed-derivatives philosophy.
