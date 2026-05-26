use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

use super::server::BridgeState;

// ---------- GET /v1/status ----------

#[derive(Serialize)]
pub struct StatusResponse {
    pub unlocked: bool,
    pub database_name: Option<String>,
    pub version: &'static str,
}

pub async fn status(State(state): State<BridgeState>) -> Json<StatusResponse> {
    let db_guard = state.db_handle.lock();
    let (unlocked, database_name) = match db_guard {
        Ok(guard) => match guard.as_ref() {
            Some(db) => (
                true,
                db.path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string()),
            ),
            None => (false, None),
        },
        Err(_) => (false, None),
    };

    Json(StatusResponse {
        unlocked,
        database_name,
        version: env!("CARGO_PKG_VERSION"),
    })
}

// ---------- GET /v1/entries?domain=example.com ----------

#[derive(Deserialize)]
pub struct ListEntriesQuery {
    pub domain: Option<String>,
}

#[derive(Serialize)]
pub struct EntrySummary {
    pub uuid: String,
    pub title: String,
    pub username: String,
    pub url: String,
}

pub async fn list_entries(
    State(state): State<BridgeState>,
    Query(query): Query<ListEntriesQuery>,
) -> Result<Json<Vec<EntrySummary>>, StatusCode> {
    let entries = with_unlocked_db(&state, |db| {
        let all = db.get_all_entries();
        match query.domain.as_deref() {
            Some(domain) if !domain.is_empty() => all
                .into_iter()
                .filter(|e| matches_domain(&e.url, domain))
                .map(to_summary)
                .collect::<Vec<_>>(),
            _ => all.into_iter().map(to_summary).collect::<Vec<_>>(),
        }
    })?;

    Ok(Json(entries))
}

fn to_summary(e: crate::kdbx::EntryData) -> EntrySummary {
    EntrySummary {
        uuid: e.uuid,
        title: e.title,
        username: e.username,
        url: e.url,
    }
}

// Match a requested domain against an entry's URL field. Strips the scheme
// and any path/query/fragment, then compares the host suffix so that an
// entry pointing at `https://accounts.example.com/login` also matches a
// request for `example.com`.
fn matches_domain(entry_url: &str, requested_domain: &str) -> bool {
    let entry_host = extract_host(entry_url);
    let requested = requested_domain.trim().to_ascii_lowercase();
    let entry_host = entry_host.to_ascii_lowercase();

    if entry_host.is_empty() || requested.is_empty() {
        return false;
    }

    entry_host == requested
        || entry_host.ends_with(&format!(".{}", requested))
        || requested.ends_with(&format!(".{}", entry_host))
}

fn extract_host(url: &str) -> String {
    let without_scheme = match url.find("://") {
        Some(idx) => &url[idx + 3..],
        None => url,
    };
    let host_end = without_scheme
        .find(|c: char| c == '/' || c == '?' || c == '#')
        .unwrap_or(without_scheme.len());
    let host = &without_scheme[..host_end];
    // Strip optional userinfo and port
    let host = host.rsplit_once('@').map(|(_, h)| h).unwrap_or(host);
    let host = host.split_once(':').map(|(h, _)| h).unwrap_or(host);
    host.trim_start_matches("www.").to_string()
}

// ---------- GET /v1/entries/:id/password ----------

#[derive(Serialize)]
pub struct PasswordResponse {
    pub password: String,
    pub username: String,
    pub title: String,
}

pub async fn entry_password(
    State(state): State<BridgeState>,
    Path(uuid): Path<String>,
) -> Result<Json<PasswordResponse>, StatusCode> {
    let entry = with_unlocked_db(&state, |db| db.get_entry(&uuid).ok())?;

    match entry {
        Some(e) => Ok(Json(PasswordResponse {
            password: e.password,
            username: e.username,
            title: e.title,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ---------- POST /v1/password/generate ----------

#[derive(Deserialize)]
pub struct GeneratePasswordRequest {
    #[serde(default = "default_length")]
    pub length: usize,
    #[serde(default = "default_true")]
    pub uppercase: bool,
    #[serde(default = "default_true")]
    pub lowercase: bool,
    #[serde(default = "default_true")]
    pub numbers: bool,
    #[serde(default = "default_true")]
    pub symbols: bool,
}

fn default_length() -> usize {
    20
}
fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub struct GeneratePasswordResponse {
    pub password: String,
}

pub async fn generate_password(
    Json(req): Json<GeneratePasswordRequest>,
) -> Result<Json<GeneratePasswordResponse>, StatusCode> {
    use rand::Rng;

    if req.length == 0 || req.length > 256 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut charset = String::new();
    if req.uppercase {
        charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
    if req.lowercase {
        charset.push_str("abcdefghijklmnopqrstuvwxyz");
    }
    if req.numbers {
        charset.push_str("0123456789");
    }
    if req.symbols {
        charset.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?");
    }
    if charset.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let chars: Vec<char> = charset.chars().collect();
    let mut rng = rand::rng();
    let password: String = (0..req.length)
        .map(|_| chars[rng.random_range(0..chars.len())])
        .collect();

    Ok(Json(GeneratePasswordResponse { password }))
}

// ---------- helpers ----------

// Run a closure with a live `&Database` reference, returning 423 Locked when
// no DB is open and 500 if the AppState mutex is poisoned. The closure runs
// synchronously while the mutex is held — handlers must not .await inside.
fn with_unlocked_db<T, F>(state: &BridgeState, f: F) -> Result<T, StatusCode>
where
    F: FnOnce(&crate::kdbx::Database) -> T,
{
    let guard = state
        .db_handle
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match guard.as_ref() {
        Some(db) => Ok(f(db)),
        None => Err(StatusCode::LOCKED), // 423
    }
}

// IntoResponse for `()` already exists in axum, but we keep this here as a
// hook for adding tracing / audit-log later without touching call sites.
#[allow(dead_code)]
fn _audited<R: IntoResponse>(r: R) -> R {
    r
}
