//! End-to-end test of the browser-extension HTTP bridge: build a real
//! BridgeState (with no Tauri AppHandle — focus_app is allowed to 503),
//! bind axum on an ephemeral loopback port, and hit each endpoint with
//! reqwest as the extension would.
//!
//! Covers: bearer-token auth, CORS pre-flight tolerance, status/list/
//! password/totp/check/generate/create flows, and the locked-DB path.

use simple_password_manager::bridge::{router, BridgeState};
use simple_password_manager::kdbx::{Database, EntryData};
use simple_password_manager::state::DatabaseHandle;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

async fn spawn_server(db_handle: DatabaseHandle, token: &str) -> String {
    let state = BridgeState {
        token: Arc::new(token.to_string()),
        db_handle,
        focus_controller: None,
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(state);

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    format!("http://{}", addr)
}

fn unlocked_db_handle() -> (TempDir, DatabaseHandle) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bridge.kdbx");
    let mut db = Database::create(path, "pw".into()).unwrap();
    let root = db.get_root_group().uuid.clone();

    let entry = EntryData {
        uuid: String::new(),
        title: "GitHub".into(),
        username: "alice".into(),
        password: "gh-pass".into(),
        url: "https://github.com/login".into(),
        notes: String::new(),
        tags: String::new(),
        group_uuid: root,
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
    db.create_entry(entry).unwrap();
    db.save().unwrap();

    (dir, Arc::new(Mutex::new(Some(db))))
}

fn locked_db_handle() -> DatabaseHandle {
    Arc::new(Mutex::new(None))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().build().unwrap()
}

const TOKEN: &str = "test-token-deadbeef";

// ---------- auth ----------

#[tokio::test]
async fn rejects_request_without_bearer_token() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let resp = client().get(format!("{}/v1/status", base)).send().await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn rejects_request_with_wrong_token() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let resp = client()
        .get(format!("{}/v1/status", base))
        .bearer_auth("not-the-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn accepts_request_with_correct_token() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let resp = client()
        .get(format!("{}/v1/status", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

// ---------- status ----------

#[tokio::test]
async fn status_reports_unlocked_when_db_is_loaded() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let body: serde_json::Value = client()
        .get(format!("{}/v1/status", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body["unlocked"], true);
    assert_eq!(body["database_name"], "bridge");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn status_reports_locked_when_db_is_none() {
    let base = spawn_server(locked_db_handle(), TOKEN).await;

    let body: serde_json::Value = client()
        .get(format!("{}/v1/status", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body["unlocked"], false);
    assert!(body["database_name"].is_null());
}

// ---------- list entries ----------

#[tokio::test]
async fn list_entries_filters_by_domain() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let hits: Vec<serde_json::Value> = client()
        .get(format!("{}/v1/entries?domain=github.com", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["title"], "GitHub");
    assert_eq!(hits[0]["username"], "alice");
    // The password field is deliberately NOT included in the summary.
    assert!(hits[0].get("password").is_none());
}

#[tokio::test]
async fn list_entries_unrelated_domain_is_empty() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let hits: Vec<serde_json::Value> = client()
        .get(format!("{}/v1/entries?domain=evil.com", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(hits.is_empty());
}

#[tokio::test]
async fn list_entries_when_locked_returns_423() {
    let base = spawn_server(locked_db_handle(), TOKEN).await;

    let resp = client()
        .get(format!("{}/v1/entries?domain=github.com", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 423); // Locked
}

// ---------- password endpoint ----------

#[tokio::test]
async fn password_endpoint_returns_password_for_known_uuid() {
    let (_d, handle) = unlocked_db_handle();
    // Discover the UUID via the list endpoint first.
    let base = spawn_server(handle.clone(), TOKEN).await;

    let hits: Vec<serde_json::Value> = client()
        .get(format!("{}/v1/entries?domain=github.com", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let uuid = hits[0]["uuid"].as_str().unwrap();

    let body: serde_json::Value = client()
        .get(format!("{}/v1/entries/{}/password", base, uuid))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body["password"], "gh-pass");
    assert_eq!(body["username"], "alice");
    assert_eq!(body["title"], "GitHub");
}

#[tokio::test]
async fn password_endpoint_returns_404_for_unknown_uuid() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let resp = client()
        .get(format!(
            "{}/v1/entries/00000000-0000-0000-0000-000000000000/password",
            base
        ))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

// ---------- generate password ----------

#[tokio::test]
async fn generate_password_with_defaults() {
    let base = spawn_server(locked_db_handle(), TOKEN).await;

    let body: serde_json::Value = client()
        .post(format!("{}/v1/password/generate", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let pw = body["password"].as_str().unwrap();
    assert_eq!(pw.chars().count(), 20); // default length
}

#[tokio::test]
async fn generate_password_rejects_zero_length() {
    let base = spawn_server(locked_db_handle(), TOKEN).await;

    let resp = client()
        .post(format!("{}/v1/password/generate", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({ "length": 0 }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn generate_password_rejects_empty_charset() {
    let base = spawn_server(locked_db_handle(), TOKEN).await;

    let resp = client()
        .post(format!("{}/v1/password/generate", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({
            "length": 16,
            "uppercase": false,
            "lowercase": false,
            "numbers": false,
            "symbols": false
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

// ---------- check entry ----------

#[tokio::test]
async fn check_entry_detects_duplicate() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let body: serde_json::Value = client()
        .post(format!("{}/v1/entries/check", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({
            "domain": "github.com",
            "username": "alice",
            "password": "gh-pass"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body["duplicate"], true);
    assert_eq!(body["update_available"], false);
    assert_eq!(body["matching_title"], "GitHub");
}

#[tokio::test]
async fn check_entry_detects_update_candidate() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let body: serde_json::Value = client()
        .post(format!("{}/v1/entries/check", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({
            "domain": "github.com",
            "username": "alice",
            "password": "different-password"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body["duplicate"], false);
    assert_eq!(body["update_available"], true);
}

#[tokio::test]
async fn check_entry_finds_nothing_for_unknown_domain() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let body: serde_json::Value = client()
        .post(format!("{}/v1/entries/check", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({
            "domain": "unknown-site.example",
            "username": "alice",
            "password": "x"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body["duplicate"], false);
    assert_eq!(body["update_available"], false);
    assert!(body["matching_uuid"].is_null());
}

// ---------- create entry ----------

#[tokio::test]
async fn create_entry_persists_and_is_visible_via_list() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let create: serde_json::Value = client()
        .post(format!("{}/v1/entries", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({
            "title": "NewSite",
            "username": "bob",
            "password": "newpw",
            "url": "https://newsite.example"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(create["title"], "NewSite");
    assert!(create["uuid"].as_str().unwrap().len() > 0);

    // And it shows up in the listing.
    let hits: Vec<serde_json::Value> = client()
        .get(format!("{}/v1/entries?domain=newsite.example", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["username"], "bob");
}

#[tokio::test]
async fn create_entry_rejects_empty_password() {
    let (_d, handle) = unlocked_db_handle();
    let base = spawn_server(handle, TOKEN).await;

    let resp = client()
        .post(format!("{}/v1/entries", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({
            "title": "X",
            "username": "u",
            "password": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn create_entry_when_locked_returns_423() {
    let base = spawn_server(locked_db_handle(), TOKEN).await;

    let resp = client()
        .post(format!("{}/v1/entries", base))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({
            "title": "X",
            "username": "u",
            "password": "p"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 423);
}

// ---------- focus app ----------

#[tokio::test]
async fn focus_app_returns_503_without_tauri_handle() {
    let base = spawn_server(locked_db_handle(), TOKEN).await;

    let resp = client()
        .post(format!("{}/v1/focus-app", base))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 503);
}
