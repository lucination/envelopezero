# EnvelopeZero MVP

Single-container app on `:8080` (Axum API + static React), with separate Postgres and Mailpit for dev email capture.

## Stack
- Backend: Rust + Axum + SQLx
- Frontend: React + Vite
- DB: PostgreSQL 16
- Dev email: Mailpit

## MVP feature flags
- `FEATURE_PASSKEYS=false` (passkeys disabled for MVP)
- `FEATURE_MULTI_BUDGET=false` (single default budget mode)
- `DEV_SEED=true` (idempotent startup seed)

## Run with Docker Compose

```bash
docker compose -f infra/docker-compose.yml up --build
```

Services:
- App: <http://localhost:8080>
- API health: <http://localhost:8080/api/health>
- Postgres: `localhost:5432`
- Mailpit UI: <http://localhost:8025>

## Auth flow (magic-link only)
1. Enter email in app.
2. Open Mailpit (<http://localhost:8025>) and click the link.
3. App receives `?token=...` and verifies via `/api/auth/magic-link/verify`.
4. Bearer token stored in localStorage and used for authenticated API calls.

## Local dev without Docker app container

Start infra only:
```bash
docker compose -f infra/docker-compose.yml up -d postgres mailpit
```

Run backend:
```bash
cd apps/api
cp .env.example .env
cargo run
```

Run frontend:
```bash
cd apps/web
npm ci
npm run dev
```

## API surface (MVP)
All under `/api`:
- Auth: magic link request/verify, me
- Budgets: list/create
- Accounts: CRUD
- Supercategories: CRUD
- Categories: CRUD
- Transactions: CRUD with split details
- Dashboard totals: inflow/outflow/available

## Quality checks
```bash
./scripts/check.sh
```

Includes rustfmt + clippy + tests and web tests/build.

## Deterministic smoke + browser E2E runbook

Start clean stack:
```bash
docker compose -f infra/docker-compose.yml down -v
docker compose -f infra/docker-compose.yml up --build -d
```

Install browser test deps once:
```bash
cd apps/web
npm ci
npx playwright install --with-deps chromium
```

Run API smoke (health/auth/budgets + Mailpit delivery):
```bash
cd ../..
./scripts/smoke.sh
```

Run full browser MVP E2E:
```bash
npm run e2e
```

MVP E2E coverage includes:
- magic-link request + verify (token sourced from Mailpit API)
- callback/session persistence + logout
- budget single-default mode + multi-budget conflict gating
- accounts, supercategories, categories, transactions CRUD
- dashboard total recalculation after tx create/update/delete
- empty-state visibility checks
- mobile viewport sanity check (390x844)

### TailScale-bound URL checks
Compose binds app/mailpit to `${TAILSCALE_IP:-127.0.0.1}`. To test over your TailScale IP:
```bash
export TAILSCALE_IP=100.x.y.z
docker compose -f infra/docker-compose.yml up --build -d
EZ_APP_URL="http://$TAILSCALE_IP:8080" EZ_MAILPIT_URL="http://$TAILSCALE_IP:8025" ./scripts/smoke.sh
EZ_APP_URL="http://$TAILSCALE_IP:8080" EZ_MAILPIT_URL="http://$TAILSCALE_IP:8025" npm run e2e
```
