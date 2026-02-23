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
            let json_val: JsonValue = serde_json::to_value(value).unwrap_or(JsonValue::Null);
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
                    rendered.replace("True", "true").replace("False", "false")
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
        Ok(rendered) => rendered.replace("True", "true").replace("False", "false"),
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
            error!("Global variable '{}' cannot be empty", global_var.name);
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
                                    let key = serde_json::to_string(item).unwrap_or_default();
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
                        error!("Merge item '{}' value is not valid JSON", merge_item);
                        process::exit(1);
                    }
                } else {
                    error!("Merge item '{}' not found in context", merge_item);
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
///
/// Injects `resource_name` as a special variable (like `stack_name` and `stack_env`)
/// containing the current resource's name. Any global values that contain deferred
/// template expressions (e.g., `{{ resource_name }}`) are re-rendered at this point.
pub fn get_full_context(
    engine: &TemplateEngine,
    global_context: &HashMap<String, String>,
    resource: &crate::resource::manifest::Resource,
    stack_env: &str,
) -> HashMap<String, String> {
    debug!("Getting full context for {}...", resource.name);

    // Inject resource_name into the context so it's available in props and re-rendered globals
    let mut context_with_resource_name = global_context.clone();
    context_with_resource_name.insert("resource_name".to_string(), resource.name.clone());

    // Re-render any global values that contain deferred template expressions.
    // This allows globals (e.g., global_tags) to use {{ resource_name }} which couldn't
    // be resolved at global rendering time since the resource wasn't known yet.
    let resolved_context =
        re_render_context_with_deferred_vars(engine, &context_with_resource_name);

    let prop_context = render_properties(engine, &resource.props, &resolved_context, stack_env);

    let mut full_context = resolved_context;
    for (k, v) in prop_context {
        full_context.insert(k, v);
    }

    debug!("Full context for {}: {:?}", resource.name, full_context);
    full_context
}

/// Re-render context values that contain deferred template expressions (`{{ ... }}`).
/// This is used to resolve variables like `resource_name` that weren't available
/// when globals were initially rendered.
fn re_render_context_with_deferred_vars(
    engine: &TemplateEngine,
    context: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut result = context.clone();

    for (key, value) in context {
        if value.contains("{{") {
            match engine.render(value, context) {
                Ok(rendered) => {
                    let rendered = rendered.replace("True", "true").replace("False", "false");
                    debug!(
                        "Re-rendered deferred global [{}]: {} -> {}",
                        key, value, rendered
                    );
                    result.insert(key.clone(), rendered);
                }
                Err(e) => {
                    debug!(
                        "Warning: could not re-render deferred global '{}': {}",
                        key, e
                    );
                }
            }
        }
    }

    result
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::manifest::{Property, Resource};

    /// Helper to create a minimal Resource for testing.
    fn make_resource(name: &str, props: Vec<Property>) -> Resource {
        Resource {
            name: name.to_string(),
            r#type: "resource".to_string(),
            file: None,
            sql: None,
            run: None,
            props,
            exports: vec![],
            protected: vec![],
            description: String::new(),
            r#if: None,
            skip_validation: None,
            auth: None,
        }
    }

    /// Helper to create a Property with a simple string value.
    fn make_prop(name: &str, value: &str) -> Property {
        Property {
            name: name.to_string(),
            value: Some(serde_yaml::Value::String(value.to_string())),
            values: None,
            description: String::new(),
            merge: None,
        }
    }

    #[test]
    fn test_resource_name_available_in_full_context() {
        let engine = TemplateEngine::new();
        let mut global_context = HashMap::new();
        global_context.insert("stack_name".to_string(), "my-stack".to_string());
        global_context.insert("stack_env".to_string(), "dev".to_string());

        let resource = make_resource("cross_account_role", vec![]);

        let ctx = get_full_context(&engine, &global_context, &resource, "dev");

        assert_eq!(ctx.get("resource_name").unwrap(), "cross_account_role");
        // Existing variables still present
        assert_eq!(ctx.get("stack_name").unwrap(), "my-stack");
        assert_eq!(ctx.get("stack_env").unwrap(), "dev");
    }

    #[test]
    fn test_resource_name_usable_in_props() {
        let engine = TemplateEngine::new();
        let mut global_context = HashMap::new();
        global_context.insert("stack_name".to_string(), "my-stack".to_string());
        global_context.insert("stack_env".to_string(), "dev".to_string());

        let resource = make_resource(
            "cross_account_role",
            vec![make_prop("tag_value", "{{ resource_name }}")],
        );

        let ctx = get_full_context(&engine, &global_context, &resource, "dev");

        assert_eq!(ctx.get("tag_value").unwrap(), "cross_account_role");
    }

    #[test]
    fn test_resource_name_resolves_in_deferred_globals() {
        let engine = TemplateEngine::new();
        let mut global_context = HashMap::new();
        global_context.insert("stack_name".to_string(), "my-stack".to_string());
        global_context.insert("stack_env".to_string(), "dev".to_string());
        // Simulate a global that was rendered at startup but contained {{ resource_name }}
        // which couldn't be resolved then, so it's preserved as a literal.
        global_context.insert(
            "global_tags".to_string(),
            r#"[{"Key":"stackql:resource-name","Value":"{{ resource_name }}"}]"#.to_string(),
        );

        let resource = make_resource("cross_account_role", vec![]);

        let ctx = get_full_context(&engine, &global_context, &resource, "dev");

        let global_tags = ctx.get("global_tags").unwrap();
        assert!(
            global_tags.contains("cross_account_role"),
            "global_tags should contain the resolved resource name, got: {}",
            global_tags
        );
        assert!(
            !global_tags.contains("{{ resource_name }}"),
            "global_tags should not contain unresolved template expression"
        );
    }

    #[test]
    fn test_resource_name_varies_per_resource() {
        let engine = TemplateEngine::new();
        let mut global_context = HashMap::new();
        global_context.insert("stack_name".to_string(), "my-stack".to_string());
        global_context.insert("stack_env".to_string(), "dev".to_string());
        global_context.insert(
            "global_tags".to_string(),
            r#"[{"Key":"res","Value":"{{ resource_name }}"}]"#.to_string(),
        );

        let res1 = make_resource("vpc_network", vec![]);
        let res2 = make_resource("storage_bucket", vec![]);

        let ctx1 = get_full_context(&engine, &global_context, &res1, "dev");
        let ctx2 = get_full_context(&engine, &global_context, &res2, "dev");

        assert_eq!(ctx1.get("resource_name").unwrap(), "vpc_network");
        assert_eq!(ctx2.get("resource_name").unwrap(), "storage_bucket");
        assert!(ctx1.get("global_tags").unwrap().contains("vpc_network"));
        assert!(ctx2.get("global_tags").unwrap().contains("storage_bucket"));
    }

    #[test]
    fn test_re_render_context_no_templates_is_noop() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("stack_name".to_string(), "my-stack".to_string());
        context.insert("plain_value".to_string(), "no templates here".to_string());

        let result = re_render_context_with_deferred_vars(&engine, &context);

        assert_eq!(result.get("stack_name").unwrap(), "my-stack");
        assert_eq!(result.get("plain_value").unwrap(), "no templates here");
    }

    #[test]
    fn test_re_render_context_resolves_deferred_vars() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("resource_name".to_string(), "my_resource".to_string());
        context.insert(
            "tag".to_string(),
            "resource:{{ resource_name }}".to_string(),
        );

        let result = re_render_context_with_deferred_vars(&engine, &context);

        assert_eq!(result.get("tag").unwrap(), "resource:my_resource");
    }
}
