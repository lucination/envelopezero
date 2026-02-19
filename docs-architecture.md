# EnvelopeZero Architecture Notes

## Stack
- API: Rust + Axum + SQLx
- DB: Postgres
- Web: React + TypeScript (mobile-first)

## Auth requirements
- No passwords.
- Methods supported: magic link + passkeys.
- One user can have many auth methods.
- Must always keep at least one active method.

## Next Milestones
1. Implement real token hashing + email delivery provider.
2. Add WebAuthn registration + authentication ceremony endpoints.
3. Sessions + refresh token model.
4. Budget domain: accounts, categories, envelopes, transactions.
5. Responsive dashboard + PWA improvements.
