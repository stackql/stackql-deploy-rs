// commands/info.rs

//! # Info Command Module
//!
//! This module handles the `info` command, which displays detailed version and configuration information
//! about the StackQL Deploy application. It also lists installed providers and running servers.
//!
//! ## Features
//! - Displays version information for the StackQL Deploy CLI.
//! - Retrieves and displays StackQL binary version, SHA, platform, and binary path.
//! - Lists all running local StackQL servers by PID and port.
//! - Displays installed providers and their versions.
//! - Lists contributors if available via the `CONTRIBUTORS` environment variable.
//!
//! ## Example Usage
//! ```bash
//! ./stackql-deploy info
//! ```
//! This command will output various details about the application, library, providers, and contributors.

use std::process;

use clap::Command;
use colored::*;
use log::error;

use crate::utils::display::print_unicode_box;
use crate::utils::platform::get_platform;
use crate::utils::server::find_all_running_servers;
use crate::utils::stackql::{get_installed_providers, get_stackql_path, get_version};

/// Defines the `info` command for the CLI application.
pub fn command() -> Command {
    Command::new("info").about("Display version information")
}

/// Executes the `info` command.
pub fn execute() {
    print_unicode_box(
        "Getting program information...",
        crate::utils::display::BorderColor::Green,
    );

    // Get stackql version
    let version_info = match get_version() {
        Ok(info) => info,
        Err(e) => {
            error!("Failed to retrieve version info: {}", e);
            process::exit(1);
        }
    };

    // Get platform
    let platform = get_platform();

    // Get binary path
    let binary_path = match get_stackql_path() {
        Some(path) => path.to_string_lossy().to_string(),
        _none => "Not found".to_string(),
    };

    // Get all running StackQL servers
    let running_servers = find_all_running_servers();

    // Get installed providers
    let providers = get_installed_providers().unwrap_or_default();

    // Print information
    println!("{}", "stackql-deploy CLI".green().bold());
    println!("  Version: 0.1.0\n");

    println!("{}", "StackQL Library".green().bold());
    println!("  Version: {}", version_info.version);
    println!("  SHA: {}", version_info.sha);
    println!("  Platform: {:?}", platform);
    println!("  Binary Path: {}", binary_path);

    // Display running servers
    println!("\n{}", "Local StackQL Servers".green().bold());
    if running_servers.is_empty() {
        println!("  None");
    } else {
        for server in running_servers {
            println!("  PID: {}, Port: {}", server.pid, server.port);
        }
    }

    // Display installed providers
    println!("\n{}", "Installed Providers".green().bold());
    if providers.is_empty() {
        println!("  No providers installed");
    } else {
        for provider in providers {
            println!("  {} {}", provider.name.bold(), provider.version);
        }
    }

    // Display contributors from embedded contributors.csv
    let raw_contributors = include_str!("../../contributors.csv");
    let contributors: Vec<&str> = raw_contributors
        .lines()
        .filter(|s| !s.trim().is_empty())
        .collect();

    if !contributors.is_empty() {
        println!("\n{}", "Special Thanks to our Contributors".green().bold());

        for chunk in contributors.chunks(4) {
            println!("  {}", chunk.join(", "));
        }
    }
}
