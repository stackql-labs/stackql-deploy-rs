// commands/build.rs

//! # Build Command Module
//!
//! This module handles the `build` command, which is responsible for creating or updating resources
//! within a specified stack environment.
//!
//! ## Features
//! - Accepts a stack directory and environment as input arguments.
//! - Displays a deployment message with the provided inputs.
//!
//! ## Example Usage
//! ```bash
//! ./stackql-deploy build /path/to/stack/production prod
//! ```
//! The above command deploys resources from the specified stack directory to the `prod` environment.

use clap::{ArgMatches, Command};

use crate::commands::common_args::{
    dry_run, env_file, env_var, log_level, on_failure, show_queries, stack_dir, stack_env,
    FailureAction,
};
use crate::utils::display::print_unicode_box;
use crate::utils::logging::initialize_logger;
use log::{debug, info};

/// Defines the `build` command for the CLI application.
pub fn command() -> Command {
    Command::new("build")
        .about("Create or update resources")
        .arg(stack_dir())
        .arg(stack_env())
        .arg(log_level())
        .arg(env_file())
        .arg(env_var())
        .arg(dry_run())
        .arg(show_queries())
        .arg(on_failure())
}

/// Executes the `build` command.
pub fn execute(matches: &ArgMatches) {
    let stack_dir = matches.get_one::<String>("stack_dir").unwrap();
    let stack_env = matches.get_one::<String>("stack_env").unwrap();

    // Extract the common arguments
    let log_level = matches.get_one::<String>("log-level").unwrap();
    let env_file = matches.get_one::<String>("env-file").unwrap();
    let env_vars = matches.get_many::<String>("env");
    let dry_run = matches.get_flag("dry-run");
    let show_queries = matches.get_flag("show-queries");
    let on_failure = matches.get_one::<FailureAction>("on-failure").unwrap();

    // Initialize the logger
    initialize_logger(log_level);

    print_unicode_box(&format!(
        "🚀 Deploying stack: [{}] to environment: [{}]",
        stack_dir, stack_env
    ));

    info!("Stack Directory: {}", stack_dir);

    println!("Log Level: {}", log_level);
    debug!("Log Level: {}", log_level);
    println!("Environment File: {}", env_file);

    if let Some(vars) = env_vars {
        println!("Environment Variables:");
        for var in vars {
            println!("  - {}", var);
        }
    }

    println!("Dry Run: {}", dry_run);
    println!("Show Queries: {}", show_queries);
    println!("On Failure: {:?}", on_failure);

    // Actual implementation would go here
}
