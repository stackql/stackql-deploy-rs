// resource/mod.rs

//! # Resource Module
//!
//! This module contains functionality for working with resources in a stack.
//! It includes submodules for manifest handling, operations, queries, and exports.
//!
//! Resources are the fundamental building blocks of a stack, and this module
//! provides the tools needed to load, manipulate, and process them.

// pub mod exports;
pub mod manifest;
pub mod validation;
// pub mod operations;
// pub mod queries;

// /// Creates a combined error type for resource operations.
// #[derive(thiserror::Error, Debug)]
// pub enum ResourceError {
//     #[error("Manifest error: {0}")]
//     Manifest(#[from] manifest::ManifestError),

//     #[error("Operation error: {0}")]
//     Operation(#[from] operations::OperationError),

//     #[error("Query error: {0}")]
//     Query(#[from] queries::QueryError),

//     #[error("Export error: {0}")]
//     Export(#[from] exports::ExportError),

//     #[error("I/O error: {0}")]
//     Io(#[from] std::io::Error),

//     #[allow(dead_code)]
//     #[error("Other error: {0}")]
//     Other(String),
// }

// /// Type alias for resource operation results
// pub type _Result<T> = std::result::Result<T, ResourceError>;
