// utils/platform.rs

//! # Platform Utility Module
//!
//! This module provides utilities for detecting the operating system platform
//! and retrieving the appropriate binary name for the `stackql` application.
//!
//! ## Features
//! - Detects the current operating system (Windows, macOS, Linux).
//! - Returns the platform-specific `stackql` binary name.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::platform::{get_platform, get_binary_name, Platform};
//!
//! let platform = get_platform();
//! let binary_name = get_binary_name();
//!
//! println!("Platform: {:?}", platform);
//! println!("Binary Name: {}", binary_name);
//! ```

use crate::app::STACKQL_BINARY_NAME;

/// Enum representing supported platforms.
#[derive(Debug, PartialEq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Unknown,
}

/// Determine the current operating system
pub fn get_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else if cfg!(target_os = "linux") {
        Platform::Linux
    } else {
        Platform::Unknown
    }
}

/// Get the appropriate binary name based on platform
pub fn get_binary_name() -> String {
    STACKQL_BINARY_NAME.to_string()
}
