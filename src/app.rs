// app.rs

//! # StackQL Deploy Application Constants
//!
//! This module defines various constants and configuration values for the StackQL Deploy application.
//! It includes general application metadata, default settings, supported providers, and paths to templates.
//!
//! ## Usage Example
//! ```rust
//! use crate::app::{APP_NAME, APP_VERSION, DEFAULT_SERVER_HOST, DEFAULT_SERVER_PORT};
//!
//! println!("{} v{} running on {}:{}",
//!     APP_NAME, APP_VERSION, DEFAULT_SERVER_HOST, DEFAULT_SERVER_PORT
//! );
//! ```
//!
//! This module also contains sub-modules for template-related constants specific to
//! AWS, Azure, and Google platforms.

/// Application name
pub const APP_NAME: &str = "stackql-deploy";

/// Application version (sourced from Cargo.toml)
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application author
pub const APP_AUTHOR: &str = "Jeffrey Aven <javen@stackql.io>";

/// Application description
pub const APP_DESCRIPTION: &str = "Model driven IaC using stackql";

/// Default server host
pub const DEFAULT_SERVER_HOST: &str = "localhost";

/// Default StackQL (PostgreSQL protocol) server port
pub const DEFAULT_SERVER_PORT: u16 = 5444;

/// Default StackQL (PostgreSQL protocol) server port as a string
pub const DEFAULT_SERVER_PORT_STR: &str = "5444";

/// Local server addresses
pub const LOCAL_SERVER_ADDRESSES: [&str; 3] = ["localhost", "0.0.0.0", "127.0.0.1"];

/// Default log file name
pub const DEFAULT_LOG_FILE: &str = "stackql.log";

/// Default log level
pub const LOG_LEVELS: &[&str] = &["trace", "debug", "info", "warn", "error"];

/// Default log level for the application
pub const DEFAULT_LOG_LEVEL: &str = "info";

/// Supported cloud providers for the `--provider` argument in the `init` command
pub const SUPPORTED_PROVIDERS: [&str; 3] = ["aws", "google", "azure"];

/// Default provider for `init` command
pub const DEFAULT_PROVIDER: &str = "azure";

/// StackQL binary name (platform dependent)
#[cfg_attr(
    target_os = "windows",
    doc = "StackQL binary name (platform dependent)"
)]
#[cfg(target_os = "windows")]
pub const STACKQL_BINARY_NAME: &str = "stackql.exe";

#[cfg_attr(
    not(target_os = "windows"),
    doc = "StackQL binary name (platform dependent)"
)]
#[cfg(not(target_os = "windows"))]
pub const STACKQL_BINARY_NAME: &str = "stackql";

/// Base URL for StackQL releases
pub const STACKQL_RELEASE_BASE_URL: &str = "https://releases.stackql.io/stackql/latest";

/// Commands exempt from binary check
pub const EXEMPT_COMMANDS: [&str; 1] = ["init"];

/// The base URL for GitHub template repository
pub const GITHUB_TEMPLATE_BASE: &str =
    "https://raw.githubusercontent.com/stackql/stackql-deploy-rust/main/template-hub/";

/// Template constants for AWS
pub mod aws_templates {
    pub const RESOURCE_TEMPLATE: &str =
        include_str!("../template-hub/aws/starter/resources/example_vpc.iql.template");
    pub const MANIFEST_TEMPLATE: &str =
        include_str!("../template-hub/aws/starter/stackql_manifest.yml.template");
    pub const README_TEMPLATE: &str =
        include_str!("../template-hub/aws/starter/README.md.template");
}

/// Template constants for Azure
pub mod azure_templates {
    pub const RESOURCE_TEMPLATE: &str =
        include_str!("../template-hub/azure/starter/resources/example_res_grp.iql.template");
    pub const MANIFEST_TEMPLATE: &str =
        include_str!("../template-hub/azure/starter/stackql_manifest.yml.template");
    pub const README_TEMPLATE: &str =
        include_str!("../template-hub/azure/starter/README.md.template");
}

/// Template constants for Google
pub mod google_templates {
    pub const RESOURCE_TEMPLATE: &str =
        include_str!("../template-hub/google/starter/resources/example_vpc.iql.template");
    pub const MANIFEST_TEMPLATE: &str =
        include_str!("../template-hub/google/starter/stackql_manifest.yml.template");
    pub const README_TEMPLATE: &str =
        include_str!("../template-hub/google/starter/README.md.template");
}
