---
name: security-hardener
description: Refuerza seguridad de APIs backend con autenticacion robusta, autorizacion por recurso y mitigacion de fallos comunes. Usar cuando se implementen endpoints, auth, sesiones, JWT o revision de riesgos.
---

# Security Hardener

## Objetivo
Reducir superficie de ataque y prevenir fallos de autenticacion/autorizacion en APIs de mensajeria.

## Workflow recomendado
1. Modelar amenazas del endpoint (lectura, escritura, enumeracion, abuso).
2. Aplicar autenticacion fuerte y validacion de claims.
3. Aplicar autorizacion por recurso (owner, member, admin, etc.).
4. Blindar respuestas y codigos de error para evitar filtraciones.
5. Agregar controles de abuso (rate limit, lockout, auditoria).

## Reglas obligatorias
- Hashear contrasenas con `argon2` y salt unico por usuario.
- Implementar JWT con `EdDSA` o `RS256` y rotacion de llaves.
- Validar autorizacion por recurso, no solo autenticacion.
- Revisar y mitigar ID Enumeration en endpoints.
- Validacion estricta de input y claims de JWT.

## Guardrails de seguridad
- No exponer IDs secuenciales sensibles sin controles de acceso.
- Responder errores sin filtrar informacion sensible.
- Aplicar rate limiting en login, OTP y endpoints criticos.
- Definir expiracion corta para access tokens y estrategia de refresh segura.
- Registrar eventos sensibles sin loggear secretos ni tokens completos.

## Checklist rapido
- [ ] Hash seguro de credenciales
- [ ] Firma y validacion JWT correctas
- [ ] Control de acceso por propietario o rol
- [ ] Sin filtracion por enumeracion de IDs
- [ ] Protecciones anti abuso activas
