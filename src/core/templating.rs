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
use regex::Regex;

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
fn load_sql_queries(
    file_path: &Path,
) -> (
    HashMap<String, String>,
    HashMap<String, HashMap<String, u32>>,
) {
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
                    queries.insert(
                        anchor_key.clone(),
                        query_buffer.join("\n").trim().to_string(),
                    );
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
            queries.insert(
                anchor_key.clone(),
                query_buffer.join("\n").trim().to_string(),
            );
            options.insert(anchor_key, anchor_options);
        }
    }

    (queries, options)
}

/// Pre-process Jinja2 inline dict expressions that Tera doesn't support.
///
/// Converts patterns like `{{ { "Key": var, ... } | filter }}` into
/// Tera-compatible form by resolving the dict from context variables
/// and injecting the result as a temporary context variable.
///
/// For example:
///   `{{ { "Description": description, "Path": path } | generate_patch_document }}`
/// becomes:
///   `{{ __inline_dict_0 | generate_patch_document }}`
/// with `__inline_dict_0` set to the constructed JSON object in context.
fn preprocess_inline_dicts(template: &str, context: &mut HashMap<String, String>) -> String {
    // Match {{ { ... } | filter_name }}
    // This regex captures the dict body and the filter expression
    let re = Regex::new(r"\{\{\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}\s*\|\s*(\w+)\s*\}\}").unwrap();

    let mut result = template.to_string();
    let mut counter = 0;

    // We need to iterate carefully since we're modifying the string
    loop {
        let captures = re.captures(&result);
        if captures.is_none() {
            break;
        }
        let caps = captures.unwrap();
        let full_match = caps.get(0).unwrap();
        let dict_body = caps.get(1).unwrap().as_str().trim();
        let filter_name = caps.get(2).unwrap().as_str();

        // Parse the dict body: "Key": var, "Key2": var2
        let mut obj = serde_json::Map::new();
        for entry in split_dict_entries(dict_body) {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            if let Some((key_part, val_part)) = entry.split_once(':') {
                let key = key_part
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                let var_name = val_part.trim();

                // Look up the variable in context
                let value = context.get(var_name).cloned().unwrap_or_default();

                // Try to parse as JSON, otherwise use as string
                let json_val = match serde_json::from_str::<serde_json::Value>(&value) {
                    Ok(v) => v,
                    Err(_) => serde_json::Value::String(value),
                };
                obj.insert(key, json_val);
            }
        }

        let var_name = format!("__inline_dict_{}", counter);
        let json_str = serde_json::to_string(&serde_json::Value::Object(obj)).unwrap_or_default();
        context.insert(var_name.clone(), json_str);

        let replacement = format!("{{{{ {} | {} }}}}", var_name, filter_name);
        result = format!(
            "{}{}{}",
            &result[..full_match.start()],
            replacement,
            &result[full_match.end()..]
        );
        counter += 1;
    }

    result
}

/// Split dict entries by commas, but respect nested braces and quoted strings.
fn split_dict_entries(s: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = String::new();
    let mut brace_depth = 0;
    let mut in_quote = false;
    let mut quote_char = ' ';

    for ch in s.chars() {
        match ch {
            '"' | '\'' if !in_quote => {
                in_quote = true;
                quote_char = ch;
                current.push(ch);
            }
            c if in_quote && c == quote_char => {
                in_quote = false;
                current.push(ch);
            }
            '{' if !in_quote => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' if !in_quote => {
                brace_depth -= 1;
                current.push(ch);
            }
            ',' if !in_quote && brace_depth == 0 => {
                entries.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.trim().is_empty() {
        entries.push(current.trim().to_string());
    }
    entries
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

        // Pre-process inline dict expressions and render with filters
        let mut ctx = temp_context.clone();
        let processed_query = preprocess_inline_dicts(query, &mut ctx);

        let template_name = format!("{}__{}", res_name, key);
        match engine.render_with_filters(&template_name, &processed_query, &ctx) {
            Ok(rendered) => {
                debug!("[{}] [{}] rendered query:\n\n{}\n", res_name, key, rendered);
                rendered_queries.insert(key.clone(), rendered);
            }
            Err(e) => {
                error!("Error rendering query for [{}] [{}]: {}", res_name, key, e);
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

    let mut temp_context = prepare_query_context(full_context);
    let processed = preprocess_inline_dicts(template_string, &mut temp_context);
    let template_name = format!("{}__inline", resource_name);

    match engine.render_with_filters(&template_name, &processed, &temp_context) {
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
