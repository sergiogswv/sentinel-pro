//! Módulo de detección de patrones de código
//!
//! Este módulo analiza el codebase para identificar estilos consistentes,
//! convenciones de nombrado y patrones recurrentes.

pub mod style;

#[allow(unused_imports)]
pub use style::{CodeStyleProfile, StyleAnalyzer};
