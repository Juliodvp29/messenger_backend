# Messenger Backend

[![CI - Messenger Backend](https://github.com/Juliodvp29/messenger_backend/actions/workflows/ci.yml/badge.svg)](https://github.com/Juliodvp29/messenger_backend/actions/workflows/ci.yml)

Backend modular en Rust para una aplicación de mensajería estilo WhatsApp/Telegram con cifrado E2E.

## Fases de Desarrollo

| Fase | Descripción | Estado |
|------|------------|--------|
| 01 | Fundamentos y Configuración | ✅ Completado |
| 02 | Base de Datos y Migraciones | ✅ Completado |
| 03 | Autenticación y Seguridad | ✅ Completado |
| 04 | Cifrado E2E | 🔄 En desarrollo |
| 05 | Chats y Mensajes | ⏳ Pendiente |
| 06 | Tiempo Real (WebSockets) | ⏳ Pendiente |
| 07-13 | Stories, Notificaciones, etc. | ⏳ Pendiente |

## Arquitectura

El proyecto utiliza un **Cargo Workspace** con la siguiente estructura:
- `api/` — Handlers Axum, routing, middleware
- `domain/` — Entidades puras de negocio y lógica core (sin dependencias de infraestructura)
- `infrastructure/` — PostgreSQL, Redis, MinIO, email, SMS
- `shared/` — Tipos comunes, configuración tipada, errores

```
crates/
├── api/
│   ├── src/
│   │   ├── handlers/   # auth, chats, messages, etc.
│   │   ├── middleware/ # auth, rate_limit, logging
│   │   ├── routes.rs
│   │   └── main.rs
├── domain/
│   ├── src/
│   │   ├── models/     # User, Chat, Message, etc.
│   │   ├── errors.rs
│   │   └── value_objects/
├── infrastructure/
│   ├── src/
│   │   ├── db/         # Queries sqlx, repositorios
│   │   ├── redis/      # Cliente Redis, pub/sub, cache
│   │   ├── storage/   # MinIO/S3
│   │   └── email/     # SMTP
└── shared/
    ├── src/
    │   ├── config.rs  # Configuracion tipada
    │   └── types.rs # Tipos comunes
```

## Stack Tecnológico

- **Lenguaje**: Rust (edition 2024, stable 1.94+)
- **Web Framework**: Axum 0.7
- **Base de Datos**: PostgreSQL 17 + SQLx (queries typesafe)
- **Cache y Mensajeria**: Redis 8
- **Storage**: MinIO (compatible AWS S3)
- **Email**: Lettre + SMTP
- **SMS OTP**: Proveedor externo (Twilio/AWS SNS)

## Endpoints — Fase 03 (Autenticación y Seguridad)

Todos los endpoints requieren formato JSON (`Content-Type: application/json`).

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| POST | `/auth/register` | Envía OTP de registro | No |
| POST | `/auth/verify-phone` | Verifica OTP y crea usuario | No |
| POST | `/auth/login` | Envía OTP de login | No |
| POST | `/auth/login/verify` | Verifica OTP y retorna tokens | No |
| POST | `/auth/refresh` | Renueva access token | No |
| POST | `/auth/logout` | Cierra sesión actual | Bearer |
| GET | `/auth/sessions` | Lista sesiones activas | Bearer |
| DELETE | `/auth/sessions/:id` | Elimina sesión específica | Bearer |
| POST | `/auth/recover` | Envía OTP de recuperación | No |
| POST | `/auth/recover/verify` | Verifica recovery y retorna tokens | No |
| POST | `/auth/2fa/setup` | Inicia setup 2FA | Bearer |
| POST | `/auth/2fa/setup/verify` | Verifica código TOTP | Bearer |
| POST | `/auth/2fa/verify` | Verifica TOTP después de login | No |
| GET | `/health` | Health check | No |

### Flujo de Registro

```bash
# 1. Solicitar OTP
curl -X POST http://localhost:3000/auth/register \
  -H "Content-Type: application/json" \
  -d '{"phone":"+573001234567","device_id":"uuid","device_name":"TestPhone","device_type":"android"}'

# 2. Verificar OTP (el código llega a Redis: otp:register:{phone})
redis-cli GET otp:register:+573001234567

# 3. Confirmar registro
curl -X POST http://localhost:3000/auth/verify-phone \
  -H "Content-Type: application/json" \
  -d '{"phone":"+573001234567","code":"123456"}'

# Response: { "access_token": "...", "refresh_token": "...", "expires_in": 900, "user": {...} }
```

### Flujo de Login

```bash
# 1. Solicitar OTP
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"phone":"+573001234567","device_id":"uuid","device_name":"TestPhone","device_type":"android"}'

# 2. Verificar OTP
curl -X POST http://localhost:3000/auth/login/verify \
  -H "Content-Type: application/json" \
  -d '{"phone":"+573001234567","code":"123456"}'
```

## Inicio Rápido

### Prerrequisitos
- Docker y Docker Compose
- Rust (stable toolchain)
- `make` (opcional)

### Instalación

```bash
# 1. Clonar el repositorio
git clone https://github.com/Juliodvp29/messenger_backend.git
cd messenger_backend

# 2. Copiar archivo de entorno
cp .env.example .env
# Editar .env con JWT_SECRET válido (64 bytes en base64)

# 3. Levantar infraestructura
make dev

# 4. Aplicar migraciones pendientes (si hay)
sqlx migrate run --database-url $DATABASE__URL

# 5. Ejecutar la API
cargo run -p api
```

### Servicios Levantados

| Servicio | Puerto | Descripción |
|----------|--------|-----------|
| PostgreSQL | 5434 | Base de datos principal |
| Redis | 6379 | Cache y cola de mensajes |
| MinIO | 9000 | Storage (S3 compatible) |
| MinIO Console | 9001 | Web UI para archivos |
| Mailhog | 1025 | Servidor SMTP (dev) |
| Mailhog UI | 8025 | Ver emails enviados |

### Makefile

```bash
make dev      # Levanta todos los servicios
make stop    # Detiene servicios
make migrate # Ejecuta migraciones
make seed    # Carga datos de desarrollo
make test    # Ejecuta tests
make lint    # Verifica formato y linting
```

## Convenciones de Código

- Rama principal: `develop`
- Commits: Conventional Commits (`feat`, `fix`, `chore`, `docs`, `test`, `refactor`)
- Usar `sqlx::query!` / `sqlx::query_as!` para queries typesafe
- Nunca interpolar input de usuario en SQL
- Dirección de dependencias: `api -> domain -> infrastructure -> shared`
- Errores centralizados con `thiserror`
- Middleware de autenticación con JWT

## Documentación Adicional

- [backend_roadmap.md](./backend_roadmap.md) — Plan de desarrollo completo
- [AGENTS.md](./AGENTS.md) — Instrucciones para agentes IA

## Seguridad

- Autenticación: OTP + PIN local (el servidor nunca conoce el PIN)
- Sesiones: JWT con rotación de refresh token (one-time use)
- 2FA: TOTP con códigos de respaldo
- Rate limiting en endpoints de login/OTP
- El servidor es un relay cifrado — nunca desencripta mensajes

## Documentación Adicional

- [backend_roadmap.md](./backend_roadmap.md) — Plan de desarrollo completo
- [AGENTS.md](./AGENTS.md) — Instrucciones para agentes IA