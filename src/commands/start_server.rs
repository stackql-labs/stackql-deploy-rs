// commands/start_server.rs

//! # Start Server Command Module
//!
//! This module provides the `start-server` command for the StackQL Deploy application.
//! The `start-server` command initializes and starts a local StackQL server based on the
//! specified configuration options such as mTLS, custom authentication, and logging levels.
//!
//! ## Features
//! - Validates if the server is already running before attempting to start a new instance.
//! - Supports configuration of mTLS and custom authentication via JSON inputs.
//! - Allows setting of logging levels for better observability.
//! - Uses global configuration for host and port.
//!
//! ## Example Usage
//! ```bash
//! ./stackql-deploy start-server --registry "http://localhost:8000" --log-level INFO
//! ```

use std::process;

use clap::{Arg, ArgAction, ArgMatches, Command};
use colored::*;

use crate::app::LOCAL_SERVER_ADDRESSES;
use crate::globals::{server_host, server_port};
use crate::utils::display::print_unicode_box;
use crate::utils::server::{is_server_running, start_server, StartServerOptions};

/// Configures the `start-server` command for the CLI application.
pub fn command() -> Command {
    Command::new("start-server")
        .about("Start the stackql server")
        .arg(
            Arg::new("registry")
                .short('r')
                .long("registry")
                .help("[OPTIONAL] Custom registry URL")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("mtls_config")
                .short('m')
                .long("mtls-config")
                .help("[OPTIONAL] mTLS configuration for the server (JSON object)")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("custom_auth_config")
                .short('a')
                .long("custom-auth-config")
                .help("[OPTIONAL] Custom provider authentication configuration for the server (JSON object)")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("log_level")
                .short('l')
                .long("log-level")
                .help("[OPTIONAL] Server log level (default: WARN)")
                .value_parser(["TRACE", "DEBUG", "INFO", "WARN", "ERROR", "FATAL"])
                .action(ArgAction::Set),
        )
}

/// Executes the `start-server` command.
pub fn execute(matches: &ArgMatches) {
    print_unicode_box("🚀 Starting stackql server...");

    // Use global vars for host and port
    let port = server_port();
    let host = server_host().to_string();

    // Validate host - must be localhost or 0.0.0.0
    if !LOCAL_SERVER_ADDRESSES.contains(&host.as_str()) {
        eprintln!(
            "{}",
            "Error: Host must be 'localhost' or '0.0.0.0' for local server setup.".red()
        );
        eprintln!("The start-server command is only for starting a local server instance.");
        process::exit(1);
    }

    // Check if server is already running
    if is_server_running(port) {
        println!(
            "{}",
            format!(
                "Server is already running on port {}. No action needed.",
                port
            )
            .yellow()
        );
        process::exit(0);
    }

    // Get optional settings
    let registry = matches.get_one::<String>("registry").cloned();
    let mtls_config = matches.get_one::<String>("mtls_config").cloned();
    let custom_auth_config = matches.get_one::<String>("custom_auth_config").cloned();
    let log_level = matches.get_one::<String>("log_level").cloned();

    // Create server options
    let options = StartServerOptions {
        host: host.clone(),
        port,
        registry,
        mtls_config,
        custom_auth_config,
        log_level,
    };

    // Start the server
    match start_server(&options) {
        Ok(_pid) => {
            println!(
                "{}",
                format!("Server is listening on {}:{}", options.host, options.port).green()
            );
        }
        Err(e) => {
            eprintln!("{}", format!("Failed to start server: {}", e).red());
            process::exit(1);
        }
    }
}
