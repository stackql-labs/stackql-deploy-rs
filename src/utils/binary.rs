// utils/binary.rs

//! # Binary Utility Module
//!
//! This module provides utility functions for locating and verifying the `stackql` binary.
//! It supports checking the binary's presence in the system `PATH` or the current directory
//! and retrieving the full path to the binary if it exists.
//!
//! ## Features
//! - Checks if the `stackql` binary is available in the system's `PATH`.
//! - Retrieves the full path of the `stackql` binary from the current directory or `PATH`.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::binary::{binary_exists_in_path, get_binary_path};
//!
//! if binary_exists_in_path() {
//!     if let Some(path) = get_binary_path() {
//!         println!("Found stackql binary at: {:?}", path);
//!     }
//! }
//! ```

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Check if the stackql binary exists in PATH
pub fn binary_exists_in_path() -> bool {
    let binary_name = super::platform::get_binary_name();
    let status = if super::platform::get_platform() == super::platform::Platform::Windows {
        Command::new("where").arg(&binary_name).status()
    } else {
        Command::new("which").arg(&binary_name).status()
    };

    status.map(|s| s.success()).unwrap_or(false)
}

/// Get the full path to the stackql binary
pub fn get_binary_path() -> Option<PathBuf> {
    let binary_name = super::platform::get_binary_name();

    // First check current directory
    if let Ok(current_dir) = env::current_dir() {
        let binary_path = current_dir.join(&binary_name);
        if binary_path.exists() && binary_path.is_file() {
            return Some(binary_path);
        }
    }

    // Then check PATH
    if binary_exists_in_path() {
        if let Ok(paths) = env::var("PATH") {
            for path in env::split_paths(&paths) {
                let full_path = path.join(&binary_name);
                if full_path.exists() && full_path.is_file() {
                    return Some(full_path);
                }
            }
        }
    }

    None
}
