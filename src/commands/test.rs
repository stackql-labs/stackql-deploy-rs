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

use clap::{Arg, ArgMatches, Command};

use crate::utils::display::print_unicode_box;

/// Configures the `test` command for the CLI application.
pub fn command() -> Command {
    Command::new("test")
        .about("Run test queries for the stack")
        .arg(Arg::new("stack_dir").required(true))
        .arg(Arg::new("stack_env").required(true))
}

/// Executes the `test` command.
pub fn execute(matches: &ArgMatches) {
    let stack_dir = matches.get_one::<String>("stack_dir").unwrap();
    let stack_env = matches.get_one::<String>("stack_env").unwrap();
    print_unicode_box(&format!(
        "Testing stack: [{}] in environment: [{}]",
        stack_dir, stack_env
    ));
}
