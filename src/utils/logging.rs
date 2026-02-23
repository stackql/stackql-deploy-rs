// utils/logging.rs

use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;
use std::path::Path;

/// Colors for different log levels when printing to the terminal
struct LevelColors;

impl LevelColors {
    // ANSI color codes
    const RED: &'static str = "\x1B[31m";
    const YELLOW: &'static str = "\x1B[33m";
    const GREEN: &'static str = "\x1B[32m";
    const CYAN: &'static str = "\x1B[36m";
    const MAGENTA: &'static str = "\x1B[35m";
    const RESET: &'static str = "\x1B[0m";

    /// Get the color code for a given log level
    fn get_color(level: log::Level) -> &'static str {
        match level {
            log::Level::Error => Self::RED,
            log::Level::Warn => Self::YELLOW,
            log::Level::Info => Self::GREEN,
            log::Level::Debug => Self::CYAN,
            log::Level::Trace => Self::MAGENTA,
        }
    }
}

/// Initializes the logger with a specified log level.
///
/// Formats logs as follows:
/// - Standard: [timestamp LEVEL stackql_deploy] message
/// - Debug/Trace: [timestamp LEVEL file_name (line_num)] message
///
/// Log levels are color-coded in the terminal output.
pub fn initialize_logger(log_level: &str) {
    let level = match log_level.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    let mut builder = Builder::new();

    builder.format(|buf, record| {
        let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%SZ");
        let level_str = record.level();
        let color = LevelColors::get_color(level_str);
        let reset = LevelColors::RESET;

        if record.level() <= log::Level::Info {
            // For info, warn, error: [timestamp LEVEL stackql_deploy] message
            writeln!(
                buf,
                "[{} {}{}{} stackql_deploy] {}",
                timestamp,
                color,
                level_str,
                reset,
                record.args()
            )
        } else {
            // For debug, trace: [timestamp LEVEL file_name (line_num)] message
            let file = record.file().unwrap_or("<unknown>");
            let file_name = Path::new(file)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(file);

            writeln!(
                buf,
                "[{} {}{}{} {} ({})] {}",
                timestamp,
                color,
                level_str,
                reset,
                file_name,
                record.line().unwrap_or(0),
                record.args()
            )
        }
    });

    // Set the default log level
    builder.filter_level(level);

    // Initialize the logger
    builder.init();
}
