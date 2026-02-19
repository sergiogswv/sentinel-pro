//! Módulo de integración con IA
//!
//! Proporciona funcionalidades para:
//! - Consultas a diferentes proveedores de IA (Anthropic, Gemini)
//! - Detección automática de frameworks
//! - Análisis de arquitectura de código
//! - Detección y validación de frameworks de testing
//! - Sistema de caché para optimizar consultas

pub mod analysis;
pub mod cache;
pub mod client;
pub mod framework;
pub mod testing;
pub mod utils;

// Re-exports públicos
pub use analysis::analizar_arquitectura;
pub use cache::limpiar_cache;
pub use client::{TaskType, consultar_ia_dinamico, obtener_embeddings};
pub use framework::{detectar_framework_con_ia, listar_modelos_gemini};
pub use testing::{TestingStatus, detectar_testing_framework};
