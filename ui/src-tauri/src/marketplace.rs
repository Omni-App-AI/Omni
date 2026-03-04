use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Cache TTL: 30 minutes.
const CACHE_TTL: Duration = Duration::from_secs(30 * 60);

struct CachedEntry<T> {
    data: T,
    cached_at: Instant,
}

impl<T> CachedEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            cached_at: Instant::now(),
        }
    }

    fn is_fresh(&self) -> bool {
        self.cached_at.elapsed() < CACHE_TTL
    }
}

struct MarketplaceCache {
    search: HashMap<String, CachedEntry<MarketplaceListResponse>>,
    details: HashMap<String, CachedEntry<MarketplaceDetailResponse>>,
    categories: Option<CachedEntry<Vec<MarketplaceCategory>>>,
}

impl MarketplaceCache {
    fn new() -> Self {
        Self {
            search: HashMap::new(),
            details: HashMap::new(),
            categories: None,
        }
    }
}

/// HTTP client for the Omni marketplace API.
#[derive(Clone)]
pub struct MarketplaceClient {
    client: reqwest::Client,
    base_url: String,
    cache: Arc<RwLock<MarketplaceCache>>,
}

// ── API Response Types ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfo {
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub verified_publisher: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceExtension {
    pub id: String,
    pub name: String,
    pub short_description: Option<String>,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub trust_level: String,
    pub total_downloads: i64,
    pub average_rating: f64,
    pub review_count: i64,
    pub latest_version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub publisher: Option<PublisherInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceListResponse {
    pub extensions: Vec<MarketplaceExtension>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub changelog: Option<String>,
    pub permissions: Option<serde_json::Value>,
    pub tools: Option<serde_json::Value>,
    pub manifest: Option<serde_json::Value>,
    pub min_omni_version: Option<String>,
    pub wasm_size_bytes: Option<i64>,
    pub checksum: Option<String>,
    pub scan_status: Option<String>,
    pub scan_score: Option<f64>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceDetailResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub trust_level: String,
    pub total_downloads: i64,
    pub average_rating: f64,
    pub review_count: i64,
    pub latest_version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub publisher: Option<PublisherInfo>,
    pub latest: Option<VersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceCategory {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoriesResponse {
    pub categories: Vec<MarketplaceCategory>,
}

// ── Client Implementation ───────────────────────────────────────

impl MarketplaceClient {
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Omni-Desktop/0.1.0")
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("Failed to build marketplace HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            cache: Arc::new(RwLock::new(MarketplaceCache::new())),
        }
    }

    /// Search/list marketplace extensions with optional filters.
    /// Results are cached for 30 minutes. Pass `force_refresh` to bypass cache.
    pub async fn search(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        sort: Option<&str>,
        trust: Option<&str>,
        page: Option<i64>,
        limit: Option<i64>,
        force_refresh: bool,
    ) -> Result<MarketplaceListResponse, String> {
        tracing::info!(
            "marketplace::search called -- query={:?} category={:?} sort={:?} trust={:?} page={:?} limit={:?} force_refresh={}",
            query, category, sort, trust, page, limit, force_refresh
        );

        let cache_key = format!(
            "{}|{}|{}|{}|{}|{}",
            query.unwrap_or(""),
            category.unwrap_or(""),
            sort.unwrap_or(""),
            trust.unwrap_or(""),
            page.unwrap_or(1),
            limit.unwrap_or(24),
        );

        // Check cache (unless forced refresh)
        if !force_refresh {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.search.get(&cache_key) {
                if entry.is_fresh() {
                    tracing::info!("marketplace::search -- returning cached result");
                    return Ok(entry.data.clone());
                }
            }
        }

        let mut url = format!("{}/extensions", self.base_url);
        let mut params = Vec::new();

        if let Some(q) = query {
            if !q.is_empty() {
                params.push(format!(
                    "q={}",
                    urlencoding::encode(q)
                ));
            }
        }
        if let Some(c) = category {
            params.push(format!("category={}", urlencoding::encode(c)));
        }
        if let Some(s) = sort {
            params.push(format!("sort={}", urlencoding::encode(s)));
        }
        if let Some(t) = trust {
            params.push(format!("trust={}", urlencoding::encode(t)));
        }
        if let Some(p) = page {
            params.push(format!("page={}", p));
        }
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        tracing::info!("marketplace::search -- GET {}", url);

        let response = match self.client.get(&url).send().await {
            Ok(resp) => {
                tracing::info!("marketplace::search -- HTTP {} {}", resp.status(), url);
                resp
            }
            Err(e) => {
                tracing::error!("marketplace::search -- send failed: {}", e);
                return Err(format!("Marketplace request failed: {}", e));
            }
        };

        let response = match response.error_for_status() {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("marketplace::search -- HTTP error: {}", e);
                return Err(format!("Marketplace API error: {}", e));
            }
        };

        let result = match response.json::<MarketplaceListResponse>().await {
            Ok(data) => {
                tracing::info!(
                    "marketplace::search -- parsed OK, {} extensions, total={}",
                    data.extensions.len(),
                    data.total
                );
                data
            }
            Err(e) => {
                tracing::error!("marketplace::search -- JSON parse failed: {}", e);
                return Err(format!("Failed to parse marketplace response: {}", e));
            }
        };

        // Store in cache
        let mut cache = self.cache.write().await;
        cache.search.insert(cache_key, CachedEntry::new(result.clone()));

        Ok(result)
    }

    /// Get full extension detail including latest version info.
    /// Results are cached for 30 minutes. Pass `force_refresh` to bypass cache.
    pub async fn get_detail(
        &self,
        extension_id: &str,
        force_refresh: bool,
    ) -> Result<MarketplaceDetailResponse, String> {
        tracing::info!("marketplace::get_detail called -- id={} force_refresh={}", extension_id, force_refresh);

        // Check cache
        if !force_refresh {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.details.get(extension_id) {
                if entry.is_fresh() {
                    tracing::info!("marketplace::get_detail -- returning cached result for {}", extension_id);
                    return Ok(entry.data.clone());
                }
            }
        }

        let url = format!(
            "{}/extensions/{}",
            self.base_url,
            urlencoding::encode(extension_id)
        );

        tracing::info!("marketplace::get_detail -- GET {}", url);

        let result = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("marketplace::get_detail -- send failed for {}: {}", extension_id, e);
                format!("Marketplace request failed: {}", e)
            })?
            .error_for_status()
            .map_err(|e| {
                tracing::error!("marketplace::get_detail -- HTTP error for {}: {}", extension_id, e);
                format!("Marketplace API error: {}", e)
            })?
            .json::<MarketplaceDetailResponse>()
            .await
            .map_err(|e| {
                tracing::error!("marketplace::get_detail -- JSON parse failed for {}: {}", extension_id, e);
                format!("Failed to parse extension detail: {}", e)
            })?;

        tracing::info!("marketplace::get_detail -- OK for {}", extension_id);

        // Store in cache
        let mut cache = self.cache.write().await;
        cache.details.insert(extension_id.to_string(), CachedEntry::new(result.clone()));

        Ok(result)
    }

    /// Get all marketplace categories with extension counts.
    /// Results are cached for 30 minutes. Pass `force_refresh` to bypass cache.
    pub async fn get_categories(&self, force_refresh: bool) -> Result<Vec<MarketplaceCategory>, String> {
        // Check cache
        if !force_refresh {
            let cache = self.cache.read().await;
            if let Some(ref entry) = cache.categories {
                if entry.is_fresh() {
                    return Ok(entry.data.clone());
                }
            }
        }

        let url = format!("{}/categories", self.base_url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Marketplace request failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Marketplace API error: {}", e))?
            .json::<CategoriesResponse>()
            .await
            .map_err(|e| format!("Failed to parse categories: {}", e))?;

        // Store in cache
        let mut cache = self.cache.write().await;
        cache.categories = Some(CachedEntry::new(resp.categories.clone()));

        Ok(resp.categories)
    }

    /// Download a WASM extension to a temporary directory with a reconstructed manifest.
    ///
    /// Returns the path to the temp directory (suitable for `install_from_path`).
    pub async fn download_extension(
        &self,
        extension_id: &str,
        detail: &MarketplaceDetailResponse,
    ) -> Result<std::path::PathBuf, String> {
        // Pin the download to the exact version from the detail response so the
        // WASM binary always matches the manifest we reconstruct.  Without this
        // the server independently resolves "latest" from the DB, which can
        // differ if the update_latest_version trigger fires between get_detail
        // and this download call.
        let target_version = detail
            .latest
            .as_ref()
            .map(|v| v.version.clone())
            .or_else(|| detail.latest_version.clone());

        tracing::info!(
            "[DOWNLOAD] extension_id={}, target_version={:?}, detail.latest_version={:?}, detail.latest.version={:?}",
            extension_id,
            target_version,
            detail.latest_version,
            detail.latest.as_ref().map(|v| &v.version),
        );

        let mut url = format!(
            "{}/extensions/{}/download",
            self.base_url,
            urlencoding::encode(extension_id)
        );

        if let Some(ref ver) = target_version {
            url.push_str(&format!("?version={}", urlencoding::encode(ver)));
        }

        tracing::info!("[DOWNLOAD] GET {}", url);

        // Download WASM binary (reqwest follows 308 redirect automatically)
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;

        let final_url = response.url().clone();
        let status = response.status();
        tracing::info!(
            "[DOWNLOAD] response status={}, final_url={} (after redirects)",
            status,
            final_url
        );

        let response = response
            .error_for_status()
            .map_err(|e| format!("Download API error: {}", e))?;

        let wasm_bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to download WASM: {}", e))?;

        tracing::info!("[DOWNLOAD] received {} bytes", wasm_bytes.len());

        // Compute a quick hash of the WASM to detect if content actually changed
        let wasm_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            wasm_bytes.as_ref().hash(&mut hasher);
            hasher.finish()
        };
        tracing::info!("[DOWNLOAD] WASM content hash={:#018x}, size={}", wasm_hash, wasm_bytes.len());

        if wasm_bytes.len() < 4 || &wasm_bytes[..4] != b"\0asm" {
            tracing::error!("[DOWNLOAD] NOT a valid WASM module! first 4 bytes: {:?}", &wasm_bytes[..4.min(wasm_bytes.len())]);
            return Err("Downloaded file is not a valid WASM module".to_string());
        }

        // Create temp directory
        let temp_dir = std::env::temp_dir()
            .join("omni-marketplace-downloads")
            .join(extension_id);

        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)
                .map_err(|e| format!("Failed to clean temp dir: {}", e))?;
        }
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        // Write WASM file
        let wasm_filename = "extension.wasm";
        let wasm_path = temp_dir.join(wasm_filename);
        std::fs::write(&wasm_path, &wasm_bytes)
            .map_err(|e| format!("Failed to write WASM file: {}", e))?;

        // Reconstruct manifest from detail response
        let manifest_toml = self.reconstruct_manifest(detail, wasm_filename)?;
        let manifest_path = temp_dir.join("omni-extension.toml");
        std::fs::write(&manifest_path, &manifest_toml)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        Ok(temp_dir)
    }

    /// Reconstruct a valid omni-extension.toml from marketplace detail data.
    fn reconstruct_manifest(
        &self,
        detail: &MarketplaceDetailResponse,
        wasm_filename: &str,
    ) -> Result<String, String> {
        tracing::info!(
            "[MANIFEST] reconstruct_manifest -- detail.latest present={}, detail.latest_version={:?}",
            detail.latest.is_some(),
            detail.latest_version,
        );

        // If the API returned the full original manifest, use it (with patched entrypoint)
        if let Some(latest) = &detail.latest {
            tracing::info!(
                "[MANIFEST] latest.version={}, latest.manifest present={}, latest.checksum={:?}",
                latest.version,
                latest.manifest.is_some(),
                latest.checksum,
            );
            if let Some(manifest_json) = &latest.manifest {
                tracing::info!(
                    "[MANIFEST] attempting to parse full manifest JSON (first 300 chars): {}",
                    {
                        let s = manifest_json.to_string();
                        if s.len() > 300 { format!("{}...", &s[..300]) } else { s }
                    }
                );
                // Parse the manifest JSON, patch the entrypoint, and convert to TOML
                match serde_json::from_value::<omni_extensions::manifest::ExtensionManifest>(
                    manifest_json.clone(),
                ) {
                    Ok(mut manifest) => {
                        // The manifest JSONB stored in the DB may contain a
                        // stale extension.version (e.g. publisher bumped the
                        // version column but the embedded manifest blob still
                        // had the old value).  The authoritative version is
                        // latest.version from the extension_versions row, so
                        // always override here.
                        let authoritative_version = &latest.version;
                        if manifest.extension.version != *authoritative_version {
                            tracing::warn!(
                                "[MANIFEST] version mismatch! manifest.extension.version={} but latest.version={} -- overriding with authoritative version",
                                manifest.extension.version,
                                authoritative_version,
                            );
                            manifest.extension.version = authoritative_version.clone();
                        }
                        tracing::info!(
                            "[MANIFEST] full manifest parsed OK -- id={}, version={}, entrypoint={}",
                            manifest.extension.id,
                            manifest.extension.version,
                            manifest.runtime.entrypoint,
                        );
                        manifest.runtime.entrypoint = wasm_filename.to_string();
                        return toml::to_string_pretty(&manifest)
                            .map_err(|e| format!("Failed to serialize manifest to TOML: {}", e));
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[MANIFEST] full manifest parse FAILED: {} -- falling back to minimal manifest",
                            e
                        );
                    }
                }
            } else {
                tracing::info!("[MANIFEST] latest.manifest is None -- falling back to minimal manifest");
            }
        } else {
            tracing::info!("[MANIFEST] detail.latest is None -- falling back to minimal manifest");
        }

        // Fallback: build a minimal manifest from the detail fields
        let version = detail
            .latest
            .as_ref()
            .map(|v| v.version.clone())
            .unwrap_or_else(|| {
                detail
                    .latest_version
                    .clone()
                    .unwrap_or_else(|| "0.1.0".to_string())
            });
        tracing::info!("[MANIFEST] fallback -- using version={}", version);

        let publisher_name = detail
            .publisher
            .as_ref()
            .and_then(|p| p.display_name.clone())
            .or_else(|| detail.publisher.as_ref().map(|p| p.username.clone()))
            .unwrap_or_else(|| "Unknown".to_string());

        let description = detail
            .description
            .clone()
            .or_else(|| detail.short_description.clone())
            .unwrap_or_default();

        let mut toml_str = format!(
            r#"[extension]
id = {id}
name = {name}
version = {version}
author = {author}
description = {description}
"#,
            id = toml_quote(&detail.id),
            name = toml_quote(&detail.name),
            version = toml_quote(&version),
            author = toml_quote(&publisher_name),
            description = toml_quote(&description),
        );

        if let Some(ref license) = detail.license {
            toml_str.push_str(&format!("license = {}\n", toml_quote(license)));
        }
        if let Some(ref homepage) = detail.homepage {
            toml_str.push_str(&format!("homepage = {}\n", toml_quote(homepage)));
        }
        if let Some(ref repository) = detail.repository {
            toml_str.push_str(&format!("repository = {}\n", toml_quote(repository)));
        }
        if !detail.categories.is_empty() {
            let cats: Vec<String> = detail.categories.iter().map(|c| toml_quote(c)).collect();
            toml_str.push_str(&format!("categories = [{}]\n", cats.join(", ")));
        }

        toml_str.push_str(&format!(
            r#"
[runtime]
type = "wasm"
entrypoint = {entrypoint}
"#,
            entrypoint = toml_quote(wasm_filename),
        ));

        // Add permissions from latest version info
        if let Some(latest) = &detail.latest {
            if let Some(perms_json) = &latest.permissions {
                if let Ok(perms) =
                    serde_json::from_value::<Vec<serde_json::Value>>(perms_json.clone())
                {
                    for perm in &perms {
                        let cap = perm
                            .get("capability")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let reason = perm
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Required by extension");
                        toml_str.push_str(&format!(
                            "\n[[permissions]]\ncapability = {}\nreason = {}\n",
                            toml_quote(cap),
                            toml_quote(reason),
                        ));
                        if let Some(scope) = perm.get("scope") {
                            if !scope.is_null() {
                                if let Ok(scope_toml) = toml::to_string(scope) {
                                    toml_str
                                        .push_str(&format!("scope = {}\n", scope_toml.trim()));
                                }
                            }
                        }
                    }
                }
            }

            // Add tools from latest version info.
            // Build each tool as a JSON object and let the toml crate serialize
            // the entire [[tools]] array-of-tables entry so that nested
            // `parameters` schemas produce correct TOML (sub-tables, not strings).
            if let Some(tools_json) = &latest.tools {
                if let Ok(tools) =
                    serde_json::from_value::<Vec<serde_json::Value>>(tools_json.clone())
                {
                    // Build a wrapper: { tools = [ {name, description, parameters}, ... ] }
                    let tool_entries: Vec<serde_json::Value> = tools
                        .iter()
                        .map(|tool| {
                            let mut entry = serde_json::Map::new();
                            entry.insert(
                                "name".to_string(),
                                tool.get("name")
                                    .cloned()
                                    .unwrap_or(serde_json::Value::String("unknown".to_string())),
                            );
                            entry.insert(
                                "description".to_string(),
                                tool.get("description")
                                    .cloned()
                                    .unwrap_or(serde_json::Value::String(String::new())),
                            );
                            if let Some(params) = tool.get("parameters") {
                                entry.insert("parameters".to_string(), params.clone());
                            }
                            serde_json::Value::Object(entry)
                        })
                        .collect();

                    let wrapper = serde_json::json!({ "tools": tool_entries });
                    if let Ok(toml_val) =
                        serde_json::from_value::<toml::Value>(wrapper)
                    {
                        if let Ok(fragment) = toml::to_string(&toml_val) {
                            toml_str.push_str(&format!("\n{}", fragment));
                        }
                    }
                }
            }
        }

        Ok(toml_str)
    }
}

/// Quote a string for TOML output.
fn toml_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

// ── URL Encoding ────────────────────────────────────────────────

mod urlencoding {
    /// Percent-encode a string for use in URL query parameters.
    pub fn encode(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'~' => result.push(byte as char),
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", byte));
                }
            }
        }
        result
    }
}
