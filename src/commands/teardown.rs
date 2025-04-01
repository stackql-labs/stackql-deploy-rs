// commands/teardown.rs

//! # Teardown Command Module
//!
//! This module provides the `teardown` command for the StackQL Deploy application.
//! The `teardown` command deprovisions resources for a given stack in a specified environment.
//! It accepts the same arguments as the `build` and `plan` commands and is intended to
//! reverse all operations performed during provisioning.
//!
//! ## Features
//! - Deprovisioning of a specified stack in a given environment.
//! - Uses a declarative approach to identify resources that should be destroyed.
//! - Intended to be used as a cleanup or rollback mechanism.
//!
//! ## Example Usage
//! ```bash
//! ./stackql-deploy teardown /path/to/stack dev
//! ```

use clap::{ArgMatches, Command};

use crate::commands::common_args::{
    dry_run, env_file, env_var, log_level, on_failure, show_queries, stack_dir, stack_env,
    FailureAction,
};
use crate::utils::display::print_unicode_box;

/// Configures the `teardown` command for the CLI application.
pub fn command() -> Command {
    Command::new("teardown")
        .about("Teardown a provisioned stack")
        .arg(stack_dir())
        .arg(stack_env())
        .arg(log_level())
        .arg(env_file())
        .arg(env_var())
        .arg(dry_run())
        .arg(show_queries())
        .arg(on_failure())
}

/// Executes the `teardown` command.
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
        "Tearing down stack: [{}] in environment: [{}]",
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

    // Here you would implement the actual teardown functionality

    println!("🚧 teardown complete (dry run: {})", dry_run);
}
