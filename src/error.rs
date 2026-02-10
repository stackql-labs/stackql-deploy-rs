// error.rs

//! # Error Handling Module
//!
//! This module provides custom error handling for the StackQL Deploy application.
//! It defines a comprehensive `AppError` enum that encapsulates various error conditions
//! the application may encounter. Implementations of standard traits like `Display` and `Error`
//! are provided to allow seamless integration with Rust's error handling ecosystem.
//!
//! # Usage Example
//! ```rust
//! use crate::error::AppError;
//!
//! fn example_function() -> Result<(), AppError> {
//!     Err(AppError::BinaryNotFound)
//! }
//! ```

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

// ============================
// Application Error Definitions
// ============================

/// Represents errors that may occur within the application.
///
/// This enum provides a common error type that encapsulates various issues such as:
/// - Missing binary files
/// - Failed command execution
/// - I/O errors
#[derive(Debug)]
pub enum AppError {
    /// Error returned when the stackql binary is not found.
    BinaryNotFound,

    /// Error returned when a command fails to execute.
    ///
    /// The error message is stored as a `String` for detailed reporting.
    CommandFailed(String),

    /// Wrapper for standard I/O errors.
    ///
    /// This variant allows propagating errors originating from `std::io` operations.
    IoError(std::io::Error),
}

// ============================
// Display Trait Implementation
// ============================

impl fmt::Display for AppError {
    /// Formats the `AppError` for user-friendly output.
    ///
    /// This implementation converts each variant into a descriptive error message.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BinaryNotFound => write!(f, "The stackql binary was not found"),
            Self::CommandFailed(msg) => write!(f, "Command failed: {}", msg),
            Self::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

// ============================
// Error Trait Implementation
// ============================

impl Error for AppError {}

// ============================
// Conversion From std::io::Error
// ============================

impl From<std::io::Error> for AppError {
    /// Converts a standard I/O error into an `AppError::IoError`.
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

// ============================
// Utility Functions
// ============================

/// Attempts to retrieve the binary path, returning an `AppError` if not found.
///
/// This function calls `get_binary_path()` from the `utils::binary` module and converts
/// an `Option<PathBuf>` to a `Result<PathBuf, AppError>`.
///
/// # Errors
/// - Returns `AppError::BinaryNotFound` if the binary path cannot be located.
///
/// # Example
/// ```rust
/// use crate::error::{get_binary_path_with_error, AppError};
///
/// match get_binary_path_with_error() {
///     Ok(path) => println!("Binary found at: {:?}", path),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn get_binary_path_with_error() -> Result<PathBuf, AppError> {
    crate::utils::binary::get_binary_path().ok_or(AppError::BinaryNotFound)
}
