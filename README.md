# EnvelopeZero

A mobile-first budgeting app (YNAB-inspired) with:
- **Rust** API server
- **PostgreSQL** database
- **React** frontend
- Passwordless auth: **magic link** + **passkeys**

## Monorepo Layout

- `apps/api` — Rust backend (Axum + SQLx)
- `apps/web` — React frontend (Vite + TypeScript)
- `infra` — Docker compose and DB bootstrap

## Quick Start

### 1) Start Postgres

```bash
docker compose -f infra/docker-compose.yml up -d
```

### 2) API

```bash
cd apps/api
cp .env.example .env
cargo run
```

### 3) Web

```bash
cd apps/web
npm install
npm run dev
```

## Local quality gate

We use **Husky hooks + CI** (pre-commit checks + commit message lint).

One-time setup after cloning:

```bash
npm install
rustup toolchain install nightly --component rustfmt
```

Run full local checks manually:

```bash
./scripts/check.sh
```

> Formatting uses nightly rustfmt with:
> - `group_imports = "StdExternalCrate"`
> - `imports_granularity = "Item"` (one import per line)

## Auth model

Users can have multiple auth methods and can add/remove methods.
Constraint: each account must always keep **at least 1 active auth method**.

Current methods:
- Magic link (request + verify)
- Passkey (registration challenge flow + credential persistence)

> Note: WebAuthn cryptographic verification is scaffolded at the data-flow layer and is the next hardening step.
