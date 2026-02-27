// lib/utils.rs

//! # Utility Functions
//!
//! Low-level StackQL execution utilities, retry logic, export handling,
//! provider management, and script execution.
//! Matches the Python `lib/utils.py` implementation.

use std::collections::HashMap;
use std::process;
use std::thread;
use std::time::{Duration, Instant};

use log::{debug, error, info, warn};
use pgwire_lite::PgwireLite;

use crate::utils::query::{execute_query, QueryResult};

/// Exit with error message. Matches Python's `catch_error_and_exit`.
pub fn catch_error_and_exit(msg: &str) -> ! {
    error!("{}", msg);
    eprintln!("stackql-deploy operation failed");
    process::exit(1);
}

/// Execute a StackQL SELECT query with retry logic.
/// Returns rows as Vec<HashMap<String, String>>.
/// Matches Python's `run_stackql_query`.
pub fn run_stackql_query(
    query: &str,
    client: &mut PgwireLite,
    suppress_errors: bool,
    retries: u32,
    delay: u32,
) -> Vec<HashMap<String, String>> {
    let mut attempt = 0;
    let mut last_error: Option<String> = None;

    while attempt <= retries {
        debug!(
            "Executing stackql query on attempt {}:\n\n{}\n",
            attempt + 1,
            query
        );

        match execute_query(query, client) {
            Ok(result) => match result {
                QueryResult::Data {
                    columns,
                    rows,
                    notices,
                } => {
                    // Check for error notices
                    for notice in &notices {
                        if notice.contains("error") || notice.starts_with("ERROR") {
                            last_error = Some(notice.clone());
                            if !suppress_errors && attempt == retries {
                                catch_error_and_exit(&format!(
                                    "Error during stackql query execution:\n\n{}\n",
                                    notice
                                ));
                            }
                        }
                    }

                    if rows.is_empty() {
                        debug!("Stackql query executed successfully, retrieved 0 items.");
                        if attempt < retries {
                            thread::sleep(Duration::from_secs(delay as u64));
                            attempt += 1;
                            continue;
                        }
                        return Vec::new();
                    }

                    // Convert to Vec<HashMap>
                    let col_names: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();

                    let result_maps: Vec<HashMap<String, String>> = rows
                        .iter()
                        .map(|row| {
                            let mut map = HashMap::new();
                            for (i, col_name) in col_names.iter().enumerate() {
                                let value = row
                                    .values
                                    .get(i)
                                    .cloned()
                                    .unwrap_or_else(|| "NULL".to_string());
                                map.insert(col_name.clone(), value);
                            }
                            map
                        })
                        .collect();

                    // Check for error in results
                    if !result_maps.is_empty() {
                        if let Some(err) = result_maps[0].get("error") {
                            last_error = Some(err.clone());
                            if !suppress_errors {
                                if attempt == retries {
                                    catch_error_and_exit(&format!(
                                        "Error during stackql query execution:\n\n{}\n",
                                        err
                                    ));
                                } else {
                                    error!("Attempt {} failed:\n\n{}\n", attempt + 1, err);
                                }
                            }
                            thread::sleep(Duration::from_secs(delay as u64));
                            attempt += 1;
                            continue;
                        }

                        // Check for count query
                        if let Some(count_str) = result_maps[0].get("count") {
                            debug!("Stackql query executed successfully, count: {}", count_str);
                            if let Ok(count) = count_str.parse::<i64>() {
                                if count > 1 {
                                    catch_error_and_exit(&format!(
                                        "Detected more than one resource matching query criteria, expected 0 or 1, got {}",
                                        count
                                    ));
                                }
                            }
                            return result_maps;
                        }
                    }

                    debug!(
                        "Stackql query executed successfully, retrieved {} items.",
                        result_maps.len()
                    );
                    return result_maps;
                }
                QueryResult::Command(msg) => {
                    debug!("Command result: {}", msg);
                    return Vec::new();
                }
                QueryResult::Empty => {
                    debug!("Empty result from query");
                    if attempt < retries {
                        thread::sleep(Duration::from_secs(delay as u64));
                        attempt += 1;
                        continue;
                    }
                    return Vec::new();
                }
            },
            Err(e) => {
                last_error = Some(e.clone());
                if attempt == retries {
                    if !suppress_errors {
                        catch_error_and_exit(&format!(
                            "Exception during stackql query execution:\n\n{}\n",
                            e
                        ));
                    }
                } else {
                    error!("Exception on attempt {}:\n\n{}\n", attempt + 1, e);
                }
            }
        }

        thread::sleep(Duration::from_secs(delay as u64));
        attempt += 1;
    }

    debug!(
        "All attempts ({}) to execute the query completed.",
        retries + 1
    );

    // If suppress_errors and we have an error, return error marker
    if suppress_errors {
        if let Some(err) = last_error {
            let mut error_map = HashMap::new();
            error_map.insert("_stackql_deploy_error".to_string(), err);
            return vec![error_map];
        }
    }

    Vec::new()
}

/// Execute a StackQL DML/DDL command with retry logic.
/// Matches Python's `run_stackql_command`.
pub fn run_stackql_command(
    command: &str,
    client: &mut PgwireLite,
    ignore_errors: bool,
    retries: u32,
    retry_delay: u32,
) -> String {
    let mut attempt = 0;

    // Handle REGISTRY PULL command format
    let processed_command = if command.starts_with("REGISTRY PULL") {
        let re = regex::Regex::new(r"(REGISTRY PULL \w+)(::v[\d\.]+)?").unwrap();
        if let Some(caps) = re.captures(command) {
            let provider = caps.get(1).map_or("", |m| m.as_str());
            if let Some(version) = caps.get(2) {
                format!("{} {}", provider, &version.as_str()[2..])
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        }
    } else {
        command.to_string()
    };

    while attempt <= retries {
        debug!(
            "Executing stackql command (attempt {}):\n\n{}\n",
            attempt + 1,
            processed_command
        );

        match execute_query(&processed_command, client) {
            Ok(result) => match result {
                QueryResult::Data { notices, .. } => {
                    // Check for errors in notices
                    for notice in &notices {
                        if error_detected_in_notice(notice) && !ignore_errors {
                            if attempt < retries {
                                warn!(
                                        "Dependent resource(s) may not be ready, retrying in {} seconds (attempt {} of {})...",
                                        retry_delay, attempt + 1, retries + 1
                                    );
                                thread::sleep(Duration::from_secs(retry_delay as u64));
                                attempt += 1;
                                continue;
                            } else {
                                catch_error_and_exit(&format!(
                                    "Error during stackql command execution:\n\n{}\n",
                                    notice
                                ));
                            }
                        }
                    }
                    let msg = notices.join("\n");
                    if !msg.is_empty() {
                        debug!("Stackql command executed successfully:\n\n{}\n", msg);
                    }
                    return msg;
                }
                QueryResult::Command(msg) => {
                    debug!("Stackql command executed successfully:\n\n{}\n", msg);
                    return msg;
                }
                QueryResult::Empty => {
                    debug!("Command executed with empty result");
                    return String::new();
                }
            },
            Err(e) => {
                if !ignore_errors {
                    if attempt < retries {
                        warn!(
                            "Command failed, retrying in {} seconds (attempt {} of {})...",
                            retry_delay,
                            attempt + 1,
                            retries + 1
                        );
                        thread::sleep(Duration::from_secs(retry_delay as u64));
                        attempt += 1;
                        continue;
                    }
                    catch_error_and_exit(&format!(
                        "Exception during stackql command execution:\n\n{}\n",
                        e
                    ));
                } else {
                    debug!("Command failed (ignored): {}", e);
                    return String::new();
                }
            }
        }
    }

    String::new()
}

/// Check if a notice/message indicates an error.
fn error_detected_in_notice(msg: &str) -> bool {
    msg.starts_with("http response status code: 4")
        || msg.starts_with("http response status code: 5")
        || msg.starts_with("error:")
        || msg.starts_with("disparity in fields to insert")
        || msg.starts_with("cannot find matching operation")
}

/// Run a test query and check if count == 1 (exists) or count == 0 (deleted).
/// Matches Python's `run_test`.
pub fn run_test(
    resource_name: &str,
    query: &str,
    client: &mut PgwireLite,
    delete_test: bool,
) -> bool {
    let result = run_stackql_query(query, client, true, 0, 5);

    if result.is_empty() {
        if delete_test {
            debug!("Delete test result true for [{}]", resource_name);
            return true;
        } else {
            debug!("Test result false for [{}]", resource_name);
            return false;
        }
    }

    // Check for error markers
    if result[0].contains_key("_stackql_deploy_error") || result[0].contains_key("error") {
        if delete_test {
            return true;
        }
        return false;
    }

    if let Some(count_str) = result[0].get("count") {
        if let Ok(count) = count_str.parse::<i64>() {
            if delete_test {
                if count == 0 {
                    debug!("Delete test result true for [{}]", resource_name);
                    return true;
                } else {
                    debug!(
                        "Delete test result false for [{}], expected 0 got {}",
                        resource_name, count
                    );
                    return false;
                }
            } else if count == 1 {
                debug!("Test result true for [{}]", resource_name);
                return true;
            } else {
                debug!(
                    "Test result false for [{}], expected 1 got {}",
                    resource_name, count
                );
                return false;
            }
        }
    }

    // If no count field, for non-delete test consider any result as exists
    if !delete_test && !result.is_empty() {
        return true;
    }

    false
}

/// Perform retries on a test query.
/// Matches Python's `perform_retries`.
pub fn perform_retries(
    resource_name: &str,
    query: &str,
    retries: u32,
    delay: u32,
    client: &mut PgwireLite,
    delete_test: bool,
) -> bool {
    let start = Instant::now();
    let mut attempt = 0;

    while attempt < retries {
        let result = run_test(resource_name, query, client, delete_test);
        if result {
            return true;
        }
        let elapsed = start.elapsed().as_secs();
        info!(
            "attempt {}/{}: retrying in {} seconds ({} seconds elapsed).",
            attempt + 1,
            retries,
            delay,
            elapsed
        );
        thread::sleep(Duration::from_secs(delay as u64));
        attempt += 1;
    }

    false
}

/// Show a query in logs if show_queries is enabled.
pub fn show_query(show_queries: bool, query: &str) {
    if show_queries {
        info!("query:\n\n{}\n", query);
    }
}

/// Pull providers using the StackQL server.
/// Matches Python's `pull_providers`.
pub fn pull_providers(providers: &[String], client: &mut PgwireLite) {
    let installed = run_stackql_query("SHOW PROVIDERS", client, false, 0, 5);

    for provider in providers {
        if provider.contains("::") {
            // Versioned provider
            let parts: Vec<&str> = provider.splitn(2, "::").collect();
            let name = parts[0];
            let version = parts[1];

            let found = installed.iter().any(|p| {
                p.get("name").is_some_and(|n| n == name)
                    && p.get("version").is_some_and(|v| v == version)
            });

            if found {
                info!("Provider '{}' is already installed.", provider);
            } else {
                // Check if a higher version is installed
                let higher_installed = installed.iter().any(|p| {
                    p.get("name").is_some_and(|n| n == name)
                        && p.get("version")
                            .is_some_and(|v| is_version_higher(v, version))
                });

                if higher_installed {
                    info!(
                        "Provider '{}' - a higher version is already installed.",
                        provider
                    );
                } else {
                    info!("Pulling provider '{}'...", provider);
                    let cmd = format!("REGISTRY PULL {}", provider);
                    let msg = run_stackql_command(&cmd, client, false, 0, 5);
                    if !msg.is_empty() {
                        info!("{}", msg);
                    }
                }
            }
        } else {
            let found = installed.iter().any(|p| p.get("name") == Some(provider));

            if found {
                info!("Provider '{}' is already installed.", provider);
            } else {
                info!("Pulling provider '{}'...", provider);
                let cmd = format!("REGISTRY PULL {}", provider);
                let msg = run_stackql_command(&cmd, client, false, 0, 5);
                if !msg.is_empty() {
                    info!("{}", msg);
                }
            }
        }
    }
}

/// Compare version strings. Returns true if installed > requested.
fn is_version_higher(installed: &str, requested: &str) -> bool {
    let parse = |v: &str| -> u64 { v.replace(['v', '.'], "").parse::<u64>().unwrap_or(0) };
    parse(installed) > parse(requested)
}

/// Update global context with exported values.
///
/// Each export is stored under two keys:
///
/// - **`{key}`** — the global (unscoped) key.  This can be overridden by a
///   subsequent resource that exports a variable with the same name, so it
///   always reflects the *most recent* export value.
///
/// - **`{resource_name}.{key}`** — the resource-scoped (fully qualified) key.
///   This is written **once** and never overwritten, so it is immutable once
///   set.  Consumers that need an unambiguous reference should use this form.
///
/// Matches Python's `export_vars`.
pub fn export_vars(
    global_context: &mut HashMap<String, String>,
    resource_name: &str,
    export_data: &HashMap<String, String>,
    protected_exports: &[String],
) {
    for (key, value) in export_data {
        let is_protected = protected_exports.contains(key);
        let display_value = if is_protected {
            "*".repeat(value.len())
        } else {
            value.clone()
        };

        // --- resource-scoped key (immutable: only written if not already set) ---
        let scoped_key = format!("{}.{}", resource_name, key);
        global_context.entry(scoped_key.clone()).or_insert_with(|| {
            info!(
                "set {} [{}] to [{}] in exports",
                if is_protected {
                    "protected variable"
                } else {
                    "variable"
                },
                scoped_key,
                display_value,
            );
            value.clone()
        });

        // --- global (unscoped) key (can be overridden by later resources) ---
        info!(
            "set {} [{}] to [{}] in exports",
            if is_protected {
                "protected variable"
            } else {
                "variable"
            },
            key,
            display_value,
        );
        global_context.insert(key.clone(), value.clone());
    }
}

/// Check if exports result can serve as a statecheck proxy.
/// Returns true if result is non-empty and has no errors.
/// Matches Python's `check_exports_as_statecheck_proxy`.
pub fn check_exports_as_statecheck_proxy(exports_result: &[HashMap<String, String>]) -> bool {
    debug!(
        "Checking exports result as statecheck proxy: {} rows",
        exports_result.len()
    );

    if exports_result.is_empty() {
        debug!("Empty exports result, treating as statecheck failure");
        return false;
    }

    // Check for error conditions
    if exports_result[0].contains_key("_stackql_deploy_error") {
        debug!("Error in exports result, treating as statecheck failure");
        return false;
    }
    if exports_result[0].contains_key("error") {
        debug!("Error in exports result, treating as statecheck failure");
        return false;
    }

    debug!("Valid exports result, treating as statecheck success");
    true
}

/// Check if all items in exports list are dicts (HashMap-like).
/// In Rust, exports from YAML can be strings or maps.
/// Matches Python's `check_all_dicts`.
pub fn check_all_dicts(items: &[serde_yaml::Value]) -> bool {
    if items.is_empty() {
        return false;
    }
    items.iter().all(|item| item.is_mapping())
}

/// Run an external script and capture output.
/// Matches Python's `run_ext_script`.
pub fn run_ext_script(
    cmd: &str,
    expected_exports: Option<&[String]>,
) -> Option<HashMap<String, String>> {
    debug!("Running external script: {}", cmd);

    let output = match std::process::Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(output) => output,
        Err(e) => {
            catch_error_and_exit(&format!("Script failed: {}", e));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    debug!("Script output: {}", stdout);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        catch_error_and_exit(&format!(
            "Script failed with status {:?}: {}",
            output.status.code(),
            stderr
        ));
    }

    match expected_exports {
        Some(exports) if !exports.is_empty() => {
            match serde_json::from_str::<HashMap<String, String>>(&stdout) {
                Ok(exported_vars) => {
                    for export in exports {
                        if !exported_vars.contains_key(export) {
                            catch_error_and_exit(&format!(
                                "Exported variable '{}' not found in script output",
                                export
                            ));
                        }
                    }
                    Some(exported_vars)
                }
                Err(_) => {
                    catch_error_and_exit(&format!(
                        "External scripts must return valid JSON: {}",
                        stdout
                    ));
                }
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // export_vars
    // ------------------------------------------------------------------

    #[test]
    fn test_export_vars_sets_global_and_scoped_key() {
        let mut ctx: HashMap<String, String> = HashMap::new();
        let mut data: HashMap<String, String> = HashMap::new();
        data.insert("role_name".to_string(), "my-role".to_string());

        export_vars(&mut ctx, "aws_cross_account_role", &data, &[]);

        // Global key
        assert_eq!(ctx.get("role_name").map(|s| s.as_str()), Some("my-role"));
        // Resource-scoped key
        assert_eq!(
            ctx.get("aws_cross_account_role.role_name")
                .map(|s| s.as_str()),
            Some("my-role"),
        );
    }

    #[test]
    fn test_export_vars_global_key_is_overridable() {
        let mut ctx: HashMap<String, String> = HashMap::new();

        // First resource exports role_name
        let mut data1 = HashMap::new();
        data1.insert("role_name".to_string(), "first-role".to_string());
        export_vars(&mut ctx, "resource_a", &data1, &[]);

        // Second resource exports role_name with a different value
        let mut data2 = HashMap::new();
        data2.insert("role_name".to_string(), "second-role".to_string());
        export_vars(&mut ctx, "resource_b", &data2, &[]);

        // Global key reflects the most recent export
        assert_eq!(
            ctx.get("role_name").map(|s| s.as_str()),
            Some("second-role")
        );
    }

    #[test]
    fn test_export_vars_scoped_key_is_immutable() {
        let mut ctx: HashMap<String, String> = HashMap::new();

        // First resource exports role_name
        let mut data1 = HashMap::new();
        data1.insert("role_name".to_string(), "original-role".to_string());
        export_vars(&mut ctx, "resource_a", &data1, &[]);

        // Simulate an accidental re-export of the same resource (e.g. called
        // twice): the scoped key must not be overwritten.
        let mut data2 = HashMap::new();
        data2.insert("role_name".to_string(), "should-not-overwrite".to_string());
        export_vars(&mut ctx, "resource_a", &data2, &[]);

        // Scoped key is unchanged
        assert_eq!(
            ctx.get("resource_a.role_name").map(|s| s.as_str()),
            Some("original-role"),
        );
        // Global key reflects the latest call (expected)
        assert_eq!(
            ctx.get("role_name").map(|s| s.as_str()),
            Some("should-not-overwrite"),
        );
    }

    #[test]
    fn test_export_vars_protected_values_are_stored_normally() {
        // Protection only affects log-masking, not what is stored
        let mut ctx: HashMap<String, String> = HashMap::new();
        let mut data = HashMap::new();
        data.insert("secret_key".to_string(), "super-secret".to_string());

        export_vars(&mut ctx, "vault", &data, &["secret_key".to_string()]);

        assert_eq!(
            ctx.get("secret_key").map(|s| s.as_str()),
            Some("super-secret")
        );
        assert_eq!(
            ctx.get("vault.secret_key").map(|s| s.as_str()),
            Some("super-secret"),
        );
    }
}
