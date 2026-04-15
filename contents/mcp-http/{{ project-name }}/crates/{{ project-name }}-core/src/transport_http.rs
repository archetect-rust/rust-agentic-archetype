//! HTTP transport for the MCP server.
//!
//! Two server variants:
//! - **External** (`serve_http`): optional OAuth Bearer token validation.
//!   `/health` and `/.well-known/oauth-authorization-server` are always public.
//! - **Internal** (`serve_internal_http`): no authentication, loopback only.
//!
//! OAuth validation uses JWKS fetched from the authorization server at startup
//! and cached in memory. JWKS is re-fetched when an unknown `kid` is
//! encountered (handles key rotation).

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::config::OAuthConfig;
use crate::server::{{ ProjectName }}Server;

// ── JWKS cache ────────────────────────────────────────────────────────────────

/// A single JWK entry from the JWKS endpoint.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // fields are part of the JWK wire format; not all are used in every key type
struct Jwk {
    #[serde(rename = "kty")]
    key_type: String,
    #[serde(default)]
    kid: Option<String>,
    #[serde(default)]
    alg: Option<String>,
    // RSA fields
    #[serde(default)]
    n: Option<String>,
    #[serde(default)]
    e: Option<String>,
    // EC fields
    #[serde(default)]
    crv: Option<String>,
    #[serde(default)]
    x: Option<String>,
    #[serde(default)]
    y: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

struct JwksCache {
    keys: Vec<Jwk>,
    fetched_at: Instant,
}

const JWKS_TTL: Duration = Duration::from_secs(3600); // 1 hour

#[derive(Clone)]
struct OAuthState {
    config: OAuthConfig,
    jwks: Arc<RwLock<Option<JwksCache>>>,
    /// Cached OAuth authorization server metadata (proxied from OIDC discovery).
    oidc_metadata: Arc<RwLock<Option<Value>>>,
    http_client: reqwest::Client,
}

impl OAuthState {
    fn new(config: OAuthConfig, http_client: reqwest::Client) -> Self {
        Self {
            config,
            jwks: Arc::new(RwLock::new(None)),
            oidc_metadata: Arc::new(RwLock::new(None)),
            http_client,
        }
    }

    /// Fetch and cache the JWKS. Re-uses cached value if within TTL.
    async fn get_jwks(&self) -> Result<Vec<Jwk>> {
        {
            let guard = self.jwks.read().await;
            if let Some(cache) = guard.as_ref() {
                if cache.fetched_at.elapsed() < JWKS_TTL {
                    return Ok(cache.keys.clone());
                }
            }
        }

        self.refresh_jwks().await
    }

    async fn refresh_jwks(&self) -> Result<Vec<Jwk>> {
        let uri = self.config.effective_jwks_uri();
        tracing::debug!(uri = %uri, "fetching JWKS");

        let resp = self.http_client.get(&uri).send().await
            .context("failed to fetch JWKS")?;
        let status = resp.status();
        if !status.is_success() {
            bail!("JWKS endpoint returned {status}");
        }
        let jwks: Jwks = resp.json().await.context("failed to parse JWKS")?;

        let mut guard = self.jwks.write().await;
        *guard = Some(JwksCache { keys: jwks.keys.clone(), fetched_at: Instant::now() });
        tracing::info!(count = jwks.keys.len(), "JWKS refreshed");
        Ok(jwks.keys)
    }

    /// Fetch and cache OIDC discovery metadata for the well-known endpoint.
    async fn get_oidc_metadata(&self) -> Result<Value> {
        {
            let guard = self.oidc_metadata.read().await;
            if let Some(meta) = guard.as_ref() {
                return Ok(meta.clone());
            }
        }

        let uri = self.config.oidc_discovery_uri();
        tracing::debug!(uri = %uri, "fetching OIDC discovery metadata");

        let resp = self.http_client.get(&uri).send().await
            .context("failed to fetch OIDC discovery metadata")?;
        let status = resp.status();
        if !status.is_success() {
            bail!("OIDC discovery endpoint returned {status}");
        }
        let meta: Value = resp.json().await.context("failed to parse OIDC discovery metadata")?;

        let mut guard = self.oidc_metadata.write().await;
        *guard = Some(meta.clone());
        Ok(meta)
    }

    /// Validate a Bearer JWT. Returns an error string on failure.
    async fn validate_token(&self, token: &str) -> Result<(), String> {
        let header = decode_header(token).map_err(|e| format!("invalid JWT header: {e}"))?;
        let kid = header.kid.as_deref();

        let mut keys = self.get_jwks().await.map_err(|e| format!("JWKS fetch failed: {e}"))?;

        // If no key matches, refresh once (handles key rotation).
        if find_key(&keys, kid).is_none() {
            keys = self.refresh_jwks().await
                .map_err(|e| format!("JWKS refresh failed: {e}"))?;
        }

        let key = find_key(&keys, kid).ok_or_else(|| {
            format!("no matching key found in JWKS for kid={kid:?}")
        })?;

        let decoding_key = jwk_to_decoding_key(key)?;

        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[&self.config.issuer]);
        if let Some(aud) = &self.config.audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }

        decode::<Value>(token, &decoding_key, &validation)
            .map_err(|e| format!("JWT validation failed: {e}"))?;

        Ok(())
    }
}

fn find_key<'a>(keys: &'a [Jwk], kid: Option<&str>) -> Option<&'a Jwk> {
    match kid {
        Some(kid) => keys.iter().find(|k| k.kid.as_deref() == Some(kid)),
        None => keys.iter().find(|k| k.key_type == "RSA" || k.key_type == "EC"),
    }
}

fn jwk_to_decoding_key(jwk: &Jwk) -> Result<DecodingKey, String> {
    match jwk.key_type.as_str() {
        "RSA" => {
            let n = jwk.n.as_deref().ok_or("RSA JWK missing 'n' field")?;
            let e = jwk.e.as_deref().ok_or("RSA JWK missing 'e' field")?;
            DecodingKey::from_rsa_components(n, e)
                .map_err(|e| format!("failed to build RSA decoding key: {e}"))
        }
        "EC" => {
            let x = jwk.x.as_deref().ok_or("EC JWK missing 'x' field")?;
            let y = jwk.y.as_deref().ok_or("EC JWK missing 'y' field")?;
            DecodingKey::from_ec_components(x, y)
                .map_err(|e| format!("failed to build EC decoding key: {e}"))
        }
        kty => Err(format!("unsupported JWK key type: {kty}")),
    }
}

// ── OAuth axum middleware ─────────────────────────────────────────────────────

/// Axum middleware that validates Bearer tokens on protected paths.
/// `/health` and `/.well-known/` paths bypass validation.
async fn oauth_middleware(
    State(oauth): State<Arc<OAuthState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();

    // Public paths — no auth required
    if path == "/health" || path.starts_with("/.well-known/") {
        return next.run(request).await;
    }

    let token = match extract_bearer_token(&request) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::WWW_AUTHENTICATE, "Bearer realm=\"{{ project-name }}\"")],
                Json(json!({"error": "missing_token", "error_description": "Authorization: Bearer <token> header is required"})),
            ).into_response();
        }
    };

    if let Err(reason) = oauth.validate_token(&token).await {
        tracing::warn!(reason = %reason, "JWT validation failed");
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer realm=\"{{ project-name }}\", error=\"invalid_token\"")],
            Json(json!({"error": "invalid_token", "error_description": reason})),
        ).into_response();
    }

    next.run(request).await
}

fn extract_bearer_token(request: &Request<Body>) -> Option<String> {
    let auth = request.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

// ── Well-known handler ────────────────────────────────────────────────────────

async fn oauth_authorization_server(
    State(oauth): State<Arc<OAuthState>>,
) -> Response {
    match oauth.get_oidc_metadata().await {
        Ok(meta) => Json(meta).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to fetch OIDC metadata");
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "temporarily_unavailable",
                "error_description": "Authorization server metadata unavailable"
            }))).into_response()
        }
    }
}

// ── Health handler ────────────────────────────────────────────────────────────

async fn health(name: String) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "name": name,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Start the external HTTP server with optional OAuth Bearer validation.
///
/// - `/health` — always public
/// - `/.well-known/oauth-authorization-server` — always public (if OAuth configured)
/// - `/mcp` — requires valid Bearer JWT (if `oauth_config` is `Some`)
pub async fn serve_http(
    server: {{ ProjectName }}Server,
    port: u16,
    oauth_config: Option<OAuthConfig>,
) -> Result<()> {
    let addr = format!("0.0.0.0:{port}");
    tracing::info!(
        addr = %addr,
        auth = %if oauth_config.is_some() { "oauth" } else { "none" },
        "starting external MCP HTTP transport",
    );

    let health_name = server.config.name.clone();
    let mcp_server = server.clone();
    let service = StreamableHttpService::new(
        move || Ok(mcp_server.clone()),
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    let mut app = axum::Router::new()
        .route("/health", get(move || health(health_name)))
        .nest_service("/mcp", service)
        .layer(CorsLayer::permissive());

    if let Some(oauth) = oauth_config {
        let http_client = reqwest::Client::new();
        let oauth_state = Arc::new(OAuthState::new(oauth, http_client));

        // Prefetch JWKS and OIDC metadata at startup (non-fatal if unavailable).
        let state_clone = oauth_state.clone();
        tokio::spawn(async move {
            if let Err(e) = state_clone.get_jwks().await {
                tracing::warn!(error = %e, "initial JWKS prefetch failed — will retry on first request");
            }
            if let Err(e) = state_clone.get_oidc_metadata().await {
                tracing::warn!(error = %e, "initial OIDC metadata prefetch failed — will retry on first request");
            }
        });

        app = app
            .route(
                "/.well-known/oauth-authorization-server",
                get(oauth_authorization_server).with_state(oauth_state.clone()),
            )
            .layer(middleware::from_fn_with_state(oauth_state, oauth_middleware));
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("external HTTP listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Start the internal HTTP server — no authentication, loopback only.
///
/// Intended for internal callers on the same host. Do not expose publicly.
pub async fn serve_internal_http(server: {{ ProjectName }}Server, port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{port}");
    tracing::info!(addr = %addr, "starting internal MCP HTTP transport (no auth)");

    let health_name = server.config.name.clone();
    let mcp_server = server.clone();
    let service = StreamableHttpService::new(
        move || Ok(mcp_server.clone()),
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    let app = axum::Router::new()
        .route("/health", get(move || health(health_name)))
        .nest_service("/mcp", service);
    // No CORS, no auth, bound to loopback only.

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("internal HTTP listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// ── Shutdown ──────────────────────────────────────────────────────────────────

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        )
        .expect("failed to register SIGTERM handler");

        tokio::select! {
            _ = ctrl_c => { tracing::info!("received Ctrl+C, shutting down"); }
            _ = sigterm.recv() => { tracing::info!("received SIGTERM, shutting down"); }
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.expect("failed to listen for Ctrl+C");
        tracing::info!("received Ctrl+C, shutting down");
    }
}
