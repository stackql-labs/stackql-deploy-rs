// resource/manifest.rs

//! # Manifest Module
//!
//! Handles loading, parsing, and managing stack manifests.
//! A manifest describes the resources that make up a stack and their configurations.
//!
//! The primary type is `Manifest`, which represents a parsed stackql_manifest.yml file.
//! This module also provides types for resources, properties, and other manifest components.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
    
    /// Value of the global variable
    pub value: String,
    
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
    
    /// Properties for the resource
    #[serde(default)]
    pub props: Vec<Property>,
    
    /// Exports from the resource
    #[serde(default)]
    pub exports: Vec<String>,
    
    /// Protected exports
    #[serde(default)]
    pub protected: Vec<String>,
    
    /// Description of the resource
    #[serde(default)]
    pub description: String,
    
    /// Condition for resource processing
    #[serde(default)]
    pub r#if: Option<String>,
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
    
    /// Value of the property
    #[serde(default)]
    pub value: Option<String>,
    
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
    /// Value for the property in this environment
    pub value: String,
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
                
                // Each property must have either a value or values
                if prop.value.is_none() && prop.values.is_none() {
                    return Err(ManifestError::MissingField(
                        format!("Property '{}' in resource '{}' has no value or values", 
                                prop.name, resource.name)
                    ));
                }
            }
            
            // Make sure exports are valid
            for export in &resource.exports {
                if export.is_empty() {
                    return Err(ManifestError::InvalidField(
                        format!("Empty export in resource '{}'", resource.name)
                    ));
                }
            }
            
            // Make sure protected exports are a subset of exports
            for protected in &resource.protected {
                if !resource.exports.contains(protected) {
                    return Err(ManifestError::InvalidField(
                        format!("Protected export '{}' not found in exports for resource '{}'", 
                                protected, resource.name)
                    ));
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
        env: &'a str,
    ) -> Option<&'a str> {
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
    
    /// Gets global variables as a map.
    pub fn globals_as_map(&self) -> HashMap<String, String> {
        self.globals
            .iter()
            .map(|g| (g.name.clone(), g.value.clone()))
            .collect()
    }
}

/// Unit tests for manifest functionality.
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_manifest() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        
        writeln!(file, "version: 1").unwrap();
        writeln!(file, "name: test-stack").unwrap();
        writeln!(file, "description: Test Stack").unwrap();
        writeln!(file, "providers:").unwrap();
        writeln!(file, "  - aws").unwrap();
        writeln!(file, "globals:").unwrap();
        writeln!(file, "  - name: region").unwrap();
        writeln!(file, "    value: us-east-1").unwrap();
        writeln!(file, "resources:").unwrap();
        writeln!(file, "  - name: test-resource").unwrap();
        writeln!(file, "    props:").unwrap();
        writeln!(file, "      - name: vpc_cidr").unwrap();
        writeln!(file, "        value: 10.0.0.0/16").unwrap();
        
        file
    }

    #[test]
    fn test_load_manifest() {
        let file = create_test_manifest();
        let manifest = Manifest::load_from_file(file.path()).unwrap();
        
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.name, "test-stack");
        assert_eq!(manifest.providers, vec!["aws"]);
        assert_eq!(manifest.globals.len(), 1);
        assert_eq!(manifest.globals[0].name, "region");
        assert_eq!(manifest.resources.len(), 1);
        assert_eq!(manifest.resources[0].name, "test-resource");
    }
    
    #[test]
    fn test_find_resource() {
        let file = create_test_manifest();
        let manifest = Manifest::load_from_file(file.path()).unwrap();
        
        let resource = manifest.find_resource("test-resource");
        assert!(resource.is_some());
        assert_eq!(resource.unwrap().name, "test-resource");
        
        let nonexistent = manifest.find_resource("nonexistent");
        assert!(nonexistent.is_none());
    }
    
    #[test]
    fn test_get_property_value() {
        // Test property with direct value
        let prop_direct = Property {
            name: "test".to_string(),
            value: Some("direct-value".to_string()),
            values: None,
            description: "".to_string(),
            merge: None,
        };
        
        assert_eq!(Manifest::get_property_value(&prop_direct, "any"), Some("direct-value"));
        
        // Test property with env-specific values
        let mut env_values = HashMap::new();
        env_values.insert("dev".to_string(), PropertyValue { value: "dev-value".to_string() });
        env_values.insert("prod".to_string(), PropertyValue { value: "prod-value".to_string() });
        
        let prop_env = Property {
            name: "test".to_string(),
            value: None,
            values: Some(env_values),
            description: "".to_string(),
            merge: None,
        };
        
        assert_eq!(Manifest::get_property_value(&prop_env, "dev"), Some("dev-value"));
        assert_eq!(Manifest::get_property_value(&prop_env, "prod"), Some("prod-value"));
        assert_eq!(Manifest::get_property_value(&prop_env, "unknown"), None);
    }
}