#!/usr/bin/env bash
set -euo pipefail

APP_URL="${EZ_APP_URL:-http://127.0.0.1:8080}"
MAILPIT_URL="${EZ_MAILPIT_URL:-http://127.0.0.1:8025}"
EMAIL="adi+smoke-$(date +%s)@example.com"

echo "[smoke] health"
curl -fsS "$APP_URL/api/health" >/dev/null

echo "[smoke] request magic link"
TOKEN=$(curl -fsS -X POST "$APP_URL/api/auth/magic-link/request" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL\"}" | sed -n 's/.*"debug_token":"\([^"]*\)".*/\1/p')

echo "[smoke] verify token"
SESSION=$(curl -fsS -X POST "$APP_URL/api/auth/magic-link/verify" \
  -H 'Content-Type: application/json' \
  -d "{\"token\":\"$TOKEN\"}")
AUTH_TOKEN=$(echo "$SESSION" | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

echo "[smoke] check budgets"
BUDGETS=$(curl -fsS "$APP_URL/api/budgets" -H "Authorization: Bearer $AUTH_TOKEN")
echo "$BUDGETS" | grep -Eq '"id":"[a-f0-9]{32}"'

echo "[smoke] check Mailpit has sent email"
curl -fsS "$MAILPIT_URL/api/v1/search?query=$EMAIL" | grep -q 'messages'

echo "smoke ok"