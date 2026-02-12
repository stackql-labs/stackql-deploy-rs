// lib/templating.rs

//! # Templating Module
//!
//! Handles loading, parsing, and rendering SQL query templates from .iql files.
//! Matches the Python `lib/templating.py` implementation.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process;

use log::{debug, error};

use crate::core::config::prepare_query_context;
use crate::resource::manifest::Resource;
use crate::template::engine::TemplateEngine;

/// Parsed query with its rendered form and options.
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub template: String,
    pub rendered: String,
    pub options: QueryOptions,
}

/// Options for a query anchor.
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    pub retries: u32,
    pub retry_delay: u32,
    pub postdelete_retries: u32,
    pub postdelete_retry_delay: u32,
}

/// Parse an anchor line to extract key and options.
/// Matches Python's `parse_anchor`.
fn parse_anchor(anchor: &str) -> (String, HashMap<String, u32>) {
    let parts: Vec<&str> = anchor.split(',').collect();
    let key = parts[0].trim().to_lowercase();
    let mut options = HashMap::new();

    for part in &parts[1..] {
        if let Some((option_key, option_value)) = part.split_once('=') {
            if let Ok(value) = option_value.trim().parse::<u32>() {
                options.insert(option_key.trim().to_string(), value);
            }
        }
    }

    (key, options)
}

/// Load SQL queries from a .iql file, split by anchors.
/// Matches Python's `load_sql_queries`.
fn load_sql_queries(file_path: &Path) -> (HashMap<String, String>, HashMap<String, HashMap<String, u32>>) {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to read query file {:?}: {}", file_path, e);
            process::exit(1);
        }
    };

    let mut queries: HashMap<String, String> = HashMap::new();
    let mut options: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let mut current_anchor: Option<String> = None;
    let mut query_buffer: Vec<String> = Vec::new();

    for line in content.lines() {
        if line.trim_start().starts_with("/*+") && line.contains("*/") {
            // Store the current query under the last anchor
            if let Some(ref anchor) = current_anchor {
                if !query_buffer.is_empty() {
                    let (anchor_key, anchor_options) = parse_anchor(anchor);
                    queries.insert(anchor_key.clone(), query_buffer.join("\n").trim().to_string());
                    options.insert(anchor_key, anchor_options);
                    query_buffer.clear();
                }
            }
            // Extract new anchor
            let start = line.find("/*+").unwrap() + 3;
            let end = line.find("*/").unwrap();
            current_anchor = Some(line[start..end].trim().to_string());
        } else {
            query_buffer.push(line.to_string());
        }
    }

    // Store the last query
    if let Some(ref anchor) = current_anchor {
        if !query_buffer.is_empty() {
            let (anchor_key, anchor_options) = parse_anchor(anchor);
            queries.insert(anchor_key.clone(), query_buffer.join("\n").trim().to_string());
            options.insert(anchor_key, anchor_options);
        }
    }

    (queries, options)
}

/// Render query templates with the full context.
/// Matches Python's `render_queries`.
fn render_queries(
    engine: &TemplateEngine,
    res_name: &str,
    queries: &HashMap<String, String>,
    context: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut rendered_queries = HashMap::new();

    // Prepare context: re-serialize JSON values
    let temp_context = prepare_query_context(context);

    for (key, query) in queries {
        debug!("[{}] [{}] query template:\n\n{}\n", res_name, key, query);

        match engine.render(query, &temp_context) {
            Ok(rendered) => {
                debug!(
                    "[{}] [{}] rendered query:\n\n{}\n",
                    res_name, key, rendered
                );
                rendered_queries.insert(key.clone(), rendered);
            }
            Err(e) => {
                error!(
                    "Error rendering query for [{}] [{}]: {}",
                    res_name, key, e
                );
                process::exit(1);
            }
        }
    }

    rendered_queries
}

/// Get queries for a resource: load from file, parse anchors, render with context.
/// Matches Python's `get_queries`.
pub fn get_queries(
    engine: &TemplateEngine,
    stack_dir: &str,
    resource: &Resource,
    full_context: &HashMap<String, String>,
) -> HashMap<String, ParsedQuery> {
    let mut result = HashMap::new();

    let template_path = if let Some(ref file) = resource.file {
        Path::new(stack_dir).join("resources").join(file)
    } else {
        Path::new(stack_dir)
            .join("resources")
            .join(format!("{}.iql", resource.name))
    };

    if !template_path.exists() {
        error!("Query file not found: {:?}", template_path);
        process::exit(1);
    }

    let (query_templates, query_options) = load_sql_queries(&template_path);
    let rendered_queries = render_queries(engine, &resource.name, &query_templates, full_context);

    for (anchor, template) in &query_templates {
        // Fix backward compatibility for preflight and postdeploy
        let normalized_anchor = match anchor.as_str() {
            "preflight" => "exists".to_string(),
            "postdeploy" => "statecheck".to_string(),
            other => other.to_string(),
        };

        let opts = query_options.get(anchor).cloned().unwrap_or_default();
        let rendered = rendered_queries.get(anchor).cloned().unwrap_or_default();

        result.insert(
            normalized_anchor.clone(),
            ParsedQuery {
                template: template.clone(),
                rendered,
                options: QueryOptions {
                    retries: *opts.get("retries").unwrap_or(&1),
                    retry_delay: *opts.get("retry_delay").unwrap_or(&0),
                    postdelete_retries: *opts.get("postdelete_retries").unwrap_or(&10),
                    postdelete_retry_delay: *opts.get("postdelete_retry_delay").unwrap_or(&5),
                },
            },
        );
    }

    debug!(
        "Queries for [{}]: {:?}",
        resource.name,
        result.keys().collect::<Vec<_>>()
    );
    result
}

/// Render an inline SQL template string.
/// Matches Python's `render_inline_template`.
pub fn render_inline_template(
    engine: &TemplateEngine,
    resource_name: &str,
    template_string: &str,
    full_context: &HashMap<String, String>,
) -> String {
    debug!(
        "[{}] inline template:\n\n{}\n",
        resource_name, template_string
    );

    let temp_context = prepare_query_context(full_context);

    match engine.render(template_string, &temp_context) {
        Ok(rendered) => {
            debug!(
                "[{}] rendered inline template:\n\n{}\n",
                resource_name, rendered
            );
            rendered
        }
        Err(e) => {
            error!(
                "Error rendering inline template for [{}]: {}",
                resource_name, e
            );
            process::exit(1);
        }
    }
}
