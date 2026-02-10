// template/context.rs

//! # Template Context Module
//!
//! Provides a type for managing template context variables.
//! The context is used to store variables and their values for template rendering.
//!
//! This module also includes functionality for merging contexts, adding/updating
//! variables, and other context-related operations.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

/// Error types that can occur during context operations.
#[derive(Debug)]
pub enum ContextError {
    /// Merging contexts failed
    MergeError(String),
    
    /// Variable not found
    NotFound(String),
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextError::MergeError(msg) => write!(f, "Context merge error: {}", msg),
            ContextError::NotFound(var) => write!(f, "Variable not found: {}", var),
        }
    }
}

impl Error for ContextError {}

/// Type alias for context operation results
pub type ContextResult<T> = Result<T, ContextError>;

/// A context for template rendering.
///
/// This stores a mapping of variable names to their string values.
#[derive(Default, Debug, Clone)]
pub struct Context {
    /// The variables in this context
    variables: HashMap<String, String>,
}

impl Context {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self { variables: HashMap::new() }
    }
    
    /// Creates a new context with initial variables.
    pub fn with_variables(variables: HashMap<String, String>) -> Self {
        Self { variables }
    }
    
    /// Adds a variable to the context.
    ///
    /// If the variable already exists, its value is updated.
    pub fn add_variable(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }
    
    /// Removes a variable from the context.
    pub fn remove_variable(&mut self, name: &str) -> Option<String> {
        self.variables.remove(name)
    }
    
    /// Gets a variable's value from the context.
    pub fn get_variable(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }
    
    /// Checks if a variable exists in the context.
    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }
    
    /// Returns all variables in the context.
    pub fn get_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }
    
    /// Creates a mutable reference to the variables.
    pub fn get_variables_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.variables
    }
    
    /// Merges another context into this one.
    ///
    /// Variables from the other context will overwrite existing variables
    /// with the same name in this context.
    pub fn merge(&mut self, other: &Context) {
        for (name, value) in &other.variables {
            self.variables.insert(name.clone(), value.clone());
        }
    }
    
    /// Creates a new context by merging with another context.
    ///
    /// This returns a new context without modifying either input context.
    pub fn merged_with(&self, other: &Context) -> Self {
        let mut result = self.clone();
        result.merge(other);
        result
    }
    
    /// Creates a child context that inherits values from this context.
    ///
    /// The child context can override values without affecting the parent.
    pub fn create_child(&self) -> Self {
        self.clone()
    }
    
    /// Adds built-in variables like date/time, unique IDs, etc.
    ///
    /// This can be extended in the future with more built-in variables.
    pub fn add_built_ins(&mut self) {
        // Add current date and time
        let now = chrono::Local::now();
        self.add_variable("current_date".to_string(), now.format("%Y-%m-%d").to_string());
        self.add_variable("current_time".to_string(), now.format("%H:%M:%S").to_string());
        self.add_variable("current_datetime".to_string(), now.format("%Y-%m-%d %H:%M:%S").to_string());
        
        // Add a unique ID
        let uuid = uuid::Uuid::new_v4().to_string();
        self.add_variable("uuid".to_string(), uuid);
    }
}

/// Unit tests for context functionality.
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_and_get_variable() {
        let mut context = Context::new();
        context.add_variable("name".to_string(), "Value".to_string());
        
        assert_eq!(context.get_variable("name"), Some(&"Value".to_string()));
        assert_eq!(context.get_variable("nonexistent"), None);
    }
    
    #[test]
    fn test_has_variable() {
        let mut context = Context::new();
        context.add_variable("name".to_string(), "Value".to_string());
        
        assert!(context.has_variable("name"));
        assert!(!context.has_variable("nonexistent"));
    }
    
    #[test]
    fn test_remove_variable() {
        let mut context = Context::new();
        context.add_variable("name".to_string(), "Value".to_string());
        
        let removed = context.remove_variable("name");
        assert_eq!(removed, Some("Value".to_string()));
        assert!(!context.has_variable("name"));
        
        let nonexistent = context.remove_variable("nonexistent");
        assert_eq!(nonexistent, None);
    }
    
    #[test]
    fn test_context_merge() {
        let mut context1 = Context::new();
        context1.add_variable("var1".to_string(), "Value1".to_string());
        context1.add_variable("common".to_string(), "OriginalValue".to_string());
        
        let mut context2 = Context::new();
        context2.add_variable("var2".to_string(), "Value2".to_string());
        context2.add_variable("common".to_string(), "NewValue".to_string());
        
        context1.merge(&context2);
        
        assert_eq!(context1.get_variable("var1"), Some(&"Value1".to_string()));
        assert_eq!(context1.get_variable("var2"), Some(&"Value2".to_string()));
        assert_eq!(context1.get_variable("common"), Some(&"NewValue".to_string()));
    }
    
    #[test]
    fn test_merged_with() {
        let mut context1 = Context::new();
        context1.add_variable("var1".to_string(), "Value1".to_string());
        
        let mut context2 = Context::new();
        context2.add_variable("var2".to_string(), "Value2".to_string());
        
        let merged = context1.merged_with(&context2);
        
        // Original contexts should be unchanged
        assert_eq!(context1.get_variable("var1"), Some(&"Value1".to_string()));
        assert_eq!(context1.get_variable("var2"), None);
        assert_eq!(context2.get_variable("var1"), None);
        assert_eq!(context2.get_variable("var2"), Some(&"Value2".to_string()));
        
        // Merged context should have both variables
        assert_eq!(merged.get_variable("var1"), Some(&"Value1".to_string()));
        assert_eq!(merged.get_variable("var2"), Some(&"Value2".to_string()));
    }
    
    #[test]
    fn test_with_initial_variables() {
        let mut variables = HashMap::new();
        variables.insert("var1".to_string(), "Value1".to_string());
        variables.insert("var2".to_string(), "Value2".to_string());
        
        let context = Context::with_variables(variables);
        
        assert_eq!(context.get_variable("var1"), Some(&"Value1".to_string()));
        assert_eq!(context.get_variable("var2"), Some(&"Value2".to_string()));
    }
    
    #[test]
    fn test_add_built_ins() {
        let mut context = Context::new();
        context.add_built_ins();
        
        assert!(context.has_variable("current_date"));
        assert!(context.has_variable("current_time"));
        assert!(context.has_variable("current_datetime"));
        assert!(context.has_variable("uuid"));
    }
}