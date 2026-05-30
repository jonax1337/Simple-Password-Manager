//! Self-hosted vault-sync server for Simple Password Manager.
//!
//! Single tenant per database file. Designed to be dropped behind a reverse
//! proxy (Caddy/Nginx) that terminates TLS — this binary serves plain HTTP.

use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use jsonwebtoken::{DecodingKey, EncodingKey};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

mod auth;
mod error;
mod storage;
mod vault;

use auth::AuthState;
use storage::Storage;

/// 16 MB cap on request body — generous for kdbx vault sizes (real-world
/// 1000-entry vaults are <1 MB) without leaving an obvious DoS hole.
const BODY_LIMIT: usize = 16 * 1024 * 1024;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = Config::from_env();
    tracing::info!(?cfg.bind, db = %cfg.db_path.display(), "starting password-wallet-server");

    let storage = Storage::open(&cfg.db_path).expect("open storage");
    let state = AuthState {
        storage,
        jwt_encode: Arc::new(EncodingKey::from_secret(cfg.jwt_secret.as_bytes())),
        jwt_decode: Arc::new(DecodingKey::from_secret(cfg.jwt_secret.as_bytes())),
    };

    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind(cfg.bind)
        .await
        .expect("bind socket");
    tracing::info!("listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve");
}

pub fn build_app(state: AuthState) -> Router {
    let public = Router::new()
        .route("/health", get(health))
        .route("/auth/signup", post(auth::signup))
        .route("/auth/login", post(auth::login))
        .route("/auth/kdf-params", post(auth::kdf_params));

    let protected = Router::new()
        .route("/vault", get(vault::get_vault))
        .route("/vault", put(vault::put_vault))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    Router::new()
        .merge(public)
        .merge(protected)
        .layer(RequestBodyLimitLayer::new(BODY_LIMIT))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Debug)]
struct Config {
    bind: SocketAddr,
    db_path: PathBuf,
    jwt_secret: String,
}

impl Config {
    fn from_env() -> Self {
        let bind: SocketAddr = std::env::var("BIND")
            .unwrap_or_else(|_| "0.0.0.0:8090".into())
            .parse()
            .expect("BIND must be host:port");
        let db_path = PathBuf::from(
            std::env::var("DB_PATH").unwrap_or_else(|_| "vault-server.db".into()),
        );
        // Refuse to start with a default secret in release. Self-hosters who
        // skip the env var get a loud failure rather than a silently-insecure
        // server.
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                eprintln!("WARN: using dev JWT_SECRET — set JWT_SECRET in production");
                "dev-secret-do-not-use-in-prod".into()
            } else {
                panic!("JWT_SECRET env var is required in release builds");
            }
        });
        Self {
            bind,
            db_path,
            jwt_secret,
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .ok()
            .map(|mut s| async move {
                s.recv().await;
            });
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
    tracing::info!("shutdown signal received");
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;

    fn fresh_app() -> Router {
        let storage = Storage::open_in_memory().unwrap();
        let secret = b"test-secret-for-jwt-no-leak";
        let state = AuthState {
            storage,
            jwt_encode: Arc::new(EncodingKey::from_secret(secret)),
            jwt_decode: Arc::new(DecodingKey::from_secret(secret)),
        };
        build_app(state)
    }

    async fn post_json(app: Router, path: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
        let res = app
            .oneshot(
                Request::post(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(json!(null));
        (status, v)
    }

    async fn auth_request(
        app: Router,
        method: &str,
        path: &str,
        token: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, HeaderValue, serde_json::Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {}", token));
        if body.is_some() {
            req = req.header("content-type", "application/json");
        }
        let body_bytes = body.map(|b| b.to_string()).unwrap_or_default();
        let res = app.oneshot(req.body(Body::from(body_bytes)).unwrap()).await.unwrap();
        let status = res.status();
        let etag = res
            .headers()
            .get("etag")
            .cloned()
            .unwrap_or(HeaderValue::from_static(""));
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(json!(null));
        (status, etag, v)
    }

    #[tokio::test]
    async fn signup_then_login_round_trips() {
        let app = fresh_app();
        let (status, body) = post_json(
            app.clone(),
            "/auth/signup",
            json!({
                "email": "a@b.test",
                "kdf_salt_b64": "c2FsdA==",
                "client_auth_hash": "deadbeefdeadbeef",
                "initial_vault_blob_b64": "aGVsbG8=",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "signup: {:?}", body);
        let token = body["token"].as_str().unwrap();
        assert!(!token.is_empty());

        let (login_status, login_body) = post_json(
            app,
            "/auth/login",
            json!({ "email": "a@b.test", "client_auth_hash": "deadbeefdeadbeef" }),
        )
        .await;
        assert_eq!(login_status, StatusCode::OK);
        assert_eq!(login_body["kdf_salt_b64"], "c2FsdA==");
    }

    #[tokio::test]
    async fn login_with_wrong_hash_fails() {
        let app = fresh_app();
        let _ = post_json(
            app.clone(),
            "/auth/signup",
            json!({
                "email": "a@b.test",
                "kdf_salt_b64": "x",
                "client_auth_hash": "correct-correct-correct",
                "initial_vault_blob_b64": "Zg==",
            }),
        )
        .await;
        let (status, _) = post_json(
            app,
            "/auth/login",
            json!({ "email": "a@b.test", "client_auth_hash": "wrong-wrong-wrong-wrong" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn vault_get_requires_auth() {
        let app = fresh_app();
        let res = app
            .oneshot(Request::get("/vault").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn vault_put_with_stale_etag_returns_409() {
        let app = fresh_app();
        let (_, signup) = post_json(
            app.clone(),
            "/auth/signup",
            json!({
                "email": "x@y.test",
                "kdf_salt_b64": "x",
                "client_auth_hash": "auth-auth-auth-auth",
                "initial_vault_blob_b64": "Zmlyc3Q=",
            }),
        )
        .await;
        let token = signup["token"].as_str().unwrap().to_string();

        let (_, etag, get_body) = auth_request(app.clone(), "GET", "/vault", &token, None).await;
        let real_etag = get_body["etag"].as_str().unwrap().to_string();
        assert!(!real_etag.is_empty());
        assert_eq!(etag.to_str().unwrap(), real_etag);

        // First update with correct etag succeeds.
        let (s1, _, _) = auth_request(
            app.clone(),
            "PUT",
            "/vault",
            &token,
            Some(json!({ "ciphertext_b64": "c2Vjb25k", "expected_etag": real_etag })),
        )
        .await;
        assert_eq!(s1, StatusCode::OK);

        // Re-using the now-stale etag fails.
        let (s2, _, _) = auth_request(
            app,
            "PUT",
            "/vault",
            &token,
            Some(json!({ "ciphertext_b64": "dGhpcmQ=", "expected_etag": real_etag })),
        )
        .await;
        assert_eq!(s2, StatusCode::CONFLICT);
    }
}
