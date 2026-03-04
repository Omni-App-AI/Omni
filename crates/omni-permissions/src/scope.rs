use std::path::{Path, PathBuf};

use crate::capability::*;

#[derive(Debug, thiserror::Error)]
pub enum ScopeViolation {
    #[error("Domain '{requested}' not in allowed list: {allowed:?}")]
    DomainNotAllowed {
        requested: String,
        allowed: Vec<String>,
    },

    #[error("Method '{requested}' not in allowed list: {allowed:?}")]
    MethodNotAllowed {
        requested: String,
        allowed: Vec<String>,
    },

    #[error("Port {requested} not in allowed list: {allowed:?}")]
    PortNotAllowed {
        requested: u16,
        allowed: Vec<u16>,
    },

    #[error("Path '{requested}' not within allowed paths: {allowed:?}")]
    PathNotAllowed {
        requested: String,
        allowed: Vec<String>,
    },

    #[error("File extension '{requested}' not in allowed list: {allowed:?}")]
    ExtensionNotAllowed {
        requested: String,
        allowed: Vec<String>,
    },

    #[error("File size {requested} exceeds maximum {max}")]
    FileTooLarge { requested: u64, max: u64 },

    #[error("Recipient '{requested}' not in allowed list: {allowed:?}")]
    RecipientNotAllowed {
        requested: String,
        allowed: Vec<String>,
    },

    #[error("Executable '{requested}' not in allowed list: {allowed:?}")]
    ExecutableNotAllowed {
        requested: String,
        allowed: Vec<String>,
    },

    #[error("Argument '{arg}' matches denied pattern '{pattern}'")]
    ArgumentDenied { arg: String, pattern: String },

    #[error("Storage limit exceeded: {requested} bytes, max {max}")]
    StorageLimitExceeded { requested: u64, max: u64 },

    #[error("Invalid URL")]
    InvalidUrl,

    #[error("Path not accessible")]
    PathNotAccessible,

    #[error("Provider '{requested}' not in allowed list: {allowed:?}")]
    ProviderNotAllowed {
        requested: String,
        allowed: Vec<String>,
    },

    #[error("Page count {requested} exceeds maximum {max}")]
    PageLimitExceeded { requested: u32, max: u32 },
}

fn domain_matches(pattern: &str, domain: &str) -> bool {
    if pattern.starts_with("*.") {
        domain.ends_with(&pattern[1..]) && domain.len() > pattern.len() - 1
    } else {
        domain == pattern
    }
}

fn validate_network_scope(
    scope: &NetworkScope,
    url: &url::Url,
    method: &str,
) -> Result<(), ScopeViolation> {
    if let Some(ref allowed_domains) = scope.domains {
        let request_domain = url
            .host_str()
            .ok_or(ScopeViolation::InvalidUrl)?;
        if !allowed_domains
            .iter()
            .any(|pattern| domain_matches(pattern, request_domain))
        {
            return Err(ScopeViolation::DomainNotAllowed {
                requested: request_domain.to_string(),
                allowed: allowed_domains.clone(),
            });
        }
    }

    if let Some(ref allowed_methods) = scope.methods {
        let upper = method.to_uppercase();
        if !allowed_methods
            .iter()
            .any(|m| m.to_uppercase() == upper)
        {
            return Err(ScopeViolation::MethodNotAllowed {
                requested: method.to_string(),
                allowed: allowed_methods.clone(),
            });
        }
    }

    if let Some(ref allowed_ports) = scope.ports {
        let port = url.port_or_known_default().unwrap_or(443);
        if !allowed_ports.contains(&port) {
            return Err(ScopeViolation::PortNotAllowed {
                requested: port,
                allowed: allowed_ports.clone(),
            });
        }
    }

    Ok(())
}

fn validate_filesystem_scope(
    scope: &FilesystemScope,
    path: &Path,
    size: Option<u64>,
) -> Result<(), ScopeViolation> {
    let canonical = path.canonicalize().unwrap_or_else(|_| {
        // File may not exist yet; try canonicalizing the parent and re-joining
        if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) {
            if let Ok(canon_parent) = parent.canonicalize() {
                return canon_parent.join(file_name);
            }
        }
        path.to_path_buf()
    });

    let path_allowed = scope.paths.iter().any(|allowed| {
        let expanded = shellexpand::tilde(allowed);
        let mut allowed_path = PathBuf::from(expanded.as_ref());

        // On Windows, if tilde expansion didn't resolve (no HOME env var),
        // fall back to dirs::home_dir
        #[cfg(target_os = "windows")]
        if allowed.starts_with('~') && expanded.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                allowed_path = home.join(allowed[1..].trim_start_matches(['/', '\\']));
            }
        }

        let canonical_allowed = allowed_path
            .canonicalize()
            .unwrap_or(allowed_path);
        canonical.starts_with(&canonical_allowed)
    });

    if !path_allowed {
        return Err(ScopeViolation::PathNotAllowed {
            requested: path.display().to_string(),
            allowed: scope.paths.clone(),
        });
    }

    if let Some(ref allowed_exts) = scope.extensions {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        if !allowed_exts.contains(&ext) {
            return Err(ScopeViolation::ExtensionNotAllowed {
                requested: ext,
                allowed: allowed_exts.clone(),
            });
        }
    }

    if let (Some(file_size), Some(max_size)) = (size, scope.max_size) {
        if file_size > max_size {
            return Err(ScopeViolation::FileTooLarge {
                requested: file_size,
                max: max_size,
            });
        }
    }

    Ok(())
}

fn validate_messaging_scope(
    scope: &MessagingScope,
    recipient: &str,
) -> Result<(), ScopeViolation> {
    if let Some(ref allowed_recipients) = scope.recipients {
        if !allowed_recipients.contains(&recipient.to_string()) {
            return Err(ScopeViolation::RecipientNotAllowed {
                requested: recipient.to_string(),
                allowed: allowed_recipients.clone(),
            });
        }
    }
    Ok(())
}

fn validate_process_scope(
    scope: &ProcessScope,
    executable: &str,
    args: &[String],
) -> Result<(), ScopeViolation> {
    if !scope
        .executables
        .iter()
        .any(|e| e == executable)
    {
        return Err(ScopeViolation::ExecutableNotAllowed {
            requested: executable.to_string(),
            allowed: scope.executables.clone(),
        });
    }

    // Check denied args first (deny takes priority)
    if let Some(ref denied_patterns) = scope.denied_args {
        for arg in args {
            for pattern in denied_patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(arg) {
                        return Err(ScopeViolation::ArgumentDenied {
                            arg: arg.clone(),
                            pattern: pattern.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_browser_scrape_scope(
    scope: &BrowserScrapeScope,
    url_str: &str,
    page_count: u32,
) -> Result<(), ScopeViolation> {
    if let Some(ref allowed_domains) = scope.domains {
        let parsed = url::Url::parse(url_str).map_err(|_| ScopeViolation::InvalidUrl)?;
        let request_domain = parsed
            .host_str()
            .ok_or(ScopeViolation::InvalidUrl)?;
        if !allowed_domains
            .iter()
            .any(|pattern| domain_matches(pattern, request_domain))
        {
            return Err(ScopeViolation::DomainNotAllowed {
                requested: request_domain.to_string(),
                allowed: allowed_domains.clone(),
            });
        }
    }

    if let Some(max) = scope.max_pages {
        if page_count > max {
            return Err(ScopeViolation::PageLimitExceeded {
                requested: page_count,
                max,
            });
        }
    }

    Ok(())
}

fn validate_search_scope(
    scope: &SearchScope,
    provider: Option<&str>,
) -> Result<(), ScopeViolation> {
    if let (Some(ref allowed_providers), Some(prov)) = (&scope.providers, provider) {
        if !allowed_providers.contains(&prov.to_string()) {
            return Err(ScopeViolation::ProviderNotAllowed {
                requested: prov.to_string(),
                allowed: allowed_providers.clone(),
            });
        }
    }
    Ok(())
}

/// Validate that a concrete request fits within the declared scope.
///
/// Returns `Ok(())` if the request is within scope, or a `ScopeViolation` error.
/// If no scope is declared (None), any request within the capability is allowed.
pub fn validate_scope(
    declared: &Capability,
    request: &CapabilityRequest,
) -> Result<(), ScopeViolation> {
    match (declared, request) {
        // Network HTTP
        (Capability::NetworkHttp(Some(scope)), CapabilityRequest::HttpRequest { url, method, .. }) => {
            validate_network_scope(scope, url, method)
        }
        (Capability::NetworkHttp(None), CapabilityRequest::HttpRequest { .. }) => Ok(()),

        // Network WebSocket
        (Capability::NetworkWebSocket(Some(scope)), CapabilityRequest::WebSocketConnect { url }) => {
            validate_network_scope(scope, url, "GET")
        }
        (Capability::NetworkWebSocket(None), CapabilityRequest::WebSocketConnect { .. }) => Ok(()),

        // Filesystem Read
        (Capability::FilesystemRead(Some(scope)), CapabilityRequest::FileRead { path, size }) => {
            validate_filesystem_scope(scope, path, *size)
        }
        (Capability::FilesystemRead(None), CapabilityRequest::FileRead { .. }) => Ok(()),

        // Filesystem Write
        (Capability::FilesystemWrite(Some(scope)), CapabilityRequest::FileWrite { path, size }) => {
            validate_filesystem_scope(scope, path, *size)
        }
        (Capability::FilesystemWrite(None), CapabilityRequest::FileWrite { .. }) => Ok(()),

        // Clipboard
        (Capability::ClipboardRead, CapabilityRequest::ClipboardRead) => Ok(()),
        (Capability::ClipboardWrite, CapabilityRequest::ClipboardWrite { .. }) => Ok(()),

        // Messaging SMS
        (Capability::MessagingSms(Some(scope)), CapabilityRequest::SendSms { recipient }) => {
            validate_messaging_scope(scope, recipient)
        }
        (Capability::MessagingSms(None), CapabilityRequest::SendSms { .. }) => Ok(()),

        // Messaging Email
        (Capability::MessagingEmail(Some(scope)), CapabilityRequest::SendEmail { recipient }) => {
            validate_messaging_scope(scope, recipient)
        }
        (Capability::MessagingEmail(None), CapabilityRequest::SendEmail { .. }) => Ok(()),

        // Search
        (Capability::SearchWeb(Some(scope)), CapabilityRequest::WebSearch { provider, .. }) => {
            validate_search_scope(scope, provider.as_deref())
        }
        (Capability::SearchWeb(None), CapabilityRequest::WebSearch { .. }) => Ok(()),

        // Process Spawn
        (Capability::ProcessSpawn(Some(scope)), CapabilityRequest::SpawnProcess { executable, args }) => {
            validate_process_scope(scope, executable, args)
        }
        (Capability::ProcessSpawn(None), CapabilityRequest::SpawnProcess { .. }) => Ok(()),

        // Scopeless capabilities
        (Capability::SystemNotifications, CapabilityRequest::ShowNotification) => Ok(()),
        (Capability::DeviceCamera, CapabilityRequest::AccessCamera) => Ok(()),
        (Capability::DeviceMicrophone, CapabilityRequest::AccessMicrophone) => Ok(()),
        (Capability::DeviceLocation, CapabilityRequest::AccessLocation) => Ok(()),

        // Storage
        (Capability::StoragePersistent(Some(scope)), CapabilityRequest::PersistData { value_size, .. }) => {
            if let Some(max) = scope.max_bytes {
                if *value_size > max {
                    return Err(ScopeViolation::StorageLimitExceeded {
                        requested: *value_size,
                        max,
                    });
                }
            }
            Ok(())
        }
        (Capability::StoragePersistent(None), CapabilityRequest::PersistData { .. }) => Ok(()),

        // Browser Scrape
        (Capability::BrowserScrape(Some(scope)), CapabilityRequest::BrowserScrape { url, page_count }) => {
            validate_browser_scrape_scope(scope, url, *page_count)
        }
        (Capability::BrowserScrape(None), CapabilityRequest::BrowserScrape { .. }) => Ok(()),

        // Mismatched capability/request -- not a scope violation, caller should ensure correct pairing
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    // --- Network HTTP ---

    #[test]
    fn test_network_http_allowed_domain() {
        let cap = Capability::NetworkHttp(Some(NetworkScope {
            domains: Some(vec!["api.example.com".to_string()]),
            methods: Some(vec!["GET".to_string()]),
            ports: None,
        }));
        let req = CapabilityRequest::HttpRequest {
            url: Url::parse("https://api.example.com/data").unwrap(),
            method: "GET".to_string(),
            body_size: None,
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_network_http_denied_domain() {
        let cap = Capability::NetworkHttp(Some(NetworkScope {
            domains: Some(vec!["api.example.com".to_string()]),
            methods: None,
            ports: None,
        }));
        let req = CapabilityRequest::HttpRequest {
            url: Url::parse("https://evil.com/steal").unwrap(),
            method: "GET".to_string(),
            body_size: None,
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::DomainNotAllowed { .. })
        ));
    }

    #[test]
    fn test_network_http_wildcard_domain() {
        let cap = Capability::NetworkHttp(Some(NetworkScope {
            domains: Some(vec!["*.github.com".to_string()]),
            methods: None,
            ports: None,
        }));
        let req_ok = CapabilityRequest::HttpRequest {
            url: Url::parse("https://api.github.com/repos").unwrap(),
            method: "GET".to_string(),
            body_size: None,
        };
        assert!(validate_scope(&cap, &req_ok).is_ok());

        // The root domain itself should not match *.github.com
        let req_root = CapabilityRequest::HttpRequest {
            url: Url::parse("https://github.com/repos").unwrap(),
            method: "GET".to_string(),
            body_size: None,
        };
        assert!(validate_scope(&cap, &req_root).is_err());
    }

    #[test]
    fn test_network_http_denied_method() {
        let cap = Capability::NetworkHttp(Some(NetworkScope {
            domains: None,
            methods: Some(vec!["GET".to_string()]),
            ports: None,
        }));
        let req = CapabilityRequest::HttpRequest {
            url: Url::parse("https://api.example.com/data").unwrap(),
            method: "DELETE".to_string(),
            body_size: None,
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::MethodNotAllowed { .. })
        ));
    }

    #[test]
    fn test_network_http_denied_port() {
        let cap = Capability::NetworkHttp(Some(NetworkScope {
            domains: None,
            methods: None,
            ports: Some(vec![443]),
        }));
        let req = CapabilityRequest::HttpRequest {
            url: Url::parse("https://api.example.com:8080/data").unwrap(),
            method: "GET".to_string(),
            body_size: None,
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::PortNotAllowed { .. })
        ));
    }

    // --- Filesystem ---

    #[test]
    fn test_filesystem_no_scope_allows_any() {
        let cap = Capability::FilesystemRead(None);
        let req = CapabilityRequest::FileRead {
            path: PathBuf::from("/etc/passwd"),
            size: None,
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_filesystem_file_too_large() {
        let tmp = std::env::temp_dir();
        let cap = Capability::FilesystemRead(Some(FilesystemScope {
            paths: vec![tmp.display().to_string()],
            extensions: None,
            max_size: Some(1000),
        }));
        let req = CapabilityRequest::FileRead {
            path: tmp.join("big.txt"),
            size: Some(5000),
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::FileTooLarge { .. })
        ));
    }

    // --- Clipboard ---

    #[test]
    fn test_clipboard_read_allowed() {
        let cap = Capability::ClipboardRead;
        let req = CapabilityRequest::ClipboardRead;
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_clipboard_write_allowed() {
        let cap = Capability::ClipboardWrite;
        let req = CapabilityRequest::ClipboardWrite { content_size: 100 };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    // --- Messaging ---

    #[test]
    fn test_messaging_allowed_recipient() {
        let cap = Capability::MessagingSms(Some(MessagingScope {
            recipients: Some(vec!["+1234567890".to_string()]),
            rate_limit: None,
        }));
        let req = CapabilityRequest::SendSms {
            recipient: "+1234567890".to_string(),
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_messaging_denied_recipient() {
        let cap = Capability::MessagingEmail(Some(MessagingScope {
            recipients: Some(vec!["alice@example.com".to_string()]),
            rate_limit: None,
        }));
        let req = CapabilityRequest::SendEmail {
            recipient: "evil@hacker.com".to_string(),
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::RecipientNotAllowed { .. })
        ));
    }

    // --- Search ---

    #[test]
    fn test_search_allowed_provider() {
        let cap = Capability::SearchWeb(Some(SearchScope {
            providers: Some(vec!["google".to_string()]),
            rate_limit: None,
        }));
        let req = CapabilityRequest::WebSearch {
            provider: Some("google".to_string()),
            query: "rust programming".to_string(),
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_search_denied_provider() {
        let cap = Capability::SearchWeb(Some(SearchScope {
            providers: Some(vec!["google".to_string()]),
            rate_limit: None,
        }));
        let req = CapabilityRequest::WebSearch {
            provider: Some("bing".to_string()),
            query: "test".to_string(),
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::ProviderNotAllowed { .. })
        ));
    }

    // --- Process ---

    #[test]
    fn test_process_allowed_executable() {
        let cap = Capability::ProcessSpawn(Some(ProcessScope {
            executables: vec!["git".to_string()],
            allowed_args: None,
            denied_args: None,
            max_concurrent: None,
        }));
        let req = CapabilityRequest::SpawnProcess {
            executable: "git".to_string(),
            args: vec!["status".to_string()],
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_process_denied_executable() {
        let cap = Capability::ProcessSpawn(Some(ProcessScope {
            executables: vec!["git".to_string()],
            allowed_args: None,
            denied_args: None,
            max_concurrent: None,
        }));
        let req = CapabilityRequest::SpawnProcess {
            executable: "rm".to_string(),
            args: vec!["-rf".to_string(), "/".to_string()],
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::ExecutableNotAllowed { .. })
        ));
    }

    #[test]
    fn test_process_denied_args() {
        let cap = Capability::ProcessSpawn(Some(ProcessScope {
            executables: vec!["git".to_string()],
            allowed_args: None,
            denied_args: Some(vec!["--force".to_string()]),
            max_concurrent: None,
        }));
        let req = CapabilityRequest::SpawnProcess {
            executable: "git".to_string(),
            args: vec!["push".to_string(), "--force".to_string()],
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::ArgumentDenied { .. })
        ));
    }

    // --- Scopeless capabilities ---

    #[test]
    fn test_scopeless_capabilities() {
        assert!(validate_scope(&Capability::SystemNotifications, &CapabilityRequest::ShowNotification).is_ok());
        assert!(validate_scope(&Capability::DeviceCamera, &CapabilityRequest::AccessCamera).is_ok());
        assert!(validate_scope(&Capability::DeviceMicrophone, &CapabilityRequest::AccessMicrophone).is_ok());
        assert!(validate_scope(&Capability::DeviceLocation, &CapabilityRequest::AccessLocation).is_ok());
    }

    // --- Storage ---

    #[test]
    fn test_storage_within_limit() {
        let cap = Capability::StoragePersistent(Some(StorageScope {
            max_bytes: Some(1_000_000),
        }));
        let req = CapabilityRequest::PersistData {
            key: "test".to_string(),
            value_size: 500,
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_storage_exceeds_limit() {
        let cap = Capability::StoragePersistent(Some(StorageScope {
            max_bytes: Some(1000),
        }));
        let req = CapabilityRequest::PersistData {
            key: "test".to_string(),
            value_size: 5000,
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::StorageLimitExceeded { .. })
        ));
    }

    // --- Browser Scrape ---

    #[test]
    fn test_browser_scrape_no_scope_allows_any() {
        let cap = Capability::BrowserScrape(None);
        let req = CapabilityRequest::BrowserScrape {
            url: "https://example.com".to_string(),
            page_count: 100,
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_browser_scrape_allowed_domain() {
        let cap = Capability::BrowserScrape(Some(BrowserScrapeScope {
            domains: Some(vec!["example.com".to_string()]),
            max_pages: None,
        }));
        let req = CapabilityRequest::BrowserScrape {
            url: "https://example.com/page".to_string(),
            page_count: 1,
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_browser_scrape_denied_domain() {
        let cap = Capability::BrowserScrape(Some(BrowserScrapeScope {
            domains: Some(vec!["example.com".to_string()]),
            max_pages: None,
        }));
        let req = CapabilityRequest::BrowserScrape {
            url: "https://evil.com/steal".to_string(),
            page_count: 1,
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::DomainNotAllowed { .. })
        ));
    }

    #[test]
    fn test_browser_scrape_wildcard_domain() {
        let cap = Capability::BrowserScrape(Some(BrowserScrapeScope {
            domains: Some(vec!["*.docs.rs".to_string()]),
            max_pages: None,
        }));
        let req = CapabilityRequest::BrowserScrape {
            url: "https://tokio.docs.rs/tokio".to_string(),
            page_count: 1,
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }

    #[test]
    fn test_browser_scrape_page_limit_exceeded() {
        let cap = Capability::BrowserScrape(Some(BrowserScrapeScope {
            domains: None,
            max_pages: Some(10),
        }));
        let req = CapabilityRequest::BrowserScrape {
            url: "https://example.com".to_string(),
            page_count: 50,
        };
        assert!(matches!(
            validate_scope(&cap, &req),
            Err(ScopeViolation::PageLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_browser_scrape_within_page_limit() {
        let cap = Capability::BrowserScrape(Some(BrowserScrapeScope {
            domains: None,
            max_pages: Some(100),
        }));
        let req = CapabilityRequest::BrowserScrape {
            url: "https://example.com".to_string(),
            page_count: 50,
        };
        assert!(validate_scope(&cap, &req).is_ok());
    }
}
