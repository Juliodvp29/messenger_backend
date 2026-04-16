.PHONY: dev stop migrate seed test lint build

# Development environment
dev:
	docker compose up -d
	@echo "Services started: postgres:5434, redis:6379, minio:9000, mailhog:8025"

stop:
	docker compose down

# Database (requires sqlx-cli)
migrate:
	sqlx migrate run

seed:
	@echo "Loading seed data..."

# Quality and build
lint:
	cargo fmt --check
	cargo clippy --workspace -- -D warnings

test:
	cargo test --workspace

build:
	cargo build --release
