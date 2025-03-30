// utils/query.rs

//! # Query Utility Module
//!
//! This module provides functions and data structures for executing SQL queries
//! against a PostgreSQL `Client`. It supports processing query results and
//! formatting them into various representations (rows, columns, notices).
//!
//! ## Features
//! - Executes SQL queries using `postgres::Client`.
//! - Formats query results into structured data (columns, rows, notices).
//! - Supports different query result types: Data, Command, and Empty.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::query::{execute_query, QueryResult};
//! use postgres::{Client, NoTls};
//!
//! let mut client = Client::connect("host=localhost user=postgres", NoTls).unwrap();
//! let result = execute_query("SELECT * FROM my_table;", &mut client).unwrap();
//!
//! match result {
//!     QueryResult::Data { columns, rows, .. } => println!("Received data with {} rows.", rows.len()),
//!     QueryResult::Command(cmd) => println!("Command executed: {}", cmd),
//!     QueryResult::Empty => println!("Query executed successfully with no result."),
//! }
//! ```

use postgres::Client;

/// Represents a column in a query result.
pub struct QueryResultColumn {
    pub name: String,
}

/// Represents a row in a query result.
pub struct QueryResultRow {
    pub values: Vec<String>,
}

/// Enum representing the possible results of a query execution.
pub enum QueryResult {
    Data {
        columns: Vec<QueryResultColumn>,
        rows: Vec<QueryResultRow>,
        #[allow(dead_code)]
        notices: Vec<String>,
    },
    Command(String),
    Empty,
}

/// Executes an SQL query and returns the result in a structured format.
pub fn execute_query(query: &str, client: &mut Client) -> Result<QueryResult, String> {
    match client.simple_query(query) {
        Ok(results) => {
            let mut columns = Vec::new();
            let mut rows = Vec::new();
            let mut command_message = String::new();

            for result in results {
                match result {
                    postgres::SimpleQueryMessage::Row(row) => {
                        if columns.is_empty() {
                            for i in 0..row.len() {
                                columns.push(QueryResultColumn {
                                    name: row.columns()[i].name().to_string(),
                                });
                            }
                        }

                        let row_values = (0..row.len())
                            .map(|i| row.get(i).unwrap_or("NULL").to_string())
                            .collect();

                        rows.push(QueryResultRow { values: row_values });
                    }
                    postgres::SimpleQueryMessage::CommandComplete(cmd) => {
                        command_message = cmd.to_string();
                    }
                    _ => {}
                }
            }

            if !columns.is_empty() {
                Ok(QueryResult::Data {
                    columns,
                    rows,
                    notices: vec![],
                })
            } else if !command_message.is_empty() {
                Ok(QueryResult::Command(command_message))
            } else {
                Ok(QueryResult::Empty)
            }
        }
        Err(e) => Err(format!("Query execution failed: {}", e)),
    }
}
