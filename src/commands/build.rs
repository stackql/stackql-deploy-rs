// commands/build.rs

//! # Build Command
//!
//! Implements the `build` (deploy) command. Creates or updates infrastructure
//! resources defined in a stack manifest.
//! This is the Rust equivalent of Python's `cmd/build.py` `StackQLProvisioner`.

use std::collections::HashMap;
use std::time::Instant;

use clap::{Arg, ArgMatches, Command};
use log::info;

use crate::commands::base::CommandRunner;
use crate::commands::common_args::{
    dry_run, env_file, env_var, log_level, on_failure, show_queries, stack_dir, stack_env,
    FailureAction,
};
use crate::core::config::get_resource_type;
use crate::core::utils::catch_error_and_exit;
use crate::utils::connection::create_client;
use crate::utils::display::{print_unicode_box, BorderColor};
use crate::utils::server::check_and_start_server;

/// Defines the `build` command for the CLI application.
pub fn command() -> Command {
    Command::new("build")
        .about("Create or update resources")
        .arg(stack_dir())
        .arg(stack_env())
        .arg(log_level())
        .arg(env_file())
        .arg(env_var())
        .arg(dry_run())
        .arg(show_queries())
        .arg(on_failure())
        .arg(
            Arg::new("output-file")
                .long("output-file")
                .help("File path to write deployment outputs as JSON")
                .num_args(1),
        )
}

/// Executes the `build` command.
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
    let output_file = matches.get_one::<String>("output-file");

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
            "Deploying stack: [{}] to environment: [{}]",
            stack_name_display, stack_env_val
        ),
        BorderColor::Yellow,
    );

    run_build(
        &mut runner,
        is_dry_run,
        is_show_queries,
        &format!("{:?}", on_failure_val),
        output_file.map(|s| s.as_str()),
    );

    if is_dry_run {
        println!("dry-run build complete");
    } else {
        println!("build complete");
    }
}

/// Main build workflow matching Python's StackQLProvisioner.run().
fn run_build(
    runner: &mut CommandRunner,
    dry_run: bool,
    show_queries: bool,
    _on_failure: &str,
    output_file: Option<&str>,
) {
    let start_time = Instant::now();

    info!(
        "deploying [{}] in [{}] environment {}",
        runner.stack_name,
        runner.stack_env,
        if dry_run { "(dry run)" } else { "" }
    );

    let resources = runner.manifest.resources.clone();

    for resource in &resources {
        print_unicode_box(
            &format!("Processing resource: [{}]", resource.name),
            BorderColor::Blue,
        );

        let res_type = get_resource_type(resource).to_string();
        info!(
            "processing resource [{}], type: {}",
            resource.name, res_type
        );

        let full_context = runner.get_full_context(resource);

        // Evaluate condition
        if !runner.evaluate_condition(resource, &full_context) {
            continue;
        }

        // Handle script type
        if res_type == "script" {
            runner.process_script_resource(resource, dry_run, &full_context);
            continue;
        }

        // Get resource queries (templates only, not yet rendered)
        let (resource_queries, inline_query) = if let Some(sql_val) = resource
            .sql
            .as_ref()
            .filter(|_| res_type == "command" || res_type == "query")
        {
            let iq = runner.render_inline_template(&resource.name, sql_val, &full_context);
            (HashMap::new(), Some(iq))
        } else {
            (runner.get_queries(resource, &full_context), None)
        };

        // Provisioning queries for resource/multi types
        let mut create_query: Option<String> = None;
        let mut create_retries = 1u32;
        let mut create_retry_delay = 0u32;
        let mut update_query: Option<String> = None;
        let mut update_retries = 1u32;
        let mut update_retry_delay = 0u32;
        let mut has_createorupdate = false;

        if res_type == "resource" || res_type == "multi" {
            if let Some(cou) = resource_queries.get("createorupdate") {
                has_createorupdate = true;
                let rendered = runner.render_query(
                    &resource.name,
                    "createorupdate",
                    &cou.template,
                    &full_context,
                );
                create_query = Some(rendered.clone());
                create_retries = cou.options.retries;
                create_retry_delay = cou.options.retry_delay;
                update_query = Some(rendered);
                update_retries = cou.options.retries;
                update_retry_delay = cou.options.retry_delay;
            } else {
                if let Some(cq) = resource_queries.get("create") {
                    create_query = Some(runner.render_query(
                        &resource.name,
                        "create",
                        &cq.template,
                        &full_context,
                    ));
                    create_retries = cq.options.retries;
                    create_retry_delay = cq.options.retry_delay;
                }
                if let Some(uq) = resource_queries.get("update") {
                    update_query = Some(runner.render_query(
                        &resource.name,
                        "update",
                        &uq.template,
                        &full_context,
                    ));
                    update_retries = uq.options.retries;
                    update_retry_delay = uq.options.retry_delay;
                }
            }

            if create_query.is_none() {
                catch_error_and_exit(
                    "iql file must include either 'create' or 'createorupdate' anchor.",
                );
            }
        }

        // Test queries - render only the ones we need
        let exists_query = resource_queries.get("exists").map(|q| {
            let rendered =
                runner.render_query(&resource.name, "exists", &q.template, &full_context);
            (rendered, q.options.clone())
        });
        let statecheck_query = resource_queries.get("statecheck").map(|q| {
            let rendered =
                runner.render_query(&resource.name, "statecheck", &q.template, &full_context);
            (rendered, q.options.clone())
        });
        let mut exports_query_str: Option<String> = resource_queries
            .get("exports")
            .map(|q| runner.render_query(&resource.name, "exports", &q.template, &full_context));
        let exports_opts = resource_queries.get("exports");
        let exports_retries = exports_opts.map_or(1, |q| q.options.retries);
        let exports_retry_delay = exports_opts.map_or(0, |q| q.options.retry_delay);

        // Handle query type with no exports
        if res_type == "query" && exports_query_str.is_none() {
            if let Some(ref iq) = inline_query {
                exports_query_str = Some(iq.clone());
            } else {
                catch_error_and_exit(
                    "Inline sql must be supplied or an iql file must be present with an 'exports' anchor for query type resources.",
                );
            }
        }

        let mut exports_result_from_proxy: Option<Vec<HashMap<String, String>>> = None;

        if res_type == "resource" || res_type == "multi" {
            let ignore_errors = res_type == "multi";
            let mut resource_exists = false;
            let mut is_correct_state = false;

            // State checking logic
            if has_createorupdate {
                // Skip all existence and state checks for createorupdate
            } else if statecheck_query.is_some() {
                // Flow 1: Traditional flow when statecheck exists
                if let Some(ref eq) = exists_query {
                    let eq_opts = resource_queries.get("exists").unwrap();
                    resource_exists = runner.check_if_resource_exists(
                        resource,
                        &eq.0,
                        eq_opts.options.retries,
                        eq_opts.options.retry_delay,
                        dry_run,
                        show_queries,
                        false,
                    );
                } else {
                    // Use statecheck as exists check
                    let sq = statecheck_query.as_ref().unwrap();
                    let sq_opts = resource_queries.get("statecheck").unwrap();
                    is_correct_state = runner.check_if_resource_is_correct_state(
                        resource,
                        &sq.0,
                        sq_opts.options.retries,
                        sq_opts.options.retry_delay,
                        dry_run,
                        show_queries,
                    );
                    resource_exists = is_correct_state;
                }

                // Pre-deployment state check for existing resources
                if resource_exists && !is_correct_state {
                    if resource.skip_validation.unwrap_or(false) {
                        info!(
                            "skipping validation for [{}] as skip_validation is set to true.",
                            resource.name
                        );
                        is_correct_state = true;
                    } else {
                        let sq = statecheck_query.as_ref().unwrap();
                        let sq_opts = resource_queries.get("statecheck").unwrap();
                        is_correct_state = runner.check_if_resource_is_correct_state(
                            resource,
                            &sq.0,
                            sq_opts.options.retries,
                            sq_opts.options.retry_delay,
                            dry_run,
                            show_queries,
                        );
                    }
                }
            } else if let Some(ref eq_str) = exports_query_str {
                // Flow 2: Optimized flow using exports as proxy
                info!(
                    "trying exports query first (fast-fail) for optimal validation for [{}]",
                    resource.name
                );
                let (state, proxy_result) = runner.check_state_using_exports_proxy(
                    resource,
                    eq_str,
                    1,
                    0,
                    dry_run,
                    show_queries,
                );
                is_correct_state = state;
                resource_exists = is_correct_state;

                if is_correct_state {
                    info!(
                        "[{}] validated successfully with fast exports query",
                        resource.name
                    );
                    exports_result_from_proxy = proxy_result;
                } else {
                    info!(
                        "fast exports validation failed, falling back to exists check for [{}]",
                        resource.name
                    );
                    exports_result_from_proxy = None;

                    if let Some(ref eq) = exists_query {
                        let eq_opts = resource_queries.get("exists").unwrap();
                        resource_exists = runner.check_if_resource_exists(
                            resource,
                            &eq.0,
                            eq_opts.options.retries,
                            eq_opts.options.retry_delay,
                            dry_run,
                            show_queries,
                            false,
                        );
                    } else {
                        resource_exists = false;
                    }
                }
            } else if let Some(ref eq) = exists_query {
                // Flow 3: Basic flow with only exists query
                let eq_opts = resource_queries.get("exists").unwrap();
                resource_exists = runner.check_if_resource_exists(
                    resource,
                    &eq.0,
                    eq_opts.options.retries,
                    eq_opts.options.retry_delay,
                    dry_run,
                    show_queries,
                    false,
                );
            } else {
                catch_error_and_exit(
                    "iql file must include either 'exists', 'statecheck', or 'exports' anchor.",
                );
            }

            // Create or update
            let mut is_created_or_updated = false;

            if !resource_exists {
                let (created, returning_row) = runner.create_resource(
                    resource,
                    create_query.as_ref().unwrap(),
                    create_retries,
                    create_retry_delay,
                    dry_run,
                    show_queries,
                    ignore_errors,
                );
                is_created_or_updated = created;

                // Capture RETURNING * result.
                if let Some(ref row) = returning_row {
                    runner.store_callback_data(&resource.name, row);
                }

                // Run callback:create block if present.
                if is_created_or_updated {
                    let cb_anchor = if resource_queries.contains_key("callback:create") {
                        Some("callback:create")
                    } else if resource_queries.contains_key("callback") {
                        Some("callback")
                    } else {
                        None
                    };
                    if let Some(anchor) = cb_anchor {
                        // Pre-extract before the mutable borrow of runner.
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
                                "create",
                                dry_run,
                                show_queries,
                            );
                        }
                    }
                }
            }

            if resource_exists && !is_correct_state {
                let (updated, returning_row) = runner.update_resource(
                    resource,
                    update_query.as_deref(),
                    update_retries,
                    update_retry_delay,
                    dry_run,
                    show_queries,
                    ignore_errors,
                );
                is_created_or_updated = updated;

                // Capture RETURNING * result.
                if let Some(ref row) = returning_row {
                    runner.store_callback_data(&resource.name, row);
                }

                // Run callback:update block if present.
                if is_created_or_updated {
                    let cb_anchor = if resource_queries.contains_key("callback:update") {
                        Some("callback:update")
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
                                "update",
                                dry_run,
                                show_queries,
                            );
                        }
                    }
                }
            }

            // Post-deploy state check
            if is_created_or_updated {
                if let Some(ref sq) = statecheck_query {
                    let sq_opts = resource_queries.get("statecheck").unwrap();
                    is_correct_state = runner.check_if_resource_is_correct_state(
                        resource,
                        &sq.0,
                        sq_opts.options.retries,
                        sq_opts.options.retry_delay,
                        dry_run,
                        show_queries,
                    );
                } else if let Some(ref eq_str) = exports_query_str {
                    info!(
                        "using exports query as post-deploy statecheck for [{}]",
                        resource.name
                    );
                    let post_retries = exports_retries;
                    let post_delay = exports_retry_delay;

                    let (state, proxy) = runner.check_state_using_exports_proxy(
                        resource,
                        eq_str,
                        post_retries,
                        post_delay,
                        dry_run,
                        show_queries,
                    );
                    is_correct_state = state;
                    if proxy.is_some() {
                        exports_result_from_proxy = proxy;
                    }
                }
            }

            if !is_correct_state && !dry_run {
                catch_error_and_exit(&format!(
                    "deployment failed for {} after post-deploy checks.",
                    resource.name
                ));
            }
        }

        // Handle command type
        if res_type == "command" {
            let (command_query, command_retries, command_retry_delay) = if let Some(ref iq) =
                inline_query
            {
                (iq.clone(), 1u32, 0u32)
            } else if let Some(cq) = resource_queries.get("command") {
                let rendered =
                    runner.render_query(&resource.name, "command", &cq.template, &full_context);
                (rendered, cq.options.retries, cq.options.retry_delay)
            } else {
                catch_error_and_exit(
                        "'sql' should be defined in the resource or the 'command' anchor needs to be supplied in the corresponding iql file for command type resources.",
                    );
            };

            runner.run_command(
                &command_query,
                command_retries,
                command_retry_delay,
                dry_run,
                show_queries,
            );
        }

        // Process exports with optimization
        if let Some(ref eq_str) = exports_query_str {
            if let Some(ref proxy_result) = exports_result_from_proxy {
                if res_type == "resource" || res_type == "multi" {
                    info!(
                        "reusing exports result from proxy for [{}]...",
                        resource.name
                    );
                    if !resource.exports.is_empty() {
                        runner.process_exports_from_result(resource, proxy_result);
                    }
                }
            } else {
                runner.process_exports(
                    resource,
                    &full_context,
                    eq_str,
                    exports_retries,
                    exports_retry_delay,
                    dry_run,
                    show_queries,
                    false,
                );
            }
        }

        if !dry_run {
            if res_type == "resource" {
                info!("successfully deployed {}", resource.name);
            } else if res_type == "query" {
                info!(
                    "successfully exported variables for query in {}",
                    resource.name
                );
            }
        }
    }

    let elapsed = start_time.elapsed();
    let elapsed_str = format!("{:.2?}", elapsed);
    info!("deployment completed in {}", elapsed_str);

    runner.process_stack_exports(dry_run, output_file, &elapsed_str);
}
