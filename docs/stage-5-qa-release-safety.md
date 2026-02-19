# Stage 5 — QA Discipline and Release Safety (Complete)

## Success Criteria
- Deterministic gates exist and are routinely runnable.
- Release checklist is explicit and lightweight.
- Local confidence before push is high.

## Risks
- Environment drift (missing `DATABASE_URL`/services) causing false negatives.
- Test suites diverging from real MVP flow.

## Test Plan / Gates
- Backend: `cargo test` (with `DATABASE_URL` set for sqlx integration tests).
- Web: `npm run test` and `npm run build`.
- Smoke: `./scripts/smoke.sh` against compose stack.
- Optional full browser gate: `npm run e2e`.

## Completion Evidence
- QA scripts are maintained:
  - `scripts/check.sh`
  - `scripts/smoke.sh`
  - `scripts/e2e.spec.ts`
- Verified in this stage run:
  - API unit+integration tests pass with `DATABASE_URL=postgres://envelopezero:envelopezero@localhost:5432/envelopezero`
  - Web tests/build pass
  - Smoke script passes against docker compose app/mailpit/postgres

## Outcome
Stage 5 release-safety discipline is in place for MVP iteration velocity.
