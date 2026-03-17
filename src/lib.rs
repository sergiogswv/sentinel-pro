//! Sentinel Pro Library - Core functionality exported for testing and embedding
//!
//! This library module provides access to Sentinel's core components including:
//! - Rule engine and custom rules system
//! - Configuration management
//! - Static analysis and language support
//! - ML-based analysis
//! - Project indexing

pub mod agents;
pub mod ai;
pub mod commands;
pub mod config;
pub mod docs;
pub mod files;
pub mod git;
pub mod index;
pub mod business_logic_guard;
pub mod ml;
pub mod rules;
pub mod stats;
pub mod telemetry;
pub mod tests;
pub mod ui;
pub mod update;

// Agent integration
pub mod agent_config;
pub mod agent_models;
pub mod agent_reporter;
pub mod agent_server;
pub mod agent_interaction;
