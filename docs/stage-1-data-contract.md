# EnvelopeZero — Stage 1 Detailed Spec

## Stage Name
Data Contract Stabilization (Foundation)

## Purpose
Protect canonical user-entered data and lock stable API/DB contracts before further feature work.

This stage is complete when a junior engineer can safely change UI behavior without accidentally corrupting core data semantics.

---

## 1) Scope

### In scope
1. Canonical write-path audit and hardening
2. Transaction/transaction-detail invariants
3. Ownership and budget boundary enforcement
4. PillID contract consistency (external IDs)
5. Minimal regression tests for invariants and contracts
6. Documentation updates for rules and examples

### Out of scope
- UX polish
- New product features
- Reconciliation/import pipelines
- Advanced monthly budgeting semantics

---

## 2) Canonical Data Rules (must be true after Stage 1)

## 2.1 Canonical input entities
Treat these as source-of-truth inputs:
- transactions (header metadata)
- transaction_details (financial line allocations)
- category assignments (if present in schema)
- auth/session events

Derived values (dashboard totals, category available, etc.) are not canonical and must not be persisted as authoritative truth.

## 2.2 Transaction model rules
1. Every transaction must have at least one transaction_detail row.
2. A transaction with one detail is a non-split transaction.
3. A transaction with more than one detail is a split transaction.
4. The transaction net amount is derived from details; header amount fields (if any) are non-authoritative.

## 2.3 Ownership rules
For all domain writes/reads:
- user must own budget
- account/category/supercategory/payee/transaction must belong to same user+budget context
- cross-user and cross-budget references are rejected

## 2.4 Identifier rules
- External APIs return and accept PillIDs for domain entities.
- UUIDs remain internal DB implementation details.

---

## 3) Required DB Constraints/Checks

> Note: implement as DB constraints/triggers where practical, and enforce in service layer as defense-in-depth.

1. `transaction_details.transaction_id` must reference existing `transactions.id`.
2. Prevent deleting a transaction if detail cleanup would violate constraints (or use cascading delete intentionally and explicitly).
3. Enforce that transaction has >=1 detail at commit-time for create/update operations.
4. PillID uniqueness constraints per table for externally exposed entities.
5. Non-null constraints for required ownership/budget references.

If a rule is not safely enforceable directly in SQL without complexity, document exactly where service-layer validation enforces it.

---

## 4) API Contract Requirements

## 4.1 Transaction write contract

### Create transaction
- Request includes header fields + `details[]`.
- Must reject empty `details[]`.
- Must reject invalid ownership references.
- Returns created transaction with detail lines and PillIDs.

### Update transaction
- Supports replacing/editing details while preserving `>=1` detail invariant.
- Must keep ownership and budget consistency.

### Delete transaction
- Removes transaction and associated details in a deterministic way.
- No orphan detail rows can remain.

## 4.2 Error contract
Validation/invariant failures should return deterministic 4xx responses with clear machine-readable codes/messages (or at minimum stable status + message patterns).

---

## 5) Validation Matrix (what we must test)

## 5.1 Unit/service-level cases
1. Create transaction with zero details -> fails
2. Create transaction with one detail -> succeeds
3. Create transaction with multiple details -> succeeds
4. Update transaction that removes all details -> fails
5. Cross-budget category on detail -> fails
6. Cross-user account/category references -> fails

## 5.2 Integration/DB cases
1. FK behavior for transaction/detail integrity
2. PillID uniqueness constraints hold
3. Ownership checks prevent unauthorized data access
4. Deletion behavior leaves no orphan rows

## 5.3 API contract cases
1. All domain responses expose PillIDs (not UUIDs)
2. PillID lookup paths resolve correctly and securely
3. Error responses stable for invalid writes

---

## 6) Deliverables

1. **Code changes**
- DB migration updates (if needed)
- service-layer validation guards
- route/handler consistency for PillID contracts

2. **Tests**
- unit tests for validation rules
- integration tests for DB constraints and ownership
- existing smoke/e2e still green

3. **Docs**
- Update `docs/design.md` only if contract wording changed
- Link this stage doc from `docs/implementation-plan.md`

---

## 7) Definition of Done (Stage 1)

Stage 1 is done only if all are true:

1. Transaction/detail invariants are enforced in code and covered by tests.
2. No known write path can create orphan or cross-scope data.
3. PillID external contract is consistent across all in-scope domain endpoints.
4. CI and local checks pass (`./scripts/check.sh`, smoke, and relevant integration tests).
5. Documentation matches implementation behavior.

---

## 8) Implementation Checklist (junior-friendly)

- [x] Read `docs/design.md` sections on canonical inputs and transaction details.
- [x] Enumerate all transaction create/update/delete handlers.
- [x] Add/verify `details.length > 0` guards on all write paths.
- [x] Verify ownership checks for budget/account/category references.
- [x] Verify no endpoint returns UUID externally for in-scope entities.
- [x] Add tests for each validation matrix case.
- [x] Run full checks locally.
- [x] Update docs and open PR with summary of invariant coverage.

---

## 10) Completion Evidence

Implemented in this stage:
- Service-layer invariant guards on transaction create/update (`splits` cannot be empty).
- DB-layer deferred constraint trigger enforcing active split cardinality for non-deleted transactions.
- Added integration test for empty-splits rejection.

Validation run:
- `DATABASE_URL=postgres://envelopezero:envelopezero@localhost:5432/envelopezero cargo test --test api_integration_sqlx` ✅
- `cargo test` unit tests ✅
- `npm run test` and `npm run build` (web baseline sanity) ✅
- `./scripts/smoke.sh` ✅

---

## 9) PR Template Notes for Stage 1 work

Every PR in this stage should include:
1. Which invariant(s) it enforces
2. Which tests were added/updated
3. Any DB vs service-layer tradeoff
4. Any known risk not addressed yet
