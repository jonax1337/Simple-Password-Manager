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

// ---------- GET /v1/entries/:id/totp ----------

#[derive(Serialize)]
pub struct TotpResponse {
    pub code: String,
    pub period: u64,
    pub remaining_seconds: u64,
    pub algorithm: &'static str,
}

pub async fn entry_totp(
    State(state): State<BridgeState>,
    Path(uuid): Path<String>,
) -> Result<Json<TotpResponse>, StatusCode> {
    let entry = with_unlocked_db(&state, |db| db.get_entry(&uuid).ok())?;
    let entry = entry.ok_or(StatusCode::NOT_FOUND)?;

    // Look for an `otp` field (or any custom field that looks like otpauth://).
    let raw = entry
        .custom_fields
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case("otp"))
        .map(|f| f.value.clone())
        .or_else(|| {
            entry
                .custom_fields
                .iter()
                .find(|f| f.value.starts_with("otpauth://"))
                .map(|f| f.value.clone())
        })
        .ok_or(StatusCode::NOT_FOUND)?;

    let result = if raw.starts_with("otpauth://") {
        crate::totp::from_otpauth_uri(&raw)
    } else {
        crate::totp::from_raw_secret(&raw)
    }
    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    Ok(Json(TotpResponse {
        code: result.code,
        period: result.period,
        remaining_seconds: result.remaining_seconds,
        algorithm: result.algorithm,
    }))
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

// ---------- POST /v1/entries ----------
//
// Creates a new entry from a captured browser login and persists the
// database to disk so the change survives a desktop-app restart.

#[derive(Deserialize)]
pub struct CreateEntryRequest {
    pub title: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Serialize)]
pub struct CreateEntryResponse {
    pub uuid: String,
    pub title: String,
    pub group_uuid: String,
}

pub async fn create_entry(
    State(state): State<BridgeState>,
    Json(req): Json<CreateEntryRequest>,
) -> Result<Json<CreateEntryResponse>, StatusCode> {
    if req.password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let title = if req.title.trim().is_empty() {
        req.url.clone()
    } else {
        req.title.clone()
    };

    let mut guard = state
        .db_handle
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let db = guard.as_mut().ok_or(StatusCode::LOCKED)?;

    let root_uuid = db.get_root_group().uuid;

    let entry_data = crate::kdbx::EntryData {
        uuid: String::new(),
        title: title.clone(),
        username: req.username,
        password: req.password,
        url: req.url,
        notes: req.notes,
        tags: String::new(),
        group_uuid: root_uuid.clone(),
        icon_id: None,
        is_favorite: false,
        created: None,
        modified: None,
        last_accessed: None,
        expiry_time: None,
        expires: false,
        usage_count: 0,
        custom_fields: Vec::new(),
        history: Vec::new(),
    };

    db.create_entry(entry_data)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    db.save()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // create_entry doesn't surface the assigned uuid, so grab the most
    // recently modified entry in the root group — that's the one we just
    // inserted (modified timestamp is set in convert flow).
    let created = db
        .get_all_entries()
        .into_iter()
        .filter(|e| e.group_uuid == root_uuid)
        .max_by_key(|e| e.modified.clone().unwrap_or_default())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CreateEntryResponse {
        uuid: created.uuid,
        title,
        group_uuid: root_uuid,
    }))
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

// ---------- POST /v1/entries/check ----------
//
// Used by the content script before showing the inline "Save this login?"
// banner — so we don't pester the user about saving credentials they
// already have stored. The check is cheap (one in-memory scan) and avoids
// the alternative of shipping every entry's password back to the extension.

#[derive(Deserialize)]
pub struct CheckEntryRequest {
    pub domain: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct CheckEntryResponse {
    /// True if a saved entry has the same domain + username + password —
    /// nothing new to save, suppress the banner.
    pub duplicate: bool,
    /// True if a saved entry has the same domain + username but a different
    /// password — an "update" candidate, banner can offer that flow.
    pub update_available: bool,
    /// The UUID of an entry that matches by domain+username, if any.
    pub matching_uuid: Option<String>,
    /// Same as matching_uuid but only when the title would help the user
    /// recognise which entry would be updated.
    pub matching_title: Option<String>,
}

pub async fn check_entry(
    State(state): State<BridgeState>,
    Json(req): Json<CheckEntryRequest>,
) -> Result<Json<CheckEntryResponse>, StatusCode> {
    let response = with_unlocked_db(&state, |db| {
        let mut duplicate = false;
        let mut update_available = false;
        let mut matching_uuid: Option<String> = None;
        let mut matching_title: Option<String> = None;

        for entry in db.get_all_entries() {
            if !matches_domain(&entry.url, &req.domain) {
                continue;
            }
            if entry.username != req.username {
                continue;
            }
            // Found same domain + username.
            if matching_uuid.is_none() {
                matching_uuid = Some(entry.uuid.clone());
                matching_title = Some(entry.title.clone());
            }
            if entry.password == req.password {
                duplicate = true;
                break;
            } else {
                update_available = true;
            }
        }

        CheckEntryResponse {
            duplicate,
            update_available,
            matching_uuid,
            matching_title,
        }
    })?;

    Ok(Json(response))
}

// ---------- POST /v1/focus-app ----------
//
// Brings the desktop window to the foreground so the user can land on the
// unlock screen without having to find the app themselves. Used by the
// extension when it sees a locked database but the current site has saved
// entries — 1Password-style "click Unlock to flip to the app".

pub async fn focus_app(State(state): State<BridgeState>) -> StatusCode {
    use tauri::Manager;

    let Some(window) = state.app_handle.get_webview_window("main") else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    let _ = window.unminimize();
    let _ = window.show();
    // Windows can refuse focus-steal from a non-foreground process unless
    // we toggle always-on-top briefly. This is what most "bring window to
    // front" helpers do and matches the behaviour KeePassXC uses.
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
    let _ = window.set_always_on_top(false);

    StatusCode::OK
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
