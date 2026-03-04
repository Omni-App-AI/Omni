use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::error::{OmniError, Result};
use crate::paths::OmniPaths;

fn default_log_level() -> String {
    "info".to_string()
}
fn default_max_history() -> usize {
    1000
}
fn default_true() -> bool {
    true
}
fn default_deny() -> String {
    "deny".to_string()
}
fn default_sensitivity() -> String {
    "balanced".to_string()
}
fn default_max_iterations() -> u32 {
    25
}
fn default_timeout() -> u64 {
    120
}

/// Root configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OmniConfig {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    #[serde(default)]
    pub agent: AgentConfig,

    #[serde(default)]
    pub guardian: GuardianConfig,

    #[serde(default)]
    pub permissions: PermissionDefaults,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub channels: ChannelsConfig,

    #[serde(default)]
    pub marketplace: MarketplaceConfig,

    #[serde(default)]
    pub mcp: McpConfig,

    #[serde(default)]
    pub lsp: LspConfig,

    #[serde(default)]
    pub code_search: CodeSearchConfig,

    /// User-defined environment variables injected into the process at startup.
    /// Useful for API keys (e.g. BRAVE_API_KEY) and tool configuration.
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

/// MCP (Model Context Protocol) server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Server identifier.
    pub name: String,
    /// Command to spawn the MCP server.
    pub command: String,
    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables for the server process.
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    /// Working directory for the server process.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Transport type: "stdio" (default) or "sse".
    #[serde(default = "default_mcp_transport")]
    pub transport: String,
    /// SSE URL (only for transport = "sse").
    #[serde(default)]
    pub url: Option<String>,
    /// Auto-start this server when Omni launches.
    #[serde(default)]
    pub auto_start: bool,
}

fn default_mcp_transport() -> String {
    "stdio".to_string()
}

/// MCP configuration section.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// Configured MCP servers.
    #[serde(default)]
    pub servers: Vec<McpServerEntry>,
}

/// LSP (Language Server Protocol) configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspConfig {
    /// Auto-start language servers when a project is opened.
    #[serde(default)]
    pub auto_start: bool,
    /// Override language server commands per language.
    /// e.g., `{ "rust": "rust-analyzer", "python": "pylsp" }`
    #[serde(default)]
    pub servers: HashMap<String, String>,
}

/// Code search configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeSearchConfig {
    /// Auto-index projects when opened.
    #[serde(default)]
    pub auto_index: bool,
    /// Languages to index (empty = all supported).
    #[serde(default)]
    pub languages: Vec<String>,
    /// Glob patterns for directories/files to exclude from indexing.
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

/// Marketplace configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketplaceConfig {
    /// Base URL for the marketplace API.
    pub api_url: String,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            api_url: "https://omniapp.org/api/v1".to_string(),
        }
    }
}

/// Pre-configured channel instance in config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInstanceConfig {
    pub channel_type: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub auto_connect: bool,
}

/// Pre-configured channel binding in config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelBindingConfig {
    /// Compound channel instance key (e.g. "discord:production").
    pub channel_instance: String,
    /// Extension ID to bind to this channel instance.
    pub extension_id: String,
    /// Optional peer filter glob (e.g. "admin-*").
    #[serde(default)]
    pub peer_filter: Option<String>,
    /// Optional group filter glob (e.g. "support-*").
    #[serde(default)]
    pub group_filter: Option<String>,
    /// Priority for conflict resolution (higher = preferred). Default 0.
    #[serde(default)]
    pub priority: i32,
}

fn default_webhook_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_webhook_port() -> u16 {
    8900
}

/// Channels configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    /// Pre-configured channel instances keyed by compound id (e.g. "discord:production").
    #[serde(default)]
    pub instances: HashMap<String, ChannelInstanceConfig>,
    /// Pre-configured channel-extension bindings.
    #[serde(default)]
    pub bindings: Vec<ChannelBindingConfig>,
    /// Bind address for the webhook server.
    /// Default: "127.0.0.1" (localhost only -- secure default).
    /// Set to "0.0.0.0" only if you need LAN access and understand the security risks.
    #[serde(default = "default_webhook_bind_address")]
    pub webhook_bind_address: String,
    /// Port for the webhook server. Default: 8900.
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            instances: HashMap::new(),
            bindings: Vec::new(),
            webhook_bind_address: default_webhook_bind_address(),
            webhook_port: default_webhook_port(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub data_dir: Option<PathBuf>,
    #[serde(default)]
    pub telemetry: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_max_history")]
    pub max_history: usize,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: None,
            telemetry: false,
            log_level: default_log_level(),
            max_history: default_max_history(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub default_model: Option<String>,
    pub endpoint: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Transport mode for streaming: "auto" (WS with SSE fallback), "ws", "sse" (default).
    #[serde(default)]
    pub transport: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Custom instructions appended to the system prompt.
    /// These are added after the default tool guidance and safety rules.
    pub system_prompt: Option<String>,
    /// Path to a file containing custom instructions (alternative to inline `system_prompt`).
    /// If both are set, the file contents are appended after the inline prompt.
    pub system_prompt_file: Option<PathBuf>,
    /// Whether to include default tool usage guidance in the system prompt.
    /// Default: true. Set to false if you want full control over the system prompt.
    #[serde(default = "default_true")]
    pub include_tool_guidance: bool,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Thinking mode: "adaptive" (recommended for Claude 4.6), "enabled", or omit to disable.
    #[serde(default)]
    pub thinking_mode: Option<String>,
    /// Thinking effort level: "low", "medium", "high" (default), "max".
    #[serde(default)]
    pub thinking_effort: Option<String>,
    /// Budget tokens for "enabled" thinking mode.
    #[serde(default)]
    pub thinking_budget: Option<u32>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: None,
            system_prompt_file: None,
            include_tool_guidance: true,
            max_iterations: default_max_iterations(),
            timeout_secs: default_timeout(),
            thinking_mode: None,
            thinking_effort: None,
            thinking_budget: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: String,
    pub custom_signatures: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub allow_override: bool,
}

impl Default for GuardianConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            sensitivity: default_sensitivity(),
            custom_signatures: None,
            allow_override: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDefaults {
    #[serde(default = "default_deny")]
    pub default_policy: String,
    #[serde(default)]
    pub trust_verified: bool,
    #[serde(default = "default_true")]
    pub audit_enabled: bool,
}

impl Default for PermissionDefaults {
    fn default() -> Self {
        Self {
            default_policy: default_deny(),
            trust_verified: false,
            audit_enabled: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub font_size: u32,
    pub show_action_feed: bool,
    pub accent_color: String,
    pub font_family: String,
    pub line_height: String,
    pub ui_density: String,
    pub sidebar_width: u32,
    pub message_style: String,
    pub max_message_width: u32,
    pub code_theme: String,
    pub show_timestamps: bool,
    pub border_radius: u32,
    pub reduce_animations: bool,
    pub high_contrast: bool,
    pub auto_update: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            font_size: 14,
            show_action_feed: true,
            accent_color: "#3b82f6".to_string(),
            font_family: "system".to_string(),
            line_height: "normal".to_string(),
            ui_density: "comfortable".to_string(),
            sidebar_width: 250,
            message_style: "bubbles".to_string(),
            max_message_width: 75,
            code_theme: "dark".to_string(),
            show_timestamps: false,
            border_radius: 8,
            reduce_animations: false,
            high_contrast: false,
            auto_update: true,
        }
    }
}

impl OmniConfig {
    /// Load config from a TOML file. Returns defaults if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: OmniConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load config from the platform default path.
    pub fn load_default() -> Result<Self> {
        let paths = OmniPaths::resolve()?;
        Self::load(&paths.config_file())
    }

    /// Save the config to a TOML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Validate the config and return a list of issues.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.general.log_level.as_str()) {
            issues.push(format!(
                "Invalid log level '{}'. Must be one of: {}",
                self.general.log_level,
                valid_log_levels.join(", ")
            ));
        }

        let valid_sensitivities = ["strict", "balanced", "permissive"];
        if !valid_sensitivities.contains(&self.guardian.sensitivity.as_str()) {
            issues.push(format!(
                "Invalid guardian sensitivity '{}'. Must be one of: {}",
                self.guardian.sensitivity,
                valid_sensitivities.join(", ")
            ));
        }

        let valid_policies = ["deny", "prompt"];
        if !valid_policies.contains(&self.permissions.default_policy.as_str()) {
            issues.push(format!(
                "Invalid default policy '{}'. Must be one of: {}",
                self.permissions.default_policy,
                valid_policies.join(", ")
            ));
        }

        for (name, provider) in &self.providers {
            if let Some(temp) = provider.temperature {
                if !(0.0..=2.0).contains(&temp) {
                    issues.push(format!(
                        "Provider '{}': temperature {} out of range [0.0, 2.0]",
                        name, temp
                    ));
                }
            }
            if let Some(ref transport) = provider.transport {
                let valid_transports = ["auto", "ws", "websocket", "sse"];
                if !valid_transports.contains(&transport.as_str()) {
                    issues.push(format!(
                        "Provider '{}': invalid transport '{}'. Must be one of: {}",
                        name,
                        transport,
                        valid_transports.join(", ")
                    ));
                }
            }
        }

        issues
    }

    /// Generate a default config file at the given path.
    pub fn generate_default_file(path: &Path) -> Result<()> {
        let config = Self::default();
        config.save(path)
    }
}

/// Watches the config file for changes and emits updated configs.
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    pub receiver: watch::Receiver<OmniConfig>,
}

impl ConfigWatcher {
    pub fn start(config_path: PathBuf, initial: OmniConfig) -> Result<Self> {
        let (tx, rx) = watch::channel(initial);
        let path = Arc::new(config_path.clone());

        let watch_path = Arc::clone(&path);
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    // Small delay to let the file finish writing
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if let Ok(new_config) = OmniConfig::load(&watch_path) {
                        let _ = tx.send(new_config);
                    }
                }
            }
        })?;

        let watch_dir = config_path
            .parent()
            .ok_or_else(|| OmniError::Config("Config path has no parent directory".into()))?;

        watcher.watch(watch_dir, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = OmniConfig::default();
        assert_eq!(config.general.log_level, "info");
        assert_eq!(config.general.max_history, 1000);
        assert!(!config.general.telemetry);
        assert!(config.guardian.enabled);
        assert_eq!(config.guardian.sensitivity, "balanced");
        assert_eq!(config.permissions.default_policy, "deny");
        assert!(config.permissions.audit_enabled);
        assert_eq!(config.agent.max_iterations, 25);
        assert_eq!(config.agent.timeout_secs, 120);
        assert_eq!(config.ui.theme, "system");
        assert_eq!(config.ui.font_size, 14);
    }

    #[test]
    fn test_parse_empty_toml() {
        let config: OmniConfig = toml::from_str("").unwrap();
        assert_eq!(config.general.log_level, "info");
        assert_eq!(config.permissions.default_policy, "deny");
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_str = r#"
[general]
telemetry = true
log_level = "debug"
max_history = 500

[providers.openai]
provider_type = "openai"
default_model = "gpt-4"
temperature = 0.7
enabled = true

[agent]
system_prompt = "You are a helpful assistant."
max_iterations = 10
timeout_secs = 60

[guardian]
enabled = false
sensitivity = "strict"
allow_override = false

[permissions]
default_policy = "prompt"
trust_verified = true
audit_enabled = false

[ui]
theme = "dark"
font_size = 16
show_action_feed = false
"#;
        let config: OmniConfig = toml::from_str(toml_str).unwrap();
        assert!(config.general.telemetry);
        assert_eq!(config.general.log_level, "debug");
        assert_eq!(config.general.max_history, 500);
        assert!(!config.guardian.enabled);
        assert_eq!(config.guardian.sensitivity, "strict");
        assert_eq!(config.permissions.default_policy, "prompt");
        assert!(config.permissions.trust_verified);
        assert_eq!(config.agent.max_iterations, 10);
        assert_eq!(config.ui.theme, "dark");
        assert_eq!(config.ui.font_size, 16);

        let provider = config.providers.get("openai").unwrap();
        assert_eq!(provider.provider_type, "openai");
        assert_eq!(provider.temperature, Some(0.7));
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
[general]
log_level = "warn"
"#;
        let config: OmniConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.log_level, "warn");
        assert_eq!(config.general.max_history, 1000); // default
        assert_eq!(config.permissions.default_policy, "deny"); // default
    }

    #[test]
    fn test_invalid_toml() {
        let result = toml::from_str::<OmniConfig>("this is not valid toml {{{");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_good_config() {
        let config = OmniConfig::default();
        let issues = config.validate();
        assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
    }

    #[test]
    fn test_validate_bad_log_level() {
        let mut config = OmniConfig::default();
        config.general.log_level = "verbose".to_string();
        let issues = config.validate();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("log level"));
    }

    #[test]
    fn test_validate_bad_sensitivity() {
        let mut config = OmniConfig::default();
        config.guardian.sensitivity = "ultra".to_string();
        let issues = config.validate();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("sensitivity"));
    }

    #[test]
    fn test_validate_bad_temperature() {
        let mut config = OmniConfig::default();
        config.providers.insert(
            "test".to_string(),
            ProviderConfig {
                provider_type: "openai".to_string(),
                default_model: None,
                endpoint: None,
                max_tokens: None,
                temperature: Some(3.0),
                enabled: true,
                transport: None,
            },
        );
        let issues = config.validate();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("temperature"));
    }

    #[test]
    fn test_roundtrip() {
        let config = OmniConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: OmniConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.general.log_level, deserialized.general.log_level);
        assert_eq!(
            config.permissions.default_policy,
            deserialized.permissions.default_policy
        );
        assert_eq!(
            config.agent.max_iterations,
            deserialized.agent.max_iterations
        );
    }

    #[test]
    fn test_load_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let config = OmniConfig::load(&path).unwrap();
        assert_eq!(config.general.log_level, "info");
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = OmniConfig::default();
        config.general.log_level = "debug".to_string();
        config.save(&path).unwrap();

        let loaded = OmniConfig::load(&path).unwrap();
        assert_eq!(loaded.general.log_level, "debug");
    }

    #[test]
    fn test_channels_config_default() {
        let config = OmniConfig::default();
        assert!(config.channels.instances.is_empty());
    }

    #[test]
    fn test_channels_config_parse() {
        let toml_str = r#"
[channels.instances."discord:production"]
channel_type = "discord"
display_name = "Discord Production Bot"
auto_connect = true

[channels.instances."twitter:brand-a"]
channel_type = "twitter"
display_name = "Brand A Twitter"
"#;
        let config: OmniConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.channels.instances.len(), 2);

        let discord = config.channels.instances.get("discord:production").unwrap();
        assert_eq!(discord.channel_type, "discord");
        assert_eq!(
            discord.display_name.as_deref(),
            Some("Discord Production Bot")
        );
        assert!(discord.auto_connect);

        let twitter = config.channels.instances.get("twitter:brand-a").unwrap();
        assert_eq!(twitter.channel_type, "twitter");
        assert!(!twitter.auto_connect);
    }

    #[test]
    fn test_no_channels_section_works() {
        let toml_str = r#"
[general]
log_level = "debug"
"#;
        let config: OmniConfig = toml::from_str(toml_str).unwrap();
        assert!(config.channels.instances.is_empty());
        assert_eq!(config.general.log_level, "debug");
    }

    #[test]
    fn test_env_vars_roundtrip() {
        let toml_str = r#"
[env_vars]
BRAVE_API_KEY = "sk-test-123456"
CUSTOM_VAR = "hello_world"
"#;
        let config: OmniConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.env_vars.len(), 2);
        assert_eq!(
            config.env_vars.get("BRAVE_API_KEY").unwrap(),
            "sk-test-123456"
        );
        assert_eq!(config.env_vars.get("CUSTOM_VAR").unwrap(), "hello_world");

        // Roundtrip
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: OmniConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.env_vars.len(), 2);
        assert_eq!(
            deserialized.env_vars.get("BRAVE_API_KEY").unwrap(),
            "sk-test-123456"
        );
    }

    #[test]
    fn test_env_vars_empty_default() {
        let config = OmniConfig::default();
        assert!(config.env_vars.is_empty());

        // Config without env_vars section should still deserialize
        let config2: OmniConfig = toml::from_str("[general]\nlog_level = \"info\"").unwrap();
        assert!(config2.env_vars.is_empty());
    }
}
