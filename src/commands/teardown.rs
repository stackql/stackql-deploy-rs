// commands/teardown.rs

//! # Teardown Command
//!
//! Implements the `teardown` command. Destroys provisioned resources in reverse order.
//! This is the Rust equivalent of Python's `cmd/teardown.py` `StackQLDeProvisioner`.

use std::time::Instant;

use clap::{ArgMatches, Command};
use log::{debug, info};

use crate::commands::base::CommandRunner;
use crate::commands::common_args::{
    dry_run, env_file, env_var, log_level, on_failure, show_queries, stack_dir, stack_env,
    FailureAction,
};
use crate::core::config::get_resource_type;
use crate::utils::connection::create_client;
use crate::utils::display::{print_unicode_box, BorderColor};
use crate::utils::server::{check_and_start_server, stop_local_server};

/// Configures the `teardown` command for the CLI application.
pub fn command() -> Command {
    Command::new("teardown")
        .about("Teardown a provisioned stack")
        .arg(stack_dir())
        .arg(stack_env())
        .arg(log_level())
        .arg(env_file())
        .arg(env_var())
        .arg(dry_run())
        .arg(show_queries())
        .arg(on_failure())
}

/// Executes the `teardown` command.
pub fn execute(matches: &ArgMatches) {
    let stack_dir_val = matches.get_one::<String>("stack_dir").unwrap();
    let stack_env_val = matches.get_one::<String>("stack_env").unwrap();
    let env_file_val = matches.get_one::<String>("env-file").unwrap();
    let env_vars: Vec<String> = matches
        .get_many::<String>("env")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let is_dry_run = matches.get_flag("dry-run");
    let is_show_queries = matches.get_flag("show-queries");
    let on_failure_val = matches.get_one::<FailureAction>("on-failure").unwrap();

    check_and_start_server();
    let client = create_client();
    let mut runner = CommandRunner::new(
        client,
        stack_dir_val,
        stack_env_val,
        env_file_val,
        &env_vars,
    );

    let stack_name_display = if runner.stack_name.is_empty() {
        runner.stack_dir.clone()
    } else {
        runner.stack_name.clone()
    };

    print_unicode_box(
        &format!(
            "Tearing down stack: [{}] in environment: [{}]",
            stack_name_display, stack_env_val
        ),
        BorderColor::Yellow,
    );

    run_teardown(
        &mut runner,
        is_dry_run,
        is_show_queries,
        &format!("{:?}", on_failure_val),
    );

    if is_dry_run {
        print_unicode_box("dry-run teardown complete", BorderColor::Green);
    } else {
        print_unicode_box("teardown complete", BorderColor::Green);
    }

    stop_local_server();
}

/// Collect exports for all resources before teardown.
fn collect_exports(runner: &mut CommandRunner, show_queries: bool, dry_run: bool) {
    info!(
        "collecting exports for [{}] in [{}] environment",
        runner.stack_name, runner.stack_env
    );

    let resources = runner.manifest.resources.clone();

    for resource in &resources {
        let res_type = get_resource_type(resource).to_string();
        info!("getting exports for resource [{}]", resource.name);

        let mut full_context = runner.get_full_context(resource);

        if res_type == "command" {
            continue;
        }

        let (exports_query, exports_retries, exports_retry_delay) =
            if let Some(sql_val) = resource.sql.as_ref().filter(|_| res_type == "query") {
                let iq = runner.render_inline_template(&resource.name, sql_val, &full_context);
                (Some(iq), 1u32, 0u32)
            } else {
                let queries = runner.get_queries(resource, &full_context);
                // Run exists query first to capture this.* fields needed by
                // exports (e.g. this.identifier).
                if let Some(eq) = queries.get("exists") {
                    let rendered =
                        runner.render_query(&resource.name, "exists", &eq.template, &full_context);
                    let (_exists, fields) = runner.check_if_resource_exists(
                        resource,
                        &rendered,
                        eq.options.retries,
                        eq.options.retry_delay,
                        dry_run,
                        show_queries,
                        false,
                    );
                    if let Some(ref f) = fields {
                        for (k, v) in f {
                            full_context.insert(format!("{}.{}", resource.name, k), v.clone());
                        }
                    }
                }
                if let Some(eq) = queries.get("exports") {
                    let rendered =
                        runner.render_query(&resource.name, "exports", &eq.template, &full_context);
                    // During teardown use minimal retries - the resource may
                    // already be partially deleted.
                    (Some(rendered), 1u32, 0u32)
                } else {
                    (None, 1u32, 0u32)
                }
            };

        if let Some(ref eq_str) = exports_query {
            runner.process_exports(
                resource,
                &full_context,
                eq_str,
                exports_retries,
                exports_retry_delay,
                dry_run,
                show_queries,
                true, // ignore_missing_exports
            );
        }
    }
}

/// Main teardown workflow matching Python's StackQLDeProvisioner.run().
fn run_teardown(runner: &mut CommandRunner, dry_run: bool, show_queries: bool, _on_failure: &str) {
    let start_time = Instant::now();

    info!(
        "tearing down [{}] in [{}] environment {}",
        runner.stack_name,
        runner.stack_env,
        if dry_run { "(dry run)" } else { "" }
    );

    // Collect all exports first
    collect_exports(runner, show_queries, dry_run);

    // Process resources in reverse order
    let resources: Vec<_> = runner
        .manifest
        .resources
        .clone()
        .into_iter()
        .rev()
        .collect();

    for resource in &resources {
        print_unicode_box(
            &format!("Processing resource: [{}]", resource.name),
            BorderColor::Red,
        );

        let res_type = get_resource_type(resource).to_string();

        if res_type != "resource" && res_type != "multi" {
            debug!("skipping resource [{}] (type: {})", resource.name, res_type);
            continue;
        }

        info!(
            "de-provisioning resource [{}], type: {}",
            resource.name, res_type
        );

        let full_context = runner.get_full_context(resource);

        // Evaluate condition
        if !runner.evaluate_condition(resource, &full_context) {
            continue;
        }

        // Add reverse export map variables to full context
        let mut full_context = full_context;
        for export in &resource.exports {
            if let Some(map) = export.as_mapping() {
                for (key_val, lookup_val) in map {
                    let key = key_val.as_str().unwrap_or("");
                    let lookup_key = lookup_val.as_str().unwrap_or("");
                    if let Some(value) = full_context.get(lookup_key).cloned() {
                        full_context.insert(key.to_string(), value);
                    }
                }
            }
        }

        // Get resource queries (templates only)
        let resource_queries = runner.get_queries(resource, &full_context);

        // Get exists query (fallback to statecheck) - render JIT
        let (
            exists_query_str,
            exists_retries,
            exists_retry_delay,
            _postdelete_retries,
            _postdelete_retry_delay,
        ) = if let Some(eq) = resource_queries.get("exists") {
            let rendered =
                runner.render_query(&resource.name, "exists", &eq.template, &full_context);
            (
                rendered,
                eq.options.retries,
                eq.options.retry_delay,
                eq.options.postdelete_retries,
                eq.options.postdelete_retry_delay,
            )
        } else if let Some(sq) = resource_queries.get("statecheck") {
            info!(
                "exists query not defined for [{}], trying statecheck query as exists query.",
                resource.name
            );
            let rendered =
                runner.render_query(&resource.name, "statecheck", &sq.template, &full_context);
            (
                rendered,
                sq.options.retries,
                sq.options.retry_delay,
                sq.options.postdelete_retries,
                sq.options.postdelete_retry_delay,
            )
        } else {
            info!(
                "No exists or statecheck query for [{}], skipping...",
                resource.name
            );
            continue;
        };

        // Check if delete query template exists (don't render yet — may need
        // this.* fields from the exists check).
        let has_delete_query = resource_queries.contains_key("delete");
        if !has_delete_query {
            info!(
                "delete query not defined for [{}], skipping...",
                resource.name
            );
            continue;
        }

        // Pre-delete check
        let ignore_errors = res_type == "multi";
        let resource_exists = if res_type == "multi" {
            info!("pre-delete check not supported for multi resources, skipping...");
            true
        } else {
            let (exists, fields) = runner.check_if_resource_exists(
                resource,
                &exists_query_str,
                exists_retries,
                exists_retry_delay,
                dry_run,
                show_queries,
                false,
            );
            // If the exists query captured fields, inject them as this.* so
            // the delete query can reference them.
            if let Some(ref f) = fields {
                for (k, v) in f {
                    full_context.insert(format!("{}.{}", &resource.name, k), v.clone());
                }
            }
            exists
        };

        // Render the delete query now (after exists fields are available).
        let dq = resource_queries.get("delete").unwrap();
        let delete_query =
            runner.render_query(&resource.name, "delete", &dq.template, &full_context);
        let delete_retries = dq.options.retries;
        let delete_retry_delay = dq.options.retry_delay;

        // Delete
        if resource_exists {
            let returning_row = runner.delete_resource(
                resource,
                &delete_query,
                delete_retries,
                delete_retry_delay,
                dry_run,
                show_queries,
                ignore_errors,
            );

            // Capture RETURNING * result.
            if let Some(ref row) = returning_row {
                runner.store_callback_data(&resource.name, row);
            }

            // Run callback:delete block if present.
            let cb_anchor = if resource_queries.contains_key("callback:delete") {
                Some("callback:delete")
            } else if resource_queries.contains_key("callback") {
                Some("callback")
            } else {
                None
            };
            if let Some(anchor) = cb_anchor {
                if let Some(q) = resource_queries.get(anchor) {
                    let cb_template = q.template.clone();
                    let cb_retries = q.options.retries;
                    let cb_delay = q.options.retry_delay;
                    let cb_sc_field = q.options.short_circuit_field.clone();
                    let cb_sc_value = q.options.short_circuit_value.clone();
                    let cb_ctx = runner.get_full_context(resource);
                    let rendered_cb =
                        runner.render_query(&resource.name, anchor, &cb_template, &cb_ctx);
                    runner.run_callback(
                        resource,
                        &rendered_cb,
                        cb_retries,
                        cb_delay,
                        cb_sc_field.as_deref(),
                        cb_sc_value.as_deref(),
                        "delete",
                        dry_run,
                        show_queries,
                    );
                }
            }
        } else {
            info!(
                "resource [{}] does not exist, skipping delete",
                resource.name
            );
            continue;
        }

        // Confirm deletion - single check, don't poll excessively.
        // Cloud Control deletes are async; if the resource is still
        // visible on the first check that's expected, move on.
        let (still_exists, _) = runner.check_if_resource_exists(
            resource,
            &exists_query_str,
            1,
            0,
            dry_run,
            show_queries,
            true, // delete_test
        );

        if !still_exists {
            info!("successfully deleted {}", resource.name);
        } else {
            info!(
                "[{}] delete dispatched (resource may still be deleting asynchronously)",
                resource.name
            );
        }
    }

    let elapsed = start_time.elapsed();
    info!("teardown completed in {:.2?}", elapsed);
}
