---
name: sqlx-query-validator
description: Valida y optimiza consultas SQL para Postgres con SQLx, priorizando seguridad, tipos correctos y rendimiento. Usar cuando se escriban queries, migraciones, repositorios o cambios de esquema.
---

# SQLx Query Validator

## Objetivo
Reducir errores de queries en runtime y mejorar el rendimiento de acceso a datos.

## Workflow recomendado
1. Verificar tablas, columnas y constraints existentes antes de editar SQL.
2. Escribir consulta parametrizada con `sqlx::query!` o `sqlx::query_as!`.
3. Validar tipos, nullability y aliases contra structs de Rust.
4. Revisar plan de ejecucion y proponer indices cuando aplique.
5. Confirmar impacto en paginacion y ordenamiento estable.

## Reglas obligatorias
- Validar esquema de Postgres antes de sugerir SQL.
- Usar placeholders parametrizados; nunca interpolacion directa.
- Preferir `sqlx::query!` y `sqlx::query_as!` para validacion en compilacion.
- Sugerir indices en filtros y ordenamientos frecuentes (`created_at`, `sender_id` y equivalentes).
- Considerar `EXPLAIN ANALYZE` para consultas criticas o lentas.

## Guardrails de calidad
- Evitar `SELECT *`; seleccionar columnas explicitas.
- Evitar N+1 queries cuando se puede resolver con join o batching.
- Proponer claves compuestas cuando el patron real de filtro lo requiere.
- Mantener consistencia de nombres (`*_id`, `created_at`, `updated_at`).

## Checklist rapido
- [ ] Query validada contra esquema
- [ ] Tipos Rust y Postgres correctos
- [ ] Sin riesgo de SQL injection
- [ ] Indices sugeridos cuando hay scans costosos
- [ ] Paginacion y ordenamiento estables
