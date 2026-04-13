---
name: realtime-logic-expert
description: Disena logica de mensajeria en tiempo real con concurrencia segura, WebSockets y estados de entrega/lectura. Usar en chat, presencia, pub-sub, reconexion de clientes y manejo de heartbeats.
---

# Realtime Logic Expert

## Objetivo
Garantizar entrega confiable y manejo robusto de conexiones en escenarios de mensajeria en tiempo real.

## Workflow recomendado
1. Definir topologia de eventos: productor, fan-out, consumidores y persistencia.
2. Implementar difusion con `tokio::sync::broadcast` para chats grupales.
3. Modelar estados `sent`, `delivered`, `read` con coordinacion en Redis.
4. Implementar heartbeat, timeout y limpieza en desconexion.
5. Definir reconexion con resync incremental de mensajes pendientes.

## Reglas obligatorias
- Usar `tokio::sync::broadcast` para fan-out de grupos.
- Gestionar estados de entrega y lectura con Redis.
- Manejar desconexiones limpias (cancel tasks, unsubscribe, cleanup).
- Evitar bloqueos largos dentro de loops de WebSocket.
- Manejar backpressure, drops de canal y reintentos.

## Guardrails de concurrencia
- Evitar locks de larga duracion en rutas calientes.
- Separar lectura y escritura de socket en tareas dedicadas.
- Definir limites de buffer y politicas de descarte.
- Instrumentar eventos clave: connect, disconnect, lag, resend.

## Salida esperada
- Flujo de eventos en tiempo real.
- Estrategia de estado y consistencia.
- Politica de desconexion y reconexion.
- Riesgos operativos y mitigaciones (lag, perdida temporal, picos).
