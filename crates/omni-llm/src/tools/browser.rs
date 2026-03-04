//! Web scraping tool with two tiers:
//! - **extract** (Tier 1): Fast HTML parsing via `scraper` crate -- no browser needed
//! - **browser** (Tier 2): Puppeteer stealth sidecar -- JS rendering, anti-bot evasion
//! - **crawl**: Multi-page site crawling via either tier
//!
//! Gated by `browser.scrape` permission.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Maximum content per page (500KB).
const MAX_PAGE_CONTENT: usize = 500 * 1024;
/// Maximum total content for crawl results (5MB).
const MAX_CRAWL_TOTAL: usize = 5 * 1024 * 1024;
/// Maximum pages in a single crawl.
const MAX_CRAWL_PAGES: usize = 100;
/// Maximum crawl depth.
const MAX_CRAWL_DEPTH: usize = 5;
/// Politeness delay between extract-mode requests (ms).
const EXTRACT_DELAY_MS: u64 = 1000;
/// Timeout for single-page scrape/screenshot commands (60s).
const SCRAPE_TIMEOUT_SECS: u64 = 60;
/// Timeout for crawl commands (10 minutes).
const CRAWL_TIMEOUT_SECS: u64 = 600;

// ---------------------------------------------------------------------------
// BrowserSidecar -- manages the Node.js Puppeteer process
// ---------------------------------------------------------------------------

struct BrowserSidecar {
    #[allow(dead_code)]
    child: Mutex<Option<Child>>,
    stdin_writer: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    pending_requests:
        Arc<Mutex<std::collections::HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
    next_id: AtomicU64,
    /// Set to true when the stdout reader detects the sidecar process has exited.
    dead: Arc<AtomicBool>,
}

impl BrowserSidecar {
    /// Spawn the sidecar, returning once the "ready" event is received.
    async fn launch(sidecar_dir: &std::path::Path) -> Result<Self> {
        // Find node executable
        let node = which_node()?;

        // Auto-install deps if needed (async to avoid blocking the runtime)
        let node_modules = sidecar_dir.join("node_modules");
        if !node_modules.exists() {
            tracing::info!("Installing browser sidecar dependencies...");
            let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
            let status = Command::new(npm)
                .args(["install", "--production"])
                .current_dir(sidecar_dir)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .status()
                .await
                .map_err(|e| {
                    LlmError::ToolCall(format!(
                        "Failed to run npm install in {}: {e}. Ensure Node.js/npm is installed.",
                        sidecar_dir.display()
                    ))
                })?;
            if !status.success() {
                return Err(LlmError::ToolCall(
                    "npm install failed for browser sidecar".to_string(),
                ));
            }
        }

        let bridge_path = sidecar_dir.join("bridge.js");
        let mut child = Command::new(&node)
            .arg(&bridge_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .current_dir(sidecar_dir)
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| LlmError::ToolCall(format!("Failed to spawn browser sidecar: {e}")))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // Drain stderr in background to prevent buffer deadlock
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(target: "browser_sidecar_stderr", "{}", line);
                }
            });
        }

        let stdin_writer = Arc::new(Mutex::new(Some(stdin)));
        let pending_requests: Arc<
            Mutex<std::collections::HashMap<u64, oneshot::Sender<serde_json::Value>>>,
        > = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let dead = Arc::new(AtomicBool::new(false));

        // Create a oneshot channel for the "ready" signal
        let (ready_tx, ready_rx) = oneshot::channel::<()>();
        let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

        // Spawn stdout reader task -- when stdout closes, mark sidecar as dead
        // and fail all pending requests so callers don't hang.
        let pending_clone = pending_requests.clone();
        let ready_tx_clone = ready_tx.clone();
        let dead_clone = dead.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };

                // Route response by ID
                if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                    let mut pending = pending_clone.lock().await;
                    if let Some(sender) = pending.remove(&id) {
                        if let Some(error) = msg.get("error") {
                            let _ = sender.send(serde_json::json!({
                                "__error": error.as_str().unwrap_or("unknown error")
                            }));
                        } else if let Some(result) = msg.get("result") {
                            let _ = sender.send(result.clone());
                        }
                    }
                }
                // Events (crawl_progress, etc.) are logged but not routed
                else if let Some(event) = msg.get("event").and_then(|v| v.as_str()) {
                    match event {
                        "ready" => {
                            tracing::info!("Browser sidecar ready");
                            let mut guard = ready_tx_clone.lock().await;
                            if let Some(tx) = guard.take() {
                                let _ = tx.send(());
                            }
                        }
                        "crawl_progress" => {
                            if let Some(data) = msg.get("data") {
                                tracing::debug!("Crawl progress: {data}");
                            }
                        }
                        "crawl_error" => {
                            if let Some(data) = msg.get("data") {
                                tracing::warn!("Crawl error: {data}");
                            }
                        }
                        "warning" => {
                            if let Some(data) = msg.get("data") {
                                tracing::warn!("Browser sidecar warning: {data}");
                            }
                        }
                        _ => tracing::debug!("Browser event: {event}"),
                    }
                }
            }

            // Stdout closed -- sidecar process has exited
            tracing::warn!("Browser sidecar stdout closed -- process died");
            dead_clone.store(true, Ordering::Release);

            // Fail all pending requests so callers don't hang forever
            let mut pending = pending_clone.lock().await;
            for (_, sender) in pending.drain() {
                let _ = sender.send(serde_json::json!({
                    "__error": "Browser sidecar process died"
                }));
            }
        });

        // Wait for the "ready" event (with timeout)
        if tokio::time::timeout(std::time::Duration::from_secs(10), ready_rx)
            .await
            .is_err()
        {
            tracing::warn!("Browser sidecar did not send 'ready' within 10s, proceeding anyway");
        }

        Ok(Self {
            child: Mutex::new(Some(child)),
            stdin_writer,
            pending_requests,
            next_id: AtomicU64::new(1),
            dead,
        })
    }

    /// Returns true if the sidecar process is still alive.
    fn is_alive(&self) -> bool {
        !self.dead.load(Ordering::Acquire)
    }

    /// Send a JSON-RPC command and await the response.
    /// Timeout is chosen per-method: short for scrape/screenshot, long for crawl.
    async fn send_command(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Check liveness before sending
        if !self.is_alive() {
            return Err(LlmError::ToolCall(
                "Browser sidecar is dead -- will relaunch on next request".to_string(),
            ));
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = serde_json::json!({ "id": id, "method": method, "params": params });
        let line = format!("{}\n", serde_json::to_string(&msg).unwrap());

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, tx);
        }

        {
            let mut writer_guard = self.stdin_writer.lock().await;
            let writer = writer_guard.as_mut().ok_or_else(|| {
                LlmError::ToolCall("Browser sidecar stdin closed".to_string())
            })?;
            if let Err(e) = writer.write_all(line.as_bytes()).await {
                // Write failed -- sidecar stdin is broken, mark dead
                self.dead.store(true, Ordering::Release);
                return Err(LlmError::ToolCall(format!(
                    "Failed to write to browser sidecar: {e}"
                )));
            }
            writer.flush().await.ok();
        }

        // Per-method timeout: scrape/screenshot get 60s, crawl gets 10min
        let timeout_secs = match method {
            "crawl" => CRAWL_TIMEOUT_SECS,
            _ => SCRAPE_TIMEOUT_SECS,
        };

        let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx)
            .await
            .map_err(|_| {
                // Clean up the pending request on timeout
                let pending = self.pending_requests.clone();
                let id = id;
                tokio::spawn(async move {
                    pending.lock().await.remove(&id);
                });
                LlmError::ToolCall(format!(
                    "Browser sidecar timed out ({timeout_secs}s) on '{method}'"
                ))
            })?
            .map_err(|_| LlmError::ToolCall("Browser sidecar channel closed".to_string()))?;

        // Check for error response
        if let Some(error) = result.get("__error").and_then(|v| v.as_str()) {
            return Err(LlmError::ToolCall(format!("Browser sidecar error: {error}")));
        }

        Ok(result)
    }

    /// Kill the sidecar process (best-effort).
    async fn kill(&self) {
        // Close stdin to signal graceful shutdown
        {
            let mut guard = self.stdin_writer.lock().await;
            *guard = None;
        }
        // Kill the child process
        let mut guard = self.child.lock().await;
        if let Some(ref mut child) = *guard {
            let _ = child.kill().await;
        }
        *guard = None;
        self.dead.store(true, Ordering::Release);
    }
}

fn which_node() -> Result<String> {
    let candidates = if cfg!(windows) {
        vec!["node.exe", "node"]
    } else {
        vec!["node"]
    };

    for candidate in &candidates {
        if let Ok(output) = std::process::Command::new(candidate)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                return Ok(candidate.to_string());
            }
        }
    }

    Err(LlmError::ToolCall(
        "Node.js not found on PATH. Install Node.js to use browser scraping mode.".to_string(),
    ))
}

fn sidecar_dir() -> std::path::PathBuf {
    // Look relative to the executable, then fall back to known paths
    if let Ok(exe) = std::env::current_exe() {
        let candidates = [
            exe.parent()
                .unwrap_or(std::path::Path::new("."))
                .join("sidecar/browser"),
            exe.parent()
                .unwrap_or(std::path::Path::new("."))
                .join("../sidecar/browser"),
        ];
        for c in &candidates {
            if c.join("bridge.js").exists() {
                return c.clone();
            }
        }
    }

    // Fallback: look from CWD
    let cwd_candidates = [
        std::path::PathBuf::from("sidecar/browser"),
        std::path::PathBuf::from("../../sidecar/browser"),
    ];
    for c in &cwd_candidates {
        if c.join("bridge.js").exists() {
            return c.clone();
        }
    }

    // Last resort
    std::path::PathBuf::from("sidecar/browser")
}

// ---------------------------------------------------------------------------
// Tier 1: Extract mode -- fast HTML parsing without a browser
// ---------------------------------------------------------------------------

/// Result of extracting a page, including links for crawl reuse.
struct ExtractedPage {
    json: serde_json::Value,
    links: Vec<String>,
}

/// Fetch a URL and extract clean text content using the `scraper` crate.
/// Returns both the JSON result and the discovered links (to avoid re-fetching in crawl mode).
async fn extract_page(
    client: &reqwest::Client,
    url: &str,
    selectors: &[String],
    output_format: &str,
) -> Result<ExtractedPage> {
    let response = client
        .get(url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| LlmError::ToolCall(format!("Failed to fetch '{}': {e}", url)))?;

    let status = response.status().as_u16();
    if status >= 400 {
        return Err(LlmError::ToolCall(format!(
            "HTTP {status} for '{url}'"
        )));
    }

    // Download with size limit
    let max_download = 2 * 1024 * 1024; // 2MB HTML limit
    let mut body_bytes = Vec::new();
    let mut stream = response;
    while let Some(chunk) = stream
        .chunk()
        .await
        .map_err(|e| LlmError::ToolCall(format!("Failed to read response: {e}")))?
    {
        body_bytes.extend_from_slice(&chunk);
        if body_bytes.len() >= max_download {
            break;
        }
    }

    let html = String::from_utf8_lossy(&body_bytes);
    let document = scraper::Html::parse_document(&html);

    // Always extract links (cheap, needed by crawl mode)
    let links = extract_links_from_html(&document, url);

    if !selectors.is_empty() {
        // Extract specific selectors
        let mut extracted = serde_json::Map::new();
        for sel_str in selectors {
            if let Ok(selector) = scraper::Selector::parse(sel_str) {
                let elements: Vec<String> = document
                    .select(&selector)
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                extracted.insert(sel_str.clone(), serde_json::json!(elements));
            }
        }
        return Ok(ExtractedPage {
            json: serde_json::json!({
                "url": url,
                "title": extract_title(&document),
                "selectors": extracted,
                "mode_used": "extract",
            }),
            links,
        });
    }

    // Full content extraction -- remove scripts, styles, navs
    let content = extract_body_text(&document, output_format);
    let title = extract_title(&document);

    let truncated = content.len() > MAX_PAGE_CONTENT;
    let content = if truncated {
        let boundary = super::util::floor_char_boundary(&content, MAX_PAGE_CONTENT);
        format!(
            "{}\n\n[Content truncated at {}KB]",
            &content[..boundary],
            MAX_PAGE_CONTENT / 1024
        )
    } else {
        content
    };

    let excerpt = content
        .chars()
        .take(200)
        .collect::<String>()
        .trim()
        .to_string();

    Ok(ExtractedPage {
        json: serde_json::json!({
            "url": url,
            "title": title,
            "content": content,
            "excerpt": excerpt,
            "links_found": links.len(),
            "mode_used": "extract",
            "truncated": truncated,
        }),
        links,
    })
}

fn extract_title(document: &scraper::Html) -> String {
    scraper::Selector::parse("title")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .map(|el| el.text().collect::<String>().trim().to_string())
        .unwrap_or_default()
}

fn extract_body_text(document: &scraper::Html, output_format: &str) -> String {
    // Selectors for content we want to SKIP
    let skip_tags = ["script", "style", "nav", "footer", "header", "aside", "noscript"];

    // Try to find main content areas first
    let content_selectors = ["main", "article", "[role=main]", ".content", "#content"];

    for sel_str in &content_selectors {
        if let Ok(selector) = scraper::Selector::parse(sel_str) {
            let mut found = document.select(&selector).peekable();
            if found.peek().is_some() {
                let text: String = found
                    .flat_map(|el| el.text())
                    .collect::<Vec<_>>()
                    .join(" ");
                let cleaned = clean_whitespace(&text);
                if cleaned.len() > 100 {
                    return if output_format == "html" {
                        // Return raw HTML of the content element
                        document
                            .select(&selector)
                            .map(|el| el.html())
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        cleaned
                    };
                }
            }
        }
    }

    // Fallback: extract all text from body, skipping unwanted elements
    if let Ok(body_sel) = scraper::Selector::parse("body") {
        if let Some(body) = document.select(&body_sel).next() {
            let mut parts = Vec::new();
            collect_text_recursive(body, &skip_tags, &mut parts);
            return clean_whitespace(&parts.join(" "));
        }
    }

    String::new()
}

fn collect_text_recursive(
    element: scraper::ElementRef,
    skip_tags: &[&str],
    parts: &mut Vec<String>,
) {
    let tag = element.value().name();
    if skip_tags.contains(&tag) {
        return;
    }

    for child in element.children() {
        if let Some(text) = child.value().as_text() {
            let t = text.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
        } else if let Some(el) = scraper::ElementRef::wrap(child) {
            collect_text_recursive(el, skip_tags, parts);
        }
    }
}

fn clean_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }
    result.trim().to_string()
}

fn extract_links_from_html(document: &scraper::Html, base_url: &str) -> Vec<String> {
    let Ok(sel) = scraper::Selector::parse("a[href]") else {
        return Vec::new();
    };

    let base = url::Url::parse(base_url).ok();
    let mut seen = HashSet::new();
    let mut links = Vec::new();

    for element in document.select(&sel) {
        if let Some(href) = element.value().attr("href") {
            let resolved = if let Some(ref base) = base {
                base.join(href).map(|u| u.to_string()).ok()
            } else {
                Some(href.to_string())
            };

            if let Some(url) = resolved {
                if url.starts_with("http://") || url.starts_with("https://") {
                    // Strip fragment and deduplicate
                    let clean = url.split('#').next().unwrap_or(&url).to_string();
                    if seen.insert(clean.clone()) {
                        links.push(clean);
                    }
                }
            }
        }
    }

    links
}

/// Compile a glob pattern into a regex for URL matching.
fn compile_url_glob(pattern: &str) -> Option<regex::Regex> {
    let regex_str = pattern
        .chars()
        .map(|ch| match ch {
            '*' => ".*".to_string(),
            '?' => ".".to_string(),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                format!("\\{ch}")
            }
            c => c.to_string(),
        })
        .collect::<String>();

    regex::Regex::new(&format!("^{regex_str}$")).ok()
}

/// Simple glob matching for URL patterns (used in tests; crawl loop uses compile_url_glob).
#[cfg(test)]
fn url_glob_match(pattern: &str, url: &str) -> bool {
    compile_url_glob(pattern)
        .map(|re| re.is_match(url))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// WebScrapeTool -- NativeTool implementation
// ---------------------------------------------------------------------------

/// Native tool for web scraping with two tiers + crawl mode.
pub struct WebScrapeTool {
    client: reqwest::Client,
    /// Lazy-initialized sidecar wrapped in Arc so we can clone it out and
    /// release the init mutex before sending commands (avoids holding the
    /// mutex for the entire 120s command timeout).
    sidecar: Arc<Mutex<Option<Arc<BrowserSidecar>>>>,
}

impl WebScrapeTool {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self {
            client,
            sidecar: Arc::new(Mutex::new(None)),
        }
    }

    /// Ensure the sidecar is launched and return a cloned Arc to it.
    /// If the sidecar has died (crash, OOM), it is killed and relaunched.
    /// The init mutex is only held during launch, not during command execution.
    async fn get_sidecar(&self) -> Result<Arc<BrowserSidecar>> {
        let mut guard = self.sidecar.lock().await;

        // Check if existing sidecar is dead and needs replacement
        if let Some(ref existing) = *guard {
            if !existing.is_alive() {
                tracing::warn!("Browser sidecar is dead, killing and relaunching...");
                existing.kill().await;
                *guard = None;
            }
        }

        if guard.is_none() {
            let dir = sidecar_dir();
            if !dir.join("bridge.js").exists() {
                return Err(LlmError::ToolCall(format!(
                    "Browser sidecar not found at {}. Ensure sidecar/browser/bridge.js exists.",
                    dir.display()
                )));
            }
            *guard = Some(Arc::new(BrowserSidecar::launch(&dir).await?));
        }
        Ok(guard.as_ref().unwrap().clone())
    }

    async fn execute_extract(
        &self,
        url: &str,
        selectors: &[String],
        output_format: &str,
    ) -> Result<serde_json::Value> {
        extract_page(&self.client, url, selectors, output_format)
            .await
            .map(|p| p.json)
    }

    async fn execute_browser(
        &self,
        url: &str,
        selectors: &[String],
        output_format: &str,
        wait_for: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut params = serde_json::json!({
            "url": url,
            "outputFormat": output_format,
        });

        if !selectors.is_empty() {
            params["selectors"] = serde_json::json!(selectors);
        }
        if let Some(wf) = wait_for {
            params["waitFor"] = serde_json::json!(wf);
        }

        // Try once, and if the sidecar died mid-request, relaunch and retry
        let sidecar = self.get_sidecar().await?;
        match sidecar.send_command("scrape", params.clone()).await {
            Ok(result) => Ok(result),
            Err(e) if e.to_string().contains("sidecar") => {
                tracing::warn!("Browser scrape failed ({e}), retrying with fresh sidecar...");
                let sidecar = self.get_sidecar().await?;
                sidecar.send_command("scrape", params).await
            }
            Err(e) => Err(e),
        }
    }

    async fn execute_crawl(
        &self,
        url: &str,
        max_pages: usize,
        max_depth: usize,
        url_pattern: Option<&str>,
        output_format: &str,
        use_browser: bool,
    ) -> Result<serde_json::Value> {
        let max_pages = max_pages.min(MAX_CRAWL_PAGES);
        let max_depth = max_depth.min(MAX_CRAWL_DEPTH);

        if use_browser {
            // Delegate to sidecar (mutex released after getting Arc clone)
            let mut params = serde_json::json!({
                "url": url,
                "maxPages": max_pages,
                "maxDepth": max_depth,
                "outputFormat": output_format,
            });
            if let Some(pat) = url_pattern {
                params["urlPattern"] = serde_json::json!(pat);
            }

            // Try once, retry on sidecar death
            let sidecar = self.get_sidecar().await?;
            return match sidecar.send_command("crawl", params.clone()).await {
                Ok(result) => Ok(result),
                Err(e) if e.to_string().contains("sidecar") => {
                    tracing::warn!("Browser crawl failed ({e}), retrying with fresh sidecar...");
                    let sidecar = self.get_sidecar().await?;
                    sidecar.send_command("crawl", params).await
                }
                Err(e) => Err(e),
            };
        }

        // Extract-mode crawl in Rust (BFS)
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();
        let mut total_content_size: usize = 0;

        // Pre-compile URL pattern regex once (avoids recompiling per URL)
        let pattern_regex = url_pattern.and_then(compile_url_glob);

        queue.push_back((url.to_string(), 0usize));

        while let Some((current_url, depth)) = queue.pop_front() {
            if results.len() >= max_pages {
                break;
            }

            // Normalize
            let normalized = current_url.split('#').next().unwrap_or(&current_url).to_string();
            if visited.contains(&normalized) {
                continue;
            }
            visited.insert(normalized.clone());

            // Check URL pattern (using pre-compiled regex)
            if let Some(ref re) = pattern_regex {
                if !re.is_match(&normalized) {
                    continue;
                }
            }

            // Fetch page (extract_page returns both content and links)
            match extract_page(&self.client, &normalized, &[], output_format).await {
                Ok(extracted) => {
                    let content_len = extracted.json["content"]
                        .as_str()
                        .map(|s| s.len())
                        .unwrap_or(0);
                    total_content_size += content_len;

                    results.push(serde_json::json!({
                        "url": normalized,
                        "title": extracted.json["title"],
                        "content": extracted.json["content"],
                        "excerpt": extracted.json["excerpt"],
                        "depth": depth,
                    }));

                    // Stop if total content exceeds limit
                    if total_content_size >= MAX_CRAWL_TOTAL {
                        break;
                    }

                    // Enqueue discovered links (no re-fetch needed)
                    if depth < max_depth {
                        for link in &extracted.links {
                            let clean = link.split('#').next().unwrap_or(link).to_string();
                            if !visited.contains(&clean) {
                                queue.push_back((clean, depth + 1));
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Crawl: failed to fetch {}: {e}", normalized);
                }
            }

            // Politeness delay
            if !queue.is_empty() && results.len() < max_pages {
                tokio::time::sleep(std::time::Duration::from_millis(EXTRACT_DELAY_MS)).await;
            }
        }

        let total = results.len();
        Ok(serde_json::json!({
            "pages": results,
            "total": total,
            "urls_visited": visited.len(),
            "mode_used": "extract",
            "truncated": total_content_size >= MAX_CRAWL_TOTAL,
        }))
    }
}

#[async_trait]
impl NativeTool for WebScrapeTool {
    fn name(&self) -> &str {
        "web_scrape"
    }

    fn description(&self) -> &str {
        "Scrape web content from URLs. Three modes:\n\
         - 'extract': Fast HTML parsing without a browser (handles most static/SSR sites)\n\
         - 'browser': Headless browser with anti-bot stealth (for JS-heavy or protected sites)\n\
         - 'crawl': Multi-page site crawling with link following and depth limits\n\
         Returns clean text/markdown content ready for analysis."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to scrape"
                },
                "mode": {
                    "type": "string",
                    "enum": ["extract", "browser", "crawl"],
                    "description": "Scraping mode: 'extract' (fast, no JS), 'browser' (JS rendering + anti-bot), 'crawl' (multi-page). Default: 'extract'."
                },
                "selectors": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "CSS selectors to extract specific elements (optional, extracts full content if omitted)"
                },
                "max_pages": {
                    "type": "integer",
                    "description": "Maximum pages to crawl (crawl mode only, default: 10, max: 100)"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum link-following depth (crawl mode only, default: 2, max: 5)"
                },
                "url_pattern": {
                    "type": "string",
                    "description": "Glob pattern to filter crawled URLs (e.g. 'https://docs.example.com/*')"
                },
                "wait_for": {
                    "type": "string",
                    "description": "CSS selector to wait for before extracting (browser mode only)"
                },
                "output_format": {
                    "type": "string",
                    "enum": ["markdown", "text", "html"],
                    "description": "Output format (default: 'text' for extract, 'markdown' for browser)"
                },
                "use_browser": {
                    "type": "boolean",
                    "description": "Force browser mode for crawling (default: false, uses extract mode)"
                }
            },
            "required": ["url"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::BrowserScrape(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'url' parameter is required".to_string()))?;

        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(LlmError::ToolCall(format!(
                "Invalid URL: must start with http:// or https://"
            )));
        }

        let mode = params["mode"].as_str().unwrap_or("extract");
        let output_format = params["output_format"].as_str().unwrap_or(
            if mode == "browser" { "markdown" } else { "text" },
        );
        let selectors: Vec<String> = params["selectors"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let wait_for = params["wait_for"].as_str();

        match mode {
            "extract" => self.execute_extract(url, &selectors, output_format).await,
            "browser" => {
                self.execute_browser(url, &selectors, output_format, wait_for)
                    .await
            }
            "crawl" => {
                let max_pages = params["max_pages"].as_u64().unwrap_or(10) as usize;
                let max_depth = params["max_depth"].as_u64().unwrap_or(2) as usize;
                let url_pattern = params["url_pattern"].as_str();
                let use_browser = params["use_browser"].as_bool().unwrap_or(false);

                self.execute_crawl(
                    url,
                    max_pages,
                    max_depth,
                    url_pattern,
                    output_format,
                    use_browser,
                )
                .await
            }
            _ => Err(LlmError::ToolCall(format!(
                "Unknown mode '{}'. Must be 'extract', 'browser', or 'crawl'.",
                mode
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_scrape_schema() {
        let tool = WebScrapeTool::new();
        assert_eq!(tool.name(), "web_scrape");
        assert_eq!(tool.required_capability().capability_key(), "browser.scrape");
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("url")));
    }

    #[test]
    fn test_url_glob_match() {
        assert!(url_glob_match("https://example.com/*", "https://example.com/page"));
        assert!(url_glob_match("https://example.com/*", "https://example.com/a/b/c"));
        assert!(!url_glob_match("https://example.com/*", "https://other.com/page"));
        assert!(url_glob_match("https://*.docs.rs/*", "https://tokio.docs.rs/tokio"));
        assert!(!url_glob_match("https://example.com/docs/*", "https://example.com/api/v1"));
    }

    #[test]
    fn test_clean_whitespace() {
        assert_eq!(clean_whitespace("  hello   world  "), "hello world");
        assert_eq!(clean_whitespace("a\n\n\nb"), "a b");
        assert_eq!(clean_whitespace(""), "");
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>Test Page</title></head><body></body></html>";
        let doc = scraper::Html::parse_document(html);
        assert_eq!(extract_title(&doc), "Test Page");
    }

    #[test]
    fn test_extract_title_missing() {
        let html = "<html><body>No title</body></html>";
        let doc = scraper::Html::parse_document(html);
        assert_eq!(extract_title(&doc), "");
    }

    #[test]
    fn test_extract_body_text() {
        let html = r#"<html><body>
            <nav>Navigation</nav>
            <main><h1>Hello</h1><p>World content here.</p></main>
            <footer>Footer stuff</footer>
        </body></html>"#;
        let doc = scraper::Html::parse_document(html);
        let text = extract_body_text(&doc, "text");
        assert!(text.contains("Hello"));
        assert!(text.contains("World content here"));
        // Should prefer main content, skipping nav/footer
    }

    #[test]
    fn test_extract_body_text_no_main() {
        let html = r#"<html><body>
            <script>var x = 1;</script>
            <style>.red { color: red; }</style>
            <div><p>Actual content here.</p></div>
        </body></html>"#;
        let doc = scraper::Html::parse_document(html);
        let text = extract_body_text(&doc, "text");
        assert!(text.contains("Actual content here"));
        assert!(!text.contains("var x = 1"));
        assert!(!text.contains(".red"));
    }

    #[test]
    fn test_extract_links_from_html() {
        let html = r##"<html><body>
            <a href="/page1">Page 1</a>
            <a href="https://example.com/page2">Page 2</a>
            <a href="#fragment">Fragment</a>
            <a href="mailto:test@example.com">Email</a>
        </body></html>"##;
        let doc = scraper::Html::parse_document(html);
        let links = extract_links_from_html(&doc, "https://example.com");
        assert!(links.contains(&"https://example.com/page1".to_string()));
        assert!(links.contains(&"https://example.com/page2".to_string()));
        // mailto and fragment-only should be excluded
        assert!(!links.iter().any(|l| l.contains("mailto")));
    }

    #[tokio::test]
    async fn test_extract_mode_invalid_url() {
        let tool = WebScrapeTool::new();
        let result = tool
            .execute(serde_json::json!({
                "url": "not-a-url",
                "mode": "extract",
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid URL"));
    }

    #[tokio::test]
    async fn test_unknown_mode() {
        let tool = WebScrapeTool::new();
        let result = tool
            .execute(serde_json::json!({
                "url": "https://example.com",
                "mode": "magic",
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown mode"));
    }

    #[test]
    fn test_sidecar_dir_resolution() {
        // Should not panic -- returns a PathBuf regardless
        let dir = sidecar_dir();
        assert!(!dir.as_os_str().is_empty());
    }
}
