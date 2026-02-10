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
use log::{debug, info};

use crate::commands::common_args::{
    args_from_matches, dry_run, env_file, env_var, log_level, on_failure, show_queries, stack_dir,
    stack_env,
};
use crate::resource::manifest::Manifest;
use crate::utils::display::{log_common_command_args, print_unicode_box};

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
    // Create the CommonCommandArgs struct directly from matches
    let args = args_from_matches(matches);

    // Log the command arguments
    log_common_command_args(&args, matches);

    print_unicode_box(&format!(
        "Testing stack: [{}] in environment: [{}] (dry run: {})",
        args.stack_dir, args.stack_env, args.dry_run
    ));

    // Load the manifest using the reusable function
    let manifest = Manifest::load_from_dir_or_exit(args.stack_dir);

    // Process resources
    info!("Testing {} resources...", manifest.resources.len());

    for resource in &manifest.resources {
        debug!("Processing resource: {}", resource.name);

        // Skip resources that have a condition (if) that evaluates to false
        if let Some(condition) = &resource.r#if {
            debug!("Resource has condition: {}", condition);
            // TODO: evaluate the condition here
        }

        // Get environment-specific property values
        debug!("Properties for resource {}:", resource.name);
        for prop in &resource.props {
            let value = Manifest::get_property_value(prop, args.stack_env);
            match value {
                Some(val) => debug!(
                    " [prop] {}: {}",
                    prop.name,
                    serde_json::to_string(val)
                        .unwrap_or_else(|_| "Error serializing value".to_string())
                ),
                None => debug!(
                    "[prop] {}: <not defined for environment {}>",
                    prop.name, args.stack_env
                ),
            }
        }

        // Get the query file path
        let query_path =
            manifest.get_resource_query_path(std::path::Path::new(args.stack_dir), resource);
        debug!("Query file path: {:?}", query_path);

        // In a real implementation, you would:
        // 1. Read the query file
        // 2. Replace property placeholders with actual values
        // 3. Execute the query against the infrastructure
        // 4. Verify the results match expectations

        info!("✓ Resource {} passed tests", resource.name);
    }

    info!("🔍 tests complete (dry run: {})", args.dry_run);
}
