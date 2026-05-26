use axum::{
    extract::State,
    http::{header, HeaderName, HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::auth::{token_matches, BridgeFile};
use super::handlers;
use crate::state::DatabaseHandle;

/// Shared state passed to every Bridge HTTP handler.
#[derive(Clone)]
pub struct BridgeState {
    pub token: Arc<String>,
    pub db_handle: DatabaseHandle,
}

/// Boots the HTTP server on 127.0.0.1 with an OS-chosen port. The server
/// runs until the process exits — there is no explicit shutdown, the OS
/// reaps the socket when the Tauri app terminates, and the bridge.json
/// cleanup in main.rs handles state.
pub async fn start(db_handle: DatabaseHandle) -> std::io::Result<BridgeFile> {
    let token = super::auth::generate_token();
    let state = BridgeState {
        token: Arc::new(token.clone()),
        db_handle,
    };

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let app = router(state);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("[bridge] HTTP server exited with error: {}", e);
        }
    });

    Ok(BridgeFile {
        port,
        token,
        pid: std::process::id(),
    })
}

fn router(state: BridgeState) -> Router {
    // CORS: allow only chrome-extension:// and moz-extension:// origins. We
    // can't statically pin a single extension ID (we don't know ours until
    // the extension is built/installed), so we use a predicate.
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-pw-domain"),
        ])
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin
                .to_str()
                .ok()
                .map(|s| s.starts_with("chrome-extension://") || s.starts_with("moz-extension://"))
                .unwrap_or(false)
        }));

    Router::new()
        .route("/v1/status", get(handlers::status))
        .route(
            "/v1/entries",
            get(handlers::list_entries).post(handlers::create_entry),
        )
        .route("/v1/entries/{id}/password", get(handlers::entry_password))
        .route("/v1/password/generate", post(handlers::generate_password))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer_token,
        ))
        .layer(cors)
        .with_state(state)
}

async fn require_bearer_token(
    State(state): State<BridgeState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let header_value = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let candidate = header_value
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !token_matches(&state.token, candidate) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

// Helper so handlers can build the `X-Pw-Domain` header response when echoing
// the requested domain back. Currently unused in the public API but kept here
// so the cors `allow_headers` reference resolves.
#[allow(dead_code)]
fn _header_value(s: &str) -> HeaderValue {
    HeaderValue::from_str(s).unwrap_or(HeaderValue::from_static(""))
}

// Make the address representable for logs without forcing every caller to
// import SocketAddr just to print it.
#[allow(dead_code)]
pub fn loopback_addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}
