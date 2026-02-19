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

## Auth Model

Users can have multiple auth methods and can add/remove methods.
Constraint: each account must always keep **at least 1 active auth method**.

Initial methods:
- Magic link (email token)
- Passkey (WebAuthn credential)
