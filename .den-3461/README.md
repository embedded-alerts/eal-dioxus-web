# eal-dioxus-web

Server-rendered Dioxus candidate inbox for Embedded Alerts.

This application is a read-only API client. It does not own PostgreSQL state, crawl arbitrary URLs, open an unauthenticated WebSocket, approve or reject matches, or send notifications. It renders the tenant-scoped candidate read model, source boundaries, immutable page provenance, and score evidence supplied by `eal-api`.

## Development

```bash
cp .env.example .env
cargo run
```

Required configuration:

- `APP_ENV=development|test`; production intentionally fails startup.
- `EAL_API_BASE_URL` points to `eal-api` and may not contain credentials.
- `EAL_TENANT_ID` is the temporary server-side tenant selector until Shared Auth claims are certified.
- `HOST` and `PORT` configure the listener.

API redirects are blocked, requests have connection/overall timeouts, decoded responses are capped at 4 MiB, and tenant identity never comes from browser query parameters.

## Production gates

1. Shared Auth replaces the development tenant header.
2. Candidate, source, revision, alert-rule, and delivery state is tenant-scoped and durable in PostgreSQL/pgvector.
3. Authenticated tenant-filtered events replace process-local WebSockets.
4. DEN-3460 provides approval state, a durable outbox, cooldown/grouping, provider idempotency, receipts, retries, and dead letters.
5. Explicit origin/CSP and restart/cross-tenant canaries pass in `embedded-alerts-test`.

## Validation

```bash
python3 scripts/verify_repo.py
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Linear: DEN-3461; related DEN-3459, DEN-3460, DEN-3462.
