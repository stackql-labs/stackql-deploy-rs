// lib/env.rs

//! # Environment Variable Handling
//!
//! Loads environment variables from .env files and CLI overrides,
//! matching the Python `load_env_vars` and `parse_env_var` functions.

use std::collections::HashMap;
use std::path::Path;

use log::debug;

/// Load environment variables from a .env file and apply CLI overrides.
///
/// # Arguments
/// * `env_file` - Path to the .env file (relative to cwd)
/// * `overrides` - Additional KEY=VALUE pairs from `-e` CLI flags
pub fn load_env_vars(env_file: &str, overrides: &[String]) -> HashMap<String, String> {
    let mut env_vars = HashMap::new();

    // Load from .env file
    let dotenv_path = Path::new(env_file);
    if dotenv_path.exists() {
        debug!("Loading environment variables from: {}", env_file);
        match dotenvy::from_path_iter(dotenv_path) {
            Ok(iter) => {
                for (key, value) in iter.flatten() {
                    debug!("  Loaded env var: {}", key);
                    env_vars.insert(key, value);
                }
            }
            Err(e) => {
                debug!("Warning: could not load .env file: {}", e);
            }
        }
    } else {
        debug!("No .env file found at: {}", env_file);
    }

    // Apply overrides from -e flags
    for override_str in overrides {
        if let Some((key, value)) = parse_env_var(override_str) {
            debug!("  Override env var: {}", key);
            env_vars.insert(key, value);
        }
    }

    env_vars
}

/// Parse a single KEY=VALUE environment variable string.
fn parse_env_var(s: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}
