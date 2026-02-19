# Domain Action Plan (PillID migration + budgeting core)

## Phase 1 (done)
- Introduce domain model module (`apps/api/src/models.rs`)
- Rename identity concept from `ezid` to `pillid`
- Add PillID generator helper (`new_pillid`) backed by `pillid` crate
- Add core entities:
  - User
  - Budget
  - Supercategory
  - Category
  - Payee
  - Account
  - Transaction
  - TransactionDetail
  - AccessToken

## Phase 2 (next)
- Add DB migrations for pillid columns and relationships
- Keep UUID primary keys internal; expose `pillid` publicly
- Add unique indexes for `pillid` per table

## Phase 3
- Repository layer for CRUD + soft-delete filters
- Service layer enforcing budget/account/category transaction invariants

## Phase 4
- API endpoints for domain resources
- Permission checks by `user_pillid`
- Integration tests around lifecycle flows

## Notes
- Current auth stays passwordless (magic-link + passkeys)
- User model intentionally has no password field
