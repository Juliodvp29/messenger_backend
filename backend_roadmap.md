# Backend Roadmap — Messenger App

## Rust + Axum + PostgreSQL 17 + Redis 8

<!--
  INSTRUCCIONES PARA LA IA QUE LEA ESTE DOCUMENTO:

  Este documento es el plan de desarrollo completo del backend de una app
  de mensajería cifrada tipo WhatsApp/Telegram.

  Estructura de cada fase:
    - META: qué debe estar funcionando al terminar la fase
    - CONTEXTO: decisiones de diseño ya tomadas que no se deben contradecir
    - SUBFASES: pasos ordenados con criterios de completitud explícitos
    - DEPENDENCIAS: qué fases deben estar completas antes de empezar esta
    - GITFLOW: nombre de la rama que corresponde a esta fase

  Reglas que la IA debe respetar siempre:
    1. El stack es Rust + Axum. No sugerir otros lenguajes o frameworks.
    2. El schema PostgreSQL ya está definido y no debe modificarse sin justificación.
    3. Usar sqlx con queries typesafe — no ORMs.
    4. Los mensajes NUNCA se desencriptan en el servidor. El servidor es un relay.
    5. Redis tiene 7 roles distintos — no mezclarlos (ver Fase 11).
    6. Seguir el orden de las fases. No saltarse dependencias.
-->

---

## METADATOS DEL PROYECTO

```yaml
proyecto: Messenger Backend
stack:
  lenguaje: Rust (edition 2026, stable 1.94+)
  framework_web: Axum
  base_de_datos: PostgreSQL 17
  cache_y_mensajeria: Redis 8
  storage: MinIO
  email: Lettre + SMTP
  sms_otp: Proveedor externo (Twilio / AWS SNS)

metodologia:
  versionado: GitFlow
  ramas_principales: [main, develop]
  prefijo_features: "feature/FASE-{N}.{M}-{descripcion}"
  prefijo_releases: "release/{X}.Y.Z"
  prefijo_hotfix: "hotfix/{descripcion}"
  commits: Conventional Commits (feat, fix, chore, docs, test, refactor)

arquitectura: >
  Workspace de Cargo con 4 crates:
    - api          → handlers Axum, routing, middleware
    - domain       → entidades, value objects, lógica de negocio pura (sin deps de infra)
    - infrastructure → db, redis, s3, email, sms
    - shared       → tipos comunes, errores, configuración

cifrado:
  protocolo: Signal Protocol / X3DH (Extended Triple Diffie-Hellman)
  mensajes: AES-256-GCM (ciphertext + IV almacenados, nunca plaintext)
  claves: identity_key (IK), signed_prekey (SPK), one_time_prekeys (OPK)
  principio: "El servidor nunca lee el contenido. Es un relay cifrado."

schema_sql: >
  Ya definido y versionado. Ver schema.sql en el repositorio.
  No modificar tablas existentes — crear migraciones nuevas si se necesita cambiar algo.
```

---

## RESUMEN DE FASES

| #   | Fase                        | Entrega                                         | Dependencias |
| --- | --------------------------- | ----------------------------------------------- | ------------ |
| 01  | Fundamentos y Configuración | Repositorio compila, CI verde, Docker up        | —            |
| 02  | Base de Datos y Migraciones | sqlx conectado, todas las migraciones aplicadas | 01           |
| 03  | Autenticación y Seguridad   | Registro, login, JWT, 2FA, sesiones             | 02           |
| 04  | Cifrado E2E                 | Upload de claves, X3DH, key bundles             | 03           |
| 05  | Chats y Mensajes            | CRUD completo, paginación, adjuntos S3          | 04           |
| 06  | Tiempo Real                 | WebSockets, Redis Pub/Sub, presencia, typing    | 05           |
| 07  | Stories y Reacciones        | CRUD stories, vistas, privacidad granular       | 05           |
| 08  | Notificaciones              | Push FCM/APNs, in-app, Redis queues             | 06           |
| 09  | Grupos y Canales            | Roles, invitaciones, rotación de claves         | 05           |
| 10  | Búsqueda y Contactos        | Búsqueda de usuarios, sincronización de agenda  | 03           |
| 11  | Performance y Caché         | Redis caché, rate limiting, query optimization  | 06           |
| 12  | Observabilidad y Hardening  | Logs, métricas, security review                 | 11           |
| 13  | Despliegue y Producción     | Docker multi-stage, CI/CD completo, runbooks    | 12           |

---

## FASE 01 — Fundamentos y Configuración

```yaml
meta: >
  Repositorio listo para desarrollo. Un `cargo build` limpio pasa sin warnings.
  CI verde en el primer push. Docker Compose levanta todos los servicios locales.
  Antes de escribir una línea de lógica de negocio, la infraestructura de desarrollo
  está sólida.

gitflow_rama: "feature/FASE-01-fundamentos"
dependencias: []
```

### 1.1 — Workspace Rust y estructura de crates

**Objetivo:** Monorepo Rust organizado. La separación en crates previene dependencias circulares y acelera compilación incremental.

```
messenger_backend/
├── Cargo.toml              ← workspace root con [workspace.dependencies]
├── crates/
│   ├── api/                ← handlers, routing, middleware Axum
│   │   └── src/
│   │       ├── handlers/   ← un módulo por dominio: auth, chats, messages, etc.
│   │       ├── middleware/ ← authn, rate_limit, request_id, logging
│   │       └── routes.rs   ← registro de todas las rutas
│   ├── domain/             ← sin dependencias de infraestructura
│   │   └── src/
│   │       ├── models/     ← structs de dominio (User, Chat, Message, etc.)
│   │       ├── errors.rs   ← AppError enum con thiserror
│   │       └── value_objects/ ← PhoneNumber, UserId, etc. con validación
│   ├── infrastructure/     ← implementaciones concretas
│   │   └── src/
│   │       ├── db/         ← queries sqlx, repositorios
│   │       ├── redis/      ← cliente Redis, pub/sub, caché
│   │       ├── storage/    ← S3/R2 client
│   │       └── email/      ← lettre SMTP
│   └── shared/             ← tipos y utilidades compartidos entre crates
│       └── src/
│           ├── config.rs   ← struct Config con todas las env vars
│           └── types.rs    ← tipos primitivos compartidos
├── migrations/             ← archivos SQL de sqlx migrate
├── seeds/                  ← datos de desarrollo (nunca en producción)
├── tests/                  ← integration tests (necesitan DB y Redis)
├── docker-compose.yml
├── Makefile
└── .env.example
```

**Criterio de completitud:** `cargo build --workspace` sin warnings. `cargo clippy --workspace -- -D warnings` sin errores.

---

### 1.2 — Dependencias principales (Cargo.toml)

**Objetivo:** Centralizar versiones en `[workspace.dependencies]` para evitar conflictos entre crates.

```toml
# Cargo.toml (workspace root) — versiones a usar
[workspace.dependencies]

# Web
axum = { version = "0.7", features = ["ws", "multipart"] }
tokio = { version = "1", features = ["full"] }
tower = { version = "0.4" }
tower-http = { version = "0.5", features = ["cors", "trace", "compression-gzip"] }

# Base de datos
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls", "macros", "uuid", "chrono", "json"] }

# Redis
redis = { version = "0.25", features = ["tokio-comp", "connection-manager", "aio"] }

# Serialización
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Autenticación y seguridad
jsonwebtoken = "9"
argon2 = "0.5"
totp-rs = { version = "5", features = ["qr"] }

# Tipos utilitarios
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
validator = { version = "0.18", features = ["derive"] }

# Errores
thiserror = "1"
anyhow = "1"

# Logging y métricas
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# AWS S3
aws-sdk-s3 = "1"
aws-config = "1"

# Email
lettre = { version = "0.11", features = ["tokio1", "tokio1-native-tls"] }

# Configuración
config = "0.14"

# Tests
fake = { version = "2.9", features = ["derive"] }
```

**Criterio de completitud:** `cargo update` sin conflictos. Cada crate declara solo las dependencias que usa directamente.

---

### 1.3 — GitFlow: configuración y convenciones

**Objetivo:** Establecer el flujo de trabajo antes de escribir código. Todo el equipo (o la IA al asistir) debe conocer estas reglas.

**Inicialización:**

```bash
git flow init
# Branch names:
#   Production: main
#   Development: develop
#   Feature prefix: feature/
#   Release prefix: release/
#   Hotfix prefix: hotfix/
#   Version tag prefix: v
```

**Flujo por subfase:**

```bash
# Iniciar subfase
git flow feature start FASE-01.3-gitflow-setup

# Durante el desarrollo: commits frecuentes
git commit -m "chore(gitflow): initialize git-flow configuration"
git commit -m "docs(gitflow): add branch naming conventions to README"

# Al completar la subfase: PR a develop → review → merge
git flow feature finish FASE-01.3-gitflow-setup
# → merge a develop, rama eliminada
```

**Convenciones de commits (Conventional Commits):**

```
feat(auth): implement JWT refresh token rotation
fix(db): correct connection pool exhaustion under load
chore(ci): add cargo-audit step to GitHub Actions pipeline
docs(api): document POST /auth/register endpoint
test(messages): add integration test for cursor pagination
refactor(domain): extract PhoneNumber value object
perf(redis): batch OPK inserts to reduce round trips
```

**Reglas de branch protection (configurar en GitHub/GitLab):**

- `main` y `develop`: requieren PR + al menos 1 review aprobado + CI verde
- Direct push bloqueado en `main` y `develop`
- Ramas `feature/*` se eliminan automáticamente después del merge

**Criterio de completitud:** Repositorio en GitHub/GitLab con branch protection activo. CI corre en cada push.

---

### 1.4 — Docker Compose: entorno local completo

**Objetivo:** Un solo `docker compose up` levanta todo lo necesario para desarrollo. El binario Rust corre en el host para máxima velocidad de compilación.

```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:17-alpine
    environment:
      POSTGRES_DB: messenger_dev
      POSTGRES_USER: messenger
      POSTGRES_PASSWORD: messenger_secret
    ports:
      - "5434:5434"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U messenger -d messenger_dev"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes # AOF persistence
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

  minio:
    image: minio/minio
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin123
    ports:
      - "9000:9000" # S3 API
      - "9001:9001" # Web console
    volumes:
      - minio_data:/data

  mailhog:
    image: mailhog/mailhog
    ports:
      - "1025:1025" # SMTP
      - "8025:8025" # Web UI para ver emails

volumes:
  postgres_data:
  redis_data:
  minio_data:
```

**Makefile:**

```makefile
.PHONY: dev stop migrate seed test lint build

dev:
	docker compose up -d
	@echo "Servicios: postgres:5432, redis:6379, minio:9000, mailhog:8025"

stop:
	docker compose down

migrate:
	sqlx migrate run --database-url $(DATABASE_URL)

seed:
	@if [ "$$APP_ENV" = "production" ]; then echo "ERROR: No seeds en producción"; exit 1; fi
	psql $(DATABASE_URL) -f seeds/dev.sql

test:
	cargo test --workspace

test-integration:
	cargo test --workspace --features integration

lint:
	cargo fmt --check
	cargo clippy --workspace -- -D warnings
	cargo audit

build:
	cargo build --release
```

**Criterio de completitud:** `make dev` levanta todos los servicios con healthchecks verdes. `make lint` pasa sin errores.

---

### 1.5 — Variables de entorno y configuración tipada

**Objetivo:** Toda la configuración viene de variables de entorno. La app valida su configuración al arrancar y falla con mensaje claro si falta algo.

```bash
# .env.example — versionar este archivo, NO el .env real
APP_ENV=development          # development | staging | production

# Base de datos
DATABASE_URL=postgres://messenger:messenger_secret@localhost:5432/messenger_dev
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=2

# Redis
REDIS_URL=redis://localhost:6379
REDIS_MAX_CONNECTIONS=20

# JWT — generar con: openssl rand -base64 64
JWT_SECRET=REEMPLAZAR_CON_64_BYTES_ALEATORIOS
JWT_REFRESH_SECRET=REEMPLAZAR_CON_64_BYTES_DISTINTOS
JWT_ACCESS_TTL_SECONDS=900        # 15 minutos
JWT_REFRESH_TTL_SECONDS=2592000   # 30 días

# Servidor
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# S3 / Storage
S3_ENDPOINT=http://localhost:9000
S3_BUCKET=messenger-attachments
S3_ACCESS_KEY_ID=minioadmin
S3_SECRET_ACCESS_KEY=minioadmin123
S3_REGION=us-east-1

# Email
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_FROM=noreply@messenger.local

# SMS (para OTP de registro y recuperación)
SMS_PROVIDER=twilio             # twilio | aws_sns
SMS_API_KEY=REEMPLAZAR
SMS_SENDER_ID=+1234567890

# Seguridad
RATE_LIMIT_ENABLED=true
CORS_ORIGINS=http://localhost:3001,http://localhost:19006
```

```rust
// shared/src/config.rs — estructura tipada, validada al arrancar
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub app_env: AppEnv,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub s3: S3Config,
    pub smtp: SmtpConfig,
}

impl Config {
    /// Carga y valida la configuración. Panic con mensaje claro si falta algo.
    pub fn load() -> Self {
        config::Config::builder()
            .add_source(config::Environment::default().separator("_"))
            .build()
            .expect("Error leyendo configuración")
            .try_deserialize()
            .expect("Configuración inválida — revisar variables de entorno")
    }
}
```

**Criterio de completitud:** La app arranca y muestra error claro si falta `JWT_SECRET`. `Config::load()` cubre todas las variables necesarias.

---

### 1.6 — CI/CD inicial con GitHub Actions

**Objetivo:** Pipeline que corre en cada push y bloquea merges si algo falla.

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: ["**"]
  pull_request:
    branches: [develop, main]

jobs:
  quality:
    name: Calidad de código
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2 # caché de compilación entre runs

      - name: Formato
        run: cargo fmt --check

      - name: Linting
        run: cargo clippy --workspace -- -D warnings

      - name: Auditoría de seguridad
        run: |
          cargo install cargo-audit --locked
          cargo audit

  tests:
    name: Tests unitarios
    runs-on: ubuntu-latest
    needs: quality
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Tests
        run: cargo test --workspace

  integration:
    name: Tests de integración
    runs-on: ubuntu-latest
    needs: tests
    if: github.ref == 'refs/heads/develop' || startsWith(github.ref, 'refs/heads/release/')
    services:
      postgres:
        image: postgres:17-alpine
        env:
          POSTGRES_DB: messenger_test
          POSTGRES_USER: messenger
          POSTGRES_PASSWORD: messenger_secret
        ports: ["5434:5434"]
        options: --health-cmd pg_isready --health-interval 5s
      redis:
        image: redis:7-alpine
        ports: ["6379:6379"]
        options: --health-cmd "redis-cli ping" --health-interval 5s
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Tests de integración
        env:
          DATABASE_URL: postgres://messenger:messenger_secret@localhost:5432/messenger_test
          REDIS_URL: redis://localhost:6379
        run: cargo test --workspace --features integration
```

**Criterio de completitud:** CI verde en el primer push a `develop`. Badge de CI en el README.

---

## FASE 02 — Base de Datos y Migraciones

```yaml
meta: >
  sqlx conectado a PostgreSQL con connection pool configurado.
  Todas las migraciones del schema aplicadas y verificadas.
  Los structs de dominio mapean correctamente a las tablas.
  Tests de integración básicos pasan con transacciones que hacen rollback.

gitflow_rama: "feature/FASE-02-database"
dependencias: ["FASE-01"]
```

### 2.1 — Conexión y pool con sqlx

**Objetivo:** `PgPool` compartido en `AppState` de Axum. Health check endpoint que verifica la DB.

```rust
// infrastructure/src/db/mod.rs
use sqlx::postgres::PgPoolOptions;

pub async fn create_pool(database_url: &str, max_conn: u32, min_conn: u32) -> PgPool {
    PgPoolOptions::new()
        .max_connections(max_conn)
        .min_connections(min_conn)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(600))
        .test_before_acquire(true)  // verifica que la conexión sigue viva
        .connect(database_url)
        .await
        .expect("No se pudo conectar a PostgreSQL")
}

// api/src/handlers/health.rs
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1").fetch_one(&state.db).await.is_ok();
    let redis_ok = state.redis.ping().await.is_ok();

    if db_ok && redis_ok {
        Json(json!({ "status": "ok", "db": "ok", "redis": "ok" }))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({ "status": "degraded" })))
    }
}
```

**Configuración del pool por entorno:**
| Parámetro | Desarrollo | Staging | Producción |
|-----------|-----------|---------|------------|
| max_connections | 20 | 50 | 100+ (con PgBouncer) |
| min_connections | 2 | 5 | 10 |
| acquire_timeout | 5s | 5s | 3s |

**Criterio de completitud:** `GET /health` retorna 200 con `db: "ok"` y `redis: "ok"`.

---

### 2.2 — Sistema de migraciones

**Objetivo:** Todas las tablas del schema creadas via `sqlx migrate run`. Migraciones versionadas en el repositorio.

**Orden de migraciones** (cada archivo es atómico, se aplica en transacción):

```
migrations/
├── 0001_extensions_and_enums.sql       ← pgcrypto, pg_trgm, btree_gin + todos los CREATe TYPE
├── 0002_functions_helpers.sql          ← trigger_set_updated_at, normalize_e164, enforce_private_chat_limit
├── 0003_users_and_keys.sql             ← users, user_profiles, user_keys, one_time_prekeys
├── 0004_sessions_contacts_blocks.sql   ← user_sessions, contacts (con trigger normalize), user_blocks
├── 0005_chats_and_participants.sql     ← chats (con CHECKs), chat_settings, chat_participants + trigger
├── 0006_messages.sql                   ← messages (con CHECKs), message_attachments, message_reactions, message_status
├── 0007_stories.sql                    ← stories, story_privacy_exceptions, story_views
├── 0008_notifications_audit.sql        ← notifications, audit_log + create_audit_log_partitions + DO$$ block
├── 0009_indexes.sql                    ← TODOS los índices (separados para aplicarlos concurrently)
├── 0010_rls_policies.sql               ← ALTER TABLE ... ENABLE ROW LEVEL SECURITY + políticas
└── 0011_views_and_utility_functions.sql ← v_chat_previews, v_active_stories, get_user_public_keys, mark_messages_read
```

> **REGLA CRÍTICA:** Nunca modificar un archivo de migración ya aplicado en producción.
> Para corregir algo, crear una nueva migración `0012_fix_algo.sql`.
> sqlx guarda el hash SHA-256 de cada archivo y detecta modificaciones retroactivas.

**Correr migraciones:**

```bash
# Desarrollo (automático al arrancar la app)
sqlx migrate run --database-url $DATABASE_URL

# Producción (paso separado del deploy, antes de actualizar el binario)
sqlx migrate run --database-url $DATABASE_URL
# Verificar que todas están aplicadas:
sqlx migrate info --database-url $DATABASE_URL
```

**Criterio de completitud:** `sqlx migrate info` muestra todas las migraciones en estado `Applied`. El schema coincide exactamente con `schema.sql`.

---

### 2.3 — Modelos de dominio y queries typesafe

**Objetivo:** Structs Rust que mapean a las tablas. Separación entre modelo de DB y modelo de dominio.

```rust
// domain/src/models/user.rs

/// Fila completa de la tabla users — solo para uso interno
/// NUNCA exponer directamente en la API (contiene password_hash)
#[derive(Debug, sqlx::FromRow)]
pub struct DbUser {
    pub id: Uuid,
    pub username: Option<String>,
    pub phone: String,
    pub email: Option<String>,
    pub password_hash: String,   // ← nunca sale de la capa de infrastructure
    pub two_fa_enabled: bool,
    pub is_active: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Modelo de dominio — seguro para pasar entre capas y serializar a JSON
#[derive(Debug, Clone, serde::Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: Option<String>,
    pub phone: String,           // nunca exponer en respuestas públicas
    pub two_fa_enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl From<DbUser> for User {
    fn from(db: DbUser) -> Self {
        Self {
            id: db.id,
            username: db.username,
            phone: db.phone,
            two_fa_enabled: db.two_fa_enabled,
            created_at: db.created_at,
        }
    }
}
```

**Patrón de query typesafe:**

```rust
// infrastructure/src/db/users.rs

pub async fn find_by_phone(pool: &PgPool, phone: &str) -> Result<Option<DbUser>, sqlx::Error> {
    // sqlx verifica esta query contra la DB en tiempo de compilación
    // Si el schema cambia y la query ya no es válida → error de compilación
    sqlx::query_as!(
        DbUser,
        r#"
        SELECT id, username, phone, email, password_hash,
               two_fa_enabled, is_active, last_seen_at, created_at, deleted_at
        FROM users
        WHERE phone = $1 AND deleted_at IS NULL
        "#,
        phone
    )
    .fetch_optional(pool)
    .await
}
```

> **IMPORTANTE para CI:** Ejecutar `cargo sqlx prepare --workspace` antes de cada commit.
> Genera `sqlx-data.json` que permite compilar sin conexión a DB en CI.
> Agregar este archivo al repositorio y verificar en CI que está actualizado.

**Criterio de completitud:** `cargo build --workspace` pasa con queries verificadas. `sqlx-data.json` en el repo y actualizado.

---

### 2.4 — Seeds de desarrollo

**Objetivo:** Datos de prueba realistas para desarrollo local. No corren en producción.

```sql
-- seeds/dev.sql
-- GUARDIA: este script no debe correr en producción
-- La app verifica APP_ENV antes de permitir el comando make seed

INSERT INTO users (id, phone, password_hash, username, is_active) VALUES
  ('00000000-0000-0000-0000-000000000001', '+573001000001', '$argon2id$...hash_de_password123...', 'alice', true),
  ('00000000-0000-0000-0000-000000000002', '+573001000002', '$argon2id$...hash_de_password123...', 'bob', true),
  ('00000000-0000-0000-0000-000000000003', '+573001000003', '$argon2id$...hash_de_password123...', 'carlos', true);

-- Insertar perfiles
INSERT INTO user_profiles (user_id, display_name) VALUES
  ('00000000-0000-0000-0000-000000000001', 'Alice'),
  ('00000000-0000-0000-0000-000000000002', 'Bob'),
  ('00000000-0000-0000-0000-000000000003', 'Carlos');

-- Chat privado entre Alice y Bob
INSERT INTO chats (id, type, created_by) VALUES
  ('10000000-0000-0000-0000-000000000001', 'private', '00000000-0000-0000-0000-000000000001');

INSERT INTO chat_participants (chat_id, user_id, role) VALUES
  ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', 'member'),
  ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000002', 'member');
```

**Criterio de completitud:** `make seed` carga los datos sin errores. `make dev && make seed` da un entorno de desarrollo funcional.

---

### 2.5 — Tests de integración con base de datos

**Objetivo:** Tests que verifican las queries reales contra PostgreSQL. Cada test es aislado y no contamina a otros.

```rust
// tests/db/users_test.rs

#[sqlx::test]  // sqlx::test crea una DB limpia por test y hace rollback al final
async fn test_create_user(pool: PgPool) {
    let phone = "+573001234567";
    let hash = "hash_de_prueba";

    let user = create_user(&pool, phone, hash).await.expect("Debería crear el usuario");

    assert_eq!(user.phone, phone);
    assert!(user.is_active);
    assert!(user.deleted_at.is_none());

    // Verificar que el número se normalizó a E.164
    let found = find_by_phone(&pool, phone).await.unwrap().unwrap();
    assert_eq!(found.phone, phone);
}

#[sqlx::test]
async fn test_private_chat_limit(pool: PgPool) {
    // El trigger de la DB debe rechazar un 3er participante en chat privado
    let chat_id = create_private_chat(&pool).await;
    add_participant(&pool, chat_id, user_id_1).await.unwrap();
    add_participant(&pool, chat_id, user_id_2).await.unwrap();

    let result = add_participant(&pool, chat_id, user_id_3).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("check_violation"));
}
```

**Criterio de completitud:** `make test-integration` pasa. Los tests prueban al menos: crear usuario, crear chat, enviar mensaje, consumir OPK atómicamente.

---

## FASE 03 — Autenticación y Seguridad

```yaml
meta: >
  Un usuario puede registrarse, verificar su número, iniciar sesión,
  obtener tokens JWT, renovarlos, habilitar 2FA con TOTP, y revocar sesiones.
  Todos los endpoints protegidos requieren token válido.

gitflow_rama: "feature/FASE-03-autenticacion"
dependencias: ["FASE-02"]

endpoints:
  - POST   /auth/register
  - POST   /auth/verify-phone
  - POST   /auth/login
  - POST   /auth/refresh
  - POST   /auth/logout
  - POST   /auth/2fa/setup
  - POST   /auth/2fa/verify-setup
  - POST   /auth/2fa/challenge
  - POST   /auth/2fa/disable
  - GET    /auth/sessions
  - DELETE /auth/sessions/:id
  - DELETE /auth/sessions
  - POST   /auth/forgot-password
  - POST   /auth/reset-password
```

### 3.1 — Registro de usuario

**Flujo:**

1. Cliente envía `{ phone, password, device_info }`
2. Validar formato E.164 del teléfono (misma lógica que el trigger de DB)
3. Verificar que el teléfono no está registrado (responder igual si está o no — no revelar)
4. Hash de contraseña con Argon2id
5. Generar OTP de 6 dígitos, guardarlo en Redis: `SETEX otp:register:{phone} 600 {code}`
6. Enviar OTP por SMS
7. Retornar `202 Accepted` con `{ message: "Código enviado" }`

**Rate limiting en este endpoint:**

```
clave Redis: rate:register:{ip_address}
límite: 5 intentos por hora por IP
acción si se excede: 429 Too Many Requests con Retry-After header
```

**Parámetros Argon2id:**

```rust
Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(
    65536,  // m_cost: 64 MB de memoria
    3,      // t_cost: 3 iteraciones
    4,      // p_cost: 4 hilos paralelos
    None,
).unwrap())
```

**Al verificar el OTP (POST /auth/verify-phone):**

1. Obtener OTP de Redis, verificar que coincide
2. `DEL otp:register:{phone}` — uso único
3. Insertar usuario en DB (dentro de transacción)
4. Insertar user_profile con defaults de privacidad
5. Insertar user_keys placeholder (campos vacíos — cliente subirá claves en Fase 4)
6. Emitir tokens JWT y crear sesión
7. Retornar `201 Created` con tokens y perfil básico

**Criterio de completitud:** Un usuario nuevo puede registrarse y obtener tokens. El número incorrecto en verify-phone retorna 400 sin revelar si el número existe.

---

### 3.2 — Login y emisión de tokens

**Flujo:**

1. Cliente envía `{ phone, password, device_id, device_name, device_type, push_token? }`
2. Buscar usuario por teléfono — si no existe: 401 genérico (mismo mensaje que contraseña incorrecta)
3. Verificar contraseña con `argon2::verify_password`
4. Si `two_fa_enabled = true`: retornar `202` con `{ two_fa_required: true, temp_token: JWT_corto }` — no emitir access/refresh tokens todavía
5. Si 2FA no requerido o ya verificado: emitir tokens

**Estructura de los tokens:**

```rust
// Access token — claims
#[derive(serde::Serialize, serde::Deserialize)]
struct AccessClaims {
    sub: Uuid,          // user_id
    sid: Uuid,          // session_id (en user_sessions)
    exp: i64,           // NOW + 900 segundos (15 min)
    iat: i64,
}

// Refresh token — opaco, almacenado en Redis
// El token en sí es un UUID v4 aleatorio
// En Redis: SETEX refresh:{SHA256(token)} 2592000 {session_json}
// SHA256 del token como clave — nunca el token en claro
```

**Rotación de refresh token (one-time use):**

```
Al usar el refresh token:
1. Calcular SHA256(token_recibido)
2. GET refresh:{hash} de Redis → obtener session_json
3. DEL refresh:{hash} — invalidar el token usado
4. Generar nuevo refresh token UUID
5. SETEX refresh:{SHA256(nuevo_token)} {session_json}
6. Retornar nuevo access token + nuevo refresh token
```

**Criterio de completitud:** Login retorna tokens. Usar el mismo refresh token dos veces retorna 401 (el segundo uso falla porque ya fue eliminado).

---

### 3.3 — Middleware de autenticación (Tower)

**Objetivo:** Capa de middleware que valida el JWT y hace disponible el usuario autenticado en todos los handlers.

```rust
// api/src/middleware/auth.rs

/// Extractor que cualquier handler puede usar para obtener el usuario autenticado
#[derive(Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

/// El middleware:
/// 1. Extrae el header Authorization: Bearer {token}
/// 2. Valida la firma JWT con JWT_SECRET
/// 3. Verifica que la sesión (sid) no esté revocada
///    — Primero en Redis (caché): GET session:{session_id} → hit: ok
///    — Si no está en caché: consultar user_sessions en DB
///    — Si está activa: cachear en Redis con TTL = tiempo restante del access token
/// 4. Inyectar AuthUser como Extension en el request
/// 5. Si cualquier paso falla: retornar 401 con mensaje genérico
```

**Rutas públicas** (no pasan por el middleware de auth):

```
POST /auth/register
POST /auth/verify-phone
POST /auth/login
POST /auth/refresh
POST /auth/forgot-password
POST /auth/reset-password
GET  /health
GET  /ws  (tiene su propia validación por query param)
```

**Criterio de completitud:** Un request con token expirado retorna 401. Un request con token válido de una sesión revocada retorna 401. Un request válido pasa al handler con `AuthUser` disponible.

---

### 3.4 — Autenticación de dos factores TOTP

**Setup:**

```
POST /auth/2fa/setup
  → Generar secreto TOTP (32 bytes aleatorios, encoding base32)
  → Guardar secreto CIFRADO en user_keys (no en texto plano)
  → Retornar: { secret_b32, qr_url, backup_codes: [10 UUIDs] }
  → Guardar backup_codes como hashes bcrypt en tabla aparte

POST /auth/2fa/verify-setup (con el código que genera el authenticator)
  → Verificar que el cliente puede generar códigos válidos
  → Marcar two_fa_enabled = true en users
  → Registrar acción two_fa_enable en audit_log
```

**Challenge durante login:**

```
POST /auth/2fa/challenge
  Body: { temp_token, code }
  → Validar temp_token (JWT de vida corta, 5 minutos)
  → Verificar código TOTP con ventana ±1 paso (30s de tolerancia para clock skew)
  → Anti-replay: SETEX totp:used:{user_id}:{code} 90 1 — si ya existe: 401
  → Si código válido: emitir access token + refresh token completos
  → Si es backup code: marcarlo como usado, no permitir reutilización
```

**Criterio de completitud:** Un código TOTP válido usado dos veces en la misma ventana de 30s retorna 401 en el segundo intento.

---

### 3.5 — Gestión de sesiones multi-device

```
GET /auth/sessions
  → Listar todas las filas de user_sessions donde user_id = auth_user.user_id
  → Marcar con is_current: true la sesión del token actual (comparar session_id)
  → Retornar: [{ id, device_name, device_type, ip_address, last_active_at, is_current }]

DELETE /auth/sessions/:session_id
  → Verificar que la sesión pertenece al usuario autenticado
  → DELETE FROM user_sessions WHERE id = session_id
  → DEL session:{session_id} en Redis (caché de validación)
  → DEL refresh:{hash} en Redis si se tiene el hash (o usar patrón de búsqueda)
  → Registrar en audit_log

DELETE /auth/sessions (revocar todas excepto la actual)
  → DELETE FROM user_sessions WHERE user_id = auth_user AND id != current_session_id
  → Limpiar todas las claves de sesión de Redis del usuario
```

**Criterio de completitud:** Después de `DELETE /auth/sessions`, solo la sesión actual sigue válida. Cualquier otro access token retorna 401.

---

### 3.6 — Recuperación de contraseña

```
POST /auth/forgot-password
  Body: { phone }
  → SIEMPRE retornar 200 (no revelar si el número existe)
  → Si el número existe: generar OTP 6 dígitos
  → SETEX otp:reset:{phone} 900 {code}  ← 15 minutos
  → Enviar OTP por SMS
  → Rate limit: 3 intentos por número por hora

POST /auth/reset-password
  Body: { phone, code, new_password }
  → GET otp:reset:{phone} de Redis
  → Verificar que el código coincide
  → DEL otp:reset:{phone} — uso único
  → Hash nueva contraseña con Argon2id
  → UPDATE users SET password_hash = $1 WHERE phone = $2
  → DELETE FROM user_sessions WHERE user_id = (SELECT id FROM users WHERE phone = $2)
  → Limpiar todas las sesiones de Redis del usuario
  → Registrar password_change en audit_log con ip_address
```

**Criterio de completitud:** Después de reset exitoso, el login con la contraseña anterior retorna 401. El token OTP de reset no puede reutilizarse.

---

## FASE 04 — Cifrado E2E (Signal Protocol)

```yaml
meta: >
  Los usuarios pueden subir sus claves públicas al servidor.
  El servidor puede entregar el key bundle de un usuario para establecer
  sesiones X3DH. El servidor nunca conoce las claves privadas ni puede
  leer el contenido de los mensajes.

gitflow_rama: "feature/FASE-04-e2e-encryption"
dependencias: ["FASE-03"]

endpoints:
  - POST /keys/upload
  - POST /keys/upload-prekeys    ← reponer OPKs sin cambiar identity_key
  - GET  /keys/:user_id
  - GET  /keys/:user_id/fingerprint
  - GET  /keys/me/count          ← cuántas OPKs quedan

principio_inmutable: >
  El servidor almacena ÚNICAMENTE claves públicas.
  Las claves privadas NUNCA salen del dispositivo del cliente.
  content_encrypted es opaco para el servidor — nunca lo decodifica.
```

### 4.1 — Upload de claves públicas

**Cuándo se invoca:**

- Al completar el registro (primer setup de claves)
- Al rotar el signed_prekey (cada ~7 días)
- Al reponer el pool de OPKs (cuando prekey_count < 20)

**Validaciones del servidor antes de guardar:**

1. `identity_key` es una clave pública Ed25519 válida (32 bytes en base64)
2. `signed_prekey_sig` es una firma válida: `Ed25519.verify(identity_key, signed_prekey, signature)`
3. Las OPKs tienen formato correcto: array de `{ id: integer, key: base64_string }`
4. No hay IDs de OPK duplicados en el batch ni con OPKs existentes del usuario

```
POST /keys/upload
  Body: {
    identity_key: "base64...",
    signed_prekey: { id: 1, key: "base64...", signature: "base64..." },
    one_time_prekeys: [{ id: 1, key: "base64..." }, ...]  ← mínimo 100 al registrarse
  }

  Flujo:
  → Verificar firma del SPK con la identity_key
  → UPSERT en user_keys (identity_key, signed_prekey, etc.)
  → INSERT en one_time_prekeys (bulk) con CONFLICT DO NOTHING para idempotencia
  → UPDATE user_keys SET prekey_count = (SELECT COUNT(*) FROM one_time_prekeys WHERE user_id = ?)
  → Retornar 200 con { prekey_count }
```

**Encabezado de alerta en CUALQUIER respuesta autenticada:**

```
Si (SELECT prekey_count FROM user_keys WHERE user_id = auth_user) < 20:
  → Agregar header: X-Prekey-Count: {n}
  → El cliente, al verlo, sube más OPKs en background
```

**Criterio de completitud:** Un usuario sube 100 OPKs. `one_time_prekeys` tiene 100 filas con `is_consumed = false`.

---

### 4.2 — Key bundle para iniciar sesión X3DH

```
GET /keys/:user_id
  → Verificar que el solicitante no está bloqueado por el target (y viceversa)
  → Llamar a la función PostgreSQL get_user_public_keys(target_user_id)
    — La función consume una OPK atómicamente con FOR UPDATE SKIP LOCKED
    — Si no hay OPKs: retorna null en one_time_prekey (X3DH sin OPK es válido, menor forward secrecy)
  → Retornar: {
      identity_key,
      signed_prekey: { id, key, signature },
      one_time_prekey: { id, key } | null
    }
  → Si prekey_count del target < 20: loguear warning (el target debería subir más OPKs pronto)
```

**Lo que el cliente hace con este bundle (fuera del servidor):**

```
1. Generar ephemeral key pair (EK) en el dispositivo
2. Calcular DH1 = DH(sender_IK_priv, recipient_SPK_pub)
3. Calcular DH2 = DH(EK_priv, recipient_IK_pub)
4. Calcular DH3 = DH(EK_priv, recipient_SPK_pub)
5. Si OPK disponible: DH4 = DH(EK_priv, recipient_OPK_pub)
6. master_secret = KDF(DH1 || DH2 || DH3 || DH4?)
7. Derivar clave AES-256 y IV inicial del master_secret
8. Cifrar el mensaje con AES-256-GCM
9. Enviar al servidor: ciphertext + IV + sender_IK_pub + sender_EK_pub + SPK_id_usado + OPK_id_usado
```

**Criterio de completitud:** Cada llamada a `GET /keys/:user_id` consume una OPK diferente. Después de 100 llamadas, `prekey_count = 0` y el header `X-Prekey-Count` aparece en respuestas del usuario target.

---

### 4.3 — Fingerprint para verificación fuera de banda

```
GET /keys/:user_id/fingerprint
  → Obtener identity_key del solicitante y del target
  → fingerprint = SHA256(min(IK_a, IK_b) || max(IK_a, IK_b))
    (ordenar lexicográficamente para que sea el mismo resultado en ambos lados)
  → Retornar: { fingerprint: "hex_string_60_chars", your_key: "...", their_key: "..." }

  → Si la identity_key del target cambió desde la última vez que se vio:
    → Marcar en la respuesta: { key_changed: true, changed_at: timestamp }
    → El cliente debe avisar al usuario: "La clave de seguridad de {nombre} cambió"
```

**Criterio de completitud:** El fingerprint es idéntico cuando A consulta el fingerprint de B y cuando B consulta el fingerprint de A.

---

## FASE 05 — Chats y Mensajes

```yaml
meta: >
  CRUD completo de chats y mensajes. Paginación eficiente por cursor.
  Adjuntos subidos directo a S3 desde el cliente. Reacciones.
  Todo el contenido llega y sale cifrado — el servidor no lo inspecciona.

gitflow_rama: "feature/FASE-05-chats-mensajes"
dependencias: ["FASE-04"]

endpoints:
  - POST   /chats
  - GET    /chats
  - GET    /chats/:id
  - PATCH  /chats/:id
  - DELETE /chats/:id
  - POST   /chats/:id/messages
  - GET    /chats/:id/messages
  - PATCH  /chats/:id/messages/:msg_id
  - DELETE /chats/:id/messages/:msg_id
  - POST   /chats/:id/messages/read
  - POST   /chats/:id/messages/:msg_id/reactions
  - DELETE /chats/:id/messages/:msg_id/reactions/:emoji
  - POST   /attachments/upload-url
  - POST   /attachments/confirm
```

### 5.1 — Gestión de chats

**Crear chat privado:**

```
POST /chats
  Body: { type: "private", participant_id: UUID } | { type: "group", name, participant_ids: [] }

  Para tipo "private":
  → Verificar que no existe ya un chat privado entre los dos usuarios:
    SELECT c.id FROM chats c
    JOIN chat_participants cp1 ON cp1.chat_id = c.id AND cp1.user_id = $sender
    JOIN chat_participants cp2 ON cp2.chat_id = c.id AND cp2.user_id = $recipient
    WHERE c.type = 'private' AND cp1.left_at IS NULL AND cp2.left_at IS NULL
    LIMIT 1
  → Si existe: retornar el chat existente (idempotente)
  → Si no existe: INSERT chats + INSERT chat_participants (2 filas)
  → El trigger enforce_private_chat_limit en la DB garantiza máximo 2 participantes

  Para tipo "group":
  → Crear chat con type = 'group'
  → Insertar creador con role = 'owner'
  → Insertar demás participantes con role = 'member'
  → El cliente debe generar una group_key AES-256, cifrarla para cada participante
    y enviarla en el body como encryption_key_enc por participante
```

**Listar chats (pantalla principal):**

```
GET /chats?cursor={cursor}&limit={50}
  → Usar la vista v_chat_previews del schema
  → Cursor: base64(is_pinned DESC, last_message_at DESC, chat_id)
  → El último mensaje viene cifrado — el cliente lo descifra para mostrar preview
  → Incluir unread_count: COUNT de message_status WHERE user_id = auth_user AND status != 'read'
```

**Criterio de completitud:** Crear chat privado dos veces devuelve el mismo chat_id. El chat 'self' existe para cada usuario al registrarse.

---

### 5.2 — Envío y recepción de mensajes

**Enviar mensaje:**

```
POST /chats/:chat_id/messages
  Body: {
    content_encrypted: "base64_aes_gcm_ciphertext",
    content_iv: "base64_12_bytes",
    message_type: "text" | "image" | "audio" | "video" | "file" | "location",
    reply_to_id: UUID | null,
    is_forwarded: bool,
    attachment_ids: [UUID],   ← UUIDs de message_attachments ya subidos y confirmados
    metadata: { ... }         ← solo para tipos no-text (dimensiones, duración, etc.)
  }

  Validaciones:
  → Verificar membership: el sender es participante activo del chat
  → Verificar bloqueos: si es chat privado, ninguno ha bloqueado al otro
  → Si message_type = 'text': content_encrypted y content_iv son obligatorios
  → Si attachment_ids no vacío: verificar que los attachments pertenecen al sender y están confirmados

  Al insertar:
  → INSERT messages (retorna el mensaje completo)
  → INSERT message_status para cada participante activo con status = 'sent'
  → Publicar evento en Redis Pub/Sub: PUBLISH user:{recipient_id}:events {event_json}
  → Retornar 201 con el mensaje insertado
```

**Listar mensajes con paginación por cursor:**

```
GET /chats/:chat_id/messages?cursor={cursor}&limit={50}&direction={before|after}

  Cursor basado en (created_at, id) — evita duplicados/saltos:
  Sin cursor: los 50 mensajes más recientes
  Con cursor: los 50 anteriores o posteriores al cursor

  Query:
  SELECT * FROM messages
  WHERE chat_id = $1
    AND deleted_at IS NULL
    AND (created_at, id) < ($cursor_ts, $cursor_id)  ← si direction = before
  ORDER BY created_at DESC, id DESC
  LIMIT 51  ← pedir 51 para saber si hay más página

  Si se retornan 51 resultados: has_more = true, incluir next_cursor
  Retornar los 50 primeros en orden cronológico (ASC) para que el cliente los muestre correctamente
```

**Criterio de completitud:** La paginación con 1000 mensajes en un chat retorna páginas consistentes sin saltos ni duplicados aunque lleguen mensajes nuevos durante la navegación.

---

### 5.3 — Adjuntos con S3 (upload directo desde cliente)

**Flujo de dos pasos:**

```
Paso 1: POST /attachments/upload-url
  Body: { file_type: "image/jpeg", file_size: 1048576, chat_id: UUID }

  Validaciones:
  → file_type debe estar en la lista de MIME types permitidos
  → file_size no debe exceder el límite según el tipo
  → sender es participante del chat

  Respuesta:
  → Generar object key: attachments/{chat_id}/{uuid}/{filename}
  → Generar PUT URL pre-firmada de S3 con TTL 5 minutos
  → Crear registro pendiente en message_attachments con una flag `confirmed = false`
  → Retornar: { upload_url, attachment_id, expires_at }

Paso 2: POST /attachments/confirm
  Body: { attachment_id: UUID, message_id: UUID, encryption_key_enc: "...", encryption_iv: "..." }

  → Verificar que el objeto existe en S3 (HEAD request)
  → Marcar attachment como confirmed = true
  → Actualizar message_id en el attachment
  → Encolar job de thumbnail generation en Redis (si es imagen o video)
  → Retornar 200
```

**Límites por tipo:**
| Tipo | MIME types | Tamaño máximo |
|------|-----------|---------------|
| Imagen | image/jpeg, image/png, image/webp, image/gif | 25 MB |
| Video | video/mp4, video/quicktime, video/webm | 100 MB |
| Audio | audio/mpeg, audio/ogg, audio/aac, audio/wav | 25 MB |
| Archivo | cualquiera | 100 MB |

**Criterio de completitud:** El cliente puede subir una imagen, confirmarla, y el mensaje contiene el attachment_id. Los thumbnails se generan asíncronamente sin bloquear la respuesta.

---

### 5.4 — Estado de mensajes y reacciones

**Marcar como leído:**

```
POST /chats/:chat_id/messages/read
  Body: { up_to: "timestamp ISO8601" }
  → Llamar a la función PostgreSQL mark_messages_read(user_id, chat_id, up_to)
  → Publicar evento en Redis: { type: "messages_read", chat_id, user_id, up_to }
  → El sender original recibirá el evento por WebSocket y actualizará sus ticks
```

**Reacciones:**

```
POST /chats/:chat_id/messages/:msg_id/reactions
  Body: { reaction: "👍" }
  → Verificar que reaction está en el conjunto permitido (configurar en el servidor)
  → INSERT message_reactions ON CONFLICT DO NOTHING
  → Publicar evento reaction_added en Redis Pub/Sub
  → Retornar 201 (o 200 si ya existía — idempotente)

DELETE /chats/:chat_id/messages/:msg_id/reactions/:emoji
  → DELETE FROM message_reactions WHERE message_id AND user_id AND reaction = emoji
  → Publicar evento reaction_removed
```

---

## FASE 06 — Tiempo Real (WebSockets + Redis Pub/Sub)

```yaml
meta: >
  Los mensajes llegan en tiempo real sin polling.
  Múltiples instancias del servidor pueden escalar horizontalmente
  gracias a Redis Pub/Sub como bus de eventos.
  Presencia y typing indicators funcionan sin consultas a la DB.

gitflow_rama: "feature/FASE-06-realtime"
dependencias: ["FASE-05"]

endpoints:
  - GET /ws  ← upgrade a WebSocket

evento_formato: |
  {
    "type": "new_message" | "message_edited" | "message_deleted" |
            "reaction_added" | "reaction_removed" | "messages_read" |
            "user_online" | "user_offline" | "typing_start" | "typing_stop" |
            "call_incoming" | "key_changed",
    "payload": { ... },
    "timestamp": "ISO8601"
  }
```

### 6.1 — WebSocket handler en Axum

```rust
// api/src/handlers/ws.rs

/// Estado compartido entre todos los handlers — el DashMap vive aquí
pub struct WsState {
    /// user_id → lista de senders WS activos (multi-device)
    pub connections: Arc<DashMap<Uuid, Vec<Arc<Mutex<WebSocketSender>>>>>,
}

/// Upgrade HTTP → WebSocket
/// La autenticación va en query param porque el protocolo WS no soporta headers custom:
///   ws://host/ws?token=JWT_ACCESS_TOKEN
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Validar el JWT del query param ANTES del upgrade
    let claims = validate_jwt(&params.token, &state.config.jwt_secret)?;

    ws.on_upgrade(move |socket| handle_socket(socket, claims.user_id, claims.session_id, state))
}

async fn handle_socket(socket: WebSocket, user_id: Uuid, session_id: Uuid, state: AppState) {
    // 1. Registrar conexión en DashMap
    // 2. Actualizar presencia en Redis: SET presence:{user_id} 1 EX 65
    // 3. Publicar evento user_online a todos los contactos del usuario
    // 4. Suscribirse al canal Redis: user:{user_id}:events
    // 5. Loop: select! entre mensajes entrantes del cliente y eventos de Redis
    // 6. Heartbeat: enviar ping cada 30s, cerrar si no hay pong en 10s
    // Al desconectar: limpiar DashMap, DEL presence:{user_id}, publicar user_offline
}
```

**Criterio de completitud:** Abrir dos conexiones WS con el mismo usuario (multi-device) funciona. Un mensaje enviado llega a ambas conexiones.

---

### 6.2 — Redis Pub/Sub para distribución entre instancias

**Canales:**

```
user:{user_id}:events  → eventos dirigidos a un usuario específico
system:broadcasts      → eventos globales (mantenimiento, announcements)
```

**Flujo cuando llega un mensaje nuevo:**

```
1. Handler POST /messages inserta en DB
2. Handler publica en Redis:
   PUBLISH user:{recipient_id}:events {
     "type": "new_message",
     "payload": { mensaje completo cifrado },
     "timestamp": "..."
   }
3. CADA instancia del servidor tiene un task Tokio suscrito a Redis
4. La instancia que tiene conexiones activas del recipient recibe el evento
5. Busca en su DashMap local al recipient
6. Si está conectado: envía el evento JSON por WebSocket
7. Si no está conectado: el evento se descarta de Redis (la DB ya tiene el mensaje,
   la notificación push se encola separadamente — ver Fase 8)
```

**Criterio de completitud:** Con dos instancias del servidor en Docker, un mensaje enviado a la instancia A llega en tiempo real al usuario conectado a la instancia B.

---

### 6.3 — Presencia y typing indicators

**Presencia (online/offline):**

```
Online:  SET presence:{user_id} 1 EX 65
         → Se renueva cada 60s con el heartbeat del WS
         → TTL de 65s = 5s de margen si el ping llega tarde

Offline: Al cerrar el socket → DEL presence:{user_id}
         → UPDATE users SET last_seen_at = NOW()
         → Solo esta operación toca la DB — no los 60s de heartbeat

Consultar si alguien está online:
  GET presence:{user_id} → "1" = online, nil = offline
  El campo last_seen_at en la DB da el último momento offline
```

**Typing indicators (no se persisten en DB):**

```
Cliente envía por WS: { "type": "typing_start", "chat_id": UUID }
  → El servidor reenvía a los otros participantes del chat (Redis Pub/Sub)
  → No hay INSERT en ninguna tabla

Cliente envía por WS: { "type": "typing_stop", "chat_id": UUID }
  → Ídem

El servidor también puede inferir typing_stop si no llega typing_start en 5s
```

**Criterio de completitud:** `GET presence:{user_id}` en Redis muestra "1" cuando el usuario tiene una conexión WS activa. Al cerrar el socket, la clave desaparece en máximo 65 segundos.

---

### 6.4 — Gap sync al reconectar

```
El cliente guarda localmente el timestamp del último mensaje recibido.
Al reconectar el WebSocket:
  1. Cliente envía: { "type": "sync_request", "since": "2024-01-15T10:30:00Z" }
  2. Servidor responde con todos los mensajes de todos los chats del usuario
     llegados después de "since" que aún no tiene el cliente
  3. También retorna los message_status que cambiaron desde "since"

Alternativamente (más simple): el cliente llama a GET /chats/:id/messages?since=timestamp
para cada chat donde sospecha que perdió mensajes (heurística del cliente).
```

---

## FASE 07 — Stories y Reacciones

```yaml
meta: >
  Los usuarios pueden publicar estados (stories) que expiran en 24h.
  La privacidad es granular (5 modos). Las vistas y reacciones funcionan
  en tiempo real. La expiración es automática via pg_cron.

gitflow_rama: "feature/FASE-07-stories"
dependencias: ["FASE-05"]

endpoints:
  - POST   /stories
  - GET    /stories
  - GET    /stories/my
  - DELETE /stories/:id
  - POST   /stories/:id/view
  - POST   /stories/:id/react
  - GET    /stories/:id/views
```

### 7.1 — CRUD de stories

**Publicar story:**

```
POST /stories
  Body: {
    content_url: "https://s3.../story_media/...",
    content_type: "image" | "video",
    caption: "texto opcional",
    privacy: "everyone" | "contacts" | "contacts_except" | "selected" | "only_me",
    exceptions: [user_id, ...]   ← para privacidad contacts_except o selected
  }

  → INSERT stories
  → Si exceptions no vacío: INSERT story_privacy_exceptions
  → expires_at = created_at + INTERVAL '24 hours' (la DB usa DEFAULT)
  → Publicar evento en Redis para contactos que pueden ver el story
```

**Listar stories de contactos:**

```
GET /stories
  → Usar vista v_active_stories del schema
  → Filtrar según privacidad de cada story:
    - everyone: siempre visible
    - contacts: visible si auth_user está en contacts del publicador
    - contacts_except: visible si auth_user está en contacts Y NO en story_privacy_exceptions
    - selected: visible solo si auth_user está en story_privacy_exceptions con is_excluded=false
    - only_me: nunca visible para otros
  → Ordenar: historias no vistas primero, luego las vistas, por updated_at DESC
  → Agrupar por usuario: { user, stories: [...] }
```

**Expiración automática (pg_cron, configurado en Fase 13):**

```sql
-- Cada hora, marcar stories expiradas
UPDATE stories SET deleted_at = NOW()
WHERE expires_at < NOW() AND deleted_at IS NULL;
```

**Criterio de completitud:** Una story publicada con `privacy: "only_me"` no aparece en el `GET /stories` de ningún otro usuario.

---

### 7.2 — Vistas y reacciones a stories

```
POST /stories/:id/view
  → Verificar que el auth_user puede ver esta story (misma lógica de privacidad)
  → INSERT story_views ON CONFLICT (story_id, viewer_id) DO UPDATE SET viewed_at = NOW()
  → Publicar evento en Redis al autor: { type: "story_viewed", viewer_id, story_id }
  → Retornar 200

POST /stories/:id/react
  Body: { reaction: "❤️" }
  → Verificar que el auth_user puede ver la story
  → UPDATE story_views SET reaction = $emoji WHERE story_id AND viewer_id = auth_user
    (la vista debe existir — no se puede reaccionar sin ver)
  → Publicar evento al autor: { type: "story_reaction", viewer_id, reaction }

GET /stories/:id/views
  → Solo accesible por el autor de la story
  → Retornar: [{ viewer: { id, display_name, avatar_url }, viewed_at, reaction }]
  → Ordenar por viewed_at DESC
```

---

## FASE 08 — Notificaciones

```yaml
meta: >
  Los usuarios reciben notificaciones push cuando están offline.
  Las notificaciones in-app se muestran cuando están en la app.
  Las preferencias de silenciado se respetan.

gitflow_rama: "feature/FASE-08-notificaciones"
dependencias: ["FASE-06"]

endpoints:
  - GET    /notifications
  - PATCH  /notifications/:id/read
  - PATCH  /notifications/read-all
  - DELETE /notifications/read
  - PATCH  /chats/:id/settings   ← silenciar/pinear/archivar chat
```

### 8.1 — Push notifications (FCM + APNs)

**Arquitectura:**

```
Cuando un evento requiere notificación push:
1. Verificar si el usuario está online (GET presence:{user_id} en Redis)
   → Si está online: el WebSocket ya entregó el evento, no enviar push
   → Si está offline: encolar push notification

2. Verificar preferencias:
   → chat_settings.is_muted = true → NO enviar push
   → chat_settings.muted_until > NOW() → NO enviar push

3. Encolar en Redis:
   LPUSH push:queue { user_id, session_ids: [...], type, payload_json }

4. Worker Tokio (loop con BRPOP):
   → Obtener job de la cola
   → Para cada session_id del usuario: obtener push_token de user_sessions
   → Enviar a FCM (Android/Web) o APNs (iOS) según device_type
   → Si el token es inválido (error 400/410 de FCM/APNs): DELETE FROM user_sessions WHERE id = session_id
   → Si falla temporalmente: reintentar hasta 3 veces con backoff exponencial
```

**Payload del push (mínimo para privacidad):**

```json
{
  "type": "new_message",
  "chat_id": "uuid",
  "sender_id": "uuid"
}
```

> **NUNCA** incluir el contenido del mensaje en el payload del push.
> El cliente recibe la notificación, abre la app, y obtiene el mensaje por WebSocket o REST.

**Criterio de completitud:** Con el usuario offline (sin conexión WS activa), un mensaje nuevo genera un push notification. El token inválido se elimina de user_sessions automáticamente.

---

### 8.2 — Notificaciones in-app

```
GET /notifications?cursor={cursor}&limit={20}
  → SELECT * FROM notifications WHERE user_id = auth_user AND is_read = false
  → Cursor basado en created_at DESC
  → Retornar: [{ id, type, data, is_read, created_at }]

PATCH /notifications/:id/read
  → UPDATE notifications SET is_read = true, read_at = NOW() WHERE id AND user_id = auth_user

PATCH /notifications/read-all
  → UPDATE notifications SET is_read = true, read_at = NOW()
    WHERE user_id = auth_user AND is_read = false

DELETE /notifications/read
  → DELETE FROM notifications WHERE user_id = auth_user AND is_read = true
```

**Configuración de chat (silenciar, pinear, archivar):**

```
PATCH /chats/:id/settings
  Body: { is_muted?, muted_until?, is_pinned?, pin_order?, is_archived? }
  → UPSERT chat_settings (user_id, chat_id) con los campos recibidos
```

---

## FASE 09 — Grupos y Canales

```yaml
meta: >
  Los grupos tienen roles jerárquicos. Los participantes pueden añadirse y
  eliminarse. La clave de grupo se rota al cambiar la membresía.
  Los canales solo permiten broadcast de admins.

gitflow_rama: "feature/FASE-09-grupos"
dependencias: ["FASE-05"]

endpoints:
  - POST   /chats/:id/participants
  - DELETE /chats/:id/participants/:user_id
  - PATCH  /chats/:id/participants/:user_id/role
  - POST   /chats/:id/invite-link
  - DELETE /chats/:id/invite-link
  - POST   /chats/join/:slug
  - PATCH  /chats/:id  ← actualizar nombre, avatar, descripción del grupo
```

### 9.1 — Gestión de participantes y roles

**Jerarquía de roles** (mayor número = más permisos):

```
owner (4) > admin (3) > moderator (2) > member (1)

Reglas:
- Solo owner puede: eliminar el grupo, transferir ownership, degradar admins
- Admins pueden: añadir/eliminar members, promover members a moderator
- Moderators pueden: eliminar mensajes de members
- Members: solo enviar y leer mensajes
- Un admin NO puede cambiar el rol de otro admin ni del owner
```

**Añadir participante:**

```
POST /chats/:id/participants
  Body: { user_id: UUID, encryption_key_enc: "base64..." }

  → Verificar que el solicitante tiene rol >= admin en el grupo
  → Verificar que el usuario target no está bloqueando al solicitante
  → Verificar que el grupo no excede max_members (si aplica)
  → INSERT chat_participants
  → El encryption_key_enc es la group_key cifrada para el nuevo participante
    (el cliente generó esto antes de hacer el request)
  → Publicar evento participant_added
  → Registrar en audit_log
```

**Eliminar participante / salir del grupo:**

```
DELETE /chats/:id/participants/:user_id
  → Si user_id == auth_user: es "salir del grupo" (cualquiera puede)
  → Si user_id != auth_user: requiere rol >= admin Y target tiene menor rol
  → UPDATE chat_participants SET left_at = NOW()
  → CRÍTICO: rotar la group_key
    1. El servidor marca en la respuesta que la key fue rotada
    2. Un admin activo del grupo genera nueva group_key AES-256
    3. El admin llama a POST /chats/:id/rotate-key con la nueva key cifrada para cada miembro restante
    4. UPDATE chat_participants SET encryption_key_enc = ... para cada miembro
```

**Criterio de completitud:** Al eliminar un participante, los mensajes nuevos usan una group_key que el participante eliminado no puede derivar.

---

### 9.2 — Invitaciones y links

```
POST /chats/:id/invite-link
  → Solo admins y owner pueden generar links
  → Generar slug: 16 bytes aleatorios en base62 (URL-safe, ~22 caracteres)
  → UPDATE chats SET invite_link = slug
  → Retornar: { invite_link: "https://app.io/join/{slug}" }

DELETE /chats/:id/invite-link
  → UPDATE chats SET invite_link = NULL
  → El link anterior ya no funciona

POST /chats/join/:slug
  → SELECT * FROM chats WHERE invite_link = slug AND deleted_at IS NULL
  → Verificar que el chat es de tipo group o channel (private no tiene invites)
  → Verificar max_members si aplica
  → Verificar que el usuario no es ya participante activo
  → INSERT chat_participants con role = 'member'
  → En la respuesta: incluir la group_key cifrada para el nuevo miembro
    (un admin debe aprovisionar esto — flujo de key exchange pendiente)
```

---

## FASE 10 — Búsqueda y Contactos

```yaml
meta: >
  Los usuarios pueden buscar otros usuarios por username o teléfono.
  La sincronización de agenda revela solo quiénes están registrados,
  sin revelar quiénes NO están. La privacidad es respetada.

gitflow_rama: "feature/FASE-10-busqueda"
dependencias: ["FASE-03"]

endpoints:
  - GET  /users/search
  - GET  /users/:id/profile
  - POST /contacts/sync
  - GET  /contacts
  - POST /contacts
  - DELETE /contacts/:id
  - POST /users/:id/block
  - DELETE /users/:id/block
```

### 10.1 — Búsqueda de usuarios

```
GET /users/search?q={query}&limit={20}

  Lógica de búsqueda:
  → Si query empieza con @: buscar por username exacto o prefijo
  → Si query empieza con +: buscar por teléfono E.164 exacto (normalize_e164 primero)
  → Otro caso: buscar en username y display_name con ILIKE

  Filtros de privacidad (siempre aplicar):
  → Excluir usuarios que han bloqueado al auth_user
  → Excluir usuarios que el auth_user ha bloqueado
  → Excluir usuarios con deleted_at IS NOT NULL
  → Excluir usuarios con is_active = false

  Rate limit: 30 requests/min por usuario (Redis)

  Retornar: [{ id, username, display_name, avatar_url }]
  → NUNCA retornar phone en búsquedas
```

**Criterio de completitud:** Un usuario bloqueado no aparece en búsquedas del bloqueador ni el bloqueador aparece en las búsquedas del bloqueado.

---

### 10.2 — Sincronización de agenda con privacidad

```
POST /contacts/sync
  Body: { hashes: ["sha256_del_numero_e164", ...] }

  Flujo (privacy-preserving):
  1. El cliente hace SHA-256 de cada número de la agenda ANTES de enviarlo
     SHA-256(normalize_e164("+573001234567")) → "abc123..."
  2. El servidor mantiene en Redis un set: phone_hashes con SHA-256 de todos los números registrados
     (se actualiza al registrar/eliminar usuarios)
  3. El servidor calcula la intersección: hashes_recibidos ∩ phone_hashes
  4. Para los hashes que coinciden: buscar el user_id correspondiente
  5. Retornar solo los que coincidieron: [{ hash, user_id, username, display_name }]

  El servidor nunca aprende qué números del cliente NO están registrados.

  Actualizar contacts en DB:
  → Para cada coincidencia: UPSERT contacts SET contact_id = user_id WHERE owner_id = auth_user AND hash = match
```

**Mantener el set de hashes actualizado:**

```
Al registrar un usuario nuevo:
  SADD phone_hashes {SHA-256(phone)}

Al eliminar (soft delete) un usuario:
  SREM phone_hashes {SHA-256(phone)}

Si Redis se reinicia: reconstruir el set desde la DB con un script de warmup.
```

---

## FASE 11 — Performance y Caché

```yaml
meta: >
  El sistema responde en <100ms para el percentil 95.
  Redis está siendo usado correctamente en sus 7 roles.
  El rate limiting protege contra abuso.
  Las queries más costosas están optimizadas.

gitflow_rama: "feature/FASE-11-performance"
dependencias: ["FASE-06"]
```

### 11.1 — Los 7 roles de Redis (no mezclar)

```yaml
# Redis se usa para 7 cosas distintas. Cada una tiene su namespace y TTL.

roles:
  1_cache_perfiles:
    clave: "profile:{user_id}"
    valor: JSON del perfil público del usuario
    ttl: 300  # 5 minutos
    invalidar_cuando: PATCH /users/profile

  2_sesiones:
    clave: "session:{session_id}"
    valor: JSON con user_id y device_id
    ttl: igual_al_access_token  # 900 segundos
    invalidar_cuando: logout, revocación de sesión

  3_presencia:
    clave: "presence:{user_id}"
    valor: "1"
    ttl: 65  # renovar cada 60s con heartbeat WS
    invalidar_cuando: desconexión WS (DEL explícito)

  4_otp_y_rate_limits:
    claves:
      - "otp:register:{phone}"  TTL 600
      - "otp:reset:{phone}"     TTL 900
      - "totp:used:{user_id}:{code}"  TTL 90
      - "rate:{endpoint}:{identifier}"  TTL variable
    valor: código OTP o contador numérico

  5_pubsub:
    canales:
      - "user:{user_id}:events"
      - "system:broadcasts"
    nota: "Pub/Sub no usa keyspace — los mensajes no se almacenan"

  6_job_queues:
    claves:
      - "push:queue"          ← lista, LPUSH/BRPOP
      - "thumbnails:queue"    ← lista para worker de thumbnails
    valor: JSON del job
    nota: "Usar BRPOP con timeout. No hay TTL — los jobs se consumen."

  7_refresh_tokens:
    clave: "refresh:{SHA256_del_token}"
    valor: JSON con user_id, session_id, device_id
    ttl: 2592000  # 30 días
    invalidar_cuando: uso del token (rotación), logout, revocación de sesión
```

**Regla:** Si quieres usar Redis para algo nuevo, primero verificar si encaja en alguno de los 7 roles. Si no encaja, revisar si realmente necesita Redis o si la DB es suficiente.

---

### 11.2 — Rate limiting con sliding window

**Implementación con Lua (atómica en Redis):**

```lua
-- rate_limit.lua
-- KEYS[1]: clave del rate limit (ej: "rate:login:{ip}")
-- ARGV[1]: límite máximo de requests
-- ARGV[2]: ventana en segundos
-- ARGV[3]: timestamp actual en milliseconds
-- Retorna: { requests_en_ventana, allowed (0/1) }

local now = tonumber(ARGV[3])
local window = tonumber(ARGV[2]) * 1000
local limit = tonumber(ARGV[1])

-- Eliminar entradas fuera de la ventana
redis.call('ZREMRANGEBYSCORE', KEYS[1], 0, now - window)

-- Contar requests en la ventana actual
local count = redis.call('ZCARD', KEYS[1])

if count < limit then
    -- Añadir el request actual
    redis.call('ZADD', KEYS[1], now, now .. math.random())
    redis.call('EXPIRE', KEYS[1], tonumber(ARGV[2]) + 1)
    return {count + 1, 1}  -- 1 = permitido
else
    return {count, 0}  -- 0 = bloqueado
end
```

**Límites configurados:**

```yaml
rate_limits:
  auth_endpoints: # /auth/login, /auth/register, /auth/forgot-password
    limit: 10
    window_seconds: 60
    key: "rate:auth:{ip_address}"

  message_send: # POST /chats/:id/messages
    limit: 100
    window_seconds: 60
    key: "rate:messages:{user_id}"

  search: # GET /users/search
    limit: 30
    window_seconds: 60
    key: "rate:search:{user_id}"

  attachment_upload: # POST /attachments/upload-url
    limit: 20
    window_seconds: 60
    key: "rate:upload:{user_id}"

  api_general: # todos los demás endpoints autenticados
    limit: 300
    window_seconds: 60
    key: "rate:api:{user_id}"
```

**Headers en la respuesta:**

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 87
X-RateLimit-Reset: 1705329600  ← timestamp Unix cuando se resetea la ventana
Retry-After: 23               ← solo en respuestas 429
```

---

### 11.3 — Optimizaciones de queries

**Queries a revisar en esta fase:**

```sql
-- 1. EXPLAIN ANALYZE de las queries más frecuentes
EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
SELECT ... FROM messages WHERE chat_id = $1 AND deleted_at IS NULL
ORDER BY created_at DESC, id DESC LIMIT 51;
-- Verificar: usa idx_messages_chat_created, no hace seq scan

-- 2. Caché de perfil — evitar query a DB en cada request autenticado
-- En lugar de SELECT * FROM users JOIN user_profiles ...
-- Usar Redis: GET profile:{user_id} → si miss: query DB y cachear

-- 3. VACUUM y autovacuum — configurar para tablas de alta escritura
ALTER TABLE messages SET (
  autovacuum_vacuum_scale_factor = 0.01,  -- vacuum más frecuente (1% de filas muertas)
  autovacuum_analyze_scale_factor = 0.005
);
```

---

## FASE 12 — Observabilidad y Hardening

```yaml
meta: >
  La aplicación tiene logs estructurados en JSON.
  Las métricas de negocio e infraestructura son visibles en Prometheus/Grafana.
  El audit_log captura todas las acciones sensibles.
  La superficie de ataque está revisada.

gitflow_rama: "feature/FASE-12-observabilidad"
dependencias: ["FASE-11"]

endpoints:
  - GET /metrics  ← solo desde IPs internas / con token interno
```

### 12.1 — Logging estructurado

**Configuración de tracing:**

```rust
// shared/src/logging.rs

pub fn init_logging(app_env: &AppEnv) {
    let fmt = match app_env {
        AppEnv::Development => tracing_subscriber::fmt()
            .pretty()  // legible para humanos en desarrollo
            .init(),
        _ => tracing_subscriber::fmt()
            .json()    // JSON estructurado en staging/producción
            .init(),
    };
}
```

**Campos mínimos en CADA log de request:**

```json
{
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "INFO",
  "request_id": "uuid-v4-generado-al-inicio-del-request",
  "method": "POST",
  "path": "/chats/uuid/messages",
  "status_code": 201,
  "latency_ms": 23,
  "user_id": "uuid-del-usuario-autenticado",
  "ip": "192.168.1.1"
}
```

**PROHIBIDO loguear (en cualquier nivel):**

```yaml
campos_prohibidos:
  - password (en cualquier forma)
  - password_hash
  - jwt_secret, jwt_refresh_secret
  - refresh_token (el valor, no el hash)
  - access_token (el valor)
  - content_encrypted (contenido de mensajes)
  - content_iv
  - identity_key, private_key (cualquier clave privada)
  - two_fa_secret
  - backup_codes
  - otp_code (el código en sí)
  - cualquier PII del usuario: email, phone en logs de nivel INFO+
```

---

### 12.2 — Métricas con Prometheus

```rust
// Métricas a exponer en GET /metrics

// Contadores
messages_sent_total{type="text|image|video|audio|file"}
auth_attempts_total{result="success|failure", reason="bad_password|2fa_failed|account_inactive"}
push_notifications_sent_total{platform="fcm|apns", result="success|failed|invalid_token"}

// Gauges (valores actuales)
active_ws_connections           // conexiones WebSocket abiertas ahora
db_pool_size{state="idle|active"}
redis_connected_clients

// Histogramas (distribución de latencias)
http_request_duration_seconds{method, path, status}
db_query_duration_seconds{query_type}
```

---

### 12.3 — Security hardening checklist

```yaml
headers_http:
  - "Strict-Transport-Security: max-age=31536000; includeSubDomains"
  - "X-Content-Type-Options: nosniff"
  - "X-Frame-Options: DENY"
  - "Content-Security-Policy: default-src 'none'"
  - "Referrer-Policy: strict-origin-when-cross-origin"

verificar_endpoints:
  - Todo endpoint que modifica datos requiere autenticación
  - Los endpoints de admin verifican rol adecuado
  - Los IDs en URLs pertenecen al usuario autenticado (no acceder a recursos de otros)
  - Los attachments en S3 tienen URLs pre-firmadas con TTL, no son públicos

dependencias:
  - cargo audit pasa sin vulnerabilidades conocidas
  - cargo outdated revisado — actualizar deps con CVEs activos

audit_log_cobertura:
  - login (exitoso y fallido)
  - password_change
  - two_fa_enable / two_fa_disable
  - device_added / device_removed
  - account_delete
  - participant_add / participant_remove (en grupos)
  - block_user / unblock_user
  - key_rotation
```

---

## FASE 13 — Despliegue y Producción

```yaml
meta: >
  El binario de Rust está containerizado en una imagen Docker pequeña.
  El pipeline CI/CD despliega automáticamente a staging desde develop
  y a producción desde main con aprobación manual.
  Los runbooks operacionales están documentados.

gitflow_rama: "release/1.0.0"  ← primera release a producción
dependencias: ["FASE-12"]
```

### 13.1 — Dockerfile multi-stage con cargo-chef

```dockerfile
# Dockerfile
# Estrategia: cargo-chef para caché eficiente de dependencias
# Sin cargo-chef: cada cambio en src/ recompila TODAS las dependencias (10-20 min)
# Con cargo-chef: solo recompila el código propio (<2 min)

# Stage 1: Planificación de dependencias
FROM rust:1.78-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Compilación de dependencias (se cachea si recipe.json no cambia)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Compilación del código propio
COPY . .
# sqlx en modo offline — usa sqlx-data.json del repo
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin messenger_backend

# Stage 4: Imagen de runtime mínima
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/messenger_backend .
COPY migrations/ ./migrations/  # sqlx migrate run al arrancar

EXPOSE 3000
CMD ["./messenger_backend"]
```

**Tamaños esperados:**

- Imagen de build completa: ~2 GB
- Imagen de runtime final: ~50 MB
- Binario: ~15-25 MB (Rust compila estático)

---

### 13.2 — Pipeline CI/CD completo

```yaml
# .github/workflows/deploy.yml

name: Deploy

on:
  push:
    branches: [develop, main]

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Docker image
        run: |
          docker build -t ghcr.io/${{ github.repository }}:${{ github.sha }} .
          docker push ghcr.io/${{ github.repository }}:${{ github.sha }}

  deploy-staging:
    needs: build-and-push
    if: github.ref == 'refs/heads/develop'
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - name: Deploy a staging
        run: |
          # 1. Correr migraciones ANTES de actualizar el binario
          docker run --env-file .env.staging \
            ghcr.io/${{ github.repository }}:${{ github.sha }} \
            ./messenger_backend migrate
          # 2. Actualizar el servicio
          docker service update --image ghcr.io/${{ github.repository }}:${{ github.sha }} messenger_api
      - name: Smoke tests post-deploy
        run: |
          curl -f https://staging.app.io/health || exit 1

  deploy-production:
    needs: build-and-push
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    environment:
      name: production # ← requiere aprobación manual en GitHub
    steps:
      - name: Deploy a producción
        run: |
          # Ídem al de staging pero con variables de producción
          echo "Desplegando versión ${{ github.sha }} a producción"
```

---

### 13.3 — Runbooks operacionales

#### Runbook: Migraciones en producción (zero downtime)

```
1. Las migraciones deben ser backwards compatible:
   - Agregar columnas nullable (nunca NOT NULL sin DEFAULT)
   - No eliminar columnas en la misma migración que elimina su uso en código
   - Usar el patrón expand-contract para cambios breaking

2. Orden de operaciones:
   a. Correr migración en producción (la DB tiene ambas versiones de schema)
   b. Desplegar nuevo binario (que usa la nueva schema)
   c. (opcional) Migración de cleanup que elimina columnas/tablas viejas

3. Verificar antes de correr:
   sqlx migrate info --database-url $PROD_DATABASE_URL
   # Revisar que todas las migraciones previas están en estado "Applied"

4. Rollback:
   Si la migración falla a mitad: PostgreSQL hace rollback automático (transacción)
   Si el binario nuevo falla: desplegar el binario anterior (el schema es compatible)
```

#### Runbook: Token comprometido — forzar logout

```bash
# Si se sospecha que un refresh token fue comprometido:

# 1. Identificar el user_id afectado
# 2. Eliminar TODAS las sesiones del usuario de la DB
DELETE FROM user_sessions WHERE user_id = '{user_id}';

# 3. Limpiar sesiones de Redis (puede requerir SCAN si no se tiene el session_id)
# En Redis:
redis-cli --scan --pattern "session:*" | xargs redis-cli del

# 4. Limpiar refresh tokens de Redis del usuario
# (Requiere haber guardado el mapeo user_id → session_ids en Redis también)

# 5. El usuario será deslogueado en el próximo request que haga
# 6. Notificar al usuario que debe volver a iniciar sesión (push notification o email)
# 7. Registrar en audit_log con metadata del incidente
```

#### Runbook: Pool de OPKs agotado para un usuario

```bash
# Si prekey_count de un usuario llega a 0:
# Los nuevos contactos que intenten iniciar sesión con ese usuario
# recibirán un key bundle sin OPK (X3DH sin OPK — menor forward secrecy, sigue funcionando)

# Diagnóstico:
SELECT uk.user_id, uk.prekey_count, u.username, u.last_seen_at
FROM user_keys uk
JOIN users u ON u.id = uk.user_id
WHERE uk.prekey_count = 0;

# Acción: el servidor ya envía X-Prekey-Count header cuando < 20
# Si el usuario está activo y no repone: puede estar offline o con bug en el cliente
# No hay acción de emergencia en el servidor — la funcionalidad se degrada gracefully
```

#### Runbook: Crear partición de audit_log manualmente

```sql
-- Si el job de pg_cron falló y el mes siguiente no tiene partición:

SELECT create_audit_log_partitions(3);
-- Crea particiones para el mes actual y los 3 siguientes

-- Verificar que se crearon:
SELECT tablename FROM pg_tables
WHERE tablename LIKE 'audit_log_%'
ORDER BY tablename;

-- Si hay inserts fallando por partición faltante, están en la default partition:
-- (solo si se configuró DEFAULT partition, ver schema.sql)
```

#### Runbook: Respuesta a incidente de seguridad

```
1. AISLAR: Si hay acceso no autorizado confirmado → revocar todas las sesiones activas
2. INVESTIGAR: Revisar audit_log con los user_id, timestamps e IPs del incidente
3. CONTENER: Rate limit agresivo en las IPs del atacante (regla temporal en nginx/load balancer)
4. NOTIFICAR: A los usuarios afectados con instrucciones de cambiar contraseña
5. REMEDIAR: Corregir el vector de ataque, desplegar fix
6. POSTMORTEM: Documentar qué pasó, cómo se detectó, cómo se contuvo, qué se mejoró
```

---

## APÉNDICE — Decisiones de diseño ya tomadas

<!--
  Esta sección existe para que la IA no proponga cambios a decisiones
  que ya fueron debatidas y resueltas. Cada decisión incluye el trade-off
  considerado y por qué se eligió esta opción.
-->

```yaml
decisiones:
  one_time_prekeys_tabla_separada:
    decision: "Tabla one_time_prekeys separada, no JSONB en user_keys"
    razon: "JSONB requería rewrite completo de la fila en cada consumo. Con tabla separada: DELETE de una sola fila, lock mínimo, SKIP LOCKED para concurrencia."
    no_revertir: true

  paginacion_por_cursor:
    decision: "Cursor basado en (created_at, id), no offset"
    razon: "OFFSET tiene costo O(n) y genera saltos cuando llegan mensajes nuevos durante la paginación. El cursor es estable y O(log n)."
    no_revertir: true

  normalize_e164_volatile:
    decision: "normalize_e164 marcada VOLATILE, no IMMUTABLE"
    razon: "Lanza excepciones (efecto secundario). IMMUTABLE no puede lanzar excepciones sin comportamiento indefinido en el planner de PostgreSQL."
    no_revertir: true

  current_setting_missing_ok:
    decision: "current_setting('app.current_user_id', true) con missing_ok=true"
    razon: "Sin missing_ok, falla en conexiones de pg_cron, scripts de backup, migration runners. Con missing_ok retorna NULL y las políticas RLS deniegan el acceso."
    no_revertir: true

  refresh_token_opaco:
    decision: "Refresh token como UUID opaco en Redis, no como JWT"
    razon: "Los JWT no pueden revocarse individualmente sin un blocklist. Un token opaco en Redis se puede DEL en O(1). La rotación one-time-use previene reutilización."
    no_revertir: true

  servidor_relay_cifrado:
    decision: "El servidor nunca descifra content_encrypted"
    razon: "Principio fundamental de privacidad. El servidor es un relay. Violar este principio requeriría rediseño completo del modelo de cifrado."
    no_revertir: true

  presencia_en_redis_no_db:
    decision: "La presencia online/offline se almacena en Redis, no en PostgreSQL"
    razon: "Un usuario activo renovaría su presencia cada 60s. Con 10k usuarios activos: 10k UPDATEs/min en PostgreSQL. Redis maneja esto trivialmente."
    no_revertir: true

  argon2id_parametros:
    decision: "Argon2id con m=65536, t=3, p=4"
    razon: "Parámetros recomendados por OWASP para 2024. Balance entre seguridad (resistencia a GPU attacks) y latencia (<500ms en hardware moderno)."
    no_revertir: true
```

---

## APÉNDICE — Checklist de completitud por fase

```
Fase 01: [ ] cargo build limpio  [ ] CI verde  [ ] Docker Compose up  [ ] make lint pasa
Fase 02: [ ] sqlx migrate info = all Applied  [ ] GET /health = 200  [ ] Tests integración pasan
Fase 03: [ ] Registro + verificación OTP  [ ] Login + JWT  [ ] 2FA TOTP  [ ] Revocación sesiones
Fase 04: [ ] Upload 100 OPKs  [ ] GET /keys/:id consume OPK atómicamente  [ ] Fingerprint idempotente
Fase 05: [ ] CRUD chats  [ ] Paginación cursor estable  [ ] Adjunto S3 confirmado  [ ] Reacciones
Fase 06: [ ] WS multi-device  [ ] Redis Pub/Sub entre instancias  [ ] Presencia en Redis  [ ] Gap sync
Fase 07: [ ] Privacy "only_me" no visible para otros  [ ] Vistas en tiempo real  [ ] Expiración 24h
Fase 08: [ ] Push a usuario offline  [ ] Token inválido = DELETE sesión  [ ] Silenciado respetado
Fase 09: [ ] Roles jerárquicos  [ ] Rotación group_key al eliminar miembro  [ ] Invite link revocable
Fase 10: [ ] Búsqueda respeta privacidad  [ ] Sync agenda no revela no-registrados
Fase 11: [ ] 7 roles Redis correctos  [ ] Rate limit 429 con headers  [ ] p95 < 100ms
Fase 12: [ ] Logs JSON en producción  [ ] /metrics accesible  [ ] cargo audit = 0 vulnerabilidades
Fase 13: [ ] Docker image < 100MB  [ ] Deploy staging automático  [ ] Runbooks documentados
```
