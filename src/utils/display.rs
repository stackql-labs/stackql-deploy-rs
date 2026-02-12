// utils/display.rs

//! # Display Utility Module
//!
//! This module provides utility functions for rendering messages with various styles
//! including Unicode-styled message boxes and color-coded output for errors, success messages, and informational messages.
//! It leverages the `colored` crate for styling and `unicode_width` crate for handling Unicode text width.

use log::debug;
use unicode_width::UnicodeWidthStr;

use crate::commands::common_args::CommonCommandArgs;
use clap::ArgMatches;

/// Border color options for Unicode boxes, matching Python's BorderColor enum.
#[derive(Debug, Clone, Copy)]
pub enum BorderColor {
    Yellow,
    Blue,
    Green,
    Red,
    Cyan,
}

impl BorderColor {
    fn ansi_code(&self) -> &str {
        match self {
            BorderColor::Yellow => "\x1b[93m",
            BorderColor::Blue => "\x1b[94m",
            BorderColor::Green => "\x1b[92m",
            BorderColor::Red => "\x1b[91m",
            BorderColor::Cyan => "\x1b[96m",
        }
    }
}

/// Utility function to print a Unicode-styled message box
/// that correctly handles the width of emojis and other wide characters.
pub fn print_unicode_box(message: &str, color: BorderColor) {
    let border_color = color.ansi_code();
    let reset_color = "\x1b[0m";
    let lines: Vec<&str> = message.split('\n').collect();

    // Calculate width using unicode_width to properly account for emojis
    let max_length = lines
        .iter()
        .map(|line| UnicodeWidthStr::width(*line))
        .max()
        .unwrap_or(0);

    let top_border = format!(
        "{}┌{}┐{}",
        border_color,
        "─".repeat(max_length + 2),
        reset_color
    );
    let bottom_border = format!(
        "{}└{}┘{}",
        border_color,
        "─".repeat(max_length + 2),
        reset_color
    );

    println!("{}", top_border);
    for line in lines {
        // Calculate proper padding based on the visual width
        let padding = max_length - UnicodeWidthStr::width(line);
        let padded_line = format!("│ {}{} │", line, " ".repeat(padding));
        println!("{}{}{}", border_color, padded_line, reset_color);
    }
    println!("{}", bottom_border);
}

#[macro_export]
macro_rules! print_info {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        println!("{}", format!($($arg)*).blue())
    }};
}

#[macro_export]
macro_rules! print_error {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        eprintln!("{}", format!($($arg)*).red())
    }};
}

#[macro_export]
macro_rules! print_success {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        println!("{}", format!($($arg)*).green())
    }};
}

/// Log common command arguments at debug level
#[allow(dead_code)]
pub fn log_common_command_args(args: &CommonCommandArgs, matches: &ArgMatches) {
    debug!("Stack Directory: {}", args.stack_dir);
    debug!("Stack Environment: {}", args.stack_env);
    debug!("Log Level: {}", args.log_level);
    debug!("Environment File: {}", args.env_file);

    // Log environment variables if present
    if let Some(vars) = matches.get_many::<String>("env") {
        debug!("Environment Variables:");
        for var in vars {
            debug!("  - {}", var);
        }
    }

    debug!("Dry Run: {}", args.dry_run);
    debug!("Show Queries: {}", args.show_queries);
    debug!("On Failure: {:?}", args.on_failure);
}
