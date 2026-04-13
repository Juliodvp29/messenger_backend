# Agent Skills Configuration

## [Skill: rust-backend-architect]

- **Enfoque:** Arquitectura limpia en Rust (Clean Architecture).
- **Reglas:** - Usar `Axum` para el routing y `Tokio` para el runtime.
  - Implementar el patrón "Repository" para aislar la lógica de DB.
  - Manejo de errores centralizado con `AppError` usando `thiserror`.

## [Skill: sqlx-query-validator]

- **Enfoque:** Consultas seguras y eficientes en Postgres.
- **Reglas:** - Validar esquemas mediante el MCP de Postgres antes de sugerir SQL.
  - Usar macros `sqlx::query!` para validación en tiempo de compilación.
  - Sugerir índices para columnas de búsqueda frecuente (`created_at`, `sender_id`).

## [Skill: realtime-logic-expert]

- **Enfoque:** Concurrencia y WebSockets (Mensajería).
- **Reglas:** - Usar `tokio::sync::broadcast` para chats grupales.
  - Diseñar estados de "entregado" y "leído" mediante Redis.
  - Manejar desconexiones limpias (heartbeats).

## [Skill: security-hardener]

- **Enfoque:** Seguridad defensiva.
- **Reglas:** - Hashear contraseñas con `argon2`.
  - Implementar JWT con `EdDSA` o `RS256`.
  - Revisar siempre el "ID Enumeration" en los endpoints de mensajes.
