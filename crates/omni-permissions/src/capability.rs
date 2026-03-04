use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// All capabilities an extension can request
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "scope")]
pub enum Capability {
    #[serde(rename = "network.http")]
    NetworkHttp(Option<NetworkScope>),

    #[serde(rename = "network.websocket")]
    NetworkWebSocket(Option<NetworkScope>),

    #[serde(rename = "filesystem.read")]
    FilesystemRead(Option<FilesystemScope>),

    #[serde(rename = "filesystem.write")]
    FilesystemWrite(Option<FilesystemScope>),

    #[serde(rename = "clipboard.read")]
    ClipboardRead,

    #[serde(rename = "clipboard.write")]
    ClipboardWrite,

    #[serde(rename = "messaging.sms")]
    MessagingSms(Option<MessagingScope>),

    #[serde(rename = "messaging.email")]
    MessagingEmail(Option<MessagingScope>),

    /// Send/receive messages via chat channels (WhatsApp, Telegram, Discord, etc.)
    #[serde(rename = "messaging.chat")]
    MessagingChat(Option<MessagingScope>),

    #[serde(rename = "search.web")]
    SearchWeb(Option<SearchScope>),

    #[serde(rename = "process.spawn")]
    ProcessSpawn(Option<ProcessScope>),

    #[serde(rename = "system.notifications")]
    SystemNotifications,

    #[serde(rename = "device.camera")]
    DeviceCamera,

    #[serde(rename = "device.microphone")]
    DeviceMicrophone,

    #[serde(rename = "device.location")]
    DeviceLocation,

    #[serde(rename = "storage.persistent")]
    StoragePersistent(Option<StorageScope>),

    /// Schedule recurring or one-time tasks via cron expressions
    #[serde(rename = "system.scheduling")]
    SystemScheduling,

    /// Scrape web content using headless browser or HTML parsing
    #[serde(rename = "browser.scrape")]
    BrowserScrape(Option<BrowserScrapeScope>),

    /// Request AI/LLM inference from within an extension
    #[serde(rename = "ai.inference")]
    AiInference(Option<AiInferenceScope>),

    /// Send messages through connected channel plugins from within an extension
    #[serde(rename = "channel.send")]
    ChannelSend(Option<ChannelSendScope>),

    /// Interact with desktop applications via UI Automation APIs
    #[serde(rename = "app.automation")]
    AppAutomation(Option<AppAutomationScope>),

    /// Version control operations (git commit, branch, merge, etc.)
    #[serde(rename = "vcs.operations")]
    VersionControl(Option<VcsScope>),

    /// Connect to and invoke tools from MCP (Model Context Protocol) servers
    #[serde(rename = "mcp.server")]
    McpServer(Option<McpServerScope>),

    /// Code intelligence features (LSP navigation, code search, symbol lookup)
    #[serde(rename = "code.intelligence")]
    CodeIntelligence,

    /// Spawn sub-agents for parallel task execution
    #[serde(rename = "agent.spawn")]
    AgentSpawn(Option<AgentSpawnScope>),

    /// Control debug sessions (breakpoints, stepping, variable inspection)
    #[serde(rename = "debug.session")]
    Debugging,

    /// Invoke another flowchart as a sub-flow (for visual flow extensions)
    #[serde(rename = "flowchart.invoke")]
    FlowchartInvoke(Option<FlowchartInvokeScope>),
}

impl Capability {
    pub fn capability_key(&self) -> &'static str {
        match self {
            Self::NetworkHttp(_) => "network.http",
            Self::NetworkWebSocket(_) => "network.websocket",
            Self::FilesystemRead(_) => "filesystem.read",
            Self::FilesystemWrite(_) => "filesystem.write",
            Self::ClipboardRead => "clipboard.read",
            Self::ClipboardWrite => "clipboard.write",
            Self::MessagingSms(_) => "messaging.sms",
            Self::MessagingEmail(_) => "messaging.email",
            Self::MessagingChat(_) => "messaging.chat",
            Self::SearchWeb(_) => "search.web",
            Self::ProcessSpawn(_) => "process.spawn",
            Self::SystemNotifications => "system.notifications",
            Self::DeviceCamera => "device.camera",
            Self::DeviceMicrophone => "device.microphone",
            Self::DeviceLocation => "device.location",
            Self::StoragePersistent(_) => "storage.persistent",
            Self::SystemScheduling => "system.scheduling",
            Self::BrowserScrape(_) => "browser.scrape",
            Self::AiInference(_) => "ai.inference",
            Self::ChannelSend(_) => "channel.send",
            Self::AppAutomation(_) => "app.automation",
            Self::VersionControl(_) => "vcs.operations",
            Self::McpServer(_) => "mcp.server",
            Self::CodeIntelligence => "code.intelligence",
            Self::AgentSpawn(_) => "agent.spawn",
            Self::Debugging => "debug.session",
            Self::FlowchartInvoke(_) => "flowchart.invoke",
        }
    }
}

impl std::str::FromStr for Capability {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "network.http" => Ok(Self::NetworkHttp(None)),
            "network.websocket" => Ok(Self::NetworkWebSocket(None)),
            "filesystem.read" => Ok(Self::FilesystemRead(None)),
            "filesystem.write" => Ok(Self::FilesystemWrite(None)),
            "clipboard.read" => Ok(Self::ClipboardRead),
            "clipboard.write" => Ok(Self::ClipboardWrite),
            "messaging.sms" => Ok(Self::MessagingSms(None)),
            "messaging.email" => Ok(Self::MessagingEmail(None)),
            "messaging.chat" => Ok(Self::MessagingChat(None)),
            "search.web" => Ok(Self::SearchWeb(None)),
            "process.spawn" => Ok(Self::ProcessSpawn(None)),
            "system.notifications" => Ok(Self::SystemNotifications),
            "device.camera" => Ok(Self::DeviceCamera),
            "device.microphone" => Ok(Self::DeviceMicrophone),
            "device.location" => Ok(Self::DeviceLocation),
            "storage.persistent" => Ok(Self::StoragePersistent(None)),
            "system.scheduling" => Ok(Self::SystemScheduling),
            "browser.scrape" => Ok(Self::BrowserScrape(None)),
            "ai.inference" => Ok(Self::AiInference(None)),
            "channel.send" => Ok(Self::ChannelSend(None)),
            "app.automation" => Ok(Self::AppAutomation(None)),
            "vcs.operations" => Ok(Self::VersionControl(None)),
            "mcp.server" => Ok(Self::McpServer(None)),
            "code.intelligence" => Ok(Self::CodeIntelligence),
            "agent.spawn" => Ok(Self::AgentSpawn(None)),
            "debug.session" => Ok(Self::Debugging),
            "flowchart.invoke" => Ok(Self::FlowchartInvoke(None)),
            _ => Err(format!("Unknown capability: {s}")),
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.capability_key())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkScope {
    pub domains: Option<Vec<String>>,
    pub methods: Option<Vec<String>>,
    pub ports: Option<Vec<u16>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FilesystemScope {
    pub paths: Vec<String>,
    pub extensions: Option<Vec<String>>,
    pub max_size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessagingScope {
    pub recipients: Option<Vec<String>>,
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SearchScope {
    pub providers: Option<Vec<String>>,
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessScope {
    pub executables: Vec<String>,
    pub allowed_args: Option<Vec<String>>,
    pub denied_args: Option<Vec<String>>,
    pub max_concurrent: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageScope {
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BrowserScrapeScope {
    pub domains: Option<Vec<String>>,
    pub max_pages: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AiInferenceScope {
    /// Maximum tokens per request (cost control)
    pub max_tokens: Option<u32>,
    /// Rate limit: max requests per minute
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelSendScope {
    /// Restrict to specific channel instances (e.g., ["discord:production"])
    pub channels: Option<Vec<String>>,
    /// Rate limit: max messages per minute
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppAutomationScope {
    /// Allowed executable names or paths (e.g., ["notepad.exe", "C:\\Program Files\\MyApp\\app.exe"]).
    /// If None, all non-blocked executables are allowed (still subject to LOLBIN blocklist).
    pub allowed_apps: Option<Vec<String>>,
    /// Allowed action types. If None, all action types are allowed.
    /// Values: "launch", "list_windows", "find_element", "click", "type_text",
    ///         "read_text", "get_tree", "close"
    pub allowed_actions: Option<Vec<String>>,
    /// Maximum actions per minute per target application. Default: 60.
    pub rate_limit: Option<u32>,
    /// Maximum concurrent managed application processes. Default: 3.
    pub max_concurrent: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VcsScope {
    /// Allowed repository paths. If None, all repos are allowed.
    pub allowed_repos: Option<Vec<String>>,
    /// Allowed operations (e.g., ["status", "diff", "log"]). If None, all are allowed.
    pub allowed_operations: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McpServerScope {
    /// Allowed MCP server names. If None, all configured servers are allowed.
    pub allowed_servers: Option<Vec<String>>,
    /// Allowed tool names within servers. If None, all tools are allowed.
    pub allowed_tools: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentSpawnScope {
    /// Maximum concurrent sub-agents. Default: 4.
    pub max_concurrent: Option<u32>,
    /// Maximum iterations per sub-agent. Default: 25.
    pub max_iterations: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlowchartInvokeScope {
    /// Allowed target flowchart IDs (glob patterns). If empty/None, all are allowed.
    pub allowed_flowcharts: Option<Vec<String>>,
}

/// A concrete runtime request for a capability
#[derive(Debug, Clone)]
pub enum CapabilityRequest {
    HttpRequest {
        url: url::Url,
        method: String,
        body_size: Option<u64>,
    },
    WebSocketConnect {
        url: url::Url,
    },
    FileRead {
        path: PathBuf,
        size: Option<u64>,
    },
    FileWrite {
        path: PathBuf,
        size: Option<u64>,
    },
    ClipboardRead,
    ClipboardWrite {
        content_size: usize,
    },
    SendSms {
        recipient: String,
    },
    SendEmail {
        recipient: String,
    },
    SendChat {
        channel_id: String,
        recipient: String,
    },
    WebSearch {
        provider: Option<String>,
        query: String,
    },
    SpawnProcess {
        executable: String,
        args: Vec<String>,
    },
    ShowNotification,
    AccessCamera,
    AccessMicrophone,
    AccessLocation,
    PersistData {
        key: String,
        value_size: u64,
    },
    BrowserScrape {
        url: String,
        page_count: u32,
    },
    AppAutomation {
        app_name: String,
        action: String,
    },
    VersionControl {
        repo_path: Option<String>,
        operation: String,
    },
    McpToolCall {
        server_name: String,
        tool_name: String,
    },
    CodeIntelligence {
        action: String,
    },
    SpawnAgent {
        task: String,
    },
    DebugSession {
        action: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_key_all_variants() {
        let cases: Vec<(Capability, &str)> = vec![
            (Capability::NetworkHttp(None), "network.http"),
            (Capability::NetworkWebSocket(None), "network.websocket"),
            (Capability::FilesystemRead(None), "filesystem.read"),
            (Capability::FilesystemWrite(None), "filesystem.write"),
            (Capability::ClipboardRead, "clipboard.read"),
            (Capability::ClipboardWrite, "clipboard.write"),
            (Capability::MessagingSms(None), "messaging.sms"),
            (Capability::MessagingEmail(None), "messaging.email"),
            (Capability::MessagingChat(None), "messaging.chat"),
            (Capability::SearchWeb(None), "search.web"),
            (Capability::ProcessSpawn(None), "process.spawn"),
            (Capability::SystemNotifications, "system.notifications"),
            (Capability::DeviceCamera, "device.camera"),
            (Capability::DeviceMicrophone, "device.microphone"),
            (Capability::DeviceLocation, "device.location"),
            (Capability::StoragePersistent(None), "storage.persistent"),
            (Capability::SystemScheduling, "system.scheduling"),
            (Capability::BrowserScrape(None), "browser.scrape"),
            (Capability::AiInference(None), "ai.inference"),
            (Capability::ChannelSend(None), "channel.send"),
            (Capability::AppAutomation(None), "app.automation"),
            (Capability::VersionControl(None), "vcs.operations"),
            (Capability::McpServer(None), "mcp.server"),
            (Capability::CodeIntelligence, "code.intelligence"),
            (Capability::AgentSpawn(None), "agent.spawn"),
            (Capability::Debugging, "debug.session"),
            (Capability::FlowchartInvoke(None), "flowchart.invoke"),
        ];

        for (cap, expected) in cases {
            assert_eq!(cap.capability_key(), expected);
            assert_eq!(cap.to_string(), expected);
        }
    }

    #[test]
    fn test_serde_roundtrip_with_scope() {
        let cap = Capability::NetworkHttp(Some(NetworkScope {
            domains: Some(vec!["api.example.com".to_string()]),
            methods: Some(vec!["GET".to_string(), "POST".to_string()]),
            ports: Some(vec![443]),
        }));

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_without_scope() {
        let cap = Capability::ClipboardRead;
        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_filesystem_scope() {
        let cap = Capability::FilesystemRead(Some(FilesystemScope {
            paths: vec!["~/Documents".to_string()],
            extensions: Some(vec![".txt".to_string(), ".md".to_string()]),
            max_size: Some(10_000_000),
        }));

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_process_scope() {
        let cap = Capability::ProcessSpawn(Some(ProcessScope {
            executables: vec!["git".to_string(), "ls".to_string()],
            allowed_args: Some(vec!["status".to_string()]),
            denied_args: Some(vec!["--force".to_string()]),
            max_concurrent: Some(2),
        }));

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Capability::NetworkHttp(None)), "network.http");
        assert_eq!(format!("{}", Capability::DeviceCamera), "device.camera");
    }

    #[test]
    fn test_from_str_roundtrip() {
        let keys = vec![
            "network.http", "network.websocket", "filesystem.read", "filesystem.write",
            "clipboard.read", "clipboard.write", "messaging.sms", "messaging.email",
            "messaging.chat", "search.web", "process.spawn", "system.notifications", "device.camera",
            "device.microphone", "device.location", "storage.persistent",
            "system.scheduling",
            "browser.scrape",
            "ai.inference",
            "channel.send",
            "app.automation",
            "vcs.operations",
            "mcp.server",
            "code.intelligence",
            "agent.spawn",
            "debug.session",
            "flowchart.invoke",
        ];
        for key in keys {
            let cap: Capability = key.parse().unwrap();
            assert_eq!(cap.capability_key(), key);
        }
    }

    #[test]
    fn test_serde_roundtrip_browser_scrape_scope() {
        let cap = Capability::BrowserScrape(Some(BrowserScrapeScope {
            domains: Some(vec!["example.com".to_string(), "*.docs.rs".to_string()]),
            max_pages: Some(50),
        }));

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_app_automation_scope() {
        let cap = Capability::AppAutomation(Some(AppAutomationScope {
            allowed_apps: Some(vec!["notepad.exe".to_string(), "calculator.exe".to_string()]),
            allowed_actions: Some(vec!["launch".to_string(), "click".to_string()]),
            rate_limit: Some(30),
            max_concurrent: Some(2),
        }));

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn test_from_str_unknown() {
        let result: std::result::Result<Capability, _> = "network.ftp".parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown capability"));
    }
}
