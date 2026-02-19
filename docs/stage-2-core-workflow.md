# Stage 2 — Core Workflow Reliability (Complete)

## Success Criteria
- Sign-in, budgets/accounts/categories, and transaction CRUD complete without workaround.
- Empty/error states are actionable.
- Browser E2E + manual smoke are reliable.

## Risks
- Regressions in auth callback/session persistence.
- Hidden dependency ordering in CRUD setup (budget → account → supercategory → category).

## Test Plan
- Deterministic browser E2E (`npm run e2e`) for full workflow.
- Manual smoke (`./scripts/smoke.sh`) for health/auth/budget checks.
- Web unit sanity (`npm run test`) for login screen baseline.

## Completion Evidence
- Existing MVP E2E already covers sign-in callback, session persistence, full CRUD loop, and mobile viewport sanity (`scripts/e2e.spec.ts`).
- `./scripts/smoke.sh` passes against compose stack.
- `apps/web/src/App.tsx` preserves dependency guards and explicit blocked states for missing prerequisites.

## Outcome
Stage 2 reliability criteria are met for MVP workflow.
