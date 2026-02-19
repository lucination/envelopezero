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
