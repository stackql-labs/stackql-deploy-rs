// utils/stackql.rs

//! # StackQL Utility Module
//!
//! This module provides functionalities for interacting with the `stackql` binary,
//! such as retrieving version information, installed providers, and the binary path.
//! It serves as a bridge between your Rust application and the StackQL CLI tool.
//!
//! ## Features
//! - Retrieve `stackql` binary version and SHA information.
//! - List installed StackQL providers.
//! - Get the path to the `stackql` binary.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::stackql::{get_version, get_installed_providers, get_stackql_path};
//!
//! if let Ok(version_info) = get_version() {
//!     println!("StackQL Version: {}, SHA: {}", version_info.version, version_info.sha);
//! }
//!
//! if let Ok(providers) = get_installed_providers() {
//!     for provider in providers {
//!         println!("Provider: {}, Version: {}", provider.name, provider.version);
//!     }
//! }
//!
//! if let Some(path) = get_stackql_path() {
//!     println!("StackQL Binary Path: {:?}", path);
//! }
//! ```

use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use crate::utils::binary::get_binary_path;

/// Holds version information retrieved from the `stackql` binary.
pub struct VersionInfo {
    pub version: String,
    pub sha: String,
}

/// Represents a provider installed in the `stackql` environment.
pub struct Provider {
    pub name: String,
    pub version: String,
}

/// Retrieves the version and SHA information of the `stackql` binary.
pub fn get_version() -> Result<VersionInfo, String> {
    let binary_path = match get_binary_path() {
        Some(path) => path,
        _none => return Err("StackQL binary not found".to_string()),
    };

    let output = match ProcessCommand::new(&binary_path).arg("--version").output() {
        Ok(output) => output,
        Err(e) => return Err(format!("Failed to execute stackql: {}", e)),
    };

    if !output.status.success() {
        return Err("Failed to get version information".to_string());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let version_line = match output_str.lines().next() {
        Some(line) => line,
        _none => return Err("Empty version output".to_string()),
    };

    let tokens: Vec<&str> = version_line.split_whitespace().collect();
    if tokens.len() < 4 {
        return Err("Unexpected version format".to_string());
    }

    let version = tokens[1].to_string();
    let sha = tokens[3].replace("(", "").replace(")", "");

    Ok(VersionInfo { version, sha })
}

/// Retrieves a list of installed StackQL providers.
pub fn get_installed_providers() -> Result<Vec<Provider>, String> {
    let binary_path = match get_binary_path() {
        Some(path) => path,
        _none => return Err("StackQL binary not found".to_string()),
    };

    let output = match ProcessCommand::new(&binary_path)
        .arg("exec")
        .arg("SHOW PROVIDERS")
        .output()
    {
        Ok(output) => output,
        Err(e) => return Err(format!("Failed to execute stackql: {}", e)),
    };

    if !output.status.success() {
        return Err("Failed to get providers information".to_string());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut providers = Vec::new();

    for line in output_str.lines() {
        if line.contains("name") || line.contains("----") {
            continue;
        }

        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() >= 3 {
            let name = fields[1].trim().to_string();
            let version = fields[2].trim().to_string();
            if !name.is_empty() && name != "name" && !name.contains("----") {
                providers.push(Provider { name, version });
            }
        }
    }

    Ok(providers)
}

/// Retrieves the path to the `stackql` binary.
pub fn get_stackql_path() -> Option<PathBuf> {
    get_binary_path()
}
