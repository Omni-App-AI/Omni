//! Shared Webhook Server
//!
//! A single axum-based HTTP server that routes incoming webhooks to the
//! correct channel handler. Multiple channels register their routes under
//! `/{channel_id}/*` paths. This avoids needing one HTTP server per channel.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::any;
use axum::Router;
use tokio::sync::{oneshot, RwLock};

/// Type alias for webhook handler functions.
///
/// A handler receives the HTTP method, sub-path, body bytes, and headers,
/// and returns a (status_code, body) response.
pub type WebhookHandler = Arc<
    dyn Fn(Method, String, Bytes, HashMap<String, String>)
            -> std::pin::Pin<Box<dyn std::future::Future<Output = (u16, String)> + Send>>
        + Send
        + Sync,
>;

/// Shared state for the webhook router.
#[derive(Clone)]
struct AppState {
    handlers: Arc<RwLock<HashMap<String, WebhookHandler>>>,
}

/// Rate limiter state for per-IP request throttling.
#[derive(Clone)]
struct RateLimiterState {
    /// Per-IP hit counts and window start times.
    hits: Arc<RwLock<HashMap<IpAddr, (u32, Instant)>>>,
    /// Maximum requests per window.
    max_requests: u32,
    /// Window duration in seconds.
    window_secs: u64,
}

/// Shared webhook server that routes `/{channel_id}/*` to registered handlers.
pub struct WebhookServer {
    port: u16,
    bind_address: [u8; 4],
    handlers: Arc<RwLock<HashMap<String, WebhookHandler>>>,
    shutdown_tx: tokio::sync::Mutex<Option<oneshot::Sender<()>>>,
}

impl WebhookServer {
    /// Create a new webhook server on the given port, bound to localhost (127.0.0.1).
    ///
    /// This is the secure default -- only the local machine can reach the server.
    /// Use `new_with_bind_address` to override for specific use cases.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            bind_address: [127, 0, 0, 1],
            handlers: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: tokio::sync::Mutex::new(None),
        }
    }

    /// Create a new webhook server with a custom bind address.
    ///
    /// # Security Warning
    /// Binding to `[0, 0, 0, 0]` exposes the server to the entire network.
    /// Only use this if you explicitly need LAN access and understand the risks.
    pub fn new_with_bind_address(port: u16, bind_address: [u8; 4]) -> Self {
        if bind_address == [0, 0, 0, 0] {
            tracing::warn!(
                "Webhook server binding to 0.0.0.0 (all interfaces). \
                 This exposes the agent to the local network. \
                 Consider using 127.0.0.1 for security."
            );
        }
        Self {
            port,
            bind_address,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: tokio::sync::Mutex::new(None),
        }
    }

    /// Register a channel's webhook handler under path prefix `/{channel_id}/`.
    pub async fn register_handler(&self, channel_id: &str, handler: WebhookHandler) {
        self.handlers
            .write()
            .await
            .insert(channel_id.to_string(), handler);
        tracing::info!("Webhook handler registered for /{channel_id}/");
    }

    /// Remove a channel's webhook handler.
    pub async fn unregister_handler(&self, channel_id: &str) {
        self.handlers.write().await.remove(channel_id);
        tracing::info!("Webhook handler unregistered for /{channel_id}/");
    }

    /// Start the HTTP server. Idempotent -- only starts once.
    ///
    /// The server applies two security layers as middleware:
    /// 1. **Origin validation** -- blocks cross-origin browser requests (OpenClaw-style attacks)
    /// 2. **Rate limiting** -- 30 requests/minute per IP to prevent flooding
    pub async fn start(&self) -> crate::Result<()> {
        let mut guard = self.shutdown_tx.lock().await;
        if guard.is_some() {
            return Ok(()); // Already running
        }

        let state = AppState {
            handlers: self.handlers.clone(),
        };

        let rate_limiter = RateLimiterState {
            hits: Arc::new(RwLock::new(HashMap::new())),
            max_requests: 30,
            window_secs: 60,
        };

        let app = Router::new()
            .route("/{channel_id}", any(handle_webhook))
            .route("/{channel_id}/{*rest}", any(handle_webhook))
            .with_state(state)
            .layer(axum::middleware::from_fn(origin_guard))
            .layer(axum::middleware::from_fn_with_state(
                rate_limiter,
                rate_limit_guard,
            ));

        let addr = std::net::SocketAddr::from((self.bind_address, self.port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| crate::ChannelError::Config(format!("Failed to bind webhook server on {addr}: {e}")))?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        *guard = Some(shutdown_tx);

        tracing::info!(
            address = %addr,
            "Webhook server started"
        );

        tokio::spawn(async move {
            let make_service = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
            axum::serve(listener, make_service)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
            tracing::info!("Webhook server stopped");
        });

        Ok(())
    }

    /// Stop the server.
    pub async fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
    }

    /// Get the port the server is configured on.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Origin validation middleware.
///
/// Blocks cross-origin browser requests -- the core defense against OpenClaw-style
/// 0-click attacks where a malicious website sends fetch() to localhost.
///
/// Allows:
/// - Requests with no Origin header (non-browser clients, curl, webhooks from services)
/// - `tauri://localhost` (our own desktop app)
/// - `http://localhost:*` and `http://127.0.0.1:*` (local development)
/// - `https://localhost:*` and `https://127.0.0.1:*`
///
/// Blocks everything else (e.g., `https://evil.com`).
async fn origin_guard(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if let Some(origin) = req.headers().get(header::ORIGIN) {
        let origin_str = origin.to_str().unwrap_or("");
        let allowed = origin_str.starts_with("http://localhost")
            || origin_str.starts_with("https://localhost")
            || origin_str.starts_with("http://127.0.0.1")
            || origin_str.starts_with("https://127.0.0.1")
            || origin_str.starts_with("tauri://");

        if !allowed {
            tracing::warn!(
                origin = origin_str,
                "Blocked cross-origin request to webhook server"
            );
            return (
                StatusCode::FORBIDDEN,
                "Cross-origin request blocked. Only localhost origins are allowed.",
            )
                .into_response();
        }
    }

    next.run(req).await
}

/// Per-IP rate limiting middleware.
///
/// Limits each IP to `max_requests` per `window_secs` window.
/// Returns 429 Too Many Requests when exceeded.
async fn rate_limit_guard(
    axum::extract::State(state): axum::extract::State<RateLimiterState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let ip = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    let now = Instant::now();

    {
        let mut hits = state.hits.write().await;

        // Periodic cleanup: remove entries older than 2x window
        if hits.len() > 1000 {
            let cutoff = std::time::Duration::from_secs(state.window_secs * 2);
            hits.retain(|_, (_, started)| now.duration_since(*started) < cutoff);
        }

        let entry = hits.entry(ip).or_insert((0, now));

        // Reset window if expired
        if now.duration_since(entry.1).as_secs() >= state.window_secs {
            *entry = (0, now);
        }

        entry.0 += 1;

        if entry.0 > state.max_requests {
            tracing::warn!(
                ip = %ip,
                count = entry.0,
                "Rate limit exceeded on webhook server"
            );
            return (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded. Try again later.",
            )
                .into_response();
        }
    }

    next.run(req).await
}

/// Axum handler that dispatches to the correct channel handler.
async fn handle_webhook(
    method: Method,
    Path(params): Path<(String, Option<String>)>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let (channel_id, rest) = params;
    let sub_path = rest.unwrap_or_default();

    let handlers = state.handlers.read().await;
    let handler = match handlers.get(&channel_id) {
        Some(h) => h.clone(),
        None => {
            return (StatusCode::NOT_FOUND, format!("No handler for channel: {channel_id}"));
        }
    };
    drop(handlers);

    // Convert headers to HashMap<String, String>
    let header_map: HashMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str().ok().map(|val| (k.to_string(), val.to_string()))
        })
        .collect();

    let (status, response_body) = handler(method, sub_path, body, header_map).await;
    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status_code, response_body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_server_new() {
        let server = WebhookServer::new(8443);
        assert_eq!(server.port(), 8443);
    }

    #[tokio::test]
    async fn test_register_unregister_handler() {
        let server = WebhookServer::new(0);
        let handler: WebhookHandler = Arc::new(|_method, _path, _body, _headers| {
            Box::pin(async { (200, "ok".to_string()) })
        });
        server.register_handler("test", handler).await;
        assert!(server.handlers.read().await.contains_key("test"));
        server.unregister_handler("test").await;
        assert!(!server.handlers.read().await.contains_key("test"));
    }

    #[tokio::test]
    async fn test_start_stop() {
        // We can't easily test with port 0 since we bind to it,
        // but we can verify the start/stop lifecycle doesn't panic.
        // Using a high port to avoid conflicts.
        let server = WebhookServer::new(19876);
        let result = server.start().await;
        // Port might be in use, so just check it doesn't panic
        if result.is_ok() {
            server.stop().await;
        }
    }

    #[test]
    fn test_default_binds_to_localhost() {
        let server = WebhookServer::new(8900);
        assert_eq!(server.bind_address, [127, 0, 0, 1]);
    }

    #[test]
    fn test_custom_bind_address() {
        let server = WebhookServer::new_with_bind_address(8900, [0, 0, 0, 0]);
        assert_eq!(server.bind_address, [0, 0, 0, 0]);
    }
}
