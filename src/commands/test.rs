// commands/test.rs

//! # Test Command Module
//!
//! This module provides the `test` command for the StackQL Deploy application.
//! The `test` command checks whether a specified stack is in the correct desired state
//! within a given environment. It validates the current state against expected outputs
//! defined in the stack configuration.
//!
//! ## Features
//! - Validates the current infrastructure state against the desired state.
//! - Ensures all resources are correctly provisioned and meet specified requirements.
//! - Uses the same positional arguments as `build`, `plan`, and `teardown` commands.
//!
//! ## Example Usage
//! ```bash
//! ./stackql-deploy test /path/to/stack dev
//! ```

use clap::{ArgMatches, Command};

use crate::commands::common_args::{
    dry_run, env_file, env_var, log_level, on_failure, show_queries, stack_dir, stack_env,
    FailureAction,
};
use crate::utils::display::print_unicode_box;

/// Configures the `test` command for the CLI application.
pub fn command() -> Command {
    Command::new("test")
        .about("Run test queries for the stack")
        .arg(stack_dir())
        .arg(stack_env())
        .arg(log_level())
        .arg(env_file())
        .arg(env_var())
        .arg(dry_run())
        .arg(show_queries())
        .arg(on_failure())
}

/// Executes the `test` command.
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

    print_unicode_box(&format!(
        "Testing stack: [{}] in environment: [{}]",
        stack_dir, stack_env
    ));

    println!("Log Level: {}", log_level);
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

    // Here you would implement the actual test functionality

    println!("🔍 tests complete (dry run: {})", dry_run);
}
