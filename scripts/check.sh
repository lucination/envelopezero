#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

pushd "$ROOT/apps/api" >/dev/null
cargo fmt --check
cargo clippy -- -D warnings
cargo test
popd >/dev/null

pushd "$ROOT/apps/web" >/dev/null
npm ci
npm run test
npm run build
popd >/dev/null
