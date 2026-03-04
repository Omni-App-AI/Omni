use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use omni_permissions::capability::Capability;

use crate::error::{ManifestError, Result};

fn default_wasm() -> RuntimeType {
    RuntimeType::Wasm
}
fn default_memory() -> u32 {
    64
}
fn default_cpu() -> u64 {
    5000
}
fn default_concurrent() -> u32 {
    4
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub extension: ExtensionMeta,
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub config: ExtensionConfigSchema,
    #[serde(default)]
    pub permissions: Vec<PermissionDeclaration>,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default)]
    pub hooks: HookConfig,
    /// MCP servers that this extension needs. Automatically registered on activation
    /// and deregistered on deactivation. Requires `mcp.server` permission.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub icon: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    pub min_omni_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default = "default_wasm")]
    pub r#type: RuntimeType,
    pub entrypoint: String,
    #[serde(default = "default_memory")]
    pub max_memory_mb: u32,
    #[serde(default = "default_cpu")]
    pub max_cpu_ms_per_call: u64,
    #[serde(default = "default_concurrent")]
    pub max_concurrent_calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    Wasm,
    Native,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionConfigSchema {
    #[serde(default)]
    pub fields: HashMap<String, ConfigField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub r#type: String,
    pub label: String,
    pub help: Option<String>,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub required: bool,
    pub options: Option<Vec<String>>,
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDeclaration {
    pub capability: String,
    #[serde(default)]
    pub scope: Option<serde_json::Value>,
    pub reason: String,
    #[serde(default = "default_true")]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookConfig {
    #[serde(default)]
    pub on_install: bool,
    #[serde(default)]
    pub on_message: bool,
    pub on_schedule: Option<String>,
}

/// An MCP server declaration in the extension manifest.
/// When the extension is activated, these servers are automatically registered
/// with the MCP manager. When deactivated, they are removed.
/// Requires the `mcp.server` permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDeclaration {
    /// Unique name for the MCP server (will be prefixed with the extension ID).
    pub name: String,
    /// Command to launch the MCP server process (e.g. `npx`, `python`).
    pub command: String,
    /// Arguments passed to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables set for the server process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Optional working directory for the server process.
    pub working_dir: Option<String>,
}

impl ExtensionManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(ManifestError::Io)?;
        let manifest: Self = toml::from_str(&content).map_err(ManifestError::Toml)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn parse(content: &str) -> Result<Self> {
        let manifest: Self = toml::from_str(content).map_err(ManifestError::Toml)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<()> {
        // Validate extension ID format (reverse-domain):
        // - Must contain at least one dot
        // - Must be at least 5 chars (e.g. "a.b.c")
        // - Must not contain consecutive dots or start/end with a dot
        let id = &self.extension.id;
        if !id.contains('.') || id.len() < 5
            || id.starts_with('.') || id.ends_with('.')
            || id.contains("..")
        {
            return Err(ManifestError::InvalidId(id.clone()).into());
        }

        // Validate SemVer
        semver::Version::parse(&self.extension.version)
            .map_err(|_| ManifestError::InvalidVersion(self.extension.version.clone()))?;

        // Validate runtime limits are sane
        if self.runtime.max_memory_mb == 0 {
            return Err(ManifestError::InvalidRuntimeConfig(
                "max_memory_mb must be > 0".to_string(),
            )
            .into());
        }
        if self.runtime.max_cpu_ms_per_call == 0 {
            return Err(ManifestError::InvalidRuntimeConfig(
                "max_cpu_ms_per_call must be > 0".to_string(),
            )
            .into());
        }

        // Validate all declared permissions are recognized capabilities
        for perm in &self.permissions {
            perm.capability
                .parse::<Capability>()
                .map_err(|_| ManifestError::UnknownCapability(perm.capability.clone()))?;
        }

        // Validate that mcp_servers requires the mcp.server permission
        if !self.mcp_servers.is_empty() {
            let has_mcp_perm = self
                .permissions
                .iter()
                .any(|p| p.capability == "mcp.server");
            if !has_mcp_perm {
                return Err(ManifestError::InvalidRuntimeConfig(
                    "mcp_servers declared but missing 'mcp.server' permission".to_string(),
                )
                .into());
            }
        }

        // Validate tool parameter schemas are valid JSON Schema
        for tool in &self.tools {
            jsonschema::validator_for(&tool.parameters).map_err(|e| {
                ManifestError::InvalidToolSchema {
                    tool: tool.name.clone(),
                    error: e.to_string(),
                }
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEATHER_MANIFEST: &str = r#"
[extension]
id = "com.example.weather"
name = "Weather Lookup"
version = "1.2.0"
author = "Jane Doe <jane@example.com>"
description = "Fetches current weather data for any location worldwide."
license = "MIT"
homepage = "https://github.com/janedoe/omni-weather"
repository = "https://github.com/janedoe/omni-weather"
icon = "icon.png"
categories = ["utilities", "weather"]
min_omni_version = "0.5.0"

[runtime]
type = "wasm"
entrypoint = "weather.wasm"
max_memory_mb = 64
max_cpu_ms_per_call = 5000
max_concurrent_calls = 4

[config.fields.api_key]
type = "string"
label = "OpenWeatherMap API Key"
help = "Get your free API key at openweathermap.org/api"
sensitive = true
required = true

[config.fields.units]
type = "enum"
label = "Temperature Units"
options = ["celsius", "fahrenheit"]
default = "celsius"

[[permissions]]
capability = "network.http"
scope = { domains = ["api.openweathermap.org"], methods = ["GET"] }
reason = "Fetch weather data from the OpenWeatherMap API."
required = true

[[permissions]]
capability = "system.notifications"
reason = "Notify you of severe weather alerts."
required = false

[[tools]]
name = "get_weather"
description = "Get current weather for a location"
[tools.parameters]
type = "object"
required = ["location"]
[tools.parameters.properties.location]
type = "string"
description = "City name or coordinates"
[tools.parameters.properties.units]
type = "string"
enum = ["celsius", "fahrenheit"]
description = "Temperature units"

[[tools]]
name = "get_forecast"
description = "Get 5-day weather forecast for a location"
[tools.parameters]
type = "object"
required = ["location"]
[tools.parameters.properties.location]
type = "string"
description = "City name or coordinates"
[tools.parameters.properties.days]
type = "integer"
minimum = 1
maximum = 5
description = "Number of days (1-5)"

[hooks]
on_install = true
on_message = true
on_schedule = "0 */6 * * *"
"#;

    #[test]
    fn test_parse_full_weather_manifest() {
        let manifest = ExtensionManifest::parse(WEATHER_MANIFEST).unwrap();
        assert_eq!(manifest.extension.id, "com.example.weather");
        assert_eq!(manifest.extension.name, "Weather Lookup");
        assert_eq!(manifest.extension.version, "1.2.0");
        assert_eq!(manifest.extension.author, "Jane Doe <jane@example.com>");
        assert_eq!(manifest.extension.license, Some("MIT".to_string()));
        assert_eq!(manifest.extension.categories.len(), 2);
        assert_eq!(manifest.runtime.r#type, RuntimeType::Wasm);
        assert_eq!(manifest.runtime.entrypoint, "weather.wasm");
        assert_eq!(manifest.runtime.max_memory_mb, 64);
        assert_eq!(manifest.runtime.max_cpu_ms_per_call, 5000);
        assert_eq!(manifest.runtime.max_concurrent_calls, 4);
        assert_eq!(manifest.config.fields.len(), 2);
        assert!(manifest.config.fields.get("api_key").unwrap().sensitive);
        assert!(manifest.config.fields.get("api_key").unwrap().required);
        assert_eq!(manifest.permissions.len(), 2);
        assert_eq!(manifest.permissions[0].capability, "network.http");
        assert!(manifest.permissions[0].required);
        assert!(!manifest.permissions[1].required);
        assert_eq!(manifest.tools.len(), 2);
        assert_eq!(manifest.tools[0].name, "get_weather");
        assert_eq!(manifest.tools[1].name, "get_forecast");
        assert!(manifest.hooks.on_install);
        assert!(manifest.hooks.on_message);
        assert_eq!(
            manifest.hooks.on_schedule,
            Some("0 */6 * * *".to_string())
        );
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let toml = r#"
[extension]
id = "com.example.minimal"
name = "Minimal"
version = "0.1.0"
author = "Test Author"
description = "A minimal extension."

[runtime]
entrypoint = "minimal.wasm"
"#;
        let manifest = ExtensionManifest::parse(toml).unwrap();
        assert_eq!(manifest.extension.id, "com.example.minimal");
        assert_eq!(manifest.runtime.r#type, RuntimeType::Wasm);
        assert_eq!(manifest.runtime.max_memory_mb, 64);
        assert_eq!(manifest.runtime.max_cpu_ms_per_call, 5000);
        assert_eq!(manifest.runtime.max_concurrent_calls, 4);
        assert!(manifest.permissions.is_empty());
        assert!(manifest.tools.is_empty());
        assert!(!manifest.hooks.on_install);
        assert!(!manifest.hooks.on_message);
        assert!(manifest.hooks.on_schedule.is_none());
    }

    #[test]
    fn test_invalid_id_no_dot() {
        let toml = r#"
[extension]
id = "bad"
name = "Bad"
version = "1.0.0"
author = "Test"
description = "Bad ID"

[runtime]
entrypoint = "bad.wasm"
"#;
        let err = ExtensionManifest::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid extension ID"),
            "got: {err}"
        );
    }

    #[test]
    fn test_invalid_version() {
        let toml = r#"
[extension]
id = "com.example.bad"
name = "Bad"
version = "not-semver"
author = "Test"
description = "Bad version"

[runtime]
entrypoint = "bad.wasm"
"#;
        let err = ExtensionManifest::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid version"),
            "got: {err}"
        );
    }

    #[test]
    fn test_unknown_capability() {
        let toml = r#"
[extension]
id = "com.example.bad"
name = "Bad"
version = "1.0.0"
author = "Test"
description = "Bad capability"

[runtime]
entrypoint = "bad.wasm"

[[permissions]]
capability = "network.ftp"
reason = "Unknown capability"
"#;
        let err = ExtensionManifest::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Unknown capability"),
            "got: {err}"
        );
    }

    #[test]
    fn test_invalid_tool_schema() {
        let toml = r#"
[extension]
id = "com.example.bad"
name = "Bad"
version = "1.0.0"
author = "Test"
description = "Bad tool schema"

[runtime]
entrypoint = "bad.wasm"

[[tools]]
name = "bad_tool"
description = "A tool with invalid schema"
[tools.parameters]
type = "not-a-valid-type"
"#;
        let err = ExtensionManifest::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid tool schema"),
            "got: {err}"
        );
    }

    #[test]
    fn test_default_runtime_config() {
        let toml = r#"
[extension]
id = "com.example.defaults"
name = "Defaults"
version = "1.0.0"
author = "Test"
description = "Test defaults"

[runtime]
entrypoint = "test.wasm"
"#;
        let manifest = ExtensionManifest::parse(toml).unwrap();
        assert_eq!(manifest.runtime.r#type, RuntimeType::Wasm);
        assert_eq!(manifest.runtime.max_memory_mb, 64);
        assert_eq!(manifest.runtime.max_cpu_ms_per_call, 5000);
        assert_eq!(manifest.runtime.max_concurrent_calls, 4);
    }
}
