// utils/server.rs

//! # Server Utility Module
//!
//! This module provides utilities for starting, stopping, and managing StackQL server instances.
//! It supports detecting running servers, extracting process information, and managing server lifecycles
//! with functionalities to start, stop, and check server status across multiple platforms (Windows, Linux, macOS).
//!
//! ## Features
//! - Start a StackQL server on a specified host and port.
//! - Check if a server is running.
//! - Retrieve running servers by scanning processes.
//! - Stop a server by process ID (PID).
//! - Automatically detect and manage servers running on local or remote hosts.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::server::{check_and_start_server, start_server, stop_server, StartServerOptions};
//!
//! let options = StartServerOptions {
//!     host: "localhost".to_string(),
//!     port: 5444,
//!     ..Default::default()
//! };
//!
//! match start_server(&options) {
//!     Ok(pid) => println!("Server started with PID: {}", pid),
//!     Err(e) => eprintln!("Failed to start server: {}", e),
//! }
//! ```

use std::fs::OpenOptions;
use std::path::Path;
use std::process;
use std::process::{Command as ProcessCommand, Stdio};
use std::thread;
use std::time::Duration;

use log::{debug, error, info, warn};

use crate::app::{DEFAULT_LOG_FILE, LOCAL_SERVER_ADDRESSES};
use crate::globals::{server_host, server_port};
use crate::utils::binary::get_binary_path;

/// Options for starting a StackQL server
pub struct StartServerOptions {
    pub host: String,
    pub port: u16,
    pub registry: Option<String>,
    pub mtls_config: Option<String>,
    pub custom_auth_config: Option<String>,
    pub log_level: Option<String>,
}

impl Default for StartServerOptions {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: crate::app::DEFAULT_SERVER_PORT,
            registry: None,
            mtls_config: None,
            custom_auth_config: None,
            log_level: None,
        }
    }
}

/// Represents a running StackQL server process
pub struct RunningServer {
    pub pid: u32,
    pub port: u16,
}

/// Check if the stackql server is running on a specific port
pub fn is_server_running(port: u16) -> bool {
    let servers = find_all_running_servers();
    debug!(
        "is_server_running({}): found {} candidate server(s): {:?}",
        port,
        servers.len(),
        servers
            .iter()
            .map(|s| format!("pid={} port={}", s.pid, s.port))
            .collect::<Vec<_>>()
    );
    let result = servers.iter().any(|server| server.port == port);
    debug!("is_server_running({}) -> {}", port, result);
    result
}

/// Find all stackql servers that are running and their ports
pub fn find_all_running_servers() -> Vec<RunningServer> {
    let mut running_servers = Vec::new();

    if cfg!(target_os = "windows") {
        // Use PowerShell Get-CimInstance to get stackql processes with command lines
        let output = ProcessCommand::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process -Filter \"Name='stackql.exe'\" | ForEach-Object { \"PID=$($_.ProcessId) CMD=$($_.CommandLine)\" }",
            ])
            .output();

        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("PID=") {
                    if let Some(space_pos) = rest.find(" CMD=") {
                        let pid_str = &rest[..space_pos];
                        let cmdline = &rest[space_pos + 5..];
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            if let Some(port) = extract_port_from_cmdline(cmdline) {
                                debug!(
                                    "find_all_running_servers (Windows): PID {} -> port {}",
                                    pid, port
                                );
                                running_servers.push(RunningServer { pid, port });
                            }
                        }
                    }
                }
            }
        } else {
            debug!("find_all_running_servers: PowerShell command failed");
        }
    } else {
        let output = ProcessCommand::new("pgrep")
            .arg("-f")
            .arg("stackql srv")
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute pgrep"));

        if !output.stdout.is_empty() {
            let pids_str = String::from_utf8_lossy(&output.stdout).to_string();
            let pids = pids_str.trim().split('\n').collect::<Vec<&str>>();
            debug!(
                "find_all_running_servers: pgrep found {} PID(s): {:?}",
                pids.len(),
                pids
            );

            for pid_str in pids {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    // Log the full command line of this PID before attempting port extraction
                    if let Ok(ps_out) = ProcessCommand::new("ps")
                        .arg("-p")
                        .arg(pid.to_string())
                        .arg("-o")
                        .arg("args")
                        .output()
                    {
                        let cmdline = String::from_utf8_lossy(&ps_out.stdout);
                        debug!(
                            "find_all_running_servers: PID {} cmdline: {}",
                            pid,
                            cmdline.trim()
                        );
                    }
                    if let Some(port) = extract_port_from_ps(pid_str) {
                        debug!("find_all_running_servers: PID {} -> port {}", pid, port);
                        running_servers.push(RunningServer { pid, port });
                    } else {
                        debug!(
                            "find_all_running_servers: PID {} -> no --pgsrv.port found, skipping",
                            pid
                        );
                    }
                }
            }
        } else {
            debug!("find_all_running_servers: pgrep returned no matching PIDs");
        }
    }

    running_servers
}

/// Extract port from process information on Unix-like systems using `ps`
fn extract_port_from_ps(pid: &str) -> Option<u16> {
    let ps_output = ProcessCommand::new("ps")
        .arg("-p")
        .arg(pid)
        .arg("-o")
        .arg("args")
        .output()
        .ok()?;

    let ps_str = String::from_utf8_lossy(&ps_output.stdout);

    let patterns = [
        "--pgsrv.port=",
        "--pgsrv.port ",
        "pgsrv.port=",
        "pgsrv.port ",
    ];
    for pattern in patterns.iter() {
        if let Some(start_index) = ps_str.find(pattern) {
            let port_start = start_index + pattern.len();
            let port_end = ps_str[port_start..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();

            if let Ok(port) = port_end.parse::<u16>() {
                return Some(port);
            }
        }
    }

    None
}

/// Extract port from a command line string by looking for --pgsrv.port argument
fn extract_port_from_cmdline(cmdline: &str) -> Option<u16> {
    // Try --pgsrv.port=PORT format
    if let Some(pos) = cmdline.find("--pgsrv.port=") {
        let after = &cmdline[pos + "--pgsrv.port=".len()..];
        let port_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(port) = port_str.parse::<u16>() {
            return Some(port);
        }
    }
    // Try --pgsrv.port PORT format
    if let Some(pos) = cmdline.find("--pgsrv.port") {
        let after = &cmdline[pos + "--pgsrv.port".len()..];
        let trimmed = after.trim_start();
        let port_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(port) = port_str.parse::<u16>() {
            return Some(port);
        }
    }
    None
}

/// Get the PID of the running stackql server on a specific port
pub fn get_server_pid(port: u16) -> Option<u32> {
    // Use find_all_running_servers which handles platform differences
    let servers = find_all_running_servers();
    servers.iter().find(|s| s.port == port).map(|s| s.pid)
}

/// Start the stackql server with the given options
pub fn start_server(options: &StartServerOptions) -> Result<u32, String> {
    debug!(
        "start_server called: host={}, port={}",
        options.host, options.port
    );

    let binary_path = match get_binary_path() {
        Some(path) => {
            debug!("Using stackql binary at: {:?}", path);
            path
        }
        _none => return Err("stackql binary not found".to_string()),
    };

    debug!(
        "Checking if server is already running on port {}...",
        options.port
    );
    if is_server_running(options.port) {
        info!("Server is already running on port {}", options.port);
        return Ok(get_server_pid(options.port).unwrap_or(0));
    }
    debug!(
        "Server not running on port {}; proceeding to start.",
        options.port
    );

    let mut cmd = ProcessCommand::new(&binary_path);
    cmd.arg("srv");
    cmd.arg("--pgsrv.address").arg(&options.host);
    cmd.arg("--pgsrv.port").arg(options.port.to_string());

    cmd.arg("--pgsrv.debug.enable=true");
    cmd.arg("--pgsrv.loglevel=DEBUG");

    if let Some(registry) = &options.registry {
        cmd.arg("--registry").arg(registry);
    }

    if let Some(mtls_config) = &options.mtls_config {
        cmd.arg("--mtls-config").arg(mtls_config);
    }

    if let Some(custom_auth) = &options.custom_auth_config {
        cmd.arg("--custom-auth-config").arg(custom_auth);
    }

    if let Some(log_level) = &options.log_level {
        cmd.arg("--log-level").arg(log_level);
    }

    let log_path = Path::new(DEFAULT_LOG_FILE);
    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        // .append(true)
        .open(log_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    debug!("Spawning stackql server process (log -> {:?})...", log_path);
    let child = cmd
        .stdout(Stdio::from(log_file.try_clone().unwrap()))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("Failed to start server: {}", e))?;

    let pid = child.id();
    info!("Starting stackql server with PID: {}", pid);
    debug!(
        "Waiting 5 seconds for server on port {} to become ready...",
        options.port
    );
    thread::sleep(Duration::from_secs(5));

    debug!(
        "Re-checking if server is running on port {}...",
        options.port
    );
    if is_server_running(options.port) {
        info!("Server started successfully on port {}", options.port);
        Ok(pid)
    } else {
        Err("Server failed to start properly".to_string())
    }
}

/// Stop the stackql server
pub fn stop_server(port: u16) -> Result<(), String> {
    if !is_server_running(port) {
        warn!("No server running on port {}", port);
        return Ok(());
    }

    let pid = match get_server_pid(port) {
        Some(pid) => pid,
        _none => return Err("Could not determine server PID".to_string()),
    };

    info!("Stopping stackql server with PID: {}", pid);

    if cfg!(target_os = "windows") {
        ProcessCommand::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .arg(pid.to_string())
            .output()
            .map_err(|e| format!("Failed to stop server: {}", e))?;
    } else {
        ProcessCommand::new("kill")
            .arg(pid.to_string())
            .output()
            .map_err(|e| format!("Failed to stop server: {}", e))?;
    }

    Ok(())
}

/// Checks if the server is running and starts it if necessary.
///
/// This function checks if the server is local and needs to be started. If the server is not running,
/// it attempts to start it with the specified host and port.
///
/// # Arguments
///
/// * `host` - A reference to the server host address.
/// * `port` - The port number to check.
///
/// # Behavior
///
/// * If the server is already running locally, it will display a message indicating this.
/// * If a remote server is specified, it will display a message indicating the remote connection.
/// * If the server needs to be started, it will attempt to do so and indicate success or failure.
pub fn check_and_start_server() {
    let host = server_host();
    let port = server_port();

    debug!("check_and_start_server: host={}, port={}", host, port);

    if LOCAL_SERVER_ADDRESSES.contains(&host) {
        debug!(
            "Host '{}' is local; checking if server is running on port {}...",
            host, port
        );
        // Always stop any existing server to ensure a clean session
        // with the current environment (auth creds, provider versions, etc.)
        if is_server_running(port) {
            info!(
                "Stopping existing server on port {} for clean session.",
                port
            );
            if let Err(e) = stop_server(port) {
                warn!("Failed to stop existing server: {}", e);
            }
            // Brief pause to allow the port to be released
            thread::sleep(Duration::from_secs(1));
        }

        info!("Starting server...");
        let options = StartServerOptions {
            host: host.to_string(),
            port,
            ..Default::default()
        };

        if let Err(e) = start_server(&options) {
            error!("Failed to start server: {}", e);
            process::exit(1);
        }
    } else {
        debug!("Host '{}' is remote; skipping local server start.", host);
        info!("Using remote server {}:{}", host, port);
    }
}

/// Stops the local server after an operation completes.
/// Called at the end of build, test, and teardown to ensure
/// the server doesn't linger with stale auth context.
pub fn stop_local_server() {
    let host = server_host();
    let port = server_port();

    if LOCAL_SERVER_ADDRESSES.contains(&host) && is_server_running(port) {
        debug!("Stopping local server after operation.");
        if let Err(e) = stop_server(port) {
            warn!("Failed to stop server after operation: {}", e);
        }
    }
}
