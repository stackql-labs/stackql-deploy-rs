// utils/logging.rs

use env_logger;
use log::LevelFilter;

/// Sets the logger level based on the provided argument.
pub fn initialize_logger(log_level: &str) {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info, // Default to Info if unrecognized
    };

    env_logger::Builder::new().filter(None, level).init();
}
