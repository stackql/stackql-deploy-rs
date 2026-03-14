// utils/connection.rs

//! # Connection Utility Module
//!
//! This module provides functions for creating a PgwireLite client connection
//! to the StackQL server. It utilizes global configuration for host and port
//! and supports error handling during connection attempts.
//!
//! ## Features
//! - Establishes a connection to the StackQL server using `pgwire_lite::PgwireLite`.
//! - Uses global host and port settings for consistency across the application.
//! - Handles connection errors and exits the program if unsuccessful.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::connection::create_client;
//!
//! let client = create_client();
//! ```

use std::process;

use colored::*;

use crate::globals::{server_host, server_port};
use crate::utils::pgwire::PgwireLite;

/// Creates a new PgwireLite client connection
pub fn create_client() -> PgwireLite {
    let host = server_host();
    let port = server_port();

    // Create a new PgwireLite client with the server's host and port
    // Default to no TLS and default verbosity
    let client = PgwireLite::new(host, port, false, "default").unwrap_or_else(|e| {
        eprintln!("{}", format!("Failed to connect to server: {}", e).red());
        process::exit(1); // Exit the program if connection fails
    });

    println!("Connected to stackql server at {}:{}", host, port);
    println!("Using pgwire client: {}", client.libpq_version());

    client
}
