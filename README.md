# Messenger Backend

[![CI - Messenger Backend](https://github.com/Juliodvp29/messenger_backend/actions/workflows/ci.yml/badge.svg)](https://github.com/Juliodvp29/messenger_backend/actions/workflows/ci.yml)

Backend modular en Rust para una aplicación de mensajería estilo WhatsApp/Telegram con cifrado E2E.

## Fases de Desarrollo

| Fase | Descripción | Estado |
|------|------------|--------|
| 01 | Fundamentos y Configuración | ✅ Completado |
| 02 | Base de Datos y Migraciones | ✅ Completado |
| 03 | Autenticación y Seguridad | ✅ Completado |
| 04 | Cifrado E2E | ✅ Completado |
| 05 | Chats y Mensajes | ✅ Completado |
| 06 | Tiempo Real (WebSockets) | ✅ Completado |
| 07 | Stories y Reacciones | ✅ Completado |
| 08 | Notificaciones | ✅ Completado |
| 09 | Gestión de Grupos y Canales | ✅ Completado |
| 10 | Búsqueda y Contactos | ✅ Completado |
| 11 | Performance y Caché | ✅ Completado |
| 12 | Observabilidad y Hardening | ✅ Completado |
| 13 | Despliegue y Producción | ⏳ Pendiente |

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
│   │   ├── handlers/   # auth, chats, messages, stories, etc.
│   │   ├── middleware/ # auth, rate_limit, logging
│   │   ├── services/   # jwt, otp, push, storage, ws
│   │   ├── routes.rs
│   │   └── main.rs
├── domain/
│   ├── src/
│   │   ├── chat/       # Entidades y repositorios de chat/mensajes
│   │   ├── stories/    # Entidades y repositorios de historias
│   │   ├── user/       # Usuario y gestión de perfiles
│   │   ├── contact/    # Gestión de contactos
│   │   └── keys/       # Modelos para cifrado E2E (X3DH)
├── infrastructure/
│   ├── src/
│   │   ├── repositories/ # Implementaciones Postgres de repositorios
│   │   ├── db/           # Pooled connection Postgres
│   │   ├── redis/        # Cliente Redis y Pub/Sub
│   │   ├── storage/      # MinIO/S3 Adapter
│   │   └── email/        # SMTP con Lettre
└── shared/
    ├── src/
    │   ├── config.rs   # Configuración tipada (env)
    │   └── error.rs    # AppError centralizado
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

## Endpoints — Fase 05 (Chats y Mensajes)

Gestión de conversaciones y mensajería con contenido cifrado E2E.

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| POST | `/chats` | Crear chat privado o grupal | Bearer |
| GET | `/chats` | Listar chats del usuario (paginado) | Bearer |
| GET | `/chats/:id` | Obtener chat por ID | Bearer |
| PATCH | `/chats/:id` | Actualizar chat (nombre, avatar) | Bearer |
| DELETE | `/chats/:id` | Eliminar chat | Bearer |
| POST | `/chats/:id/messages` | Enviar mensaje | Bearer |
| GET | `/chats/:id/messages` | Listar mensajes (paginado) | Bearer |
| PATCH | `/chats/:id/messages/:msg_id` | Editar mensaje | Bearer |
| DELETE | `/chats/:id/messages/:msg_id` | Eliminar mensaje | Bearer |
| POST | `/chats/:id/messages/read` | Marcar mensajes como leídos | Bearer |
| POST | `/chats/:id/messages/:msg_id/reactions` | Añadir reacción | Bearer |
| DELETE | `/chats/:id/messages/:msg_id/reactions/:emoji` | Eliminar reacción | Bearer |
| POST | `/attachments/upload-url` | Generar URL prefirmada (upload_url) y URL permanente (file_url) | Bearer |
| POST | `/attachments/confirm` | Confirmar attachment subido (para mensajes de chat) | Bearer |

### Paginación de Mensajes

Los endpoints de listar mensajes usan paginación por cursor:

```bash
# Obtener mensajes (más recientes primero)
curl "http://localhost:3000/chats/{chat_id}/messages?limit=50" \
  -H "Authorization: Bearer $TOKEN"

# Obtener mensajes anteriores (before cursor)
curl "http://localhost:3000/chats/{chat_id}/messages?cursor={cursor}&direction=before&limit=50" \
  -H "Authorization: Bearer $TOKEN"
```

### Envío de Mensajes Cifrados

Los mensajes se envían cifrados (el servidor nunca los desencripta):

```bash
curl -X POST http://localhost:3000/chats/{chat_id}/messages \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content_encrypted":"BASE64_AES_GCM_CIPHERTEXT",
    "content_iv":"BASE64_12_BYTES",
    "message_type":"text"
  }'
```

## Endpoints — Fase 06 (Tiempo Real)

WebSocket para eventos en tiempo real.

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| GET | `/ws?token={jwt}` | Upgrade a WebSocket | Bearer (JWT) |

### Eventos WebSocket

El servidor envía eventos JSON por el socket:

```json
{ "type": "new_message", "payload": {...}, "timestamp": "..." }
{ "type": "typing_start", "payload": {"chat_id": "...", "user_id": "..."}, "timestamp": "..." }
{ "type": "typing_stop", "payload": {...}, "timestamp": "..." }
{ "type": "messages_read", "payload": {...}, "timestamp": "..." }
{ "type": "user_online", "payload": {"user_id": "..."}, "timestamp": "..." }
{ "type": "user_offline", "payload": {"user_id": "..."}, "timestamp": "..." }
```

### Mensajes del Cliente

El cliente envía por WebSocket:

```json
{ "type": "typing_start", "payload": {"chat_id": "..."} }
{ "type": "typing_stop", "payload": {"chat_id": "..."} }
{ "type": "sync_request", "payload": {"since": "2024-01-15T10:30:00Z"} }
```

### Redis Pub/Sub

Canales usados para distribución entre instancias:

- `user:{user_id}:events` — Eventos dirigidos a un usuario
- `chat:{chat_id}:events` — Eventos de un chat específico
- `presence:{user_id}` — Clave de presencia (TTL 65s)

## Endpoints — Fase 07 (Stories y Reacciones)

Stories efímeras con privacidad granular y limpieza automática de media.

> [!IMPORTANT]
> **Ciclo de Vida de Adjuntos**: Los archivos multimedia subidos para historias (bajo el prefijo `attachments/stories/`) se eliminan automáticamente del storage (S3/MinIO) después de **24 horas** mediante reglas de ciclo de vida configuradas en el servidor.

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| POST | `/stories` | Publicar story | Bearer |
| GET | `/stories` | Listar stories de contactos | Bearer |
| GET | `/stories/my` | Listar mis stories | Bearer |
| DELETE | `/stories/:id` | Eliminar story | Bearer |
| POST | `/stories/:id/view` | Registrar vista | Bearer |
| POST | `/stories/:id/react` | Reaccionar a story | Bearer |
| GET | `/stories/:id/views` | Ver lista de vistas | Bearer |

### Modos de Privacidad

| Privacidad | Descripción |
|------------|-------------|
| `everyone` | Visible para todos |
| `contacts` | Solo contactos |
| `contacts_except` | Contactos excepto algunos |
| `selected` | Solo usuarios seleccionados |
| `only_me` | Solo yo (no visible para otros) |

```bash
# 1. Obtener URLs (upload_url para el PUT y file_url para la DB)
curl -X POST http://localhost:3000/attachments/upload-url \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"chat_id": null, "file_name": "story.jpg", "content_type": "image/jpeg"}'

# 2. Subir archivo binario (Directo a S3/MinIO)
curl -X PUT "{upload_url_recibida}" \
  -H "Content-Type: image/jpeg" \
  --data-binary @story.jpg

# 3. Publicar story usando la file_url
curl -X POST http://localhost:3000/stories \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content_url": "{file_url_recibida}",
    "content_type": "image",
    "caption": "Mi historia efímera",
    "privacy": "contacts"
  }'
```

## Endpoints — Fase 08 (Notificaciones)

Push notifications y notificaciones in-app.

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| GET | `/notifications` | Listar notificaciones | Bearer |
| PATCH | `/notifications/:id` | Marcar notificación como leída | Bearer |
| PATCH | `/notifications/read-all` | Marcar todas como leídas | Bearer |
| DELETE | `/notifications/read` | Eliminar notificaciones leídas | Bearer |
| PATCH | `/chats/:id/settings` | Configurar chat (silenciar, pinear, archivar) | Bearer |

### Configuración de Chat

```bash
# Silenciar chat por 24 horas
curl -X PATCH http://localhost:3000/chats/{chat_id}/settings \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"is_muted": true, "muted_until": "2024-01-16T12:00:00Z"}'

# Pinear chat
curl -X PATCH http://localhost:3000/chats/{chat_id}/settings \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"is_pinned": true}'
```

### Push Notifications

El servidor envía notificaciones push cuando el usuario está offline:
- **FCM** (Firebase Cloud Messaging) para Android/Web
- **APNs** (Apple Push Notification service) para iOS

> **Nota**: El payload del push NUNCA incluye el contenido del mensaje por privacidad.
> 
## Endpoints — Fase 09 (Gestión de Grupos y Canales)

Administración avanzada de miembros, roles y seguridad en grupos/canales con cifrado E2E.

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| GET | `/chats/:id/participants` | Listar miembros del grupo | Bearer |
| POST | `/chats/:id/participants` | Añadir miembro al grupo | Bearer |
| DELETE | `/chats/:id/participants/:user_id` | Eliminar miembro del grupo | Bearer |
| PATCH | `/chats/:id/participants/:user_id/role` | Cambiar rol (member, moderator, admin) | Bearer |
| POST | `/chats/:id/invite-link` | Crear o refrescar link de invitación | Bearer |
| DELETE | `/chats/:id/invite-link` | Eliminar link de invitación | Bearer |
| POST | `/chats/join/:slug` | Unirse a grupo mediante link | Bearer |
| POST | `/chats/:id/rotate-key` | Rotar llave del grupo (E2EE) | Bearer |
| POST | `/chats/:id/transfer-ownership` | Traspasar propiedad del grupo | Bearer |

### Roles de Participante

- `owner`: Creador del grupo (permisos totales, solo 1).
- `admin`: Puede gestionar miembros y settings.
- `moderator`: Puede gestionar mensajes de otros.
- `member`: Participante estándar.

### Invitaciones por Link

Cuando un usuario se une mediante un link de invitación (`GET /chats/join/:slug`), si el grupo es E2EE, el servidor retornará `key_rotation_required: true`. Un administrador deberá ejecutar `/rotate-key` para proveer la llave del grupo al nuevo miembro de forma segura.

## Endpoints — Fase 10 (Búsqueda y Contactos)

Gestión de la agenda de contactos, bloqueo de usuarios y búsqueda global con protección de privacidad.

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| GET | `/users/search` | Búsqueda global de usuarios por username/teléfono | Bearer |
| GET | `/users/me/profile` | Obtener mi perfil completo | Bearer |
| GET | `/users/:id/profile` | Obtener perfil público de otro usuario | Bearer |
| GET | `/contacts` | Listar mi agenda de contactos | Bearer |
| POST | `/contacts` | Añadir un contacto manualmente | Bearer |
| PATCH | `/contacts/:id` | Editar nombre local de un contacto | Bearer |
| DELETE | `/contacts/:id` | Eliminar de la agenda | Bearer |
| POST | `/contacts/sync` | Sincronización masiva (Privacy-Preserving) | Bearer |
| GET | `/blocks` | Listar usuarios bloqueados | Bearer |
| POST | `/blocks/:user_id` | Bloquear usuario | Bearer |
| DELETE | `/blocks/:user_id` | Desbloquear usuario | Bearer |

### Sincronización y Privacidad

Para proteger la privacidad de los usuarios, la sincronización de contactos no envía los números de teléfono en texto plano. Se utiliza un mecanismo de **Hashing con Sal**:

1. El cliente solicita el `salt` global (actualmente gestionado por el sistema).
2. El cliente concatena el número en formato E.164 con el salt: `hash = SHA256(phone + salt)`.
3. El servidor busca coincidencias contra los hashes pre-calculados en la base de datos.

```bash
# Ejemplo de sincronización
curl -X POST http://localhost:3000/contacts/sync \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "contacts": [
      { "phone_hash": "e3b0c442...", "local_name": "Juan Pérez" },
      { "phone_hash": "8feb2331...", "local_name": "Mamá" }
    ]
  }'
```

## Endpoints — Fase 04 (Cifrado E2E)

Gestión de claves criptográficas para el protocolo X3DH.

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| POST | `/keys/upload` | Sube Identity Key y Signed Prekey | Bearer |
| POST | `/keys/upload-prekeys` | Sube lote de One-Time Prekeys | Bearer |
| GET | `/keys/me/count` | Obtiene conteo de OPKs disponibles | Bearer |
| GET | `/keys/:user_id` | Obtiene bundle de claves públicas de un usuario | Bearer |
| GET | `/keys/:user_id/fingerprint` | Obtiene huella digital de la Identity Key | Bearer |

### Flujo de Cifrado (X3DH)

```bash
# 1. El usuario sube su Identity Key y Signed Prekey al registrarse/iniciar sesión
curl -X POST http://localhost:3000/keys/upload \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"identity_key":"IK_BASE64","signed_prekey":{"id":1,"key":"SPK_BASE64","signature":"SIG_BASE64"}}'

# 2. El usuario sube un lote de One-Time Prekeys (ej. 50 claves)
curl -X POST http://localhost:3000/keys/upload-prekeys \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '[{"id":1,"key":"OPK1_BASE64"},{"id":2,"key":"OPK2_BASE64"}]'

# 3. Para iniciar un chat, se solicita el "Bundle" del destinatario
curl -X GET http://localhost:3000/keys/00000000-0000-0000-0000-000000000002 \
  -H "Authorization: Bearer $TOKEN"
```

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
  -d '{"phone":"+573001234567","code":"123456","device_id":"uuid","device_name":"TestPhone","device_type":"android"}'

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
  -d '{"phone":"+573001234567","code":"123456","device_id":"uuid","device_name":"TestPhone","device_type":"android"}'
```

## Endpoints — Fase 11 (Performance y Caché)

Redis se utiliza en 7 roles distintos para optimizar el rendimiento del sistema:

| Rol | Clave | Descripción | TTL |
|-----|-------|-------------|-----|
| 1 | `profile:{user_id}` | Cache de perfiles públicos | 5 min |
| 2 | `session:{session_id}` | Validación de sesión activa | 15 min |
| 3 | `presence:{user_id}` | Estado online/offline (WS) | 65s |
| 4 | `rate:{endpoint}:{id}` | Rate limiting (sliding window) | 60s |
| 5 | `user:{user_id}:events` | Pub/Sub para eventos | N/A |
| 6 | `push:queue` | Cola de notificaciones push | N/A |
| 7 | `refresh:{hash}` | Refresh tokens (rotación) | 7 días |

### Rate Limiting

El rate limiting usa sliding window con script Lua para operaciones atómicas:

```bash
# Headers de rate limit en respuestas
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 87
X-RateLimit-Reset: 1705329600

# Si se excede el límite
HTTP 429 Too Many Requests
Retry-After: 23
```

### Límites configurados

| Endpoint | Límite | Ventana |
|----------|--------|---------|
| `/auth/login`, `/auth/register` | 10 | 60s |
| `/users/search` | 30 | 60s |
| Envío de mensajes | 100 | 60s |
| Upload de archivos | 20 | 60s |
| API general | 300 | 60s |

### Cache de Perfiles

Los perfiles de usuario se cachean en Redis para reducir consultas a la DB:

```bash
# Obtener perfil (primero cache, luego DB)
curl -X GET http://localhost:3000/users/:id/profile \
  -H "Authorization: Bearer $TOKEN"

# Invalidar cache al actualizar perfil
# Ocurre automáticamente tras TTL de 5 minutos
```

### Optimización de PostgreSQL

La migración `018_performance_autovacuum.sql` configura autovacuum para tablas de alta escritura:

```sql
-- messages: writes más frecuentes
ALTER TABLE messages SET (
    autovacuum_vacuum_scale_factor = 0.01,
    autovacuum_analyze_scale_factor = 0.005
);
```

Los índices existentes optimizan las queries más frecuentes:
- `idx_messages_chat_created` — paginación de mensajes
- `idx_users_phone` — búsqueda por teléfono
- `idx_chat_participants_active` — miembros activos de grupo

## Endpoints — Fase 12 (Observabilidad y Hardening)

Implementación de métricas con Prometheus, tracing con request ID y monitoreo de infraestructura.

| Método | Endpoint | Descripción | Auth |
|--------|----------|-----------|------|
| GET | `/metrics` | Exportador de métricas para Prometheus | No |

### Instrumentación de Métricas

Se han implementado métricas clave para monitorizar el estado y uso del sistema:

| Métrica | Tipo | Descripción |
|---------|------|-------------|
| `messenger_http_request_duration_seconds` | Histogram | Latencia de peticiones HTTP por método y ruta |
| `messenger_auth_attempts_total` | Counter | Intentos de login y registro por tipo y resultado |
| `messenger_messages_sent_total` | Counter | Total de mensajes enviados exitosamente |
| `messenger_active_ws_connections` | Gauge | Conexiones WebSocket activas actualmente |
| `messenger_db_pool_active` | Gauge | Conexiones activas en el pool de PostgreSQL |
| `messenger_db_pool_idle` | Gauge | Conexiones idle en el pool de PostgreSQL |
| `messenger_redis_connected_clients` | Gauge | Número de clientes conectados a Redis |

### Trazabilidad (Request ID)

Cada petición genera un `request_id` (UUID v4) que se propaga en:
1. **Logs**: Cada línea de log incluye el context de la petición.
2. **Response Headers**: Se incluye el header `x-request-id` para facilitar el debug desde el cliente.
3. **Métricas**: Las latencias se pueden asociar a tipos de peticiones.

### Monitoreo en Tiempo Real

El sistema incluye un **Background Metrics Worker** que actualiza cada 15 segundos el estado de los pools de datos, garantizando que el dashboard de Grafana refleje la carga real sin penalizar el rendimiento de los handlers.

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

# 4. Aplicar migraciones pendientes
# Nota: La característica 'migrate' de sqlx debe estar habilitada en el workspace
sqlx migrate run --database-url postgres://messenger:messenger_secret@localhost:5434/messenger_dev

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

## Desarrollo — Obteniendo el OTP

En entorno local, el código OTP se almacena en Redis. Para obtenerlo:

```bash
# Después de /auth/register
docker exec messenger_backend-redis-1 redis-cli GET "otp:register:+573001234567"

# Después de /auth/login
docker exec messenger_backend-redis-1 redis-cli GET "otp:login:+573001234567"

# Después de /auth/recover
docker exec messenger_backend-redis-1 redis-cli GET "otp:recover:+573001234567"
```

Reemplazar `+573001234567` por el número de teléfono usado.

## Convenciones de Código

- Rama principal: `develop`
- Commits: Conventional Commits (`feat`, `fix`, `chore`, `docs`, `test`, `refactor`)
- Usar `sqlx::query!` / `sqlx::query_as!` para queries typesafe
- Nunca interpolar input de usuario en SQL
- Dirección de dependencias: `api -> domain -> infrastructure -> shared`
- Errores centralizados con `thiserror`
- Middleware de autenticación con JWT


## Seguridad

- Autenticación: OTP + PIN local (el servidor nunca conoce el PIN)
- Sesiones: JWT con rotación de refresh token (one-time use)
- 2FA: TOTP con códigos de respaldo
- Rate limiting en endpoints de login/OTP
- El servidor es un relay cifrado — nunca desencripta mensajes

## Documentación Adicional

- [backend_roadmap.md](./backend_roadmap.md) — Plan de desarrollo completo
- [AGENTS.md](./AGENTS.md) — Instrucciones para agentes IA