// commands/base.rs

//! # Base Command Module
//!
//! Shared resource processing logic used by build, teardown, and test commands.
//! This is the Rust equivalent of the Python `cmd/base.py` `StackQLBase` class.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process;

use log::{debug, error, info};
use pgwire_lite::PgwireLite;

use crate::core::config::{get_full_context, render_globals, render_string_value};
use crate::core::env::load_env_vars;
use crate::core::templating::{self, ParsedQuery};
use crate::core::utils::{
    catch_error_and_exit, check_exports_as_statecheck_proxy, export_vars, perform_retries,
    pull_providers, run_ext_script, run_stackql_command, run_stackql_query, show_query,
};
use crate::resource::manifest::{Manifest, Resource};
use crate::resource::validation::validate_manifest;
use crate::template::engine::TemplateEngine;

/// Core state for all command operations, equivalent to Python's StackQLBase.
pub struct CommandRunner {
    pub client: PgwireLite,
    pub engine: TemplateEngine,
    pub manifest: Manifest,
    pub global_context: HashMap<String, String>,
    pub stack_dir: String,
    pub stack_env: String,
    pub stack_name: String,
    #[allow(dead_code)]
    pub env_vars: HashMap<String, String>,
}

impl CommandRunner {
    /// Create a new CommandRunner, loading manifest, pulling providers, etc.
    pub fn new(
        mut client: PgwireLite,
        stack_dir: &str,
        stack_env: &str,
        env_file: &str,
        env_overrides: &[String],
    ) -> Self {
        let engine = TemplateEngine::new();

        // Load env vars
        let env_vars = load_env_vars(env_file, env_overrides);

        // Load manifest
        let manifest = Manifest::load_from_dir_or_exit(stack_dir);

        // Validate manifest rules
        if let Err(errors) = validate_manifest(&manifest) {
            for err in &errors {
                error!("{}", err);
            }
            catch_error_and_exit(&format!(
                "Manifest validation failed with {} error(s)",
                errors.len()
            ));
        }

        let stack_name = manifest.name.clone();

        // Render globals
        let global_context = render_globals(&engine, &env_vars, &manifest, stack_env, &stack_name);

        // Pull providers
        pull_providers(&manifest.providers, &mut client);

        Self {
            client,
            engine,
            manifest,
            global_context,
            stack_dir: stack_dir.to_string(),
            stack_env: stack_env.to_string(),
            stack_name,
            env_vars,
        }
    }

    /// Get the full context for a resource (global + resource properties).
    pub fn get_full_context(&self, resource: &Resource) -> HashMap<String, String> {
        get_full_context(
            &self.engine,
            &self.global_context,
            resource,
            &self.stack_env,
        )
    }

    /// Evaluate a resource's `if` condition. Returns true if the resource should be processed.
    pub fn evaluate_condition(
        &self,
        resource: &Resource,
        full_context: &HashMap<String, String>,
    ) -> bool {
        if let Some(ref condition) = resource.r#if {
            let rendered = render_string_value(&self.engine, condition, full_context);

            // Evaluate simple string equality/inequality conditions
            // Python uses eval(), we do simple pattern matching for safety
            match evaluate_simple_condition(&rendered) {
                Some(result) => {
                    if !result {
                        info!(
                            "Skipping resource [{}] due to condition: {}",
                            resource.name, condition
                        );
                    }
                    result
                }
                None => {
                    error!(
                        "Error evaluating condition for resource [{}]: {}",
                        resource.name, rendered
                    );
                    process::exit(1);
                }
            }
        } else {
            true // No condition, always process
        }
    }

    /// Get queries for a resource from its .iql file.
    pub fn get_queries(
        &self,
        resource: &Resource,
        full_context: &HashMap<String, String>,
    ) -> HashMap<String, ParsedQuery> {
        templating::get_queries(&self.engine, &self.stack_dir, resource, full_context)
    }

    /// Render inline SQL template.
    pub fn render_inline_template(
        &self,
        resource_name: &str,
        sql: &str,
        full_context: &HashMap<String, String>,
    ) -> String {
        templating::render_inline_template(&self.engine, resource_name, sql, full_context)
    }

    /// Render a single query template JIT with the current context.
    pub fn render_query(
        &self,
        resource_name: &str,
        anchor: &str,
        template: &str,
        full_context: &HashMap<String, String>,
    ) -> String {
        templating::render_query(&self.engine, resource_name, anchor, template, full_context)
    }

    /// Check if a resource exists using the exists query.
    #[allow(clippy::too_many_arguments)]
    pub fn check_if_resource_exists(
        &mut self,
        resource: &Resource,
        exists_query: &str,
        retries: u32,
        retry_delay: u32,
        dry_run: bool,
        show_queries: bool,
        delete_test: bool,
    ) -> bool {
        let check_type = if delete_test { "post-delete" } else { "exists" };

        if dry_run {
            info!(
                "dry run {} check for [{}]:\n\n/* exists query */\n{}\n",
                check_type, resource.name, exists_query
            );
            return false;
        }

        info!("running {} check for [{}]...", check_type, resource.name);
        show_query(show_queries, exists_query);

        let exists = perform_retries(
            &resource.name,
            exists_query,
            retries,
            retry_delay,
            &mut self.client,
            delete_test,
        );

        if delete_test {
            if exists {
                info!("[{}] still exists", resource.name);
            } else {
                info!("[{}] confirmed deleted", resource.name);
            }
        } else if exists {
            info!("[{}] exists", resource.name);
        } else {
            info!("[{}] does not exist", resource.name);
        }

        exists
    }

    /// Check if a resource is in the correct state.
    pub fn check_if_resource_is_correct_state(
        &mut self,
        resource: &Resource,
        statecheck_query: &str,
        retries: u32,
        retry_delay: u32,
        dry_run: bool,
        show_queries: bool,
    ) -> bool {
        if dry_run {
            info!(
                "dry run state check for [{}]:\n\n/* state check query */\n{}\n",
                resource.name, statecheck_query
            );
            return true;
        }

        info!("running state check for [{}]...", resource.name);
        show_query(show_queries, statecheck_query);

        let is_correct = perform_retries(
            &resource.name,
            statecheck_query,
            retries,
            retry_delay,
            &mut self.client,
            false,
        );

        if is_correct {
            info!("[{}] is in the desired state", resource.name);
        } else {
            info!("[{}] is not in the desired state", resource.name);
        }

        is_correct
    }

    /// Use exports query as a proxy for state check.
    pub fn check_state_using_exports_proxy(
        &mut self,
        resource: &Resource,
        exports_query: &str,
        retries: u32,
        retry_delay: u32,
        dry_run: bool,
        show_queries: bool,
    ) -> (bool, Option<Vec<HashMap<String, String>>>) {
        if dry_run {
            info!(
                "dry run state check using exports proxy for [{}]:\n\n/* exports as statecheck proxy */\n{}\n",
                resource.name, exports_query
            );
            return (true, None);
        }

        info!(
            "running state check using exports proxy for [{}]...",
            resource.name
        );
        show_query(show_queries, exports_query);

        let result = run_stackql_query(exports_query, &mut self.client, true, retries, retry_delay);

        let is_correct = check_exports_as_statecheck_proxy(&result);

        if is_correct {
            info!(
                "[{}] exports proxy indicates resource is in the desired state",
                resource.name
            );
            (true, Some(result))
        } else {
            info!(
                "[{}] exports proxy indicates resource is not in the desired state",
                resource.name
            );
            (false, None)
        }
    }

    /// Create a resource.
    #[allow(clippy::too_many_arguments)]
    pub fn create_resource(
        &mut self,
        resource: &Resource,
        create_query: &str,
        retries: u32,
        retry_delay: u32,
        dry_run: bool,
        show_queries: bool,
        ignore_errors: bool,
    ) -> bool {
        if dry_run {
            info!(
                "dry run create for [{}]:\n\n/* insert (create) query */\n{}\n",
                resource.name, create_query
            );
            return false;
        }

        info!("[{}] does not exist, creating...", resource.name);
        show_query(show_queries, create_query);

        let msg = run_stackql_command(
            create_query,
            &mut self.client,
            ignore_errors,
            retries,
            retry_delay,
        );
        debug!("Create response: {}", msg);
        true
    }

    /// Update a resource.
    #[allow(clippy::too_many_arguments)]
    pub fn update_resource(
        &mut self,
        resource: &Resource,
        update_query: Option<&str>,
        retries: u32,
        retry_delay: u32,
        dry_run: bool,
        show_queries: bool,
        ignore_errors: bool,
    ) -> bool {
        match update_query {
            Some(query) => {
                if dry_run {
                    info!(
                        "dry run update for [{}]:\n\n/* update query */\n{}\n",
                        resource.name, query
                    );
                    return false;
                }

                info!("updating [{}]...", resource.name);
                show_query(show_queries, query);

                let msg = run_stackql_command(
                    query,
                    &mut self.client,
                    ignore_errors,
                    retries,
                    retry_delay,
                );
                debug!("Update response: {}", msg);
                true
            }
            None => {
                info!(
                    "Update query not configured for [{}], skipping update...",
                    resource.name
                );
                false
            }
        }
    }

    /// Delete a resource.
    #[allow(clippy::too_many_arguments)]
    pub fn delete_resource(
        &mut self,
        resource: &Resource,
        delete_query: &str,
        retries: u32,
        retry_delay: u32,
        dry_run: bool,
        show_queries: bool,
        ignore_errors: bool,
    ) {
        if dry_run {
            info!(
                "dry run delete for [{}]:\n\n{}\n",
                resource.name, delete_query
            );
            return;
        }

        info!("deleting [{}]...", resource.name);
        show_query(show_queries, delete_query);

        let msg = run_stackql_command(
            delete_query,
            &mut self.client,
            ignore_errors,
            retries,
            retry_delay,
        );
        debug!("Delete response: {}", msg);
    }

    /// Run a command-type query.
    pub fn run_command(
        &mut self,
        command_query: &str,
        retries: u32,
        retry_delay: u32,
        dry_run: bool,
        show_queries: bool,
    ) {
        if dry_run {
            info!("dry run command:\n\n{}\n", command_query);
            return;
        }

        info!("running command...");
        show_query(show_queries, command_query);
        run_stackql_command(command_query, &mut self.client, false, retries, retry_delay);
    }

    /// Process exports for a resource.
    #[allow(clippy::too_many_arguments)]
    pub fn process_exports(
        &mut self,
        resource: &Resource,
        _full_context: &HashMap<String, String>,
        exports_query: &str,
        retries: u32,
        retry_delay: u32,
        dry_run: bool,
        show_queries: bool,
        ignore_missing_exports: bool,
    ) {
        let expected_exports = &resource.exports;
        if expected_exports.is_empty() {
            return;
        }

        let all_dicts = expected_exports.iter().all(|e| e.is_mapping());
        let protected_exports = &resource.protected;

        if dry_run {
            let mut export_data = HashMap::new();
            if all_dicts {
                for item in expected_exports {
                    if let Some(map) = item.as_mapping() {
                        for (_, val) in map {
                            if let Some(v) = val.as_str() {
                                export_data.insert(v.to_string(), "<evaluated>".to_string());
                            }
                        }
                    }
                }
            } else {
                for item in expected_exports {
                    if let Some(s) = item.as_str() {
                        export_data.insert(s.to_string(), "<evaluated>".to_string());
                    }
                }
            }
            export_vars(
                &mut self.global_context,
                &resource.name,
                &export_data,
                protected_exports,
            );
            info!(
                "dry run exports query for [{}]:\n\n/* exports query */\n{}\n",
                resource.name, exports_query
            );
            return;
        }

        info!("exporting variables for [{}]...", resource.name);
        show_query(show_queries, exports_query);

        let exports =
            run_stackql_query(exports_query, &mut self.client, true, retries, retry_delay);

        debug!("Exports result: {:?}", exports);

        if exports.is_empty() {
            if ignore_missing_exports {
                return;
            }
            show_query(true, exports_query);
            catch_error_and_exit(&format!("Exports query failed for {}", resource.name));
        }

        // Check for errors
        if !exports.is_empty() {
            if exports[0].contains_key("_stackql_deploy_error") {
                let err_msg = exports[0].get("_stackql_deploy_error").unwrap();
                show_query(true, exports_query);
                catch_error_and_exit(&format!(
                    "Exports query failed for {}\n\nError details:\n{}",
                    resource.name, err_msg
                ));
            }
            if exports[0].contains_key("error") {
                let err_msg = exports[0].get("error").unwrap();
                show_query(true, exports_query);
                catch_error_and_exit(&format!(
                    "Exports query failed for {}\n\nError details:\n{}",
                    resource.name, err_msg
                ));
            }
        }

        if exports.len() > 1 {
            catch_error_and_exit(&format!(
                "Exports should include one row only, received {} rows",
                exports.len()
            ));
        }

        self.process_export_data(
            resource,
            &exports,
            expected_exports,
            all_dicts,
            protected_exports,
        );
    }

    /// Process exports from an already-obtained result (e.g., from exports proxy).
    pub fn process_exports_from_result(
        &mut self,
        resource: &Resource,
        exports_result: &[HashMap<String, String>],
    ) {
        let expected_exports = &resource.exports;
        if expected_exports.is_empty() || exports_result.is_empty() {
            return;
        }

        let all_dicts = expected_exports.iter().all(|e| e.is_mapping());
        let protected_exports = &resource.protected;

        if exports_result.len() > 1 {
            catch_error_and_exit(&format!(
                "Exports should include one row only, received {} rows",
                exports_result.len()
            ));
        }

        self.process_export_data(
            resource,
            exports_result,
            expected_exports,
            all_dicts,
            protected_exports,
        );
    }

    /// Internal helper to extract export data from query results.
    fn process_export_data(
        &mut self,
        resource: &Resource,
        exports: &[HashMap<String, String>],
        expected_exports: &[serde_yaml::Value],
        all_dicts: bool,
        protected_exports: &[String],
    ) {
        let export_row = if exports.is_empty() {
            HashMap::new()
        } else {
            exports[0].clone()
        };

        let mut export_data = HashMap::new();

        for item in expected_exports {
            if all_dicts {
                if let Some(map) = item.as_mapping() {
                    for (key_val, val_val) in map {
                        let key = key_val.as_str().unwrap_or("");
                        let val = val_val.as_str().unwrap_or("");
                        // key in expected_exports maps to key in export_row
                        // val becomes the key in export_data
                        let exported_value = export_row.get(key).cloned().unwrap_or_default();
                        export_data.insert(val.to_string(), exported_value);
                    }
                }
            } else {
                let item_name = item.as_str().unwrap_or("");
                if !item_name.is_empty() {
                    let exported_value = export_row.get(item_name).cloned().unwrap_or_default();
                    export_data.insert(item_name.to_string(), exported_value);
                }
            }
        }

        export_vars(
            &mut self.global_context,
            &resource.name,
            &export_data,
            protected_exports,
        );
    }

    /// Process a script resource type.
    pub fn process_script_resource(
        &mut self,
        resource: &Resource,
        dry_run: bool,
        full_context: &HashMap<String, String>,
    ) {
        info!("Running script for {}...", resource.name);

        let script_template = match &resource.run {
            Some(s) => s.clone(),
            None => {
                catch_error_and_exit("Script resource must include 'run' key");
            }
        };

        let script = render_string_value(&self.engine, &script_template, full_context);

        if dry_run {
            let dry_run_script = script.replace("\"\"", "\"<evaluated>\"");
            info!(
                "dry run script for [{}]:\n\n{}\n",
                resource.name, dry_run_script
            );
        } else {
            info!("running script for [{}]...", resource.name);

            let export_names: Vec<String> = resource
                .exports
                .iter()
                .filter_map(|e| e.as_str().map(|s| s.to_string()))
                .collect();

            let export_names_opt = if export_names.is_empty() {
                None
            } else {
                Some(export_names.as_slice())
            };

            if let Some(ret_vars) = run_ext_script(&script, export_names_opt) {
                if !resource.exports.is_empty() {
                    info!("Exported variables from script: {:?}", ret_vars);
                    export_vars(
                        &mut self.global_context,
                        &resource.name,
                        &ret_vars,
                        &resource.protected,
                    );
                }
            }
        }
    }

    /// Process stack-level exports to a JSON output file.
    pub fn process_stack_exports(
        &self,
        dry_run: bool,
        output_file: Option<&str>,
        elapsed_time: &str,
    ) {
        let output_file = match output_file {
            Some(f) => f,
            None => return,
        };

        info!("Processing stack exports...");

        let manifest_exports = &self.manifest.exports;

        if dry_run {
            let total_vars = manifest_exports.len() + 3; // +3 for stack_name, stack_env, elapsed_time
            info!(
                "dry run: would export {} variables to {} (including automatic stack_name, stack_env, and elapsed_time)",
                total_vars, output_file
            );
            return;
        }

        let mut export_data = serde_json::Map::new();
        let mut missing_vars = Vec::new();

        // Always include stack metadata
        export_data.insert(
            "stack_name".to_string(),
            serde_json::Value::String(self.stack_name.clone()),
        );
        export_data.insert(
            "stack_env".to_string(),
            serde_json::Value::String(self.stack_env.clone()),
        );

        for var_name in manifest_exports {
            if var_name == "stack_name" || var_name == "stack_env" {
                continue;
            }

            if let Some(value) = self.global_context.get(var_name) {
                // Try to parse as JSON
                if value.starts_with('[') || value.starts_with('{') {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value) {
                        export_data.insert(var_name.clone(), parsed);
                        continue;
                    }
                }
                export_data.insert(var_name.clone(), serde_json::Value::String(value.clone()));
            } else {
                missing_vars.push(var_name.clone());
            }
        }

        if !missing_vars.is_empty() {
            catch_error_and_exit(&format!(
                "Exports failed: variables not found in context: {:?}",
                missing_vars
            ));
        }

        // Add elapsed_time
        export_data.insert(
            "elapsed_time".to_string(),
            serde_json::Value::String(elapsed_time.to_string()),
        );

        // Ensure directory exists
        if let Some(parent) = Path::new(output_file).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    catch_error_and_exit(&format!(
                        "Failed to create directory for output file: {}",
                        e
                    ));
                }
            }
        }

        // Write JSON file
        let json = serde_json::Value::Object(export_data.clone());
        match fs::write(output_file, serde_json::to_string_pretty(&json).unwrap()) {
            Ok(_) => info!(
                "Exported {} variables to {}",
                export_data.len(),
                output_file
            ),
            Err(e) => catch_error_and_exit(&format!(
                "Failed to write exports file {}: {}",
                output_file, e
            )),
        }
    }
}

/// Evaluate a simple condition expression.
/// Supports: 'value1' == 'value2', 'value1' != 'value2', true, false
fn evaluate_simple_condition(condition: &str) -> Option<bool> {
    let trimmed = condition.trim();

    // Direct boolean values
    if trimmed == "true" || trimmed == "True" {
        return Some(true);
    }
    if trimmed == "false" || trimmed == "False" {
        return Some(false);
    }

    // Equality check: 'a' == 'b'
    if let Some((left, right)) = trimmed.split_once("==") {
        let l = left.trim().trim_matches('\'').trim_matches('"');
        let r = right.trim().trim_matches('\'').trim_matches('"');
        return Some(l == r);
    }

    // Inequality check: 'a' != 'b'
    if let Some((left, right)) = trimmed.split_once("!=") {
        let l = left.trim().trim_matches('\'').trim_matches('"');
        let r = right.trim().trim_matches('\'').trim_matches('"');
        return Some(l != r);
    }

    // `in` check: 'a' in ['a', 'b']
    if trimmed.contains(" in ") {
        let parts: Vec<&str> = trimmed.splitn(2, " in ").collect();
        if parts.len() == 2 {
            let needle = parts[0].trim().trim_matches('\'').trim_matches('"');
            let haystack = parts[1].trim();
            // Simple list check
            if haystack.starts_with('[') && haystack.ends_with(']') {
                let items: Vec<&str> = haystack[1..haystack.len() - 1]
                    .split(',')
                    .map(|s| s.trim().trim_matches('\'').trim_matches('"'))
                    .collect();
                return Some(items.contains(&needle));
            }
        }
    }

    // `not in` check
    if trimmed.contains(" not in ") {
        let parts: Vec<&str> = trimmed.splitn(2, " not in ").collect();
        if parts.len() == 2 {
            let needle = parts[0].trim().trim_matches('\'').trim_matches('"');
            let haystack = parts[1].trim();
            if haystack.starts_with('[') && haystack.ends_with(']') {
                let items: Vec<&str> = haystack[1..haystack.len() - 1]
                    .split(',')
                    .map(|s| s.trim().trim_matches('\'').trim_matches('"'))
                    .collect();
                return Some(!items.contains(&needle));
            }
        }
    }

    None
}
