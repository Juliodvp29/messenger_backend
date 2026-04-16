# syntax=docker/dockerfile:1.4

# ── Build Stage ──────────────────────────────────────────────────────────────
FROM rustlang/rust:nightly-slim AS builder

WORKDIR /usr/src/app

# Install build dependencies (openssl, pkg-config)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy workspace manifest and crate-level Cargo.toml files
COPY Cargo.toml Cargo.lock ./
COPY crates/api/Cargo.toml crates/api/
COPY crates/domain/Cargo.toml crates/domain/
COPY crates/infrastructure/Cargo.toml crates/infrastructure/
COPY crates/shared/Cargo.toml crates/shared/

# Create dummy source files to cache dependency compilation
RUN mkdir -p crates/api/src crates/domain/src crates/infrastructure/src crates/shared/src \
    && touch crates/api/src/lib.rs crates/domain/src/lib.rs crates/infrastructure/src/lib.rs crates/shared/src/lib.rs \
    && echo "fn main() {}" > crates/api/src/main.rs

# Build dependencies only (cached layer)
RUN cargo build --release

# Copy actual source code
COPY crates/ crates/
COPY migrations/ migrations/

# Touch main.rs to force recompilation of our code (not deps)
RUN touch crates/api/src/main.rs

# Compile release binary
RUN cargo build --release -p api

# ── Runtime Stage ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies for TLS connections
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy compiled binary and migrations from builder
COPY --from=builder /usr/src/app/target/release/api /app/mesty-api
COPY --from=builder /usr/src/app/migrations /app/migrations

EXPOSE 3000

ENV APP_ENV=production

CMD ["/app/mesty-api"]
