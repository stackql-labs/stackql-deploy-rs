// commands/plan.rs

//! # Plan Command Module
//!
//! This module provides the `plan` command for the StackQL Deploy application.
//! The `plan` command compares the current state of infrastructure (live, not from a state file)
//! against the desired state defined by configuration files. It outputs the necessary queries
//! that would need to be run to achieve the desired state.
//!
//! ## Features
//! - Compare live infrastructure state against desired state.
//! - Generate queries required to achieve the desired state.
//! - Provide dry-run capability for previewing changes before applying.
//!
//! ## Example Usage
//! ```bash
//! ./stackql-deploy plan path/to/stack dev
//! ```

use clap::{ArgMatches, Command};

use crate::commands::common_args::{
    dry_run, env_file, env_var, log_level, on_failure, show_queries, stack_dir, stack_env,
    FailureAction,
};
use crate::utils::display::print_unicode_box;

/// Configures the `plan` command for the CLI application.
pub fn command() -> Command {
    Command::new("plan")
        .about("Plan infrastructure changes (coming soon)")
        .arg(stack_dir())
        .arg(stack_env())
        .arg(log_level())
        .arg(env_file())
        .arg(env_var())
        .arg(dry_run())
        .arg(show_queries())
        .arg(on_failure())
}

/// Executes the `plan` command.
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
        "Planning changes for stack: [{}] in environment: [{}]",
        stack_dir, stack_env
    ), crate::utils::display::BorderColor::Yellow);

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

    println!("📐 plan complete (dry run: {})", dry_run);
}
