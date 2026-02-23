// resource/validation.rs

//! # Manifest Validation Module
//!
//! Contains validation rules applied to a [`Manifest`] before any command
//! (`build`, `plan`, `teardown`, `test`) is executed.
//!
//! Each rule is a standalone function that returns either `Ok(())` or a list of
//! [`ValidationError`] values describing what failed.  New rules should be added
//! as additional functions and wired into [`validate_manifest`].
//!
//! ## Current rules
//!
//! | Rule ID                   | Description                                     |
//! |---------------------------|-------------------------------------------------|
//! | `unique-resource-names`   | Every resource name in the stack must be unique |

use std::collections::HashSet;
use std::fmt;

use crate::resource::manifest::Manifest;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// A single manifest validation failure.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Short, machine-readable identifier for the rule that was violated.
    pub rule: &'static str,

    /// Human-readable explanation of what failed.
    pub detail: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.rule, self.detail)
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run all manifest validation rules against `manifest`.
///
/// All rules are evaluated (fail-all, not fail-fast) so that callers receive a
/// complete picture of every problem at once.
///
/// Returns `Ok(())` when the manifest passes every rule, or
/// `Err(Vec<ValidationError>)` containing one entry per failing check.
pub fn validate_manifest(manifest: &Manifest) -> Result<(), Vec<ValidationError>> {
    let mut errors: Vec<ValidationError> = Vec::new();

    collect(&mut errors, validate_unique_resource_names(manifest));
    // Wire in additional rules here as the list grows:
    // collect(&mut errors, validate_some_other_rule(manifest));

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Append errors from a rule result into the accumulator.
fn collect(acc: &mut Vec<ValidationError>, result: Result<(), Vec<ValidationError>>) {
    if let Err(mut errs) = result {
        acc.append(&mut errs);
    }
}

// ---------------------------------------------------------------------------
// Rule: unique-resource-names
// ---------------------------------------------------------------------------

/// Validates that every resource `name` in the stack is unique.
///
/// Resource names must be unique because:
///
/// * Resources are processed in declaration order — a duplicate name leads to
///   ambiguous processing behaviour.
/// * Resource-scoped export keys (`{resource_name}.{export}`) are immutable
///   once written.  A second resource with the same name would attempt to write
///   the same scoped keys, making them permanently incorrect.
///
/// **Rule ID**: `unique-resource-names`
fn validate_unique_resource_names(manifest: &Manifest) -> Result<(), Vec<ValidationError>> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut errors: Vec<ValidationError> = Vec::new();

    for resource in &manifest.resources {
        if !seen.insert(resource.name.as_str()) {
            errors.push(ValidationError {
                rule: "unique-resource-names",
                detail: format!(
                    "resource name '{}' appears more than once in stack '{}'; \
                     every resource name must be unique within a stack",
                    resource.name, manifest.name,
                ),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Parse a manifest from an inline YAML string.
    ///
    /// Panics with a clear message if the YAML is malformed, so test failures
    /// are easy to diagnose.
    fn parse(yaml: &str) -> Manifest {
        serde_yaml::from_str(yaml).unwrap_or_else(|e| {
            panic!("test manifest YAML is invalid: {}\n\nYAML:\n{}", e, yaml)
        })
    }

    // -----------------------------------------------------------------------
    // Rule: unique-resource-names — positive (valid) fixture
    // -----------------------------------------------------------------------

    /// Manifest where every resource name is distinct. Should pass.
    const VALID_UNIQUE_NAMES: &str = r#"
version: 1
name: test-stack
providers:
  - aws
resources:
  - name: vpc
    props:
      - name: vpc_name
        value: my-vpc
  - name: subnet
    props:
      - name: subnet_name
        value: my-subnet
  - name: role
    props:
      - name: role_name
        value: my-role
"#;

    #[test]
    fn test_unique_resource_names_valid() {
        let manifest = parse(VALID_UNIQUE_NAMES);
        let result = validate_manifest(&manifest);
        assert!(
            result.is_ok(),
            "A manifest with distinct resource names should pass validation, got: {:?}",
            result,
        );
    }

    // -----------------------------------------------------------------------
    // Rule: unique-resource-names — negative (invalid) fixture
    // -----------------------------------------------------------------------

    /// Manifest where `vpc` appears twice. Should fail with exactly one error.
    const INVALID_DUPLICATE_NAMES: &str = r#"
version: 1
name: test-stack
providers:
  - aws
resources:
  - name: vpc
    props:
      - name: vpc_name
        value: my-vpc
  - name: role
    props:
      - name: role_name
        value: my-role
  - name: vpc
    props:
      - name: vpc_name
        value: another-vpc
"#;

    #[test]
    fn test_unique_resource_names_duplicate_fails() {
        let manifest = parse(INVALID_DUPLICATE_NAMES);
        let result = validate_manifest(&manifest);

        assert!(
            result.is_err(),
            "A manifest with duplicate resource names must fail validation",
        );

        let errors = result.unwrap_err();
        assert_eq!(
            errors.len(),
            1,
            "Expected exactly one validation error for one duplicate, got: {:?}",
            errors,
        );
        assert_eq!(
            errors[0].rule, "unique-resource-names",
            "Error must reference the correct rule ID",
        );
        assert!(
            errors[0].detail.contains("vpc"),
            "Error detail must mention the duplicate resource name 'vpc', got: {}",
            errors[0].detail,
        );
    }

    #[test]
    fn test_unique_resource_names_multiple_duplicates() {
        // Two independent duplicate pairs: 'vpc' and 'role' each appear twice.
        let yaml = r#"
version: 1
name: test-stack
providers:
  - aws
resources:
  - name: vpc
    props:
      - name: vpc_name
        value: my-vpc
  - name: vpc
    props:
      - name: vpc_name
        value: another-vpc
  - name: role
    props:
      - name: role_name
        value: my-role
  - name: role
    props:
      - name: role_name
        value: another-role
"#;
        let manifest = parse(yaml);
        let result = validate_manifest(&manifest);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(
            errors.len(),
            2,
            "Expected two errors (one per duplicate pair), got: {:?}",
            errors,
        );
    }
}
