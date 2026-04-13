---
name: rust-backend-architect
description: Disena arquitectura limpia para backends en Rust con Axum y Tokio, separando capas de API, dominio e infraestructura. Usar cuando se implementen features backend, diseno modular, contratos entre capas, repositories o manejo de errores.
---

# Rust Backend Architect

## Objetivo
Disenar cambios de backend con limites claros, bajo acoplamiento y errores consistentes de punta a punta.

## Workflow recomendado
1. Definir caso de uso y contrato de entrada y salida.
2. Implementar o extender servicio de dominio (sin dependencias HTTP).
3. Definir trait de repositorio y su implementacion en infraestructura.
4. Conectar handler Axum con DTOs y mapeo de errores.
5. Verificar que no haya filtracion de detalles de DB hacia API publica.

## Reglas obligatorias
- Usar `Axum` para routing y `Tokio` para runtime.
- Mantener flujo de dependencias: `handlers -> services/use-cases -> repositories`.
- No mezclar SQL o acceso a DB dentro de handlers.
- Repositories se exponen como traits y ocultan detalles de persistencia.
- Estandarizar errores con `AppError` usando `thiserror`.

## Guardrails de implementacion
- Handlers: parseo, validacion, auth/contexto y respuesta HTTP.
- Services/use-cases: reglas de negocio, orquestacion y politicas.
- Repositories: consultas, transacciones y mapeo de errores tecnicos.
- Evitar funciones gigantes; preferir modulos pequenos y cohesionados.

## Salida esperada
- Estructura de modulos propuesta o actualizada.
- Traits y structs involucrados en la feature.
- Flujo de errores (`infra -> domain -> HTTP`) con `AppError`.
- Trade-offs relevantes (simplicidad, escalabilidad, testabilidad).
