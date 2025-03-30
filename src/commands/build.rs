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

use clap::{Arg, ArgMatches, Command};

use crate::utils::display::print_unicode_box;

/// Defines the `build` command for the CLI application.
pub fn command() -> Command {
    Command::new("build")
        .about("Create or update resources")
        .arg(
            Arg::new("stack_dir")
                .required(true)
                .help("Path to the stack directory containing resources"),
        )
        .arg(
            Arg::new("stack_env")
                .required(true)
                .help("Environment to deploy to (e.g., `prod`, `dev`, `test`)"),
        )
}

/// Executes the `build` command.
pub fn execute(matches: &ArgMatches) {
    let stack_dir = matches.get_one::<String>("stack_dir").unwrap();
    let stack_env = matches.get_one::<String>("stack_env").unwrap();

    print_unicode_box(&format!(
        "Deploying stack: [{}] to environment: [{}]",
        stack_dir, stack_env
    ));
}
