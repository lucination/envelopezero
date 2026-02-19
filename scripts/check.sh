#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

pushd "$ROOT/apps/api" >/dev/null
cargo +nightly fmt --check
cargo clippy -- -D warnings
export DATABASE_URL="${DATABASE_URL:-postgres://envelopezero:envelopezero@localhost:5432/envelopezero}"
cargo test
popd >/dev/null

pushd "$ROOT/apps/web" >/dev/null
npm ci
npm run test
npm run build
popd >/dev/null
