# syntax=docker/dockerfile:1.4

# Build stage
FROM rustlang/rust:nightly-slim AS builder

WORKDIR /usr/src/app

# Instalar dependencias necesarias para compilar (openssl, pkg-config)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copiar el manifiesto del workspace y los Cargo.toml de las cajas
COPY Cargo.toml Cargo.lock ./
COPY crates/api/Cargo.toml crates/api/
COPY crates/domain/Cargo.toml crates/domain/
COPY crates/infrastructure/Cargo.toml crates/infrastructure/
COPY crates/shared/Cargo.toml crates/shared/

# Crear src falso para cachear dependencias
RUN mkdir -p crates/api/src crates/domain/src crates/infrastructure/src crates/shared/src \
    && touch crates/api/src/lib.rs crates/domain/src/lib.rs crates/infrastructure/src/lib.rs crates/shared/src/lib.rs \
    && echo "fn main() {}" > crates/api/src/main.rs

# Construir dependencias (cache)
RUN cargo build --release

# Copiar el verdadero código fuente
COPY crates/ crates/
COPY migrations/ migrations/

# Tocar los archivos para forzar recompilación de lo nuestro
RUN touch crates/api/src/main.rs

# ENV SQLX_OFFLINE=true # Descomentar si usas macros de query offline activado

# Compilar release
RUN cargo build --release -p api

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Instalar openssl y ca-certificates para conexiones seguras
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

# Copiar el binario compilado y el directorio de migraciones
COPY --from=builder /usr/src/app/target/release/api /app/messenger-api
COPY --from=builder /usr/src/app/migrations /app/migrations

# Exponer el puerto
EXPOSE 3000

ENV APP_ENV=production

CMD ["/app/messenger-api"]
