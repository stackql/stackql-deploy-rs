// template/engine.rs

//! # Template Engine Module
//!
//! Provides Jinja2-compatible template rendering using the Tera engine.
//! Includes custom filters matching the Python stackql-deploy implementation:
//! `from_json`, `base64_encode`, `merge_lists`, `merge_objects`,
//! `generate_patch_document`, `sql_list`, `sql_escape`.

use std::collections::HashMap;
use std::error::Error as StdError;

use base64::Engine as Base64Engine;
use serde_json::Value as JsonValue;
use tera::{Context as TeraContext, Tera};

/// Error types that can occur during template rendering.
#[derive(Debug)]
pub enum TemplateError {
    /// Variable not found in context
    VariableNotFound(String),

    /// Syntax error in template
    SyntaxError(String),

    /// Invalid template structure
    InvalidTemplate(String),

    /// Rendering error from Tera
    RenderError(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::VariableNotFound(var) => write!(f, "Variable not found: {}", var),
            TemplateError::SyntaxError(msg) => write!(f, "Template syntax error: {}", msg),
            TemplateError::InvalidTemplate(msg) => write!(f, "Invalid template: {}", msg),
            TemplateError::RenderError(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for TemplateError {}

/// Type alias for template rendering results
pub type TemplateResult<T> = Result<T, TemplateError>;

/// A structure that renders templates using Tera (Jinja2-compatible).
#[derive(Debug)]
pub struct TemplateEngine {
    #[allow(dead_code)]
    tera: Tera,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// Creates a new template engine with custom filters registered.
    pub fn new() -> Self {
        let mut tera = Tera::default();
        register_custom_filters(&mut tera);
        Self { tera }
    }

    /// Renders a template string using the provided context (HashMap<String, String>).
    pub fn render(
        &self,
        template: &str,
        context: &HashMap<String, String>,
    ) -> TemplateResult<String> {
        self.render_template(template, context)
    }

    /// Renders a template string using a HashMap<String, String> context.
    pub fn render_template(
        &self,
        template: &str,
        context: &HashMap<String, String>,
    ) -> TemplateResult<String> {
        let mut tera_context = TeraContext::new();
        for (key, value) in context {
            tera_context.insert(key, value);
        }
        self.render_with_tera_context(template, &tera_context)
    }

    /// Renders a template string using a full Tera context (supports non-string values).
    pub fn render_with_tera_context(
        &self,
        template: &str,
        context: &TeraContext,
    ) -> TemplateResult<String> {
        // Use Tera's one-off rendering
        match Tera::one_off(template, context, false) {
            Ok(rendered) => Ok(rendered),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("not found in context") {
                    // Extract the variable name from the error
                    Err(TemplateError::VariableNotFound(err_str))
                } else {
                    Err(TemplateError::RenderError(err_str))
                }
            }
        }
    }

    /// Renders a template string with context and custom filters.
    /// This method creates a fresh Tera instance with the template registered,
    /// which allows custom filters to work.
    pub fn render_with_filters(
        &self,
        template_name: &str,
        template: &str,
        context: &HashMap<String, String>,
    ) -> TemplateResult<String> {
        let mut tera = Tera::default();
        register_custom_filters(&mut tera);

        tera.add_raw_template(template_name, template)
            .map_err(|e| TemplateError::SyntaxError(full_error_chain(&e)))?;

        let mut tera_context = TeraContext::new();
        for (key, value) in context {
            tera_context.insert(key, value);
        }

        // Add uuid global function via context
        let uuid_val = uuid::Uuid::new_v4().to_string();
        tera_context.insert("uuid", &uuid_val);

        tera.render(template_name, &tera_context).map_err(|e| {
            let full_msg = full_error_chain(&e);
            if full_msg.contains("not found in context") {
                TemplateError::VariableNotFound(full_msg)
            } else {
                TemplateError::RenderError(full_msg)
            }
        })
    }
}

/// Walk the full error source chain and concatenate all messages.
/// Tera's top-level `Display` often only shows "Failed to render 'name'" while
/// the root cause (e.g., missing variable) is buried in `source()`.
fn full_error_chain(err: &dyn StdError) -> String {
    let mut parts = vec![err.to_string()];
    let mut current = err.source();
    while let Some(cause) = current {
        parts.push(cause.to_string());
        current = cause.source();
    }
    parts.join(": ")
}

/// Register all custom Jinja2 filters matching the Python implementation.
fn register_custom_filters(tera: &mut Tera) {
    tera.register_filter("from_json", filter_from_json);
    tera.register_filter("base64_encode", filter_base64_encode);
    tera.register_filter("merge_lists", filter_merge_lists);
    tera.register_filter("merge_objects", filter_merge_objects);
    tera.register_filter("generate_patch_document", filter_generate_patch_document);
    tera.register_filter("sql_list", filter_sql_list);
    tera.register_filter("sql_escape", filter_sql_escape);
}

/// from_json filter: parse a JSON string into a Tera value
fn filter_from_json(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let s = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("from_json: expected a string"))?;
    let parsed: serde_json::Value =
        serde_json::from_str(s).map_err(|e| tera::Error::msg(format!("from_json: {}", e)))?;
    Ok(tera::to_value(parsed)?)
}

/// base64_encode filter: encode a string to base64
fn filter_base64_encode(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let s = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("base64_encode: expected a string"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(s.as_bytes());
    Ok(tera::to_value(encoded)?)
}

/// merge_lists filter: merge two lists (union by JSON serialization)
fn filter_merge_lists(
    value: &tera::Value,
    args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let list1 = value
        .as_array()
        .ok_or_else(|| tera::Error::msg("merge_lists: expected an array"))?;

    let other = args
        .get("other")
        .or_else(|| args.values().next())
        .ok_or_else(|| tera::Error::msg("merge_lists: missing 'other' argument"))?;

    let list2 = other
        .as_array()
        .ok_or_else(|| tera::Error::msg("merge_lists: 'other' must be an array"))?;

    // Merge using JSON serialization for uniqueness
    let mut set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut merged = Vec::new();

    for item in list1.iter().chain(list2.iter()) {
        let key = serde_json::to_string(item).unwrap_or_default();
        if set.insert(key) {
            merged.push(item.clone());
        }
    }

    Ok(tera::to_value(merged)?)
}

/// merge_objects filter: merge two objects (dicts)
fn filter_merge_objects(
    value: &tera::Value,
    args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let obj1 = value
        .as_object()
        .ok_or_else(|| tera::Error::msg("merge_objects: expected an object"))?;

    let other = args
        .get("other")
        .or_else(|| args.values().next())
        .ok_or_else(|| tera::Error::msg("merge_objects: missing 'other' argument"))?;

    let obj2 = other
        .as_object()
        .ok_or_else(|| tera::Error::msg("merge_objects: 'other' must be an object"))?;

    let mut merged = obj1.clone();
    for (k, v) in obj2 {
        merged.insert(k.clone(), v.clone());
    }

    Ok(tera::to_value(merged)?)
}

/// generate_patch_document filter: create AWS Cloud Control API patch document
fn filter_generate_patch_document(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    // Accept either a JSON object directly or a JSON string to parse
    let obj = if let Some(o) = value.as_object() {
        o.clone()
    } else if let Some(s) = value.as_str() {
        match serde_json::from_str::<JsonValue>(s) {
            Ok(JsonValue::Object(o)) => o,
            _ => {
                return Err(tera::Error::msg(
                    "generate_patch_document: expected a JSON object or JSON string",
                ))
            }
        }
    } else {
        return Err(tera::Error::msg(
            "generate_patch_document: expected an object or JSON string",
        ));
    };

    let mut patch_doc: Vec<JsonValue> = Vec::new();
    for (key, val) in &obj {
        let patch_value = if let Some(s) = val.as_str() {
            // Try to parse as JSON
            match serde_json::from_str::<JsonValue>(s) {
                Ok(parsed) => parsed,
                Err(_) => val.clone(),
            }
        } else {
            val.clone()
        };

        patch_doc.push(serde_json::json!({
            "op": "add",
            "path": format!("/{}", key),
            "value": patch_value,
        }));
    }

    let result = serde_json::to_string(&patch_doc)
        .map_err(|e| tera::Error::msg(format!("generate_patch_document: {}", e)))?;
    Ok(tera::to_value(result)?)
}

/// sql_list filter: convert a list to SQL IN clause format
fn filter_sql_list(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let items: Vec<String> = if let Some(arr) = value.as_array() {
        arr.iter()
            .map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else {
                    v.to_string().trim_matches('"').to_string()
                }
            })
            .collect()
    } else if let Some(s) = value.as_str() {
        // Try to parse as JSON array
        match serde_json::from_str::<Vec<String>>(s) {
            Ok(parsed) => parsed,
            Err(_) => vec![s.to_string()],
        }
    } else {
        return Ok(tera::to_value("(NULL)")?);
    };

    if items.is_empty() {
        return Ok(tera::to_value("(NULL)")?);
    }

    let quoted: Vec<String> = items.iter().map(|item| format!("'{}'", item)).collect();
    let result = format!("({})", quoted.join(","));
    Ok(tera::to_value(result)?)
}

/// sql_escape filter: escape single quotes for SQL strings
fn filter_sql_escape(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let s = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("sql_escape: expected a string"))?;
    let escaped = s.replace('\'', "''");
    Ok(tera::to_value(escaped)?)
}

/// Unit tests for template engine functionality.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable_substitution() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("name".to_string(), "World".to_string());

        let result = engine.render("Hello {{ name }}!", &context).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_multiple_variables() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("first".to_string(), "Hello".to_string());
        context.insert("second".to_string(), "World".to_string());

        let result = engine
            .render("{{ first }} {{ second }}!", &context)
            .unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_variable_not_found() {
        let engine = TemplateEngine::new();
        let context = HashMap::new();

        let result = engine.render("Hello {{ name }}!", &context);
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_braces() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("json".to_string(), r#"{"key": "value"}"#.to_string());

        let result = engine.render("JSON: {{ json }}", &context).unwrap();
        assert_eq!(result, r#"JSON: {"key": "value"}"#);
    }

    #[test]
    fn test_render_with_filters_missing_var_shows_name() {
        let engine = TemplateEngine::new();
        let context = HashMap::new();

        let result = engine.render_with_filters("test_tpl", "Hello {{ missing_var }}!", &context);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("missing_var"),
            "Error should mention the missing variable name, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_render_with_filters_missing_var_is_variable_not_found() {
        let engine = TemplateEngine::new();
        let context = HashMap::new();

        let result = engine.render_with_filters("test_tpl", "{{ no_such_var }}", &context);
        match result {
            Err(TemplateError::VariableNotFound(msg)) => {
                assert!(
                    msg.contains("no_such_var"),
                    "VariableNotFound error should contain variable name, got: {}",
                    msg
                );
            }
            other => panic!("Expected VariableNotFound error, got: {:?}", other),
        }
    }
}
