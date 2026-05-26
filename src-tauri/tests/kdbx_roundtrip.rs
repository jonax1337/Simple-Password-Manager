//! End-to-end KDBX roundtrip: build a database in memory, save it,
//! reopen it from disk with a fresh `Database::open`, and verify every
//! piece of data we shipped to disk made it back.
//!
//! This runs against the *production* KDF (64 MB Argon2id) so it's
//! intentionally slow — that's the price of testing the real format,
//! including the heavy KDF path that the inline unit tests skip.

use simple_password_manager::kdbx::{CustomField, Database, EntryData};
use tempfile::TempDir;

fn blank_entry(group_uuid: &str, title: &str, password: &str) -> EntryData {
    EntryData {
        uuid: String::new(),
        title: title.into(),
        username: "user".into(),
        password: password.into(),
        url: "https://example.com".into(),
        notes: "note".into(),
        tags: "tag".into(),
        group_uuid: group_uuid.into(),
        icon_id: Some(7),
        is_favorite: false,
        created: None,
        modified: None,
        last_accessed: None,
        expiry_time: None,
        expires: false,
        usage_count: 0,
        custom_fields: Vec::new(),
        history: Vec::new(),
    }
}

#[test]
fn full_database_roundtrip_through_disk() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("vault.kdbx");

    // ---------- Build ----------
    let mut db = Database::create(path.clone(), "master-password".into()).unwrap();
    let root = db.get_root_group().uuid.clone();

    db.create_group("Work".into(), Some(root.clone()), Some(48)).unwrap();
    db.create_group("Personal".into(), Some(root.clone()), Some(49)).unwrap();
    let work = db
        .get_root_group()
        .children
        .iter()
        .find(|g| g.name == "Work")
        .unwrap()
        .uuid
        .clone();
    let personal = db
        .get_root_group()
        .children
        .iter()
        .find(|g| g.name == "Personal")
        .unwrap()
        .uuid
        .clone();

    let mut e1 = blank_entry(&work, "GitHub", "gh-password!");
    e1.is_favorite = true;
    e1.custom_fields = vec![CustomField {
        name: "Recovery".into(),
        value: "rec-code-123".into(),
        protected: true,
    }];
    db.create_entry(e1).unwrap();

    let mut e2 = blank_entry(&personal, "Bank", "bank-password!");
    e2.expires = true;
    e2.expiry_time = Some("2030-01-15T12:00".into());
    db.create_entry(e2).unwrap();

    db.save().unwrap();
    drop(db);

    // ---------- Reopen ----------
    let reopened = Database::open(path, "master-password".into()).unwrap();

    // Root keeps the file-stem as its name.
    assert_eq!(reopened.get_root_group().name, "vault");

    // Two groups under root.
    let children = reopened.get_root_group().children;
    assert_eq!(children.len(), 2);
    let names: Vec<_> = children.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Work"));
    assert!(names.contains(&"Personal"));

    // All entries survive.
    let all = reopened.get_all_entries();
    assert_eq!(all.len(), 2);

    let gh = all.iter().find(|e| e.title == "GitHub").expect("GitHub entry missing");
    assert_eq!(gh.password, "gh-password!");
    assert!(gh.is_favorite);
    assert_eq!(gh.custom_fields.len(), 1);
    assert_eq!(gh.custom_fields[0].name, "Recovery");
    assert_eq!(gh.custom_fields[0].value, "rec-code-123");
    assert!(gh.custom_fields[0].protected);
    assert_eq!(gh.icon_id, Some(7));

    let bank = all.iter().find(|e| e.title == "Bank").expect("Bank entry missing");
    assert_eq!(bank.password, "bank-password!");
    assert!(bank.expires);
    assert_eq!(bank.expiry_time.as_deref(), Some("2030-01-15T12:00"));
}

#[test]
fn reopen_with_wrong_password_fails() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("vault.kdbx");
    let _ = Database::create(path.clone(), "right".into()).unwrap();

    assert!(Database::open(path, "wrong".into()).is_err());
}

#[test]
fn production_kdf_is_argon2id_with_owasp_params() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("vault.kdbx");
    let db = Database::create(path, "pw".into()).unwrap();

    let kdf = db.get_kdf_info();
    assert_eq!(kdf.kdf_type, "Argon2id");
    // In integration-test mode we use the production defaults: 64 MB, 2 iter, 2 lanes.
    assert!(!kdf.is_weak, "production KDF should not be flagged weak");
    assert_eq!(kdf.iterations, Some(2));
    assert_eq!(kdf.memory, Some(64 * 1024 * 1024));
    assert_eq!(kdf.parallelism, Some(2));
}
