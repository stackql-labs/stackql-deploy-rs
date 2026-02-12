// commands/stop_server.rs

//! # Stop Server Command Module
//!
//! This module provides the `stop-server` command for the StackQL Deploy application.
//! The `stop-server` command stops a running StackQL server by communicating with it
//! over the specified port. This command only applies to local server instances.
//!
//! ## Features
//! - Graceful shutdown of the StackQL server.
//! - Provides feedback on successful or unsuccessful termination attempts.
//! - Uses global port configuration to identify the server to stop.
//!
//! ## Example Usage
//! ```bash
//! ./stackql-deploy stop-server
//! ```

use std::process;

use clap::{ArgMatches, Command};
use colored::*;

use crate::globals::server_port;
use crate::utils::display::print_unicode_box;
use crate::utils::server::stop_server;

/// Configures the `stop-server` command for the CLI application.
pub fn command() -> Command {
    Command::new("stop-server").about("Stop the stackql server")
}

/// Executes the `stop-server` command.
pub fn execute(_matches: &ArgMatches) {
    let port = server_port();

    print_unicode_box(
        "Stopping stackql server...",
        crate::utils::display::BorderColor::Red,
    );

    println!(
        "{}",
        format!("Processing request to stop server on port {}", port).yellow()
    );

    match stop_server(port) {
        Ok(_) => {
            println!("{}", "stackql server stopped successfully".green());
        }
        Err(e) => {
            eprintln!("{}", format!("Failed to stop server: {}", e).red());
            process::exit(1);
        }
    }
}
