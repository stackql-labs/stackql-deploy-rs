// resource/queries.rs

//! # Resource Queries Module
//!
//! Handles parsing and managing queries for resources.
//! Queries are stored in .iql files and include various types like
//! exists, create, update, delete, and statecheck.
//!
//! This module provides functionality for loading query files, parsing queries,
//! and working with query options.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use thiserror::Error;

/// Errors that can occur when working with queries.
#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Failed to read query file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Invalid query format: {0}")]
    InvalidFormat(String),

    #[error("Missing query: {0}")]
    MissingQuery(String),

    #[error("Invalid query type: {0}")]
    InvalidType(String),
}

/// Type alias for query results
pub type QueryResult<T> = Result<T, QueryError>;

/// Types of queries that can be defined in a resource file.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum QueryType {
    /// Check if a resource exists
    Exists,

    /// Preflight check (alias for Exists for backward compatibility)
    Preflight,

    /// Create a new resource
    Create,

    /// Update an existing resource
    Update,

    /// Create or update a resource (idempotent operation)
    CreateOrUpdate,

    /// Check if a resource is in the correct state
    StateCheck,

    /// Post-deployment check (alias for StateCheck for backward compatibility)
    PostDeploy,

    /// Export variables from a resource
    Exports,

    /// Delete a resource
    Delete,

    /// Execute a command
    Command,
}

impl FromStr for QueryType {
    type Err = QueryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "exists" => Ok(QueryType::Exists),
            "preflight" => Ok(QueryType::Preflight),
            "create" => Ok(QueryType::Create),
            "update" => Ok(QueryType::Update),
            "createorupdate" => Ok(QueryType::CreateOrUpdate),
            "statecheck" => Ok(QueryType::StateCheck),
            "postdeploy" => Ok(QueryType::PostDeploy),
            "exports" => Ok(QueryType::Exports),
            "delete" => Ok(QueryType::Delete),
            "command" => Ok(QueryType::Command),
            _ => Err(QueryError::InvalidType(format!(
                "Unknown query type: {}",
                s
            ))),
        }
    }
}

/// Options for a query.
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// Number of times to retry the query
    pub retries: u32,

    /// Delay between retries in seconds
    pub retry_delay: u32,

    /// Number of times to retry after deletion
    pub postdelete_retries: u32,

    /// Delay between post-deletion retries in seconds
    pub postdelete_retry_delay: u32,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            retries: 1,
            retry_delay: 0,
            postdelete_retries: 10,
            postdelete_retry_delay: 5,
        }
    }
}

/// Represents a query with its options.
#[derive(Debug, Clone)]
pub struct Query {
    /// Type of query
    pub query_type: QueryType,

    /// SQL query text
    pub sql: String,

    /// Options for the query
    pub options: QueryOptions,
}

/// Loads queries from a file.
pub fn load_queries_from_file(path: &Path) -> QueryResult<HashMap<QueryType, Query>> {
    let content = fs::read_to_string(path)?;
    parse_queries_from_content(&content)
}

/// Parses queries from content.
pub fn parse_queries_from_content(content: &str) -> QueryResult<HashMap<QueryType, Query>> {
    let mut queries = HashMap::new();
    let mut current_query_type: Option<QueryType> = None;
    let mut current_options = QueryOptions::default();
    let mut current_query = String::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Check for query anchor
        if line.starts_with("/*+") && line.contains("*/") {
            // Store previous query if exists
            if let Some(query_type) = current_query_type.take() {
                if !current_query.is_empty() {
                    queries.insert(
                        query_type.clone(),
                        Query {
                            query_type,
                            sql: current_query.trim().to_string(),
                            options: current_options,
                        },
                    );
                    current_query = String::new();
                    current_options = QueryOptions::default();
                }
            }

            // Extract new anchor
            let start = line.find("/*+").unwrap() + 3;
            let end = line.find("*/").unwrap();
            let anchor_with_options = &line[start..end].trim();

            // Handle options (like retries=5)
            let parts: Vec<&str> = anchor_with_options.split(',').collect();
            if let Ok(query_type) = QueryType::from_str(parts[0].trim()) {
                current_query_type = Some(query_type);

                // Parse options
                for part in &parts[1..] {
                    let option_parts: Vec<&str> = part.split('=').collect();
                    if option_parts.len() == 2 {
                        let option_name = option_parts[0].trim();
                        let option_value = option_parts[1].trim();

                        if let Ok(value) = option_value.parse::<u32>() {
                            match option_name {
                                "retries" => current_options.retries = value,
                                "retry_delay" => current_options.retry_delay = value,
                                "postdelete_retries" => current_options.postdelete_retries = value,
                                "postdelete_retry_delay" => {
                                    current_options.postdelete_retry_delay = value
                                }
                                _ => {} // Ignore unknown options
                            }
                        }
                    }
                }
            } else {
                current_query_type = None;
            }
        } else if let Some(_) = current_query_type {
            // Accumulate query content
            current_query.push_str(line);
            current_query.push('\n');
        }

        i += 1;
    }

    // Store last query if exists
    if let Some(query_type) = current_query_type {
        if !current_query.is_empty() {
            queries.insert(
                query_type.clone(),
                Query {
                    query_type,
                    sql: current_query.trim().to_string(),
                    options: current_options,
                },
            );
        }
    }

    Ok(queries)
}

/// Gets all queries as a simple map from query type to SQL string.
pub fn get_queries_as_map(queries: &HashMap<QueryType, Query>) -> HashMap<QueryType, String> {
    queries
        .iter()
        .map(|(k, v)| (k.clone(), v.sql.clone()))
        .collect()
}

/// Unit tests for query functionality.
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_query_file() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();

        writeln!(file, "/*+ exists */").unwrap();
        writeln!(file, "SELECT COUNT(*) as count FROM aws.ec2.vpc_tags").unwrap();
        writeln!(file, "WHERE region = '{{ region }}';").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "/*+ create, retries=3, retry_delay=5 */").unwrap();
        writeln!(file, "INSERT INTO aws.ec2.vpcs (").unwrap();
        writeln!(file, " CidrBlock,").unwrap();
        writeln!(file, " region").unwrap();
        writeln!(file, ")").unwrap();
        writeln!(file, "SELECT ").unwrap();
        writeln!(file, " '{{ vpc_cidr_block }}',").unwrap();
        writeln!(file, " '{{ region }}';").unwrap();

        file
    }

    #[test]
    fn test_parse_queries() {
        let file = create_test_query_file();
        let content = fs::read_to_string(file.path()).unwrap();

        let queries = parse_queries_from_content(&content).unwrap();

        assert_eq!(queries.len(), 2);
        assert!(queries.contains_key(&QueryType::Exists));
        assert!(queries.contains_key(&QueryType::Create));

        let create_query = queries.get(&QueryType::Create).unwrap();
        assert_eq!(create_query.options.retries, 3);
        assert_eq!(create_query.options.retry_delay, 5);
    }

    #[test]
    fn test_query_type_from_str() {
        assert_eq!(QueryType::from_str("exists").unwrap(), QueryType::Exists);
        assert_eq!(QueryType::from_str("create").unwrap(), QueryType::Create);
        assert_eq!(
            QueryType::from_str("createorupdate").unwrap(),
            QueryType::CreateOrUpdate
        );
        assert_eq!(
            QueryType::from_str("statecheck").unwrap(),
            QueryType::StateCheck
        );
        assert_eq!(QueryType::from_str("exports").unwrap(), QueryType::Exports);
        assert_eq!(QueryType::from_str("delete").unwrap(), QueryType::Delete);

        // Case insensitive
        assert_eq!(QueryType::from_str("EXISTS").unwrap(), QueryType::Exists);
        assert_eq!(QueryType::from_str("Create").unwrap(), QueryType::Create);

        // With spaces
        assert_eq!(QueryType::from_str(" exists ").unwrap(), QueryType::Exists);

        // Invalid
        assert!(QueryType::from_str("invalid").is_err());
    }

    #[test]
    fn test_get_queries_as_map() {
        let mut queries = HashMap::new();
        queries.insert(
            QueryType::Exists,
            Query {
                query_type: QueryType::Exists,
                sql: "SELECT COUNT(*) FROM table".to_string(),
                options: QueryOptions::default(),
            },
        );
        queries.insert(
            QueryType::Create,
            Query {
                query_type: QueryType::Create,
                sql: "INSERT INTO table VALUES (1)".to_string(),
                options: QueryOptions::default(),
            },
        );

        let map = get_queries_as_map(&queries);

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get(&QueryType::Exists).unwrap(),
            "SELECT COUNT(*) FROM table"
        );
        assert_eq!(
            map.get(&QueryType::Create).unwrap(),
            "INSERT INTO table VALUES (1)"
        );
    }
}
