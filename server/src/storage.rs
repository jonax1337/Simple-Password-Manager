//! SQLite-backed user + vault storage.
//!
//! Schema is intentionally tiny: one row per user, one row per vault. All
//! crypto happens on the client — `vault_blob` is opaque ciphertext, and we
//! never see the master password. The server holds an Argon2-hashed copy of
//! the *client-derived* auth proof to gate /vault access.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct Storage {
    inner: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    /// Argon2-hashed value derived from the client's auth proof. The client
    /// never sends the master password; it sends a hash that is itself
    /// derived from the password (Argon2id on the client). We Argon2 that
    /// hash again so a stolen DB doesn't immediately yield a usable
    /// auth-token-equivalent.
    pub server_auth_hash: String,
    /// Salt the client used for its first-stage Argon2id. Public — needed by
    /// the client on login to reproduce the same derivation.
    pub kdf_salt_b64: String,
}

#[derive(Debug, Clone)]
pub struct VaultRow {
    pub user_id: String,
    pub ciphertext_b64: String,
    pub etag: String,
    pub updated_at: i64,
}

impl Storage {
    pub fn open(path: &Path) -> ApiResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| ApiError::Internal(format!("open db: {}", e)))?;
        let s = Self {
            inner: Arc::new(Mutex::new(conn)),
        };
        s.init_schema()?;
        Ok(s)
    }

    pub fn open_in_memory() -> ApiResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ApiError::Internal(format!("open in-memory db: {}", e)))?;
        let s = Self {
            inner: Arc::new(Mutex::new(conn)),
        };
        s.init_schema()?;
        Ok(s)
    }

    fn init_schema(&self) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                server_auth_hash TEXT NOT NULL,
                kdf_salt_b64 TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS vaults (
                user_id TEXT PRIMARY KEY,
                ciphertext_b64 TEXT NOT NULL,
                etag TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            "#,
        )
        .map_err(|e| ApiError::Internal(format!("init schema: {}", e)))?;
        Ok(())
    }

    pub fn create_user(
        &self,
        id: &str,
        email: &str,
        server_auth_hash: &str,
        kdf_salt_b64: &str,
    ) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        let now = now_secs();
        let res = conn.execute(
            "INSERT INTO users (id, email, server_auth_hash, kdf_salt_b64, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, email, server_auth_hash, kdf_salt_b64, now],
        );
        match res {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
            {
                Err(ApiError::UserExists)
            }
            Err(e) => Err(ApiError::Internal(format!("insert user: {}", e))),
        }
    }

    pub fn find_user_by_email(&self, email: &str) -> ApiResult<Option<User>> {
        let conn = self.inner.lock().unwrap();
        let user = conn
            .query_row(
                "SELECT id, email, server_auth_hash, kdf_salt_b64
                 FROM users WHERE email = ?1",
                params![email],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        server_auth_hash: row.get(2)?,
                        kdf_salt_b64: row.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(user)
    }

    pub fn find_user_by_id(&self, id: &str) -> ApiResult<Option<User>> {
        let conn = self.inner.lock().unwrap();
        let user = conn
            .query_row(
                "SELECT id, email, server_auth_hash, kdf_salt_b64
                 FROM users WHERE id = ?1",
                params![id],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        server_auth_hash: row.get(2)?,
                        kdf_salt_b64: row.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(user)
    }

    pub fn upsert_vault(
        &self,
        user_id: &str,
        ciphertext_b64: &str,
        new_etag: &str,
        expected_etag: Option<&str>,
    ) -> ApiResult<VaultRow> {
        let conn = self.inner.lock().unwrap();
        // Read current state inside the same connection (effectively serialized
        // by our Mutex) so we can compare-and-swap on the etag.
        let current_etag: Option<String> = conn
            .query_row(
                "SELECT etag FROM vaults WHERE user_id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .optional()?;

        match (current_etag.as_deref(), expected_etag) {
            // No existing vault and client expects none — fine.
            (None, None) => {}
            // No existing vault but client expected one — bad client state.
            (None, Some(_)) => return Err(ApiError::VaultConflict),
            // Existing vault but client expected none — would overwrite.
            (Some(_), None) => return Err(ApiError::VaultConflict),
            // Both present — must match exactly.
            (Some(cur), Some(expected)) if cur == expected => {}
            (Some(_), Some(_)) => return Err(ApiError::VaultConflict),
        }

        let now = now_secs();
        conn.execute(
            "INSERT INTO vaults (user_id, ciphertext_b64, etag, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(user_id) DO UPDATE SET
                ciphertext_b64 = excluded.ciphertext_b64,
                etag = excluded.etag,
                updated_at = excluded.updated_at",
            params![user_id, ciphertext_b64, new_etag, now],
        )
        .map_err(|e| ApiError::Internal(format!("upsert vault: {}", e)))?;

        Ok(VaultRow {
            user_id: user_id.to_string(),
            ciphertext_b64: ciphertext_b64.to_string(),
            etag: new_etag.to_string(),
            updated_at: now,
        })
    }

    pub fn get_vault(&self, user_id: &str) -> ApiResult<Option<VaultRow>> {
        let conn = self.inner.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT user_id, ciphertext_b64, etag, updated_at
                 FROM vaults WHERE user_id = ?1",
                params![user_id],
                |row| {
                    Ok(VaultRow {
                        user_id: row.get(0)?,
                        ciphertext_b64: row.get(1)?,
                        etag: row.get(2)?,
                        updated_at: row.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_find_user_round_trips() {
        let s = Storage::open_in_memory().unwrap();
        s.create_user("uid-1", "a@b.test", "hash", "salt").unwrap();
        let u = s.find_user_by_email("a@b.test").unwrap().unwrap();
        assert_eq!(u.id, "uid-1");
        assert_eq!(u.kdf_salt_b64, "salt");
    }

    #[test]
    fn duplicate_email_rejected() {
        let s = Storage::open_in_memory().unwrap();
        s.create_user("uid-1", "a@b.test", "h", "s").unwrap();
        let err = s.create_user("uid-2", "a@b.test", "h", "s").unwrap_err();
        assert!(matches!(err, ApiError::UserExists));
    }

    #[test]
    fn vault_create_then_update_with_etag() {
        let s = Storage::open_in_memory().unwrap();
        s.create_user("u", "e@x", "h", "s").unwrap();
        let first = s.upsert_vault("u", "blob-1", "etag-1", None).unwrap();
        assert_eq!(first.etag, "etag-1");
        let second = s
            .upsert_vault("u", "blob-2", "etag-2", Some("etag-1"))
            .unwrap();
        assert_eq!(second.ciphertext_b64, "blob-2");
    }

    #[test]
    fn vault_etag_mismatch_returns_conflict() {
        let s = Storage::open_in_memory().unwrap();
        s.create_user("u", "e@x", "h", "s").unwrap();
        s.upsert_vault("u", "blob-1", "etag-1", None).unwrap();
        let err = s
            .upsert_vault("u", "blob-2", "etag-2", Some("wrong-etag"))
            .unwrap_err();
        assert!(matches!(err, ApiError::VaultConflict));
    }

    #[test]
    fn vault_create_with_expected_etag_when_none_exists_is_conflict() {
        let s = Storage::open_in_memory().unwrap();
        s.create_user("u", "e@x", "h", "s").unwrap();
        let err = s
            .upsert_vault("u", "blob-1", "etag-1", Some("ghost"))
            .unwrap_err();
        assert!(matches!(err, ApiError::VaultConflict));
    }
}
