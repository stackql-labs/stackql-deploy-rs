// template/mod.rs

//! # Template Module
//!
//! This module provides functionality for template rendering and context management.
//! Templates are used throughout the application to render queries and other text
//! with variable substitution.
//!
//! The module includes an engine for rendering templates and a context for managing
//! variables used in templates.

pub mod context;
pub mod engine;

// Re-export commonly used types, avoid naming conflicts by using aliases
pub use context::ContextError;
pub use engine::TemplateError as EngineTemplateError;

/// Creates a combined error type for template operations.
#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("Engine error: {0}")]
    Engine(#[from] EngineTemplateError),

    #[error("Context error: {0}")]
    Context(#[from] ContextError),

    #[error("Other error: {0}")]
    Other(String), // Keep this if you intend to handle generic errors
}

// Type alias for template operation results
pub type _TemplateResult<T> = std::result::Result<T, TemplateError>;

// If you don't plan to use `Other`, you can suppress the warning like this:
#[allow(dead_code)]
impl TemplateError {
    pub fn other(msg: &str) -> Self {
        TemplateError::Other(msg.to_string())
    }
}
