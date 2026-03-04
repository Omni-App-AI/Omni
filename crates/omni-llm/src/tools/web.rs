//! Web fetch tool for retrieving URL content.
//!
//! Gated by `network.http` permission.

use std::sync::OnceLock;

use async_trait::async_trait;
use omni_permissions::capability::Capability;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Native tool for fetching web content.
pub struct WebFetchTool {
    client: reqwest::Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

#[async_trait]
impl NativeTool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch the content of a URL. Returns the HTTP status code and response body as text. \
         Useful for reading web pages, APIs, documentation, etc."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method (GET, POST, PUT, DELETE). Defaults to GET.",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"]
                },
                "body": {
                    "type": "string",
                    "description": "Request body (for POST/PUT/PATCH)"
                },
                "headers": {
                    "type": "object",
                    "description": "Additional HTTP headers as key-value pairs"
                }
            },
            "required": ["url"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::NetworkHttp(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'url' parameter is required".to_string()))?;

        let method = params["method"].as_str().unwrap_or("GET");
        let body = params["body"].as_str().unwrap_or("");

        let mut request = match method.to_uppercase().as_str() {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            "PATCH" => self.client.patch(url),
            "HEAD" => self.client.head(url),
            _ => return Err(LlmError::ToolCall(format!("Unsupported method: {method}"))),
        };

        // Add custom headers
        if let Some(headers) = params["headers"].as_object() {
            for (key, value) in headers {
                if let Some(val) = value.as_str() {
                    request = request.header(key.as_str(), val);
                }
            }
        }

        // Add body for methods that support it
        if !body.is_empty() {
            request = request.body(body.to_string());
        }

        let mut response = request
            .send()
            .await
            .map_err(|e| LlmError::ToolCall(format!("HTTP request failed: {e}")))?;

        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        // Download in chunks, stopping at max_download to prevent OOM on huge responses
        let max_download = 2 * 1024 * 1024; // 2MB max download
        let mut body_bytes = Vec::new();
        let mut truncated_download = false;

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read response body: {e}")))?
        {
            body_bytes.extend_from_slice(&chunk);
            if body_bytes.len() >= max_download {
                truncated_download = true;
                break;
            }
        }

        let body_text = String::from_utf8_lossy(&body_bytes);

        // If content is HTML, strip the tags to save massive amounts of tokens
        let mut body_result = if content_type.to_lowercase().contains("html") {
            strip_html_tags(&body_text)
        } else {
            body_text.to_string()
        };

        // Truncate the final text string to prevent token limits
        // 100KB of text is roughly 25k-30k tokens
        let max_text_display = 100 * 1024;
        let final_truncated = body_result.len() > max_text_display;
        if final_truncated {
            body_result.truncate(max_text_display);
        }

        if final_truncated || truncated_download {
            body_result.push_str(&format!(
                "\n\n[Response truncated{}]",
                if final_truncated {
                    " to 100KB of text"
                } else {
                    " due to download limit"
                }
            ));
        }

        Ok(serde_json::json!({
            "status": status,
            "content_type": content_type,
            "body": body_result,
            "size_bytes": body_bytes.len(),
        }))
    }
}

/// Native tool for searching the web.
/// Tries providers in order: Brave → Tavily → Serper → SearXNG → DuckDuckGo fallback.
/// Each provider is tried only if its env var / API key is configured.
pub struct WebSearchTool {
    client: reqwest::Client,
}

impl WebSearchTool {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    async fn brave_search(
        &self,
        api_key: &str,
        query: &str,
        count: u64,
        country: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            urlencoding::encode(query),
            count
        );
        if let Some(cc) = country {
            url.push_str(&format!("&country={}", cc));
        }

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", api_key)
            .send()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Brave search failed: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ToolCall(format!(
                "Brave search returned status {}: {}",
                status, body
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to parse Brave response: {e}")))?;

        let results: Vec<serde_json::Value> = data["web"]["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .take(count as usize)
            .map(|r| {
                serde_json::json!({
                    "title": r["title"].as_str().unwrap_or(""),
                    "url": r["url"].as_str().unwrap_or(""),
                    "snippet": r["description"].as_str().unwrap_or(""),
                })
            })
            .collect();

        let total = results.len();
        Ok(serde_json::json!({
            "results": results,
            "total": total,
            "provider": "brave",
        }))
    }

    /// Tavily Search API -- AI-optimized search results.
    /// POST https://api.tavily.com/search  (Bearer token auth)
    async fn tavily_search(
        &self,
        api_key: &str,
        query: &str,
        count: u64,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "query": query,
            "max_results": count,
            "search_depth": "basic",
        });

        let resp = self
            .client
            .post("https://api.tavily.com/search")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Tavily search failed: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ToolCall(format!(
                "Tavily search returned status {}: {}",
                status, body
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to parse Tavily response: {e}")))?;

        let results: Vec<serde_json::Value> = data["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .take(count as usize)
            .map(|r| {
                serde_json::json!({
                    "title": r["title"].as_str().unwrap_or(""),
                    "url": r["url"].as_str().unwrap_or(""),
                    "snippet": r["content"].as_str().unwrap_or(""),
                })
            })
            .collect();

        let total = results.len();
        Ok(serde_json::json!({
            "results": results,
            "total": total,
            "provider": "tavily",
        }))
    }

    /// Serper.dev -- Google Search results via API.
    /// POST https://google.serper.dev/search  (X-API-KEY header)
    async fn serper_search(
        &self,
        api_key: &str,
        query: &str,
        count: u64,
        country: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({
            "q": query,
            "num": count,
        });
        if let Some(cc) = country {
            body["gl"] = serde_json::Value::String(cc.to_lowercase());
        }

        let resp = self
            .client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Serper search failed: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ToolCall(format!(
                "Serper search returned status {}: {}",
                status, body
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to parse Serper response: {e}")))?;

        let results: Vec<serde_json::Value> = data["organic"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .take(count as usize)
            .map(|r| {
                serde_json::json!({
                    "title": r["title"].as_str().unwrap_or(""),
                    "url": r["link"].as_str().unwrap_or(""),
                    "snippet": r["snippet"].as_str().unwrap_or(""),
                })
            })
            .collect();

        let total = results.len();
        Ok(serde_json::json!({
            "results": results,
            "total": total,
            "provider": "serper",
        }))
    }

    /// SearXNG -- self-hosted/public meta-search engine, no API key required.
    /// GET {base_url}/search?q=...&format=json
    async fn searxng_search(
        &self,
        base_url: &str,
        query: &str,
        count: u64,
    ) -> Result<serde_json::Value> {
        let url = format!(
            "{}/search?q={}&format=json&categories=general",
            base_url.trim_end_matches('/'),
            urlencoding::encode(query),
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| LlmError::ToolCall(format!("SearXNG search failed: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ToolCall(format!(
                "SearXNG returned status {}: {}",
                status, body
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to parse SearXNG response: {e}")))?;

        let results: Vec<serde_json::Value> = data["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .take(count as usize)
            .map(|r| {
                serde_json::json!({
                    "title": r["title"].as_str().unwrap_or(""),
                    "url": r["url"].as_str().unwrap_or(""),
                    "snippet": r["content"].as_str().unwrap_or(""),
                })
            })
            .collect();

        let total = results.len();
        Ok(serde_json::json!({
            "results": results,
            "total": total,
            "provider": "searxng",
        }))
    }

    async fn duckduckgo_fallback(&self, query: &str, count: u64) -> Result<serde_json::Value> {
        // Use DuckDuckGo's HTML search endpoint with POST (more reliable than GET/Lite).
        let url = "https://html.duckduckgo.com/html/";
        let form_body = format!("q={}&b=", urlencoding::encode(query));

        // Attempt the request -- retry once on HTTP 202 (DDG challenge/redirect)
        let mut html = String::new();
        let mut attempts = 0;
        let max_attempts = 2;

        while attempts < max_attempts {
            attempts += 1;
            let resp = self
                .client
                .post(url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Referer", "https://html.duckduckgo.com/")
                .header(
                    "Accept",
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                )
                .header("Accept-Language", "en-US,en;q=0.9")
                .body(form_body.clone())
                .send()
                .await
                .map_err(|e| LlmError::ToolCall(format!("DuckDuckGo search failed: {e}")))?;

            let status = resp.status().as_u16();

            if status == 202 || status == 403 || status == 429 {
                // DDG returned a challenge or rate-limit; wait briefly and retry
                if attempts < max_attempts {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    continue;
                }
                // All retries exhausted -- return a helpful fallback
                return Ok(serde_json::json!({
                    "results": [{
                        "title": "Search completed",
                        "url": format!("https://duckduckgo.com/?q={}", urlencoding::encode(query)),
                        "snippet": format!(
                            "DuckDuckGo returned HTTP {status} (rate-limited or bot-detected). \
                             Set BRAVE_API_KEY env var for reliable search, or try web_scrape \
                             with a specific URL instead."
                        ),
                    }],
                    "total": 1,
                    "provider": "duckduckgo",
                    "error": format!("HTTP {status}"),
                }));
            }

            if status >= 400 {
                return Err(LlmError::ToolCall(format!(
                    "DuckDuckGo returned HTTP {status}"
                )));
            }

            html = resp
                .text()
                .await
                .map_err(|e| LlmError::ToolCall(format!("Failed to read DDG response: {e}")))?;
            break;
        }

        // Parse results from DDG HTML.
        // DDG HTML endpoint uses:
        //   <a class="result__a" href="...">Title</a>
        //   <a class="result__snippet" ...>Snippet text...</a>
        // Attribute order varies, so match class and href independently.
        let mut results = Vec::new();
        static LINK_RE: OnceLock<regex::Regex> = OnceLock::new();
        static SNIPPET_RE: OnceLock<regex::Regex> = OnceLock::new();
        let link_re = LINK_RE.get_or_init(|| regex::Regex::new(
            r#"<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>([\s\S]*?)</a>|<a[^>]*href="([^"]+)"[^>]*class="result__a"[^>]*>([\s\S]*?)</a>"#
        ).unwrap());
        let snippet_re = SNIPPET_RE.get_or_init(|| regex::Regex::new(
            r#"<a[^>]*class="result__snippet"[^>]*>([\s\S]*?)</a>|<[^>]*class="result__snippet"[^>]*>([\s\S]*?)</[^>]*>"#
        ).unwrap());

        {
            let links: Vec<_> = link_re.captures_iter(&html).collect();
            let snippets: Vec<_> = snippet_re.captures_iter(&html).collect();

            for i in 0..links.len().min(count as usize) {
                // Extract from whichever alternation matched
                let raw_url = links[i]
                    .get(1)
                    .or_else(|| links[i].get(3))
                    .map(|m| m.as_str())
                    .unwrap_or("");
                let raw_title = links[i]
                    .get(2)
                    .or_else(|| links[i].get(4))
                    .map(|m| m.as_str())
                    .unwrap_or("");
                let snippet_raw = snippets
                    .get(i)
                    .and_then(|c| c.get(1).or_else(|| c.get(2)))
                    .map(|m| m.as_str())
                    .unwrap_or("");

                // DDG wraps result URLs in a redirect; extract the actual URL
                let clean_url = extract_ddg_redirect_url(raw_url);
                let title = strip_html_tags(raw_title);
                let snippet = strip_html_tags(snippet_raw);

                if !clean_url.is_empty() {
                    results.push(serde_json::json!({
                        "title": title.trim(),
                        "url": clean_url,
                        "snippet": snippet.trim(),
                    }));
                }
            }
        }

        // If HTML parsing yielded nothing, return a helpful message
        if results.is_empty() {
            results.push(serde_json::json!({
                "title": "Search completed",
                "url": format!("https://duckduckgo.com/?q={}", urlencoding::encode(query)),
                "snippet": "No results could be extracted from DuckDuckGo. Set BRAVE_API_KEY env var for reliable search, or use web_scrape with a specific URL.",
            }));
        }

        let total = results.len();
        Ok(serde_json::json!({
            "results": results,
            "total": total,
            "provider": "duckduckgo",
        }))
    }
}

#[async_trait]
impl NativeTool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns a list of results with titles, URLs, and snippets. \
         Use this to find documentation, solutions, current information, or discover URLs to fetch."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of results to return (1-10, default 5)"
                },
                "country": {
                    "type": "string",
                    "description": "2-letter country code (e.g. 'US', 'DE', 'JP')"
                }
            },
            "required": ["query"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::SearchWeb(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'query' parameter is required".to_string()))?;

        let count = params["count"].as_u64().unwrap_or(5).min(10).max(1);
        let country = params["country"].as_str();

        // Provider cascade: try each in order, use first that's configured.

        // 1. Brave Search (BRAVE_API_KEY)
        if let Ok(api_key) = std::env::var("BRAVE_API_KEY") {
            if !api_key.is_empty() {
                return self.brave_search(&api_key, query, count, country).await;
            }
        }

        // 2. Tavily (TAVILY_API_KEY) -- AI-optimized search
        if let Ok(api_key) = std::env::var("TAVILY_API_KEY") {
            if !api_key.is_empty() {
                return self.tavily_search(&api_key, query, count).await;
            }
        }

        // 3. Serper (SERPER_API_KEY) -- Google Search results
        if let Ok(api_key) = std::env::var("SERPER_API_KEY") {
            if !api_key.is_empty() {
                return self.serper_search(&api_key, query, count, country).await;
            }
        }

        // 4. SearXNG (SEARXNG_URL) -- self-hosted, no API key needed
        if let Ok(base_url) = std::env::var("SEARXNG_URL") {
            if !base_url.is_empty() {
                return self.searxng_search(&base_url, query, count).await;
            }
        }

        // 5. DuckDuckGo fallback (no config needed, but fragile)
        self.duckduckgo_fallback(query, count).await
    }
}

/// Strip HTML tags from a string, leaving only text content.
fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    // Collapse whitespace runs and decode common HTML entities
    let decoded = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    let mut cleaned = String::with_capacity(decoded.len());
    let mut last_was_space = false;
    for ch in decoded.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                cleaned.push(' ');
                last_was_space = true;
            }
        } else {
            cleaned.push(ch);
            last_was_space = false;
        }
    }
    cleaned.trim().to_string()
}

/// Extract the actual destination URL from a DuckDuckGo redirect wrapper.
/// DDG wraps result URLs like: `//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&rut=...`
/// This extracts the `uddg` parameter value and URL-decodes it.
fn extract_ddg_redirect_url(raw_url: &str) -> String {
    // Check if it's a DDG redirect URL
    if raw_url.contains("duckduckgo.com/l/") || raw_url.contains("duckduckgo.com/y.js") {
        // Parse out the uddg= parameter
        if let Some(uddg_start) = raw_url.find("uddg=") {
            let value_start = uddg_start + 5; // len("uddg=")
            let value_end = raw_url[value_start..]
                .find('&')
                .map(|i| value_start + i)
                .unwrap_or(raw_url.len());
            let encoded = &raw_url[value_start..value_end];
            return urlencoding::decode(encoded);
        }
    }
    // Not a redirect -- return as-is, stripping leading //
    if raw_url.starts_with("//") {
        return format!("https:{raw_url}");
    }
    raw_url.to_string()
}

/// URL-encode/decode helpers (minimal, no extra dependency).
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut result = String::with_capacity(s.len() * 3);
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        result
    }

    pub fn decode(s: &str) -> String {
        let mut result = Vec::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    result.push(byte);
                    i += 3;
                    continue;
                }
            } else if bytes[i] == b'+' {
                result.push(b' ');
                i += 1;
                continue;
            }
            result.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&result).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_fetch_schema() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.name(), "web_fetch");
        assert_eq!(tool.required_capability().capability_key(), "network.http");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["url"].is_object());
    }

    #[test]
    fn test_web_search_schema() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "web_search");
        assert_eq!(tool.required_capability().capability_key(), "search.web");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["query"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("query")));
    }

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoding::encode("hello world"), "hello%20world");
        assert_eq!(urlencoding::encode("a+b=c"), "a%2Bb%3Dc");
        assert_eq!(urlencoding::encode("rust-lang"), "rust-lang");
    }

    #[test]
    fn test_url_decoding() {
        assert_eq!(urlencoding::decode("hello%20world"), "hello world");
        assert_eq!(
            urlencoding::decode("https%3A%2F%2Fexample.com%2Fpath"),
            "https://example.com/path"
        );
        assert_eq!(urlencoding::decode("a+b"), "a b");
        assert_eq!(urlencoding::decode("no-encoding"), "no-encoding");
        assert_eq!(urlencoding::decode("trailing%2"), "trailing%2"); // malformed, pass through
    }

    #[test]
    fn test_extract_ddg_redirect_url() {
        // DDG redirect with uddg parameter
        let redirect = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage&rut=abc123";
        assert_eq!(
            extract_ddg_redirect_url(redirect),
            "https://example.com/page"
        );

        // DDG redirect without rut trailing param
        let redirect2 = "//duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org";
        assert_eq!(extract_ddg_redirect_url(redirect2), "https://rust-lang.org");

        // Direct URL (no redirect)
        assert_eq!(
            extract_ddg_redirect_url("https://example.com"),
            "https://example.com"
        );

        // Protocol-relative URL
        assert_eq!(
            extract_ddg_redirect_url("//example.com/path"),
            "https://example.com/path"
        );
    }

    #[test]
    fn test_strip_html_tags_with_entities() {
        assert_eq!(
            strip_html_tags("<b>Hello</b> &amp; <i>World</i>"),
            "Hello & World"
        );
        assert_eq!(strip_html_tags("A &lt; B &gt; C"), "A < B > C");
        assert_eq!(
            strip_html_tags("it&#39;s &quot;quoted&quot;"),
            "it's \"quoted\""
        );
        assert_eq!(strip_html_tags("  lots   of   spaces  "), "lots of spaces");
        assert_eq!(
            strip_html_tags("<span>nested <b>tags</b> here</span>"),
            "nested tags here"
        );
    }
}
