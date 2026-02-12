// commands/upgrade.rs

//! # Upgrade Command Module
//!
//! This module provides the `upgrade` command for the StackQL Deploy application.
//! The `upgrade` command downloads and installs the latest version of the StackQL binary.
//! It verifies the version of the newly installed binary to ensure the upgrade was successful.
//!
//! ## Features
//! - Automatically fetches the latest version of the StackQL binary from the official repository.
//! - Verifies the version after installation.
//! - Provides user feedback on successful or failed upgrades.
//!
//! ## Example Usage
//! ```bash
//! ./stackql-deploy upgrade
//! ```

use std::process;

use clap::Command;
use colored::*;
use log::{error, info};

use crate::utils::display::print_unicode_box;
use crate::utils::download::download_binary;
use crate::utils::stackql::get_version;

/// Configures the `upgrade` command for the CLI application.
pub fn command() -> Command {
    Command::new("upgrade").about("Upgrade stackql to the latest version")
}

/// Executes the `upgrade` command.
pub fn execute() {
    print_unicode_box("Installing or upgrading stackql...", crate::utils::display::BorderColor::Yellow);

    // Download the latest version of stackql binary
    match download_binary() {
        Ok(path) => {
            // Get the version of the newly installed binary
            match get_version() {
                Ok(version_info) => {
                    info!(
                        "Successfully installed the latest stackql binary, version ({}) at: {}",
                        version_info.version,
                        path.display().to_string().green()
                    );
                }
                Err(e) => {
                    error!("Failed to get stackql version: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Error upgrading stackql binary: {}", e);
            process::exit(1);
        }
    }
}
