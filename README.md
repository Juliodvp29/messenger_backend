# Messenger Backend

[![CI - Messenger Backend](https://github.com/Juliodvp29/messenger_backend/actions/workflows/ci.yml/badge.svg)](https://github.com/Juliodvp29/messenger_backend/actions/workflows/ci.yml)

Backend modular en Rust para una aplicación de mensajería estilo WhatsApp/Telegram.

## Arquitectura

El proyecto utiliza un **Cargo Workspace** con la siguiente estructura:
- `api`: Punto de entrada, rutas y controladores (Axum).
- `domain`: Entidades puras de negocio y lógica core.
- `infrastructure`: Implementaciones de persistencia (Postgres), cache (Redis) y servicios externos.
- `shared`: Tipos comunes, configuración tipada y utilidades.

## Stack Tecnológico

- **Lenguaje**: Rust Nightly (Edición 2024)
- **Web Framework**: Axum
- **Base de Datos**: PostgreSQL 17 (SQLx para consultas typesafe)
- **Cache**: Redis 8
- **Storage**: MinIO (compatible con AWS S3)
- **CI/CD**: GitHub Actions

## Inicio Rápido

### Prerrequisitos
- Docker y Docker Compose
- Rust Nightly
- `make` (opcional, pero recomendado)

### Instalación
1. Clonar el repositorio.
2. Copiar el archivo de env: `cp .env.example .env`.
3. Levantar la infraestructura: `make dev`.
4. Ejecutar la API: `cargo run -p api`.

## Convenciones
Consulta el archivo [CONVENTIONS.md](./CONVENTIONS.md) para conocer las reglas de ramas y mensajes de commit.
