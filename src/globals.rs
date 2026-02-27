// globals.rs

//! # Global Configuration Module
//!
//! This module provides global variables for the StackQL server configuration.
//! It manages the global host, port, and connection string settings using `OnceCell` for safe, single initialization.
//!
//! ## Features
//! - Stores global server configuration values (`host`, `port`, `connection_string`) using `OnceCell`.
//! - Provides initialization functions to set global values (`init_globals`).
//! - Exposes getter functions for retrieving configured global values from other modules.
//!
//! ## Example Usage
//! ```rust
//! use crate::globals::{init_globals, server_host, server_port, connection_string};
//!
//! fn setup() {
//!     init_globals("localhost".to_string(), 5444);
//!     println!("Host: {}", server_host());
//!     println!("Port: {}", server_port());
//!     println!("Connection String: {}", connection_string());
//! }
//! ```

use once_cell::sync::OnceCell;

use crate::app::{DEFAULT_SERVER_HOST, DEFAULT_SERVER_PORT};

// ============================
// Global Static Variables
// ============================

/// Stores the global server host.
///
/// The server host is initialized via the `init_globals` function and is only set once per application lifetime.
static STACKQL_SERVER_HOST: OnceCell<String> = OnceCell::new();

/// Stores the global server port.
///
/// The server port is initialized via the `init_globals` function and is only set once per application lifetime.
static STACKQL_SERVER_PORT: OnceCell<u16> = OnceCell::new();

/// Stores the global connection string used for database connections.
///
/// This string is generated using the `init_globals` function based on the provided host and port.
static STACKQL_CONNECTION_STRING: OnceCell<String> = OnceCell::new();

// ============================
// Initialization Function
// ============================

/// Initializes the global variables for host, port, and connection string.
///
/// This function must be called once before accessing global values via getter functions.
/// It uses `OnceCell` to ensure each value is only initialized once.
///
/// # Arguments
/// - `host` - The server host address as a `String`.
/// - `port` - The server port as a `u16`.
///
/// # Example
/// ```rust
/// use crate::globals::init_globals;
/// init_globals("localhost".to_string(), 5444);
/// ```
pub fn init_globals(host: String, port: u16) {
    // Only set if not already set (first initialization wins)
    STACKQL_SERVER_HOST.set(host.clone()).ok();
    STACKQL_SERVER_PORT.set(port).ok();

    // Create a connection string and store it globally
    let connection_string = format!(
        "host={} port={} user=stackql dbname=stackql application_name=stackql",
        host, port
    );
    STACKQL_CONNECTION_STRING.set(connection_string).ok();
}

// ============================
// Getter Functions
// ============================

/// Retrieves the configured global server host.
///
/// If the host is not set via `init_globals`, it returns the default value from `app`.
///
/// # Returns
/// - `&'static str` - The configured server host or the default host.
///
/// # Example
/// ```rust
/// use crate::globals::{init_globals, server_host};
/// init_globals("localhost".to_string(), 5444);
/// assert_eq!(server_host(), "localhost");
/// ```
pub fn server_host() -> &'static str {
    STACKQL_SERVER_HOST
        .get()
        .map_or(DEFAULT_SERVER_HOST, |s| s.as_str())
}

/// Retrieves the configured global server port.
///
/// If the port is not set via `init_globals`, it returns the default value from `app`.
///
/// # Returns
/// - `u16` - The configured server port or the default port.
///
/// # Example
/// ```rust
/// use crate::globals::{init_globals, server_port};
/// init_globals("localhost".to_string(), 5444);
/// assert_eq!(server_port(), 5444);
/// ```
pub fn server_port() -> u16 {
    STACKQL_SERVER_PORT
        .get()
        .copied()
        .unwrap_or(DEFAULT_SERVER_PORT)
}

