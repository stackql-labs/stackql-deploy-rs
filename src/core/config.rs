// lib/config.rs

//! # Configuration Module
//!
//! Handles loading manifests, rendering global variables, rendering resource
//! properties, and building the full template context. This is the Rust
//! equivalent of the Python `lib/config.py`.

use std::collections::HashMap;
use std::process;

use log::{debug, error};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use crate::resource::manifest::{Manifest, Property};
use crate::template::engine::TemplateEngine;

/// Convert a serde_yaml::Value to a SQL-compatible string representation.
/// Matching Python's `to_sql_compatible_json`.
pub fn to_sql_compatible_value(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => String::new(),
        YamlValue::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        YamlValue::Number(n) => n.to_string(),
        YamlValue::String(s) => s.clone(),
        YamlValue::Sequence(_) | YamlValue::Mapping(_) => {
            // Convert complex types to JSON strings
            let json_val: JsonValue =
                serde_json::to_value(value).unwrap_or(JsonValue::Null);
            serde_json::to_string(&json_val).unwrap_or_default()
        }
        _ => String::new(),
    }
}

/// Convert a rendered value (which may be a string, JSON, etc.) to SQL-compatible format.
/// If the value is already a valid JSON string (object/array), return it as-is.
/// If it's a plain string, return as-is. If it's a bool, normalize to lowercase.
pub fn to_sql_compatible_json(value: &str) -> String {
    // Check if it's a boolean
    if value == "True" || value == "true" {
        return "true".to_string();
    }
    if value == "False" || value == "false" {
        return "false".to_string();
    }
    value.to_string()
}

/// Render a value through the template engine.
/// Matches Python's `render_value` - handles strings, dicts, lists recursively.
pub fn render_value(
    engine: &TemplateEngine,
    value: &YamlValue,
    context: &HashMap<String, String>,
) -> String {
    match value {
        YamlValue::String(s) => {
            match engine.render(s, context) {
                Ok(rendered) => {
                    // Normalize booleans
                    let normalized = rendered
                        .replace("True", "true")
                        .replace("False", "false");
                    normalized
                }
                Err(e) => {
                    debug!("Warning rendering template: {}", e);
                    s.clone()
                }
            }
        }
        YamlValue::Mapping(map) => {
            let mut rendered_map = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    YamlValue::String(s) => s.clone(),
                    _ => format!("{:?}", k),
                };
                let rendered = render_value(engine, v, context);
                // Try to parse as JSON value, otherwise use as string
                match serde_json::from_str::<JsonValue>(&rendered) {
                    Ok(json_val) => {
                        rendered_map.insert(key, json_val);
                    }
                    Err(_) => {
                        rendered_map.insert(key, JsonValue::String(rendered));
                    }
                }
            }
            serde_json::to_string(&JsonValue::Object(rendered_map)).unwrap_or_default()
        }
        YamlValue::Sequence(seq) => {
            let mut rendered_items = Vec::new();
            for item in seq {
                let rendered = render_value(engine, item, context);
                match serde_json::from_str::<JsonValue>(&rendered) {
                    Ok(json_val) => rendered_items.push(json_val),
                    Err(_) => rendered_items.push(JsonValue::String(rendered)),
                }
            }
            serde_json::to_string(&rendered_items).unwrap_or_default()
        }
        YamlValue::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        YamlValue::Number(n) => n.to_string(),
        YamlValue::Null => String::new(),
        _ => String::new(),
    }
}

/// Render a string value through the template engine.
pub fn render_string_value(
    engine: &TemplateEngine,
    value: &str,
    context: &HashMap<String, String>,
) -> String {
    match engine.render(value, context) {
        Ok(rendered) => rendered
            .replace("True", "true")
            .replace("False", "false"),
        Err(e) => {
            debug!("Warning rendering template string: {}", e);
            value.to_string()
        }
    }
}

/// Render global variables from the manifest.
/// Matches Python's `render_globals`.
pub fn render_globals(
    engine: &TemplateEngine,
    vars: &HashMap<String, String>,
    manifest: &Manifest,
    stack_env: &str,
    stack_name: &str,
) -> HashMap<String, String> {
    let mut global_context: HashMap<String, String> = HashMap::new();
    global_context.insert("stack_env".to_string(), stack_env.to_string());
    global_context.insert("stack_name".to_string(), stack_name.to_string());

    debug!("Rendering global variables...");

    for global_var in &manifest.globals {
        // Merge global_context with vars to create complete context
        let mut combined_context = vars.clone();
        for (k, v) in &global_context {
            combined_context.insert(k.clone(), v.clone());
        }

        let rendered = render_value(engine, &global_var.value, &combined_context);

        if rendered.is_empty() {
            error!(
                "Global variable '{}' cannot be empty",
                global_var.name
            );
            process::exit(1);
        }

        let sql_compat = to_sql_compatible_json(&rendered);
        debug!(
            "Setting global variable [{}] to {}",
            global_var.name, sql_compat
        );
        global_context.insert(global_var.name.clone(), sql_compat);
    }

    global_context
}

/// Render resource properties and return the property context.
/// Matches Python's `render_properties`.
pub fn render_properties(
    engine: &TemplateEngine,
    resource_props: &[Property],
    global_context: &HashMap<String, String>,
    stack_env: &str,
) -> HashMap<String, String> {
    let mut prop_context: HashMap<String, String> = HashMap::new();
    let mut resource_context = global_context.clone();

    debug!("Rendering properties...");

    for prop in resource_props {
        // Handle 'value' field
        if let Some(ref value) = prop.value {
            let rendered = render_value(engine, value, &resource_context);
            let sql_compat = to_sql_compatible_json(&rendered);
            debug!("Setting property [{}] to {}", prop.name, sql_compat);
            prop_context.insert(prop.name.clone(), sql_compat.clone());
            resource_context.insert(prop.name.clone(), sql_compat);
        }
        // Handle 'values' (environment-specific)
        else if let Some(ref values) = prop.values {
            if let Some(env_val) = values.get(stack_env) {
                let rendered = render_value(engine, &env_val.value, &resource_context);
                let sql_compat = to_sql_compatible_json(&rendered);
                debug!(
                    "Setting property [{}] using env-specific value to {}",
                    prop.name, sql_compat
                );
                prop_context.insert(prop.name.clone(), sql_compat.clone());
                resource_context.insert(prop.name.clone(), sql_compat);
            } else {
                error!(
                    "No value specified for property '{}' in stack_env '{}'",
                    prop.name, stack_env
                );
                process::exit(1);
            }
        }

        // Handle 'merge' field
        if let Some(ref merge_items) = prop.merge {
            debug!("Processing merge for [{}]", prop.name);

            let base_value_str = prop_context.get(&prop.name).cloned();
            let mut base_value: Option<JsonValue> = base_value_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok());

            for merge_item in merge_items {
                if let Some(merge_value_str) = resource_context.get(merge_item) {
                    if let Ok(merge_value) = serde_json::from_str::<JsonValue>(merge_value_str) {
                        match (&base_value, &merge_value) {
                            (Some(JsonValue::Array(base_arr)), JsonValue::Array(merge_arr)) => {
                                // Merge lists
                                let mut merged = base_arr.clone();
                                let base_set: std::collections::HashSet<String> = base_arr
                                    .iter()
                                    .map(|v| serde_json::to_string(v).unwrap_or_default())
                                    .collect();
                                for item in merge_arr {
                                    let key =
                                        serde_json::to_string(item).unwrap_or_default();
                                    if !base_set.contains(&key) {
                                        merged.push(item.clone());
                                    }
                                }
                                base_value = Some(JsonValue::Array(merged));
                            }
                            (Some(JsonValue::Object(base_obj)), JsonValue::Object(merge_obj)) => {
                                // Merge objects
                                let mut merged = base_obj.clone();
                                for (k, v) in merge_obj {
                                    merged.insert(k.clone(), v.clone());
                                }
                                base_value = Some(JsonValue::Object(merged));
                            }
                            (None, _) => {
                                base_value = Some(merge_value.clone());
                            }
                            _ => {
                                error!(
                                    "Type mismatch or unsupported merge operation on property '{}'",
                                    prop.name
                                );
                                process::exit(1);
                            }
                        }
                    } else {
                        error!(
                            "Merge item '{}' value is not valid JSON",
                            merge_item
                        );
                        process::exit(1);
                    }
                } else {
                    error!(
                        "Merge item '{}' not found in context",
                        merge_item
                    );
                    process::exit(1);
                }
            }

            if let Some(merged_val) = base_value {
                let processed = serde_json::to_string(&merged_val).unwrap_or_default();
                prop_context.insert(prop.name.clone(), processed.clone());
                resource_context.insert(prop.name.clone(), processed);
            }
        }
    }

    prop_context
}

/// Build the full context for a resource by merging global context with resource properties.
/// Matches Python's `get_full_context`.
pub fn get_full_context(
    engine: &TemplateEngine,
    global_context: &HashMap<String, String>,
    resource: &crate::resource::manifest::Resource,
    stack_env: &str,
) -> HashMap<String, String> {
    debug!("Getting full context for {}...", resource.name);

    let prop_context = render_properties(engine, &resource.props, global_context, stack_env);

    let mut full_context = global_context.clone();
    for (k, v) in prop_context {
        full_context.insert(k, v);
    }

    debug!("Full context for {}: {:?}", resource.name, full_context);
    full_context
}

/// Prepare context for SQL query rendering.
/// JSON string values are re-serialized to ensure proper format (compact, lowercase bools).
/// Matches Python's `render_queries` context preparation.
pub fn prepare_query_context(context: &HashMap<String, String>) -> HashMap<String, String> {
    let mut prepared = HashMap::new();

    for (key, value) in context {
        // Check if the value is a valid JSON string
        if let Ok(parsed) = serde_json::from_str::<JsonValue>(value) {
            if parsed.is_object() || parsed.is_array() {
                // Re-serialize with compact format
                let json_str = serde_json::to_string(&parsed)
                    .unwrap_or_else(|_| value.clone())
                    .replace("True", "true")
                    .replace("False", "false");
                prepared.insert(key.clone(), json_str);
                continue;
            }
        }
        prepared.insert(key.clone(), value.clone());
    }

    prepared
}

/// Get the resource type, validating it against allowed types.
/// Matches Python's `get_type`.
pub fn get_resource_type(resource: &crate::resource::manifest::Resource) -> &str {
    let res_type = resource.r#type.as_str();
    match res_type {
        "resource" | "query" | "script" | "multi" | "command" => res_type,
        _ => {
            error!(
                "Resource type must be 'resource', 'script', 'multi', 'query', or 'command', got '{}'",
                res_type
            );
            process::exit(1);
        }
    }
}

/// Check if a string is valid JSON (object or array).
pub fn is_json(s: &str) -> bool {
    match serde_json::from_str::<JsonValue>(s) {
        Ok(v) => v.is_object() || v.is_array(),
        Err(_) => false,
    }
}
