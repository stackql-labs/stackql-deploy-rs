// resource/operations.rs

//! # Resource Operations Module
//!
//! Provides functionality for performing operations on resources.
//! This includes creating, updating, and deleting resources, as well as
//! checking their existence and state.
//!
//! Operations are performed by executing SQL queries against a StackQL server.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use colored::*;
use postgres::Client;

use crate::resource::manifest::Resource;
use crate::resource::queries::QueryType;
use crate::template::context::Context;
use crate::template::engine::TemplateEngine;
use crate::utils::query::{execute_query, QueryResult};

/// Errors that can occur during resource operations.
#[derive(Debug)]
pub enum OperationError {
    /// Query execution failed
    QueryError(String),
    
    /// Resource validation failed
    ValidationError(String),
    
    /// Missing required query
    MissingQuery(String),
    
    /// Operation not supported for resource type
    UnsupportedOperation(String),
    
    /// State check failed after operation
    StateCheckFailed(String),
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationError::QueryError(msg) => write!(f, "Query error: {}", msg),
            OperationError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            OperationError::MissingQuery(msg) => write!(f, "Missing query: {}", msg),
            OperationError::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
            OperationError::StateCheckFailed(msg) => write!(f, "State check failed: {}", msg),
        }
    }
}

impl Error for OperationError {}

/// Type alias for operation results
pub type OperationResult<T> = Result<T, OperationError>;

/// Result of a resource existence check.
#[derive(Debug, PartialEq)]
pub enum ExistenceStatus {
    /// Resource exists
    Exists,
    
    /// Resource does not exist
    NotExists,
    
    /// Could not determine if resource exists
    Unknown,
}

/// Result of a resource state check.
#[derive(Debug, PartialEq)]
pub enum StateStatus {
    /// Resource is in the correct state
    Correct,
    
    /// Resource is not in the correct state
    Incorrect,
    
    /// Could not determine resource state
    Unknown,
}

/// Handles resource operations.
pub struct ResourceOperator<'a> {
    /// Database client for query execution
    client: &'a mut Client,
    
    /// Template engine for rendering queries
    engine: TemplateEngine,
    
    /// Whether to run in dry-run mode
    dry_run: bool,
    
    /// Whether to show queries
    show_queries: bool,
}

impl<'a> ResourceOperator<'a> {
    /// Creates a new ResourceOperator.
    pub fn new(
        client: &'a mut Client,
        dry_run: bool,
        show_queries: bool,
    ) -> Self {
        Self {
            client,
            engine: TemplateEngine::new(),
            dry_run,
            show_queries,
        }
    }
    
    /// Checks if a resource exists.
    pub fn check_exists(
        &mut self,
        resource: &Resource,
        queries: &HashMap<QueryType, String>,
        context: &Context,
    ) -> OperationResult<ExistenceStatus> {
        // Try exists query first, then fall back to preflight (for backward compatibility), then statecheck
        let exists_query = if let Some(query) = queries.get(&QueryType::Exists) {
            query
        } else if let Some(query) = queries.get(&QueryType::Preflight) {
            query
        } else if let Some(query) = queries.get(&QueryType::StateCheck) {
            query
        } else {
            println!("  {} No exists check configured for [{}]", "ℹ️".bright_blue(), resource.name);
            return Ok(ExistenceStatus::Unknown);
        };
        
        let rendered_query = self.engine.render(exists_query, context.get_variables())
            .map_err(|e| OperationError::QueryError(e.to_string()))?;
        
        if self.dry_run {
            println!("  {} Dry run exists check for [{}]:", "🔎".bright_cyan(), resource.name);
            if self.show_queries {
                println!("{}", rendered_query);
            }
            return Ok(ExistenceStatus::NotExists); // Assume it doesn't exist in dry run
        }
        
        println!("  {} Running exists check for [{}]", "🔎".bright_cyan(), resource.name);
        if self.show_queries {
            println!("{}", rendered_query);
        }
        
        match execute_query(&rendered_query, self.client) {
            Ok(result) => match result {
                QueryResult::Data { columns, rows, .. } => {
                    if rows.is_empty() || columns.is_empty() {
                        return Ok(ExistenceStatus::NotExists);
                    }
                    
                    // Check for "count" column with value 1
                    let count_col_idx = columns.iter().position(|c| c.name == "count");
                    if let Some(idx) = count_col_idx {
                        if let Some(row) = rows.first() {
                            if let Some(count) = row.values.get(idx) {
                                if count == "1" {
                                    return Ok(ExistenceStatus::Exists);
                                } else {
                                    return Ok(ExistenceStatus::NotExists);
                                }
                            }
                        }
                    }
                    
                    Ok(ExistenceStatus::NotExists)
                },
                _ => Ok(ExistenceStatus::NotExists),
            },
            Err(e) => Err(OperationError::QueryError(format!("Exists check failed: {}", e))),
        }
    }
    
    /// Checks if a resource is in the correct state.
    pub fn check_state(
        &mut self,
        resource: &Resource,
        queries: &HashMap<QueryType, String>,
        context: &Context,
    ) -> OperationResult<StateStatus> {
        let statecheck_query = if let Some(query) = queries.get(&QueryType::StateCheck) {
            query
        } else if let Some(query) = queries.get(&QueryType::PostDeploy) {
            query
        } else {
            println!("  {} State check not configured for [{}]", "ℹ️".bright_blue(), resource.name);
            return Ok(StateStatus::Unknown);
        };
        
        let rendered_query = self.engine.render(statecheck_query, context.get_variables())
            .map_err(|e| OperationError::QueryError(e.to_string()))?;
        
        if self.dry_run {
            println!("  {} Dry run state check for [{}]:", "🔎".bright_cyan(), resource.name);
            if self.show_queries {
                println!("{}", rendered_query);
            }
            return Ok(StateStatus::Correct); // Assume correct state in dry run
        }
        
        println!("  {} Running state check for [{}]", "🔎".bright_cyan(), resource.name);
        if self.show_queries {
            println!("{}", rendered_query);
        }
        
        match execute_query(&rendered_query, self.client) {
            Ok(result) => match result {
                QueryResult::Data { columns, rows, .. } => {
                    if rows.is_empty() || columns.is_empty() {
                        return Ok(StateStatus::Incorrect);
                    }
                    
                    // Check for "count" column with value 1
                    let count_col_idx = columns.iter().position(|c| c.name == "count");
                    if let Some(idx) = count_col_idx {
                        if let Some(row) = rows.first() {
                            if let Some(count) = row.values.get(idx) {
                                if count == "1" {
                                    println!("  {} [{}] is in the desired state", "👍".green(), resource.name);
                                    return Ok(StateStatus::Correct);
                                } else {
                                    println!("  {} [{}] is not in the desired state", "👎".yellow(), resource.name);
                                    return Ok(StateStatus::Incorrect);
                                }
                            }
                        }
                    }
                    
                    println!("  {} Could not determine state for [{}]", "⚠️".yellow(), resource.name);
                    Ok(StateStatus::Unknown)
                },
                _ => {
                    println!("  {} Unexpected result type from state check", "⚠️".yellow());
                    Ok(StateStatus::Unknown)
                },
            },
            Err(e) => Err(OperationError::QueryError(format!("State check failed: {}", e))),
        }
    }
    
    /// Creates a new resource.
    pub fn create_resource(
        &mut self,
        resource: &Resource,
        queries: &HashMap<QueryType, String>,
        context: &Context,
    ) -> OperationResult<bool> {
        // Try createorupdate query first, then fall back to create
        let create_query = if let Some(query) = queries.get(&QueryType::CreateOrUpdate) {
            query
        } else if let Some(query) = queries.get(&QueryType::Create) {
            query
        } else {
            return Err(OperationError::MissingQuery(
                format!("No create or createorupdate query for resource '{}'", resource.name)
            ));
        };
        
        let rendered_query = self.engine.render(create_query, context.get_variables())
            .map_err(|e| OperationError::QueryError(e.to_string()))?;
        
        if self.dry_run {
            println!("  {} Dry run create for [{}]:", "🚧".yellow(), resource.name);
            if self.show_queries {
                println!("{}", rendered_query);
            }
            return Ok(true); // Pretend success in dry run
        }
        
        println!("  {} [{}] does not exist, creating...", "🚧".yellow(), resource.name);
        if self.show_queries {
            println!("{}", rendered_query);
        }
        
        match execute_query(&rendered_query, self.client) {
            Ok(_) => {
                println!("  {} Resource created successfully", "✓".green());
                Ok(true)
            },
            Err(e) => Err(OperationError::QueryError(format!("Create operation failed: {}", e))),
        }
    }
    
    /// Updates an existing resource.
    pub fn update_resource(
        &mut self,
        resource: &Resource,
        queries: &HashMap<QueryType, String>,
        context: &Context,
    ) -> OperationResult<bool> {
        let update_query = if let Some(query) = queries.get(&QueryType::Update) {
            query
        } else {
            println!("  {} Update query not configured for [{}], skipping update", 
                "ℹ️".bright_blue(), resource.name);
            return Ok(false);
        };
        
        let rendered_query = self.engine.render(update_query, context.get_variables())
            .map_err(|e| OperationError::QueryError(e.to_string()))?;
        
        if self.dry_run {
            println!("  {} Dry run update for [{}]:", "🚧".yellow(), resource.name);
            if self.show_queries {
                println!("{}", rendered_query);
            }
            return Ok(true); // Pretend success in dry run
        }
        
        println!("  {} Updating [{}]...", "🔧".yellow(), resource.name);
        if self.show_queries {
            println!("{}", rendered_query);
        }
        
        match execute_query(&rendered_query, self.client) {
            Ok(_) => {
                println!("  {} Resource updated successfully", "✓".green());
                Ok(true)
            },
            Err(e) => Err(OperationError::QueryError(format!("Update operation failed: {}", e))),
        }
    }
    
    /// Deletes a resource.
    pub fn delete_resource(
        &mut self,
        resource: &Resource,
        queries: &HashMap<QueryType, String>,
        context: &Context,
    ) -> OperationResult<bool> {
        let delete_query = if let Some(query) = queries.get(&QueryType::Delete) {
            query
        } else {
            return Err(OperationError::MissingQuery(
                format!("No delete query for resource '{}'", resource.name)
            ));
        };
        
        let rendered_query = self.engine.render(delete_query, context.get_variables())
            .map_err(|e| OperationError::QueryError(e.to_string()))?;
        
        if self.dry_run {
            println!("  {} Dry run delete for [{}]:", "🚧".yellow(), resource.name);
            if self.show_queries {
                println!("{}", rendered_query);
            }
            return Ok(true); // Pretend success in dry run
        }
        
        println!("  {} Deleting [{}]...", "🚧".yellow(), resource.name);
        if self.show_queries {
            println!("{}", rendered_query);
        }
        
        match execute_query(&rendered_query, self.client) {
            Ok(_) => {
                println!("  {} Resource deleted successfully", "✓".green());
                Ok(true)
            },
            Err(e) => Err(OperationError::QueryError(format!("Delete operation failed: {}", e))),
        }
    }
    
    /// Processes exports from a resource.
    pub fn process_exports(
        &mut self,
        resource: &Resource,
        queries: &HashMap<QueryType, String>,
        context: &mut Context,
    ) -> OperationResult<HashMap<String, String>> {
        let exports_query = if let Some(query) = queries.get(&QueryType::Exports) {
            query
        } else {
            println!("  {} No exports query for [{}]", "ℹ️".bright_blue(), resource.name);
            return Ok(HashMap::new());
        };
        
        let rendered_query = self.engine.render(exports_query, context.get_variables())
            .map_err(|e| OperationError::QueryError(e.to_string()))?;
        
        let mut exported_values = HashMap::new();
        
        if self.dry_run {
            println!("  {} Dry run exports for [{}]:", "📦".bright_magenta(), resource.name);
            if self.show_queries {
                println!("{}", rendered_query);
            }
            
            // Simulate exports in dry run
            for export in &resource.exports {
                let value = "<dry-run-value>".to_string();
                context.get_variables_mut().insert(export.clone(), value.clone());
                exported_values.insert(export.clone(), value);
                println!("  📤 Set [{}] to [<dry-run-value>] in exports", export);
            }
            
            return Ok(exported_values);
        }
        
        println!("  {} Exporting variables for [{}]", "📦".bright_magenta(), resource.name);
        if self.show_queries {
            println!("{}", rendered_query);
        }
        
        match execute_query(&rendered_query, self.client) {
            Ok(result) => match result {
                QueryResult::Data { columns, rows, .. } => {
                    if rows.is_empty() {
                        return Err(OperationError::QueryError("Exports query returned no rows".to_string()));
                    }
                    
                    let row = &rows[0]; // Typically exports query returns one row
                    
                    for (i, col) in columns.iter().enumerate() {
                        if i < row.values.len() && resource.exports.contains(&col.name) {
                            let value = row.values[i].clone();
                            
                            if resource.protected.contains(&col.name) {
                                let mask = "*".repeat(value.len());
                                println!("  🔒 Set protected variable [{}] to [{}] in exports", col.name, mask);
                            } else {
                                println!("  📤 Set [{}] to [{}] in exports", col.name, value);
                            }
                            
                            context.get_variables_mut().insert(col.name.clone(), value.clone());
                            exported_values.insert(col.name.clone(), value);
                        }
                    }
                    
                    Ok(exported_values)
                },
                _ => Err(OperationError::QueryError("Unexpected result from exports query".to_string())),
            },
            Err(e) => Err(OperationError::QueryError(format!("Exports query failed: {}", e))),
        }
    }
}

/// Unit tests for resource operations.
#[cfg(test)]
mod tests {
    // These would be added in a real implementation to test the operations
    // with a mock database client
}