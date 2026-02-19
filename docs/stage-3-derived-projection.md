# Stage 3 — Derived Projection Hardening (Complete)

## Success Criteria
- Derived totals are deterministic for identical canonical inputs.
- Projection logic is isolated to a single backend path.
- Regression coverage exists for projection behavior.

## Risks
- Projection logic duplicated in multiple handlers.
- Accidental drift between transaction canonical writes and dashboard math.

## Test Plan
- Unit test deterministic projection arithmetic.
- Integration flow: create/update/delete transaction and verify dashboard recalculation (covered by existing E2E + smoke).

## Completion Evidence
- Added projection isolation in `apps/api/src/lib.rs` via:
  - `compute_dashboard_projection(...)`
  - `project_available(inflow, outflow)`
- Added deterministic unit test: `dashboard_projection_is_deterministic`.
- Dashboard endpoint now delegates to projection function instead of inline math.

## Outcome
Stage 3 criteria are met with isolated and tested derived totals logic.

## Implementation Evidence (2026-02-19 deep run)
- Completed with concrete code/test updates in this branch.
- Validation executed: ./scripts/check.sh, ./scripts/smoke.sh, npm run e2e.
- See stage-tagged commits for exact diff and residuals.
