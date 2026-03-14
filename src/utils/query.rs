// utils/query.rs

//! # Query Utility Module
//!
//! This module provides functions and data structures for executing SQL queries
//! against a PgwireLite client. It supports processing query results and
//! formatting them into various representations (rows, columns, notices).
//!
//! ## Features
//! - Executes SQL queries using `pgwire_lite::PgwireLite`.
//! - Formats query results into structured data (columns, rows, notices).
//! - Supports different query result types: Data, Command, and Empty.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::query::{execute_query, QueryResult};
//! use pgwire_lite::PgwireLite;
//!
//! let mut client = PgwireLite::new("localhost", 5432, false, "default").unwrap();
//! let result = execute_query("SELECT * FROM my_table;", &mut client).unwrap();
//!
//! match result {
//!     QueryResult::Data { columns, rows, .. } => println!("Received data with {} rows.", rows.len()),
//!     QueryResult::Command(cmd) => println!("Command executed: {}", cmd),
//!     QueryResult::Empty => println!("Query executed successfully with no result."),
//! }
//! ```

use crate::utils::pgwire::{PgwireLite, Value};

/// Represents a column in a query result.
pub struct QueryResultColumn {
    pub name: String,
}

/// Represents a row in a query result.
pub struct QueryResultRow {
    pub values: Vec<String>,
}

/// Enum representing the possible results of a query execution.
pub enum QueryResult {
    Data {
        columns: Vec<QueryResultColumn>,
        rows: Vec<QueryResultRow>,
        notices: Vec<String>,
    },
    Command(String),
    Empty,
}

/// Executes an SQL query and returns the result in a structured format.
pub fn execute_query(query: &str, client: &mut PgwireLite) -> Result<QueryResult, String> {
    match client.query(query) {
        Ok(result) => {
            // Convert column names to QueryResultColumn structs
            let columns: Vec<QueryResultColumn> = result
                .column_names
                .iter()
                .map(|name| QueryResultColumn { name: name.clone() })
                .collect();

            // Convert rows to QueryResultRow structs
            let rows: Vec<QueryResultRow> = result
                .rows
                .iter()
                .map(|row_map| {
                    let values: Vec<String> = columns
                        .iter()
                        .map(|col| {
                            match row_map.get(&col.name) {
                                Some(Value::String(s)) => s.clone(),
                                Some(Value::Null) => "NULL".to_string(),
                                Some(Value::Bool(b)) => b.to_string(),
                                Some(Value::Integer(i)) => i.to_string(),
                                Some(Value::Float(f)) => f.to_string(),
                                Some(_) => "UNKNOWN_TYPE".to_string(), // For any future value types
                                None => "NULL".to_string(),
                            }
                        })
                        .collect();

                    QueryResultRow { values }
                })
                .collect();

            // Convert notices to strings
            let notices: Vec<String> = result
                .notices
                .iter()
                .map(|notice| {
                    // Get the basic message
                    let mut notice_text = notice
                        .fields
                        .get("message")
                        .cloned()
                        .unwrap_or_else(|| "Unknown notice".to_string());

                    // Add detail if available
                    if let Some(detail) = notice.fields.get("detail") {
                        notice_text.push_str("\nDETAIL: ");
                        notice_text.push_str(detail);
                    }

                    // Add hint if available
                    if let Some(hint) = notice.fields.get("hint") {
                        notice_text.push_str("\nHINT: ");
                        notice_text.push_str(hint);
                    }

                    notice_text
                })
                .collect();

            // Determine the type of result based on rows, notices, and data
            if !rows.is_empty() || !notices.is_empty() {
                // If we have rows OR notices, it's a data result
                Ok(QueryResult::Data {
                    columns,
                    rows,
                    notices,
                })
            } else if result.row_count > 0 {
                // If row_count > 0 but no rows, it was a command that affected rows
                let command_message = format!(
                    "Command completed successfully (affected {} rows)",
                    result.row_count
                );
                Ok(QueryResult::Command(command_message))
            } else {
                // Otherwise it's an empty result
                Ok(QueryResult::Empty)
            }
        }
        Err(e) => Err(format!("Query execution failed: {}", e)),
    }
}
