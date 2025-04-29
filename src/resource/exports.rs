// resource/exports.rs

//! # Resource Exports Module
//!
//! Handles exporting variables from resources.
//! Exports are used to share data between resources, such as IDs or attributes
//! that are needed for dependent resources.
//!
//! This module provides functionality for processing exports, including
//! masking protected values and updating the context with exported values.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use colored::*;

use crate::resource::manifest::Resource;
use crate::template::context::Context;

/// Errors that can occur during export operations.
#[derive(Debug)]
pub enum ExportError {
    /// Missing required export
    MissingExport(String),

    /// Invalid export format
    InvalidFormat(String),

    /// Export processing failed
    ProcessingFailed(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::MissingExport(name) => write!(f, "Missing required export: {}", name),
            ExportError::InvalidFormat(msg) => write!(f, "Invalid export format: {}", msg),
            ExportError::ProcessingFailed(msg) => write!(f, "Export processing failed: {}", msg),
        }
    }
}

impl Error for ExportError {}

/// Type alias for export operation results
pub type ExportResult<T> = Result<T, ExportError>;

/// Represents the result of processing exports.
#[derive(Debug, Clone)]
pub struct ExportOutput {
    /// Exported values
    pub values: HashMap<String, String>,

    /// Protected values that were exported (keys only)
    pub protected: Vec<String>,
}

/// Processes exports from a query result.
///
/// # Arguments
/// * `resource` - The resource being processed
/// * `row` - Row of data from query result
/// * `columns` - Column definitions from query result
/// * `dry_run` - Whether this is a dry run
///
/// # Returns
/// A map of export names to values.
pub fn process_raw_exports(
    resource: &Resource,
    row: Option<&Vec<String>>,
    columns: &[String],
    dry_run: bool,
) -> ExportResult<ExportOutput> {
    let mut exported = HashMap::new();
    let protected = resource.protected.clone();

    if dry_run {
        // For dry run, just use placeholder values
        for export_name in &resource.exports {
            exported.insert(export_name.clone(), "<dry-run-value>".to_string());
        }
    } else if let Some(row_values) = row {
        // Check if we have values to export
        if row_values.len() != columns.len() {
            return Err(ExportError::InvalidFormat(
                "Column count mismatch in export query result".to_string(),
            ));
        }

        // Extract values for each requested export
        for export_name in &resource.exports {
            // Find the column index for this export
            if let Some(idx) = columns.iter().position(|c| c == export_name) {
                if idx < row_values.len() {
                    let value = row_values[idx].clone();
                    exported.insert(export_name.clone(), value);
                } else {
                    return Err(ExportError::MissingExport(format!(
                        "Export '{}' column index out of bounds",
                        export_name
                    )));
                }
            } else {
                return Err(ExportError::MissingExport(format!(
                    "Export '{}' not found in query result",
                    export_name
                )));
            }
        }
    } else {
        // No row data
        return Err(ExportError::ProcessingFailed(
            "No row data for exports".to_string(),
        ));
    }

    Ok(ExportOutput {
        values: exported,
        protected,
    })
}

/// Updates a context with exported values.
///
/// # Arguments
/// * `context` - The context to update
/// * `exports` - The export output to apply
/// * `show_values` - Whether to print the values being exported
///
/// # Returns
/// Nothing, but updates the context in place.
pub fn apply_exports_to_context(context: &mut Context, exports: &ExportOutput, show_values: bool) {
    for (name, value) in &exports.values {
        if exports.protected.contains(name) {
            // Mask protected values in output
            if show_values {
                let mask = "*".repeat(value.len());
                println!(
                    "  🔒 Set protected variable [{}] to [{}] in exports",
                    name, mask
                );
            }
        } else {
            // Show regular exports
            if show_values {
                println!("  📤 Set [{}] to [{}] in exports", name, value);
            }
        }

        // Add to context
        context.add_variable(name.clone(), value.clone());
    }
}

/// Processes exports for all resources in a stack.
///
/// Useful for commands like teardown that need to process all exports
/// before starting operations.
///
/// # Arguments
/// * `resources` - Resources to process
/// * `context` - Context to update with exports
/// * `client` - Database client
/// * `dry_run` - Whether this is a dry run
///
/// # Returns
/// Success or error
pub fn collect_all_exports(
    resources: &Vec<serde_yaml::Value>,
    context: &mut Context,
    client: &mut postgres::Client,
    dry_run: bool,
) -> ExportResult<()> {
    let _ = client;
    let _ = dry_run;

    println!("Collecting exports for all resources...");

    for resource in resources {
        // Skip if not a resource type or has no exports
        let resource_type = resource["type"].as_str().unwrap_or("resource");
        if resource_type == "script" || resource_type == "command" {
            continue;
        }

        if !resource["exports"].is_sequence()
            || resource["exports"].as_sequence().unwrap().is_empty()
        {
            continue;
        }

        // Get resource name
        let resource_name = match resource["name"].as_str() {
            Some(name) => name,
            None => {
                eprintln!("Error: Missing 'name' for resource");
                continue;
            }
        };

        println!(
            "  {} Collecting exports for {}",
            "📦".bright_magenta(),
            resource_name
        );

        // This part would require refactoring or additional methods to properly handle
        // resource loading and processing exports. In a full implementation, we would have:
        //
        // 1. Load the resource from the manifest
        // 2. Load its queries
        // 3. Render and execute the exports query
        // 4. Process the results and update the context

        // For now, we'll simulate a simplified version
        // In a real implementation, this would use the proper loading functions
        let fake_export_values = HashMap::new(); // Would be actual values in real implementation
        let fake_protected = Vec::new();

        let fake_exports = ExportOutput {
            values: fake_export_values,
            protected: fake_protected,
        };

        apply_exports_to_context(context, &fake_exports, false);
    }

    Ok(())
}

/// Unit tests for export functionality.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::manifest::Resource;

    #[test]
    fn test_process_raw_exports() {
        // Create a test resource with exports
        let resource = Resource {
            name: "test-resource".to_string(),
            r#type: "resource".to_string(),
            file: None,
            props: Vec::new(),
            exports: vec!["id".to_string(), "name".to_string()],
            protected: vec!["id".to_string()],
            description: "".to_string(),
            r#if: None,
        };

        // Test with a row of data
        let columns = vec!["id".to_string(), "name".to_string()];
        let row = vec!["123".to_string(), "test".to_string()];

        let result = process_raw_exports(&resource, Some(&row), &columns, false).unwrap();

        assert_eq!(result.values.len(), 2);
        assert_eq!(result.values.get("id").unwrap(), "123");
        assert_eq!(result.values.get("name").unwrap(), "test");
        assert_eq!(result.protected.len(), 1);
        assert!(result.protected.contains(&"id".to_string()));

        // Test dry run
        let dry_result = process_raw_exports(&resource, None, &columns, true).unwrap();

        assert_eq!(dry_result.values.len(), 2);
        assert_eq!(dry_result.values.get("id").unwrap(), "<dry-run-value>");
        assert_eq!(dry_result.values.get("name").unwrap(), "<dry-run-value>");
    }

    #[test]
    fn test_apply_exports_to_context() {
        let mut context = Context::new();

        let mut values = HashMap::new();
        values.insert("id".to_string(), "123".to_string());
        values.insert("name".to_string(), "test".to_string());

        let exports = ExportOutput {
            values,
            protected: vec!["id".to_string()],
        };

        apply_exports_to_context(&mut context, &exports, false);

        assert_eq!(context.get_variable("id").unwrap(), "123");
        assert_eq!(context.get_variable("name").unwrap(), "test");
    }
}
