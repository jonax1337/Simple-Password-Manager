//! Self-hosted vault-sync server for Simple Password Manager.
//!
//! Single tenant per database file. Designed to be dropped behind a reverse
//! proxy (Caddy/Nginx) that terminates TLS — this binary serves plain HTTP.

use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use directories::ProjectDirs;
use jsonwebtoken::{DecodingKey, EncodingKey};
use rand::RngCore;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
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
    build_app_inner(state, true)
}

/// Variant used by tests: tests use `oneshot()` which has no socket peer,
/// so the IP-based rate limiter would fail before reaching the handler.
/// Production callers go through `build_app` and always get rate limiting.
fn build_app_inner(state: AuthState, rate_limit: bool) -> Router {
    let public = Router::new()
        .route("/auth/signup", post(auth::signup))
        .route("/auth/login", post(auth::login))
        .route("/auth/kdf-params", post(auth::kdf_params))
        .route("/auth/recovery-init", post(auth::recovery_init))
        .route("/auth/reset", post(auth::reset_password));

    // Per-IP throttle for unauthenticated endpoints: bursts of 5, refilling
    // 1 token every 6 seconds. That's ~10/minute steady-state — enough for
    // a human re-trying a typo, far below what's useful for brute-forcing
    // an Argon2id-hashed credential. Authenticated /vault traffic isn't
    // gated because the bearer token is the rate limit.
    //
    // SmartIp falls back to X-Forwarded-For/X-Real-IP when no socket peer
    // is available — which is what we want behind Caddy.
    let public = if rate_limit {
        let cfg = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(6)
                .burst_size(5)
                .key_extractor(SmartIpKeyExtractor)
                .finish()
                .expect("valid governor config"),
        );
        public.layer(GovernorLayer::new(cfg))
    } else {
        public
    };

    let unrated = Router::new().route("/health", get(health));

    let protected = Router::new()
        .route("/vaults", post(vault::create_vault))
        .route("/vaults", get(vault::list_vaults))
        .route("/vaults/{id}", get(vault::get_vault))
        .route("/vaults/{id}", put(vault::put_vault))
        .route("/vaults/{id}", axum::routing::patch(vault::rename_vault))
        .route("/vaults/{id}", axum::routing::delete(vault::delete_vault))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    Router::new()
        .merge(public)
        .merge(unrated)
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
        let data_dir = resolve_data_dir();
        if let Err(e) = fs::create_dir_all(&data_dir) {
            panic!("Failed to create data dir {}: {}", data_dir.display(), e);
        }

        let bind: SocketAddr = std::env::var("BIND")
            .unwrap_or_else(|_| "0.0.0.0:8090".into())
            .parse()
            .expect("BIND must be host:port");

        let db_path = std::env::var("DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| data_dir.join("vault.db"));

        // Auto-provision a JWT secret on first run and persist it next to the
        // DB. Same-machine restarts pick it up automatically. To rotate,
        // delete the file (this invalidates all sessions) or override with
        // JWT_SECRET. Self-hosters never have to think about this.
        let jwt_secret = match std::env::var("JWT_SECRET") {
            Ok(v) if !v.trim().is_empty() => v,
            _ => load_or_create_secret(&data_dir.join("jwt_secret"))
                .expect("Failed to load or create JWT secret"),
        };

        tracing::info!(data_dir = %data_dir.display(), "config resolved");

        Self {
            bind,
            db_path,
            jwt_secret,
        }
    }
}

/// Pick a per-OS persistent dir (LocalAppData on Win, ~/.local/share on Linux,
/// ~/Library/Application Support on macOS). Override with `DATA_DIR` env var
/// for containers and unusual setups.
fn resolve_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dirs) = ProjectDirs::from("dev", "passwordwallet", "password-wallet-server") {
        return dirs.data_dir().to_path_buf();
    }
    // Last resort: current dir. Worse than the OS default but at least the
    // server still boots.
    PathBuf::from(".")
}

/// Read an existing 32-byte hex secret, or generate one with restricted file
/// permissions on Unix (0600). On Windows the file inherits ACLs from the
/// user's profile dir — typically already owner-only.
fn load_or_create_secret(path: &Path) -> std::io::Result<String> {
    if let Ok(existing) = fs::read_to_string(path) {
        let trimmed = existing.trim();
        if trimmed.len() >= 32 {
            return Ok(trimmed.to_string());
        }
    }
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let hex = hex::encode(bytes);
    write_secret_atomic(path, &hex)?;
    tracing::warn!(
        secret_path = %path.display(),
        "auto-generated new JWT secret — keep this file safe; deleting invalidates all sessions",
    );
    Ok(hex)
}

#[cfg(unix)]
fn write_secret_atomic(path: &Path, value: &str) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut tmp = path.to_path_buf();
    tmp.set_extension("tmp");
    {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(value.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

#[cfg(not(unix))]
fn write_secret_atomic(path: &Path, value: &str) -> std::io::Result<()> {
    let mut tmp = path.to_path_buf();
    tmp.set_extension("tmp");
    fs::write(&tmp, value)?;
    // On Windows, NTFS ACLs inherited from the parent dir (LocalAppData) are
    // already owner-restricted; explicit chmod-equivalent would mean pulling
    // in `windows-acl`. Skip for now.
    fs::rename(&tmp, path)
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
        // Tests skip the rate-limit layer — oneshot() has no socket peer
        // and the IP key extractor would short-circuit before the handler.
        super::build_app_inner(state, false)
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

    fn signup_payload(email: &str, auth_hash: &str) -> serde_json::Value {
        json!({
            "email": email,
            "kdf_salt_b64": "c2FsdA==",
            "client_auth_hash": auth_hash,
            "wrapped_master_key_b64": "d3JhcHBlZC1tYXN0ZXIta2V5LWJsb2I=",
            "recovery_blob_b64": "cmVjb3ZlcnktYmxvYg==",
            "account_pubkey_b64": "",
            "wrapped_account_privkey_b64": "",
            "initial_vault_name": "Personal",
            "initial_vault_blob_b64": "aGVsbG8=",
            "initial_wrapped_vault_key_b64": "d3JhcHBlZC12YXVsdC1rZXk=",
        })
    }

    #[tokio::test]
    async fn signup_then_login_round_trips() {
        let app = fresh_app();
        let (status, body) = post_json(
            app.clone(),
            "/auth/signup",
            signup_payload("alice", "deadbeefdeadbeef"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "signup: {:?}", body);
        let token = body["token"].as_str().unwrap();
        assert!(!token.is_empty());

        let (login_status, login_body) = post_json(
            app,
            "/auth/login",
            json!({ "email": "alice", "client_auth_hash": "deadbeefdeadbeef" }),
        )
        .await;
        assert_eq!(login_status, StatusCode::OK);
        assert_eq!(login_body["kdf_salt_b64"], "c2FsdA==");
        assert_eq!(
            login_body["wrapped_master_key_b64"],
            "d3JhcHBlZC1tYXN0ZXIta2V5LWJsb2I="
        );
    }

    #[tokio::test]
    async fn login_with_wrong_hash_fails() {
        let app = fresh_app();
        let _ = post_json(
            app.clone(),
            "/auth/signup",
            signup_payload("alice", "correct-correct-correct"),
        )
        .await;
        let (status, _) = post_json(
            app,
            "/auth/login",
            json!({ "email": "alice", "client_auth_hash": "wrong-wrong-wrong-wrong" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn recovery_init_returns_blob_then_reset_swaps_credentials() {
        let app = fresh_app();
        let _ = post_json(
            app.clone(),
            "/auth/signup",
            signup_payload("bob", "old-auth-hash-1234"),
        )
        .await;

        let (s_init, init_body) = post_json(
            app.clone(),
            "/auth/recovery-init",
            json!({ "email": "bob" }),
        )
        .await;
        assert_eq!(s_init, StatusCode::OK);
        assert_eq!(init_body["recovery_blob_b64"], "cmVjb3ZlcnktYmxvYg==");

        // Reset uses the SAME client_auth_hash (because master_key didn't
        // change — we just re-wrapped it with a new password). Server
        // accepts because the proof still matches.
        let (s_reset, reset_body) = post_json(
            app.clone(),
            "/auth/reset",
            json!({
                "email": "bob",
                "client_auth_hash": "old-auth-hash-1234",
                "new_kdf_salt_b64": "bmV3LXNhbHQ=",
                "new_wrapped_master_key_b64": "bmV3LXdyYXBwZWQta2V5LWJsb2I=",
            }),
        )
        .await;
        assert_eq!(s_reset, StatusCode::OK, "reset: {:?}", reset_body);
        assert!(reset_body["token"].as_str().is_some());

        // Subsequent login returns the NEW wrapped_master_key.
        let (_, login_body) = post_json(
            app,
            "/auth/login",
            json!({ "email": "bob", "client_auth_hash": "old-auth-hash-1234" }),
        )
        .await;
        assert_eq!(login_body["kdf_salt_b64"], "bmV3LXNhbHQ=");
        assert_eq!(
            login_body["wrapped_master_key_b64"],
            "bmV3LXdyYXBwZWQta2V5LWJsb2I="
        );
    }

    #[tokio::test]
    async fn reset_with_wrong_auth_hash_rejected() {
        let app = fresh_app();
        let _ = post_json(
            app.clone(),
            "/auth/signup",
            signup_payload("carol", "real-auth-hash-1234"),
        )
        .await;
        let (status, _) = post_json(
            app,
            "/auth/reset",
            json!({
                "email": "carol",
                "client_auth_hash": "fake-auth-hash-9999",
                "new_kdf_salt_b64": "x",
                "new_wrapped_master_key_b64": "xx",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn list_vaults_requires_auth() {
        let app = fresh_app();
        let res = app
            .oneshot(Request::get("/vaults").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn signup_creates_first_vault_listable_by_owner() {
        let app = fresh_app();
        let (_, signup) = post_json(
            app.clone(),
            "/auth/signup",
            signup_payload("vaultuser", "auth-auth-auth-auth"),
        )
        .await;
        let token = signup["token"].as_str().unwrap().to_string();

        let (s, _, body) = auth_request(app, "GET", "/vaults", &token, None).await;
        assert_eq!(s, StatusCode::OK);
        let vaults = body["vaults"].as_array().unwrap();
        assert_eq!(vaults.len(), 1);
        assert_eq!(vaults[0]["name"], "Personal");
        assert_eq!(vaults[0]["role"], "owner");
        assert_eq!(
            vaults[0]["wrapped_vault_key_b64"],
            "d3JhcHBlZC12YXVsdC1rZXk="
        );
    }

    #[tokio::test]
    async fn create_then_get_then_put_vault_round_trips() {
        let app = fresh_app();
        let (_, signup) = post_json(
            app.clone(),
            "/auth/signup",
            signup_payload("rt", "auth-auth-auth-auth"),
        )
        .await;
        let token = signup["token"].as_str().unwrap().to_string();

        let (s_create, _, body) = auth_request(
            app.clone(),
            "POST",
            "/vaults",
            &token,
            Some(json!({
                "name": "Work",
                "ciphertext_b64": "d29yay1ibG9i",
                "wrapped_vault_key_b64": "d3JhcHBlZC13b3JrLWtleS1tb3JlIHRoYW4gMjQ=",
            })),
        )
        .await;
        assert_eq!(s_create, StatusCode::OK);
        let id = body["id"].as_str().unwrap().to_string();
        let first_etag = body["etag"].as_str().unwrap().to_string();

        // GET single vault
        let (s_get, _, full) =
            auth_request(app.clone(), "GET", &format!("/vaults/{}", id), &token, None).await;
        assert_eq!(s_get, StatusCode::OK);
        assert_eq!(full["ciphertext_b64"], "d29yay1ibG9i");
        assert_eq!(full["role"], "owner");

        // PUT with correct etag
        let (s1, _, _) = auth_request(
            app.clone(),
            "PUT",
            &format!("/vaults/{}", id),
            &token,
            Some(json!({
                "ciphertext_b64": "d29yay1ibG9iLXYy",
                "expected_etag": first_etag
            })),
        )
        .await;
        assert_eq!(s1, StatusCode::OK);

        // PUT with stale etag
        let (s2, _, _) = auth_request(
            app,
            "PUT",
            &format!("/vaults/{}", id),
            &token,
            Some(json!({
                "ciphertext_b64": "d29yay1ibG9iLXYz",
                "expected_etag": first_etag
            })),
        )
        .await;
        assert_eq!(s2, StatusCode::CONFLICT);
    }
}
