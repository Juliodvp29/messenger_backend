# AGENTS.md

## Build and Run

- Requires **nightly Rust** (`edition = "2024"`).
- `cargo check --workspace` - typecheck all crates.
- `cargo build --workspace` - build all crates in debug mode.
- `cargo run -p api` - run API binary.

## Workspace Structure

- `crates/api` - Axum HTTP layer (routes, handlers, middleware, app bootstrap).
- `crates/domain` - business rules, domain models and use-cases.
- `crates/infrastructure` - adapters for Postgres, Redis and external services.
- `crates/shared` - cross-cutting utilities (errors, config, common types).
- `schema.sql` - current SQL schema reference.
- `migrations/` - database migrations.

## Backend Conventions (Rust)

- Runtime and web stack: use `Tokio` + `Axum`.
- Keep dependency direction: `api -> domain -> infrastructure -> shared` (or via traits).
- Handlers should only orchestrate HTTP concerns (DTOs, auth context, status codes).
- Business logic lives in domain services/use-cases.
- DB access belongs to repository implementations, not handlers.

## Data and SQL Conventions

- Prefer `sqlx::query!` / `sqlx::query_as!` when possible.
- Never interpolate raw user input into SQL strings.
- Validate schema assumptions before changing queries.
- Consider indexes for frequent filters/sorts (`created_at`, `sender_id`, etc.).

## Error Handling

- Use centralized `AppError` with `thiserror`.
- Map infra errors to domain-safe errors, then to HTTP responses.
- Do not leak internal details (SQL errors, stack traces, secrets) in API responses.

## Security Baseline

- Current auth flow uses OTP + PIN (see workspace roadmap and api crate).
- JWT must use asymmetric signing (`EdDSA` or `RS256`) with key rotation strategy.
- Enforce authorization per resource, not only authentication.
- Review ID enumeration risk in all read/write endpoints.
- Apply rate limiting to login/OTP and other abuse-prone routes.

## Operational Notes

- Prefer small, focused modules and explicit trait boundaries.
- Add tracing for critical flows (auth, message delivery, websocket lifecycle).
- When introducing realtime features, include heartbeat and clean disconnect handling.


