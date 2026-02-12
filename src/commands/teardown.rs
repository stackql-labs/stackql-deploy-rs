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
use crate::core::utils::catch_error_and_exit;
use crate::utils::connection::create_client;
use crate::utils::display::{print_unicode_box, BorderColor};
use crate::utils::logging::initialize_logger;

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
    let log_level_val = matches.get_one::<String>("log-level").unwrap();
    let env_file_val = matches.get_one::<String>("env-file").unwrap();
    let env_vars: Vec<String> = matches
        .get_many::<String>("env")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let is_dry_run = matches.get_flag("dry-run");
    let is_show_queries = matches.get_flag("show-queries");
    let on_failure_val = matches.get_one::<FailureAction>("on-failure").unwrap();

    initialize_logger(log_level_val);

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

    println!("teardown complete (dry run: {})", is_dry_run);
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

        let full_context = runner.get_full_context(resource);

        if res_type == "command" {
            continue;
        }

        let (exports_query, exports_retries, exports_retry_delay) =
            if let Some(sql_val) = resource.sql.as_ref().filter(|_| res_type == "query") {
                let iq = runner.render_inline_template(&resource.name, sql_val, &full_context);
                (Some(iq), 1u32, 0u32)
            } else {
                let queries = runner.get_queries(resource, &full_context);
                if let Some(eq) = queries.get("exports") {
                    (
                        Some(eq.rendered.clone()),
                        eq.options.retries,
                        eq.options.retry_delay,
                    )
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

        // Get resource queries
        let resource_queries = runner.get_queries(resource, &full_context);

        // Get exists query (fallback to statecheck)
        let (
            exists_query_str,
            exists_retries,
            exists_retry_delay,
            postdelete_retries,
            postdelete_retry_delay,
        ) = if let Some(eq) = resource_queries.get("exists") {
            (
                eq.rendered.clone(),
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
            (
                sq.rendered.clone(),
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

        // Get delete query
        let (delete_query, delete_retries, delete_retry_delay) =
            if let Some(dq) = resource_queries.get("delete") {
                (
                    dq.rendered.clone(),
                    dq.options.retries,
                    dq.options.retry_delay,
                )
            } else {
                info!(
                    "delete query not defined for [{}], skipping...",
                    resource.name
                );
                continue;
            };

        // Pre-delete check
        let ignore_errors = res_type == "multi";
        let resource_exists = if res_type == "multi" {
            info!("pre-delete check not supported for multi resources, skipping...");
            true
        } else {
            runner.check_if_resource_exists(
                resource,
                &exists_query_str,
                exists_retries,
                exists_retry_delay,
                dry_run,
                show_queries,
                false,
            )
        };

        // Delete
        if resource_exists {
            runner.delete_resource(
                resource,
                &delete_query,
                delete_retries,
                delete_retry_delay,
                dry_run,
                show_queries,
                ignore_errors,
            );
        } else {
            info!(
                "resource [{}] does not exist, skipping delete",
                resource.name
            );
            continue;
        }

        // Confirm deletion
        let resource_deleted = runner.check_if_resource_exists(
            resource,
            &exists_query_str,
            postdelete_retries,
            postdelete_retry_delay,
            dry_run,
            show_queries,
            true, // delete_test
        );

        if resource_deleted {
            info!("successfully deleted {}", resource.name);
        } else if !dry_run {
            catch_error_and_exit(&format!("failed to delete {}.", resource.name));
        }
    }

    let elapsed = start_time.elapsed();
    info!("teardown completed in {:.2?}", elapsed);
}
