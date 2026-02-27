// commands/common_args.rs

//! # Common Command Arguments
//!
//! This module defines common command-line arguments that can be reused across
//! different commands in the application.

use clap::{value_parser, Arg, ArgAction};
use std::str::FromStr;

/// Possible actions to take on failure
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FailureAction {
    Rollback,
    Ignore,
    Error,
}

impl FromStr for FailureAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rollback" => Ok(FailureAction::Rollback),
            "ignore" => Ok(FailureAction::Ignore),
            "error" => Ok(FailureAction::Error),
            _ => Err(format!("Unknown failure action: {}", s)),
        }
    }
}

// Positional arguments
/// Common positional argument for the stack directory
pub fn stack_dir() -> Arg {
    Arg::new("stack_dir")
        .required(true)
        .help("Path to the stack directory containing resources")
}

/// Common positional argument for the stack environment
pub fn stack_env() -> Arg {
    Arg::new("stack_env")
        .required(true)
        .help("Environment to deploy to (e.g., `prod`, `dev`, `test`)")
}

// Optional arguments
/// Common argument for setting the log level
pub fn log_level() -> Arg {
    Arg::new("log-level")
        .long("log-level")
        .help("Set the logging level")
        .default_value("info")
        .value_parser(clap::builder::PossibleValuesParser::new([
            "trace", "debug", "info", "warn", "error",
        ]))
        .ignore_case(true)
}

/// Common argument for specifying an environment file
pub fn env_file() -> Arg {
    Arg::new("env-file")
        .long("env-file")
        .help("Environment variables file")
        .default_value(".env")
}

/// Common argument for setting additional environment variables
pub fn env_var() -> Arg {
    Arg::new("env")
        .short('e')
        .long("env")
        .help("Set additional environment variables (format: KEY=VALUE)")
        .action(ArgAction::Append)
}

/// Common argument for performing a dry run
pub fn dry_run() -> Arg {
    Arg::new("dry-run")
        .long("dry-run")
        .help("Perform a dry run of the operation")
        .action(ArgAction::SetTrue)
}

/// Common argument for showing queries in the output logs
pub fn show_queries() -> Arg {
    Arg::new("show-queries")
        .long("show-queries")
        .help("Show queries run in the output logs")
        .action(ArgAction::SetTrue)
}

/// Common argument for specifying the action on failure
pub fn on_failure() -> Arg {
    Arg::new("on-failure")
        .long("on-failure")
        .help("Action to take on failure")
        .value_parser(value_parser!(FailureAction))
        .default_value("error")
}

