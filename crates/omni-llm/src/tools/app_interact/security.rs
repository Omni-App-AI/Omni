use std::collections::{HashMap, HashSet};
use std::path::Path;

use omni_permissions::capability::AppAutomationScope;

use super::types::RateLimiterState;
use crate::error::{LlmError, Result};

/// Security guard for desktop app automation.
///
/// Enforces LOLBIN blocklist, executable allowlist, sensitive element detection,
/// and per-app rate limiting.
pub struct SecurityGuard {
    lolbin_blocklist: HashSet<String>,
    sensitive_patterns: Vec<regex::Regex>,
}

impl SecurityGuard {
    pub fn new() -> Self {
        Self {
            lolbin_blocklist: Self::default_lolbin_blocklist(),
            sensitive_patterns: Self::default_sensitive_patterns(),
        }
    }

    /// Windows Living Off the Land Binaries that could be weaponized for
    /// privilege escalation, code execution, or lateral movement.
    fn default_lolbin_blocklist() -> HashSet<String> {
        [
            // Shells and scripting engines
            "cmd.exe",
            "powershell.exe",
            "pwsh.exe",
            "wscript.exe",
            "cscript.exe",
            "bash.exe",
            "wsl.exe",
            // Execution proxies
            "mshta.exe",
            "regsvr32.exe",
            "rundll32.exe",
            "msiexec.exe",
            "installutil.exe",
            "msbuild.exe",
            "cmstp.exe",
            "pcalua.exe",
            "infdefaultinstall.exe",
            "presentationhost.exe",
            "dfsvc.exe",
            "xwizard.exe",
            // Credential/config tools
            "certutil.exe",
            "reg.exe",
            "net.exe",
            "net1.exe",
            "netsh.exe",
            "sc.exe",
            "wmic.exe",
            // Task/process management
            "schtasks.exe",
            "at.exe",
            "taskkill.exe",
            "forfiles.exe",
            // Data transfer
            "bitsadmin.exe",
            "ftp.exe",
            // System utilities that can be abused
            "explorer.exe",
            "control.exe",
            "eventvwr.exe",
            "mmc.exe",
            "diskshadow.exe",
            "dnscmd.exe",
            "eudcedit.exe",
            "conhost.exe",
            "winrm.cmd",
        ]
        .iter()
        .map(|s| s.to_lowercase())
        .collect()
    }

    /// Regex patterns for detecting sensitive UI elements beyond IsPassword.
    fn default_sensitive_patterns() -> Vec<regex::Regex> {
        [
            r"(?i)(password|passwd|secret|token|api.?key|credit.?card|cvv|ssn|social.?security)",
            r"(?i)(pin.?code|security.?code|2fa|totp|otp|auth.?code|mfa)",
        ]
        .iter()
        .filter_map(|p| regex::Regex::new(p).ok())
        .collect()
    }

    /// Check if an executable is on the LOLBIN blocklist.
    pub fn is_blocked_executable(&self, executable: &str) -> bool {
        let name = Path::new(executable)
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        self.lolbin_blocklist.contains(&name)
    }

    /// Check if an element name suggests it contains sensitive data.
    pub fn is_sensitive_element(&self, element_name: &str) -> bool {
        self.sensitive_patterns
            .iter()
            .any(|re| re.is_match(element_name))
    }

    /// Validate a launch request against LOLBIN blocklist and scope allowlist.
    pub fn validate_launch(
        &self,
        executable: &str,
        scope: &Option<AppAutomationScope>,
    ) -> Result<()> {
        // 1. LOLBIN check
        if self.is_blocked_executable(executable) {
            return Err(LlmError::ToolCall(format!(
                "Executable '{}' is blocked (known LOLBIN / security risk)",
                Path::new(executable)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| executable.to_string())
            )));
        }

        // 2. Allowlist check (if scope has allowed_apps)
        if let Some(scope) = scope {
            if let Some(ref allowed) = scope.allowed_apps {
                let name = Path::new(executable)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                let exe_lower = executable.to_lowercase();
                let matches = allowed
                    .iter()
                    .any(|a| a.to_lowercase() == name || a.to_lowercase() == exe_lower);
                if !matches {
                    return Err(LlmError::ToolCall(format!(
                        "Executable '{}' is not in the allowed application list",
                        executable
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate that an action is allowed by the scope.
    pub fn validate_action(action: &str, scope: &Option<AppAutomationScope>) -> Result<()> {
        if let Some(scope) = scope {
            if let Some(ref allowed) = scope.allowed_actions {
                if !allowed.iter().any(|a| a == action) {
                    return Err(LlmError::ToolCall(format!(
                        "Action '{}' is not in the allowed actions list",
                        action
                    )));
                }
            }
        }
        Ok(())
    }

    /// Check rate limit for a given app. Returns error if exceeded.
    pub fn check_rate_limit(
        limiters: &mut HashMap<String, RateLimiterState>,
        app_key: &str,
        max_per_minute: u32,
    ) -> Result<()> {
        let now = std::time::Instant::now();
        let entry = limiters
            .entry(app_key.to_string())
            .or_insert(RateLimiterState {
                window_start: now,
                action_count: 0,
            });

        // Reset window if more than 60 seconds have passed
        if now.duration_since(entry.window_start).as_secs() >= 60 {
            entry.window_start = now;
            entry.action_count = 0;
        }

        entry.action_count += 1;
        if entry.action_count > max_per_minute {
            return Err(LlmError::ToolCall(format!(
                "Rate limit exceeded for '{}': {} actions/minute (max: {})",
                app_key, entry.action_count, max_per_minute
            )));
        }

        Ok(())
    }

    /// Get the rate limit from scope, or default (60/min).
    pub fn rate_limit_for_scope(scope: &Option<AppAutomationScope>) -> u32 {
        scope
            .as_ref()
            .and_then(|s| s.rate_limit)
            .unwrap_or(60)
    }

    /// Get the max concurrent processes from scope, or default (3).
    pub fn max_concurrent_for_scope(scope: &Option<AppAutomationScope>) -> u32 {
        scope
            .as_ref()
            .and_then(|s| s.max_concurrent)
            .unwrap_or(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lolbin_blocklist_blocks_cmd() {
        let guard = SecurityGuard::new();
        assert!(guard.is_blocked_executable("cmd.exe"));
        assert!(guard.is_blocked_executable("CMD.EXE"));
    }

    #[test]
    fn test_lolbin_blocklist_blocks_powershell() {
        let guard = SecurityGuard::new();
        assert!(guard.is_blocked_executable("powershell.exe"));
        assert!(guard.is_blocked_executable("pwsh.exe"));
    }

    #[test]
    fn test_lolbin_blocklist_case_insensitive() {
        let guard = SecurityGuard::new();
        assert!(guard.is_blocked_executable("Cmd.Exe"));
        assert!(guard.is_blocked_executable("WSCRIPT.EXE"));
        assert!(guard.is_blocked_executable("RegSvr32.exe"));
    }

    #[test]
    fn test_lolbin_blocklist_allows_notepad() {
        let guard = SecurityGuard::new();
        assert!(!guard.is_blocked_executable("notepad.exe"));
        assert!(!guard.is_blocked_executable("calc.exe"));
        assert!(!guard.is_blocked_executable("firefox.exe"));
    }

    #[test]
    fn test_lolbin_blocklist_blocks_full_path() {
        let guard = SecurityGuard::new();
        assert!(guard.is_blocked_executable(r"C:\Windows\System32\cmd.exe"));
        assert!(guard.is_blocked_executable(r"C:\Windows\System32\powershell.exe"));
    }

    #[test]
    fn test_sensitive_patterns_detect_password() {
        let guard = SecurityGuard::new();
        assert!(guard.is_sensitive_element("Password"));
        assert!(guard.is_sensitive_element("Enter your password"));
        assert!(guard.is_sensitive_element("passwd_field"));
    }

    #[test]
    fn test_sensitive_patterns_detect_api_key() {
        let guard = SecurityGuard::new();
        assert!(guard.is_sensitive_element("API Key"));
        assert!(guard.is_sensitive_element("api_key"));
        assert!(guard.is_sensitive_element("apiKey"));
    }

    #[test]
    fn test_sensitive_patterns_detect_financial() {
        let guard = SecurityGuard::new();
        assert!(guard.is_sensitive_element("Credit Card Number"));
        assert!(guard.is_sensitive_element("CVV"));
        assert!(guard.is_sensitive_element("SSN"));
        assert!(guard.is_sensitive_element("Social Security"));
    }

    #[test]
    fn test_sensitive_patterns_detect_2fa() {
        let guard = SecurityGuard::new();
        assert!(guard.is_sensitive_element("2FA Code"));
        assert!(guard.is_sensitive_element("OTP"));
        assert!(guard.is_sensitive_element("Auth Code"));
        assert!(guard.is_sensitive_element("MFA"));
    }

    #[test]
    fn test_sensitive_patterns_case_insensitive() {
        let guard = SecurityGuard::new();
        assert!(guard.is_sensitive_element("PASSWORD"));
        assert!(guard.is_sensitive_element("Secret"));
        assert!(guard.is_sensitive_element("TOKEN"));
    }

    #[test]
    fn test_sensitive_patterns_allow_normal_names() {
        let guard = SecurityGuard::new();
        assert!(!guard.is_sensitive_element("Submit"));
        assert!(!guard.is_sensitive_element("Username"));
        assert!(!guard.is_sensitive_element("Email Address"));
        assert!(!guard.is_sensitive_element("Search"));
        assert!(!guard.is_sensitive_element("OK"));
    }

    #[test]
    fn test_validate_launch_lolbin_blocked() {
        let guard = SecurityGuard::new();
        let result = guard.validate_launch("cmd.exe", &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("blocked"));
    }

    #[test]
    fn test_validate_launch_allowed_app() {
        let guard = SecurityGuard::new();
        let scope = Some(AppAutomationScope {
            allowed_apps: Some(vec!["notepad.exe".to_string()]),
            allowed_actions: None,
            rate_limit: None,
            max_concurrent: None,
        });
        assert!(guard.validate_launch("notepad.exe", &scope).is_ok());
    }

    #[test]
    fn test_validate_launch_not_in_allowlist() {
        let guard = SecurityGuard::new();
        let scope = Some(AppAutomationScope {
            allowed_apps: Some(vec!["notepad.exe".to_string()]),
            allowed_actions: None,
            rate_limit: None,
            max_concurrent: None,
        });
        let result = guard.validate_launch("calc.exe", &scope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in the allowed"));
    }

    #[test]
    fn test_validate_launch_no_scope_allows_safe_apps() {
        let guard = SecurityGuard::new();
        assert!(guard.validate_launch("notepad.exe", &None).is_ok());
    }

    #[test]
    fn test_validate_action_allowed() {
        let scope = Some(AppAutomationScope {
            allowed_apps: None,
            allowed_actions: Some(vec!["launch".to_string(), "click".to_string()]),
            rate_limit: None,
            max_concurrent: None,
        });
        assert!(SecurityGuard::validate_action("launch", &scope).is_ok());
        assert!(SecurityGuard::validate_action("click", &scope).is_ok());
    }

    #[test]
    fn test_validate_action_blocked() {
        let scope = Some(AppAutomationScope {
            allowed_apps: None,
            allowed_actions: Some(vec!["launch".to_string()]),
            rate_limit: None,
            max_concurrent: None,
        });
        let result = SecurityGuard::validate_action("type_text", &scope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in the allowed"));
    }

    #[test]
    fn test_validate_action_no_scope_allows_all() {
        assert!(SecurityGuard::validate_action("anything", &None).is_ok());
    }

    #[test]
    fn test_rate_limit_within_limit() {
        let mut limiters = HashMap::new();
        for _ in 0..60 {
            assert!(SecurityGuard::check_rate_limit(&mut limiters, "notepad.exe", 60).is_ok());
        }
    }

    #[test]
    fn test_rate_limit_exceeded() {
        let mut limiters = HashMap::new();
        for _ in 0..60 {
            SecurityGuard::check_rate_limit(&mut limiters, "notepad.exe", 60).unwrap();
        }
        let result = SecurityGuard::check_rate_limit(&mut limiters, "notepad.exe", 60);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Rate limit exceeded"));
    }

    #[test]
    fn test_rate_limit_per_app_isolation() {
        let mut limiters = HashMap::new();
        // Fill up notepad's limit
        for _ in 0..60 {
            SecurityGuard::check_rate_limit(&mut limiters, "notepad.exe", 60).unwrap();
        }
        // calc.exe should still be allowed
        assert!(SecurityGuard::check_rate_limit(&mut limiters, "calc.exe", 60).is_ok());
    }

    #[test]
    fn test_rate_limit_defaults() {
        assert_eq!(SecurityGuard::rate_limit_for_scope(&None), 60);
        assert_eq!(SecurityGuard::max_concurrent_for_scope(&None), 3);

        let scope = Some(AppAutomationScope {
            allowed_apps: None,
            allowed_actions: None,
            rate_limit: Some(30),
            max_concurrent: Some(5),
        });
        assert_eq!(SecurityGuard::rate_limit_for_scope(&scope), 30);
        assert_eq!(SecurityGuard::max_concurrent_for_scope(&scope), 5);
    }
}
