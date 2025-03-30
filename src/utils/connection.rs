// utils/connection.rs

//! # Connection Utility Module
//!
//! This module provides functions for creating a PostgreSQL `Client` connection
//! to the StackQL server. It utilizes a global connection string configuration and
//! supports error handling during connection attempts.
//!
//! ## Features
//! - Establishes a connection to the StackQL server using `postgres::Client`.
//! - Uses a global connection string for consistency across the application.
//! - Handles connection errors and exits the program if unsuccessful.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::connection::create_client;
//!
//! let client = create_client();
//! ```

use std::process;

use colored::*;
use postgres::{Client, NoTls};

use crate::globals::connection_string;

/// Creates a new Client connection
pub fn create_client() -> Client {
    let conn_str = connection_string(); // Uses your global connection string
    Client::connect(conn_str, NoTls).unwrap_or_else(|e| {
        eprintln!("{}", format!("Failed to connect to server: {}", e).red());
        process::exit(1); // Exit the program if connection fails, so there's no returning a Result.
    })
}
