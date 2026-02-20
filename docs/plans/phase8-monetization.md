# Plan de Implementación Fase 8: Monetización y Suscripciones (Sentinel Pro)

## 🎯 Objetivo General
Transformar Sentinel Pro de una herramienta de uso libre a un producto comercial (SaaS/Software de escritorio) mediante un sistema de licenciamiento seguro, suscripciones de pago y un periodo de prueba (Trial) de 7 días, asegurando una experiencia sin fricciones para el usuario final.

---

## 🏗️ 8.1 Sistema de Licenciamiento (Licensing Engine)
Se requiere un sistema robusto que valide las licencias localmente para evitar la necesidad de hacer peticiones web (ping) en cada ejecución del CLI, lo cual afectaría el rendimiento.

- [ ] **Diseño Criptográfico:** Implementar validación local mediante firmas criptográficas (ej. Ed25519 o RSA).
- [ ] **Almacenamiento Local Seguro:** Guardar la licencia de forma validada (y ofuscada/cifrada) en el sistema del usuario (ej. en `~/.sentinel/license.key`).
- [ ] **Gestión del Periodo de Prueba (7 Días):** 
  - Generar un _Device ID_ único basado en el hardware/OS para evitar abusos del trial.
  - Almacenar fehacientemente la fecha de instalación u obtención del trial.
- [ ] **Comandos CLI de Licencia:**
  - `sentinel pro license info` (Ver estado actual de suscripción y días de prueba restantes).
  - `sentinel pro license activate <license-key>` (Registrar la herramienta).

---

## ☁️ 8.2 Backend y API de Subscripciones
Se requiere un servidor de apoyo (Backend) que reciba los pagos, genere las claves de licencia y maneje las renovaciones.

- [ ] **Integración con Plataforma de Pagos:** Integrar **Stripe** o **Lemon Squeezy** (recomendado para software por la facilidad de impuestos y webhooks de licenciamiento).
- [ ] **Definición de Planes de Suscripción:** 
  - Plan Mensual.
  - Plan Anual.
- [ ] **Desarrollo del Servidor API (Rust/NodeJS):**
  - Endpoint para validar _Device ID_ e iniciar Trial.
  - Endpoint de validación de `license-key`.
  - Webhook listener para recibir eventos de pago, renovación o cancelación de Stripe/Lemon Squeezy.
- [ ] **Base de Datos de Usuarios:** Tabla o colección para almacenar emails, User IDs, License Keys activas, estado (Válida, Expirada, Suspendida).

---

## 🔒 8.3 Hardening contra Evasión (Anti-Piracy)
Dado que es una CLI en local, es importante agregar fricciones para que no sea modificado fácilmente.

- [ ] **Verificación de Integridad Binaria:** Asegurar en la medida de lo posible que el binario de Rust no ha sido manipulado (ej. parchear las validaciones).
- [ ] **Checks Periódicos Transparentes:** Cada X días/horas, si hay internet, verificar silenciosamente con la API si la licencia sigue activa (y no ha sido revocada por un contracargo o suscripción cancelada).
- [ ] **Grace Period:** Si el CLI no puede verificar la licencia porque no hay internet, permitir el uso (mínimo unos 3 días de gracia) antes de bloquear el acceso a las funciones `PRO`.

---

## 📩 8.4 Flujo de Experiencia de Usuario (Onboarding)

- [ ] **Día 0 (Instalación):** Al correr `sentinel init`, interceptar si no hay licencia. Otorgar automáticamente 7 Días de Prueba.
- [ ] **Días 1-6:** Al correr comandos `pro`, imprimir un *warning* en amarillo corto: `"Te quedan X días de prueba. Adquiere tu licencia en sentinel-pro.dev"`.
- [ ] **Día 7+:** Bloqueo de las funciones principales. Retornar error rojo indicando que el trial expiró y redirigir a la URL de pago.
- [ ] **Emails de Transición:** (A través de Stripe/Plataforma) Enviar emails al día 1, día 5 y día 7 del trial alentando la compra.
