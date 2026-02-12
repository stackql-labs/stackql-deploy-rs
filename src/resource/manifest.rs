// resource/manifest.rs

//! # Manifest Module
//!
//! Handles loading, parsing, and managing stack manifests.
//! A manifest describes the resources that make up a stack and their configurations.
//!
//! The primary type is `Manifest`, which represents a parsed stackql_manifest.yml file.
//! This module also provides types for resources, properties, and other manifest components.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, process};

use log::{debug, error};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when working with manifests.
#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("Failed to read manifest file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to parse manifest: {0}")]
    ParseError(#[from] serde_yaml::Error),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid field: {0}")]
    InvalidField(String),
}

/// Type alias for ManifestResult
pub type ManifestResult<T> = Result<T, ManifestError>;

/// Represents a stack manifest file.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    /// Version of the manifest format
    #[serde(default = "default_version")]
    pub version: u32,

    /// Name of the stack
    pub name: String,

    /// Description of the stack
    #[serde(default)]
    pub description: String,

    /// List of providers used by the stack
    pub providers: Vec<String>,

    /// Global variables for the stack
    #[serde(default)]
    pub globals: Vec<GlobalVar>,

    /// Resources in the stack
    #[serde(default)]
    pub resources: Vec<Resource>,

    /// Stack-level exports (written to JSON output file)
    #[serde(default)]
    pub exports: Vec<String>,
}

/// Default version for manifest when not specified
fn default_version() -> u32 {
    1
}

/// Represents a global variable in the manifest.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalVar {
    /// Name of the global variable
    pub name: String,

    /// Value of the global variable - can be a string or a complex structure
    #[serde(default)]
    pub value: serde_yaml::Value,

    /// Optional description
    #[serde(default)]
    pub description: String,
}

/// Represents a resource in the manifest.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Resource {
    /// Name of the resource
    pub name: String,

    /// Type of the resource (defaults to "resource")
    #[serde(default = "default_resource_type")]
    pub r#type: String,

    /// Custom file name for resource queries (if not derived from name)
    #[serde(default)]
    pub file: Option<String>,

    /// Inline SQL for query/command type resources
    #[serde(default)]
    pub sql: Option<String>,

    /// Script command for script type resources
    #[serde(default)]
    pub run: Option<String>,

    /// Properties for the resource
    #[serde(default)]
    pub props: Vec<Property>,

    /// Exports from the resource (can be strings or {key: value} maps)
    #[serde(default)]
    pub exports: Vec<serde_yaml::Value>,

    /// Protected exports
    #[serde(default)]
    pub protected: Vec<String>,

    /// Description of the resource
    #[serde(default)]
    pub description: String,

    /// Condition for resource processing
    #[serde(default)]
    pub r#if: Option<String>,

    /// Skip validation for this resource
    #[serde(default)]
    pub skip_validation: Option<bool>,

    /// Auth configuration for the resource
    #[serde(default)]
    pub auth: Option<serde_yaml::Value>,
}

/// Default resource type value
fn default_resource_type() -> String {
    "resource".to_string()
}

/// Represents a property of a resource.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Property {
    /// Name of the property
    pub name: String,

    /// Value of the property - can be a string or a complex structure
    #[serde(default)]
    pub value: Option<serde_yaml::Value>,

    /// Environment-specific values
    #[serde(default)]
    pub values: Option<HashMap<String, PropertyValue>>,

    /// Description of the property
    #[serde(default)]
    pub description: String,

    /// Items to merge with the value
    #[serde(default)]
    pub merge: Option<Vec<String>>,
}

/// Represents a value for a property in a specific environment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PropertyValue {
    /// Value for the property in this environment - can be a string or complex structure
    pub value: serde_yaml::Value,
}

impl Manifest {
    /// Loads a manifest file from the specified path.
    pub fn load_from_file(path: &Path) -> ManifestResult<Self> {
        let content = fs::read_to_string(path)?;
        let manifest: Manifest = serde_yaml::from_str(&content)?;

        // Validate the manifest
        manifest.validate()?;

        Ok(manifest)
    }

    /// Loads a manifest file from the specified stack directory.
    pub fn load_from_stack_dir(stack_dir: &Path) -> ManifestResult<Self> {
        let manifest_path = stack_dir.join("stackql_manifest.yml");
        Self::load_from_file(&manifest_path)
    }

    /// Validates the manifest for required fields and correctness.
    fn validate(&self) -> ManifestResult<()> {
        // Check required fields
        if self.name.is_empty() {
            return Err(ManifestError::MissingField("name".to_string()));
        }

        if self.providers.is_empty() {
            return Err(ManifestError::MissingField("providers".to_string()));
        }

        // Validate each resource
        for resource in &self.resources {
            if resource.name.is_empty() {
                return Err(ManifestError::MissingField("resource.name".to_string()));
            }

            // Validate properties
            for prop in &resource.props {
                if prop.name.is_empty() {
                    return Err(ManifestError::MissingField("property.name".to_string()));
                }

                // Each property must have either a value, values, or merge
                if prop.value.is_none() && prop.values.is_none() && prop.merge.is_none() {
                    return Err(ManifestError::MissingField(format!(
                        "Property '{}' in resource '{}' has no value, values, or merge",
                        prop.name, resource.name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Gets the resource query file path for a resource.
    pub fn get_resource_query_path(&self, stack_dir: &Path, resource: &Resource) -> PathBuf {
        let file_name = match &resource.file {
            Some(file) => file.clone(),
            _none => format!("{}.iql", resource.name),
        };

        stack_dir.join("resources").join(file_name)
    }

    /// Gets the value of a property in a specific environment.
    pub fn get_property_value<'a>(
        property: &'a Property,
        env: &str,
    ) -> Option<&'a serde_yaml::Value> {
        // Direct value takes precedence
        if let Some(ref value) = property.value {
            return Some(value);
        }

        // Fall back to environment-specific values
        if let Some(ref values) = property.values {
            if let Some(env_value) = values.get(env) {
                return Some(&env_value.value);
            }
        }

        None
    }

    /// Finds a resource by name.
    pub fn find_resource(&self, name: &str) -> Option<&Resource> {
        self.resources.iter().find(|r| r.name == name)
    }

    /// Gets global variables as a map of name to YAML value.
    pub fn globals_as_map(&self) -> HashMap<String, serde_yaml::Value> {
        self.globals
            .iter()
            .map(|g| (g.name.clone(), g.value.clone()))
            .collect()
    }

    /// Loads a manifest file from the specified stack directory or exits with an error message.
    pub fn load_from_dir_or_exit(stack_dir: &str) -> Self {
        debug!("Loading manifest file from stack directory: {}", stack_dir);

        match Self::load_from_stack_dir(Path::new(stack_dir)) {
            Ok(manifest) => {
                debug!("Stack name: {}", manifest.name);
                debug!("Stack description: {}", manifest.description);
                debug!("Providers: {:?}", manifest.providers);
                debug!("Resources count: {}", manifest.resources.len());
                manifest
            }
            Err(err) => {
                error!("Failed to load manifest: {}", err);
                process::exit(1);
            }
        }
    }
}
