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

use clap::{Arg, ArgMatches, Command};

use crate::utils::display::print_unicode_box;

/// Configures the `teardown` command for the CLI application.
pub fn command() -> Command {
    Command::new("teardown")
        .about("Teardown a provisioned stack")
        .arg(Arg::new("stack_dir").required(true))
        .arg(Arg::new("stack_env").required(true))
}

/// Executes the `teardown` command.
pub fn execute(matches: &ArgMatches) {
    let stack_dir = matches.get_one::<String>("stack_dir").unwrap();
    let stack_env = matches.get_one::<String>("stack_env").unwrap();
    print_unicode_box(&format!(
        "Tearing down stack: [{}] in environment: [{}]",
        stack_dir, stack_env
    ));
}
