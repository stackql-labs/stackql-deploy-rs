// template/engine.rs

//! # Template Engine Module
//!
//! Provides functionality for rendering templates with variable substitution.
//! The engine is responsible for taking template strings and replacing variable
//! placeholders with their corresponding values from a context.
//!
//! This implementation supports the Jinja-like syntax using `{{ variable_name }}`.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

/// Error types that can occur during template rendering.
#[derive(Debug)]
pub enum TemplateError {
    /// Variable not found in context
    VariableNotFound(String),
    
    /// Syntax error in template
    SyntaxError(String),
    
    /// Invalid template structure
    InvalidTemplate(String),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateError::VariableNotFound(var) => write!(f, "Variable not found: {}", var),
            TemplateError::SyntaxError(msg) => write!(f, "Template syntax error: {}", msg),
            TemplateError::InvalidTemplate(msg) => write!(f, "Invalid template: {}", msg),
        }
    }
}

impl Error for TemplateError {}

/// Type alias for template rendering results
pub type TemplateResult<T> = Result<T, TemplateError>;

/// A structure that renders templates.
#[derive(Default, Debug)]
pub struct TemplateEngine {
    // Configuration options could be added here in the future
}

impl TemplateEngine {
    /// Creates a new template engine.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Renders a template string using the provided context.
    ///
    /// Replaces all instances of `{{ variable_name }}` with the corresponding
    /// value from the context.
    ///
    /// # Arguments
    /// * `template` - The template string to render
    /// * `context` - The context containing variable values
    ///
    /// # Returns
    /// The rendered string with all variables replaced.
    ///
    /// # Errors
    /// Returns an error if:
    /// * A variable used in the template is not found in the context
    /// * The template has syntax errors (e.g., unclosed variables)
    pub fn render(&self, template: &str, context: &HashMap<String, String>) -> TemplateResult<String> {
        let mut result = String::with_capacity(template.len());
        let mut chars = template.chars().peekable();
        
        while let Some(&c) = chars.peek() {
            match c {
                '{' => {
                    // Consume the '{'
                    chars.next();
                    
                    // Check if it's the start of a variable
                    if let Some('{') = chars.peek() {
                        // Consume the second '{'
                        chars.next();
                        
                        // Extract the variable name
                        let var_name = self.extract_variable_name(&mut chars)?;
                        
                        // Look up the variable in the context
                        match context.get(&var_name) {
                            Some(value) => result.push_str(value),
                            _none => {
                                return Err(TemplateError::VariableNotFound(var_name));
                            }
                        }
                    } else {
                        // Just a regular '{' character
                        result.push('{');
                    }
                },
                _ => {
                    // Regular character, just copy it
                    result.push(c);
                    chars.next();
                }
            }
        }
        
        Ok(result)
    }
    
    /// Extracts a variable name from a character iterator.
    ///
    /// Assumes the opening `{{` has already been consumed.
    /// Consumes characters until it finds the closing `}}`.
    fn extract_variable_name<I>(&self, chars: &mut std::iter::Peekable<I>) -> TemplateResult<String>
    where
        I: Iterator<Item = char>,
    {
        let mut var_name = String::new();
        let mut found_closing = false;
        
        while let Some(c) = chars.next() {
            match c {
                '}' => {
                    if let Some(&'}') = chars.peek() {
                        // Consume the second '}'
                        chars.next();
                        found_closing = true;
                        break;
                    } else {
                        // Single '}', still part of the variable name
                        var_name.push(c);
                    }
                },
                _ => var_name.push(c),
            }
        }
        
        if !found_closing {
            return Err(TemplateError::SyntaxError("Unclosed variable".to_string()));
        }
        
        // Trim whitespace from the variable name
        Ok(var_name.trim().to_string())
    }
    
    /// Renders a template string with built-in support for conditionals and loops.
    ///
    /// This more advanced version can process simple conditions and loops.
    /// Note: This is a placeholder for future implementation.
    #[allow(dead_code)]
    pub fn render_advanced(&self, _template: &str, _context: &HashMap<String, String>) -> TemplateResult<String> {
        // This is a placeholder for future implementation of more advanced template features
        // like conditionals and loops.
        Err(TemplateError::InvalidTemplate("Advanced rendering not implemented yet".to_string()))
    }
}

/// Unit tests for template engine functionality.
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_variable_substitution() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("name".to_string(), "World".to_string());
        
        let result = engine.render("Hello {{ name }}!", &context).unwrap();
        assert_eq!(result, "Hello World!");
    }
    
    #[test]
    fn test_multiple_variables() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("first".to_string(), "Hello".to_string());
        context.insert("second".to_string(), "World".to_string());
        
        let result = engine.render("{{ first }} {{ second }}!", &context).unwrap();
        assert_eq!(result, "Hello World!");
    }
    
    #[test]
    fn test_variable_not_found() {
        let engine = TemplateEngine::new();
        let context = HashMap::new();
        
        let result = engine.render("Hello {{ name }}!", &context);
        assert!(result.is_err());
        match result {
            Err(TemplateError::VariableNotFound(var)) => assert_eq!(var, "name"),
            _ => panic!("Expected VariableNotFound error"),
        }
    }
    
    #[test]
    fn test_unclosed_variable() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("name".to_string(), "World".to_string());
        
        let result = engine.render("Hello {{ name!", &context);
        assert!(result.is_err());
        match result {
            Err(TemplateError::SyntaxError(_)) => {},
            _ => panic!("Expected SyntaxError"),
        }
    }
    
    #[test]
    fn test_nested_braces() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("json".to_string(), r#"{"key": "value"}"#.to_string());
        
        let result = engine.render("JSON: {{ json }}", &context).unwrap();
        assert_eq!(result, r#"JSON: {"key": "value"}"#);
    }
}