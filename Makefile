.PHONY: dev stop migrate seed test lint build

# Entorno de desarrollo
dev:
	docker compose up -d
	@echo "Servicios levantados: postgres:5434, redis:6379, minio:9000, mailhog:8025"

stop:
	docker compose down

# Base de datos (requiere sqlx-cli)
migrate:
	sqlx migrate run

seed:
	@echo "Cargando datos de prueba..."
	@# (Se implementará en la Fase 02)

# Calidad y compilación
lint:
	cargo fmt --check
	cargo clippy --workspace -- -D warnings

test:
	cargo test --workspace

build:
	cargo build --release
