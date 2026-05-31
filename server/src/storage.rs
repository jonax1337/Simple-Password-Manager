//! SQLite-backed user + multi-vault storage.
//!
//! Schema v2 (multi-vault):
//!   users          — one row per account
//!   vaults         — one row per vault (a user owns 1..N)
//!   vault_members  — many-to-many between users and vaults, carries per-user
//!                    wrapped_vault_key + role. Lays the foundation for
//!                    vault sharing in a later stage.
//!
//! All crypto happens on the client. The server stores opaque ciphertext
//! and wrapping blobs; it can identify accounts and route requests but
//! cannot decrypt anything.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct Storage {
    inner: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    pub server_auth_hash: String,
    pub kdf_salt_b64: String,
    pub wrapped_master_key_b64: String,
    pub recovery_blob_b64: String,
    /// Curve25519 public key (base64), used so other accounts can wrap a
    /// vault_key for this user when sharing. Empty until a sharing-capable
    /// client signs the user up; old accounts default to empty and just
    /// can't be granted access until they re-key.
    pub account_pubkey_b64: String,
    /// Curve25519 private key encrypted with `master_key`. Stored so the
    /// user can decrypt incoming share invitations on any device after
    /// logging in.
    pub wrapped_account_privkey_b64: String,
}

#[derive(Debug, Clone)]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub owner_user_id: String,
    pub ciphertext_b64: String,
    pub etag: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// One user's relationship with one vault. The same vault has multiple
/// rows here when shared — each carrying that user's personal
/// `wrapped_vault_key`.
#[derive(Debug, Clone)]
pub struct VaultMembership {
    pub vault_id: String,
    pub user_id: String,
    pub role: VaultRole,
    pub wrapped_vault_key_b64: String,
    pub invited_at: i64,
    pub accepted_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultRole {
    Owner,
    Editor,
    Reader,
}

impl VaultRole {
    pub fn as_str(self) -> &'static str {
        match self {
            VaultRole::Owner => "owner",
            VaultRole::Editor => "editor",
            VaultRole::Reader => "reader",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(VaultRole::Owner),
            "editor" => Some(VaultRole::Editor),
            "reader" => Some(VaultRole::Reader),
            _ => None,
        }
    }

    /// Can this role mutate the vault contents?
    pub fn can_write(self) -> bool {
        matches!(self, VaultRole::Owner | VaultRole::Editor)
    }

    /// Can this role share, rename, or delete the vault?
    pub fn can_admin(self) -> bool {
        matches!(self, VaultRole::Owner)
    }
}

/// Convenience bundle returned by listing + lookup paths: vault + the
/// caller's per-membership view of it.
#[derive(Debug, Clone)]
pub struct VaultListEntry {
    pub vault: Vault,
    pub role: VaultRole,
    pub wrapped_vault_key_b64: String,
}

/// Row returned by `list_vault_members`: enough for the dashboard /
/// settings UI to render member rows with email + role.
#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub user_id: String,
    pub email: String,
    pub role: VaultRole,
    pub invited_at: i64,
    pub accepted_at: Option<i64>,
}

impl Storage {
    pub fn open(path: &Path) -> ApiResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| ApiError::Internal(format!("open db: {}", e)))?;
        // Foreign keys are off by default in SQLite — enable per-connection.
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| ApiError::Internal(format!("enable FKs: {}", e)))?;
        let s = Self {
            inner: Arc::new(Mutex::new(conn)),
        };
        s.init_schema()?;
        Ok(s)
    }

    pub fn open_in_memory() -> ApiResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ApiError::Internal(format!("open in-memory db: {}", e)))?;
        conn.execute_batch("PRAGMA foreign_keys = ON;").ok();
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
                wrapped_master_key_b64 TEXT NOT NULL,
                recovery_blob_b64 TEXT NOT NULL,
                account_pubkey_b64 TEXT NOT NULL DEFAULT '',
                wrapped_account_privkey_b64 TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS vaults (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                owner_user_id TEXT NOT NULL,
                ciphertext_b64 TEXT NOT NULL,
                etag TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (owner_user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS vault_members (
                vault_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                role TEXT NOT NULL,
                wrapped_vault_key_b64 TEXT NOT NULL,
                invited_at INTEGER NOT NULL,
                accepted_at INTEGER,
                PRIMARY KEY (vault_id, user_id),
                FOREIGN KEY (vault_id) REFERENCES vaults(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_vault_members_user ON vault_members(user_id);
            CREATE INDEX IF NOT EXISTS idx_vaults_owner ON vaults(owner_user_id);
            "#,
        )
        .map_err(|e| ApiError::Internal(format!("init schema: {}", e)))?;
        Ok(())
    }

    // -------------------- users --------------------

    pub fn create_user(
        &self,
        id: &str,
        email: &str,
        server_auth_hash: &str,
        kdf_salt_b64: &str,
        wrapped_master_key_b64: &str,
        recovery_blob_b64: &str,
        account_pubkey_b64: &str,
        wrapped_account_privkey_b64: &str,
    ) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        let now = now_secs();
        let res = conn.execute(
            "INSERT INTO users (
                id, email, server_auth_hash, kdf_salt_b64,
                wrapped_master_key_b64, recovery_blob_b64,
                account_pubkey_b64, wrapped_account_privkey_b64, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                email,
                server_auth_hash,
                kdf_salt_b64,
                wrapped_master_key_b64,
                recovery_blob_b64,
                account_pubkey_b64,
                wrapped_account_privkey_b64,
                now
            ],
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
                "SELECT id, email, server_auth_hash, kdf_salt_b64,
                        wrapped_master_key_b64, recovery_blob_b64,
                        account_pubkey_b64, wrapped_account_privkey_b64
                 FROM users WHERE email = ?1",
                params![email],
                Self::map_user,
            )
            .optional()?;
        Ok(user)
    }

    pub fn find_user_by_id(&self, id: &str) -> ApiResult<Option<User>> {
        let conn = self.inner.lock().unwrap();
        let user = conn
            .query_row(
                "SELECT id, email, server_auth_hash, kdf_salt_b64,
                        wrapped_master_key_b64, recovery_blob_b64,
                        account_pubkey_b64, wrapped_account_privkey_b64
                 FROM users WHERE id = ?1",
                params![id],
                Self::map_user,
            )
            .optional()?;
        Ok(user)
    }

    fn map_user(row: &rusqlite::Row) -> rusqlite::Result<User> {
        Ok(User {
            id: row.get(0)?,
            email: row.get(1)?,
            server_auth_hash: row.get(2)?,
            kdf_salt_b64: row.get(3)?,
            wrapped_master_key_b64: row.get(4)?,
            recovery_blob_b64: row.get(5)?,
            account_pubkey_b64: row.get(6)?,
            wrapped_account_privkey_b64: row.get(7)?,
        })
    }

    pub fn reset_credentials(
        &self,
        user_id: &str,
        new_server_auth_hash: &str,
        new_kdf_salt_b64: &str,
        new_wrapped_master_key_b64: &str,
    ) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        let n = conn.execute(
            "UPDATE users
             SET server_auth_hash = ?1, kdf_salt_b64 = ?2, wrapped_master_key_b64 = ?3
             WHERE id = ?4",
            params![
                new_server_auth_hash,
                new_kdf_salt_b64,
                new_wrapped_master_key_b64,
                user_id
            ],
        )?;
        if n == 0 {
            return Err(ApiError::NotFound);
        }
        Ok(())
    }

    // -------------------- vaults --------------------

    /// Create a new vault. Atomically inserts the vault row plus the owner's
    /// membership row carrying the wrapped vault key — owners always have
    /// a member entry so listing/auth code can stay uniform across owners,
    /// editors, and readers.
    pub fn create_vault(
        &self,
        owner_user_id: &str,
        name: &str,
        ciphertext_b64: &str,
        etag: &str,
        wrapped_vault_key_b64: &str,
    ) -> ApiResult<Vault> {
        let mut conn = self.inner.lock().unwrap();
        let now = now_secs();
        let id = Uuid::new_v4().to_string();
        let tx = conn
            .transaction()
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        tx.execute(
            "INSERT INTO vaults (id, name, owner_user_id, ciphertext_b64, etag, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![id, name, owner_user_id, ciphertext_b64, etag, now],
        )?;
        tx.execute(
            "INSERT INTO vault_members (vault_id, user_id, role, wrapped_vault_key_b64, invited_at, accepted_at)
             VALUES (?1, ?2, 'owner', ?3, ?4, ?4)",
            params![id, owner_user_id, wrapped_vault_key_b64, now],
        )?;
        tx.commit()
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(Vault {
            id,
            name: name.to_string(),
            owner_user_id: owner_user_id.to_string(),
            ciphertext_b64: ciphertext_b64.to_string(),
            etag: etag.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// List vaults accessible to this user — owned or shared. Sorted by
    /// most-recently-updated so the picker shows the active vault on top.
    pub fn list_vaults_for_user(&self, user_id: &str) -> ApiResult<Vec<VaultListEntry>> {
        let conn = self.inner.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT v.id, v.name, v.owner_user_id, v.ciphertext_b64, v.etag,
                    v.created_at, v.updated_at,
                    m.role, m.wrapped_vault_key_b64
             FROM vaults v
             INNER JOIN vault_members m ON m.vault_id = v.id
             WHERE m.user_id = ?1
             ORDER BY v.updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![user_id], |row| {
                let role_str: String = row.get(7)?;
                Ok(VaultListEntry {
                    vault: Vault {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        owner_user_id: row.get(2)?,
                        ciphertext_b64: row.get(3)?,
                        etag: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    },
                    role: VaultRole::parse(&role_str).unwrap_or(VaultRole::Reader),
                    wrapped_vault_key_b64: row.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Fetch a single vault for the caller. Returns None if either the
    /// vault doesn't exist or the caller has no membership — we collapse
    /// the two cases so we don't leak vault existence to non-members.
    pub fn get_vault_for_user(
        &self,
        user_id: &str,
        vault_id: &str,
    ) -> ApiResult<Option<VaultListEntry>> {
        let conn = self.inner.lock().unwrap();
        let entry = conn
            .query_row(
                "SELECT v.id, v.name, v.owner_user_id, v.ciphertext_b64, v.etag,
                        v.created_at, v.updated_at,
                        m.role, m.wrapped_vault_key_b64
                 FROM vaults v
                 INNER JOIN vault_members m ON m.vault_id = v.id
                 WHERE m.user_id = ?1 AND v.id = ?2",
                params![user_id, vault_id],
                |row| {
                    let role_str: String = row.get(7)?;
                    Ok(VaultListEntry {
                        vault: Vault {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            owner_user_id: row.get(2)?,
                            ciphertext_b64: row.get(3)?,
                            etag: row.get(4)?,
                            created_at: row.get(5)?,
                            updated_at: row.get(6)?,
                        },
                        role: VaultRole::parse(&role_str).unwrap_or(VaultRole::Reader),
                        wrapped_vault_key_b64: row.get(8)?,
                    })
                },
            )
            .optional()?;
        Ok(entry)
    }

    /// CAS-update vault ciphertext. Returns the new etag + updated_at.
    /// `expected_etag = None` means "create-only" semantics — used by the
    /// initial vault upload during signup so a re-run of signup can't
    /// silently clobber an existing vault on a duplicate-email retry.
    pub fn update_vault_ciphertext(
        &self,
        user_id: &str,
        vault_id: &str,
        new_ciphertext_b64: &str,
        new_etag: &str,
        expected_etag: Option<&str>,
    ) -> ApiResult<Vault> {
        let mut conn = self.inner.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        // Permission check: must be a member with write capability.
        let role_str: Option<String> = tx
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![vault_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        let role = role_str
            .as_deref()
            .and_then(VaultRole::parse)
            .ok_or(ApiError::NotFound)?;
        if !role.can_write() {
            return Err(ApiError::Unauthorized);
        }

        let current: (String, String) = tx.query_row(
            "SELECT ciphertext_b64, etag FROM vaults WHERE id = ?1",
            params![vault_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if let Some(expected) = expected_etag {
            if current.1 != expected {
                return Err(ApiError::VaultConflict);
            }
        }

        let now = now_secs();
        tx.execute(
            "UPDATE vaults SET ciphertext_b64 = ?1, etag = ?2, updated_at = ?3 WHERE id = ?4",
            params![new_ciphertext_b64, new_etag, now, vault_id],
        )?;

        let updated: Vault = tx.query_row(
            "SELECT id, name, owner_user_id, ciphertext_b64, etag, created_at, updated_at
             FROM vaults WHERE id = ?1",
            params![vault_id],
            |row| {
                Ok(Vault {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner_user_id: row.get(2)?,
                    ciphertext_b64: row.get(3)?,
                    etag: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?;
        tx.commit()
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok(updated)
    }

    /// Rename a vault — owner-only.
    pub fn rename_vault(
        &self,
        user_id: &str,
        vault_id: &str,
        new_name: &str,
    ) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        let role_str: Option<String> = conn
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![vault_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        let role = role_str
            .as_deref()
            .and_then(VaultRole::parse)
            .ok_or(ApiError::NotFound)?;
        if !role.can_admin() {
            return Err(ApiError::Unauthorized);
        }
        conn.execute(
            "UPDATE vaults SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_name, now_secs(), vault_id],
        )?;
        Ok(())
    }

    /// Delete a vault. Owner-only. CASCADE wipes vault_members and the blob.
    pub fn delete_vault(&self, user_id: &str, vault_id: &str) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        let role_str: Option<String> = conn
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![vault_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        let role = role_str
            .as_deref()
            .and_then(VaultRole::parse)
            .ok_or(ApiError::NotFound)?;
        if !role.can_admin() {
            return Err(ApiError::Unauthorized);
        }
        conn.execute("DELETE FROM vaults WHERE id = ?1", params![vault_id])?;
        Ok(())
    }

    /// Lookup a recipient for a share invite. Returns the bits needed
    /// client-side to wrap a vault_key (id + account_pubkey).
    pub fn find_share_target(&self, email: &str) -> ApiResult<Option<(String, String)>> {
        let conn = self.inner.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT id, account_pubkey_b64 FROM users WHERE email = ?1",
                params![email],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        Ok(row.filter(|(_, pubkey)| !pubkey.is_empty()))
    }

    /// Add a membership row. Permission check: the caller must already own
    /// the vault. We enforce that here instead of in the handler so an
    /// editor (or a hypothetical bug) can't accidentally invite someone.
    pub fn share_vault(
        &self,
        caller_user_id: &str,
        membership: &VaultMembership,
    ) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        // Caller must be a member with admin rights on this vault.
        let role_str: Option<String> = conn
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![membership.vault_id, caller_user_id],
                |row| row.get(0),
            )
            .optional()?;
        let caller_role = role_str
            .as_deref()
            .and_then(VaultRole::parse)
            .ok_or(ApiError::NotFound)?;
        if !caller_role.can_admin() {
            return Err(ApiError::Unauthorized);
        }
        // Recipient already a member? Update their wrapped key + role
        // instead of failing. Re-sharing a vault should be idempotent —
        // a new invite supersedes the old one (e.g. after recipient
        // re-keyed).
        let n = conn.execute(
            "INSERT INTO vault_members (vault_id, user_id, role, wrapped_vault_key_b64, invited_at, accepted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(vault_id, user_id) DO UPDATE SET
                role = excluded.role,
                wrapped_vault_key_b64 = excluded.wrapped_vault_key_b64,
                invited_at = excluded.invited_at,
                accepted_at = excluded.accepted_at",
            params![
                membership.vault_id,
                membership.user_id,
                membership.role.as_str(),
                membership.wrapped_vault_key_b64,
                membership.invited_at,
                membership.accepted_at
            ],
        )?;
        if n == 0 {
            return Err(ApiError::Internal("share insert produced 0 rows".into()));
        }
        Ok(())
    }

    /// List every member of a vault with their role + email. Caller must
    /// be a member; non-members get NotFound (we don't leak vault
    /// existence by returning Unauthorized).
    pub fn list_vault_members(
        &self,
        caller_user_id: &str,
        vault_id: &str,
    ) -> ApiResult<Vec<MemberInfo>> {
        let conn = self.inner.lock().unwrap();
        // Membership gate.
        let caller_member: Option<String> = conn
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![vault_id, caller_user_id],
                |row| row.get(0),
            )
            .optional()?;
        if caller_member.is_none() {
            return Err(ApiError::NotFound);
        }

        let mut stmt = conn.prepare(
            "SELECT m.user_id, u.email, m.role, m.invited_at, m.accepted_at
             FROM vault_members m
             INNER JOIN users u ON u.id = m.user_id
             WHERE m.vault_id = ?1
             ORDER BY CASE m.role WHEN 'owner' THEN 0 WHEN 'editor' THEN 1 ELSE 2 END,
                      m.invited_at",
        )?;
        let rows = stmt
            .query_map(params![vault_id], |row| {
                let role_str: String = row.get(2)?;
                Ok(MemberInfo {
                    user_id: row.get(0)?,
                    email: row.get(1)?,
                    role: VaultRole::parse(&role_str).unwrap_or(VaultRole::Reader),
                    invited_at: row.get(3)?,
                    accepted_at: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Change a member's role. Owner-only. Refuses to demote the last
    /// owner (would orphan the vault) and to operate on non-members.
    pub fn update_member_role(
        &self,
        caller_user_id: &str,
        vault_id: &str,
        target_user_id: &str,
        new_role: VaultRole,
    ) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        let caller_role: Option<String> = conn
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![vault_id, caller_user_id],
                |row| row.get(0),
            )
            .optional()?;
        let caller_role = caller_role
            .as_deref()
            .and_then(VaultRole::parse)
            .ok_or(ApiError::NotFound)?;
        if !caller_role.can_admin() {
            return Err(ApiError::Unauthorized);
        }

        let target_role: Option<String> = conn
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![vault_id, target_user_id],
                |row| row.get(0),
            )
            .optional()?;
        let target_role = target_role
            .as_deref()
            .and_then(VaultRole::parse)
            .ok_or(ApiError::NotFound)?;

        // Demoting the last owner orphans the vault — refuse.
        if matches!(target_role, VaultRole::Owner) && !matches!(new_role, VaultRole::Owner) {
            let owner_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM vault_members WHERE vault_id = ?1 AND role = 'owner'",
                params![vault_id],
                |row| row.get(0),
            )?;
            if owner_count <= 1 {
                return Err(ApiError::BadRequest(
                    "can't demote the last owner; transfer ownership first".into(),
                ));
            }
        }

        conn.execute(
            "UPDATE vault_members SET role = ?1 WHERE vault_id = ?2 AND user_id = ?3",
            params![new_role.as_str(), vault_id, target_user_id],
        )?;
        Ok(())
    }

    /// Drop a member's access to a vault. Owners can't remove themselves —
    /// they'd orphan the vault. Use delete_vault for that.
    pub fn unshare_vault(
        &self,
        caller_user_id: &str,
        vault_id: &str,
        target_user_id: &str,
    ) -> ApiResult<()> {
        let conn = self.inner.lock().unwrap();
        let caller_role: Option<String> = conn
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![vault_id, caller_user_id],
                |row| row.get(0),
            )
            .optional()?;
        let caller_role = caller_role
            .as_deref()
            .and_then(VaultRole::parse)
            .ok_or(ApiError::NotFound)?;
        // Either you're admin OR you're removing yourself.
        if !caller_role.can_admin() && caller_user_id != target_user_id {
            return Err(ApiError::Unauthorized);
        }
        // Refuse to remove the last owner.
        let target_role: Option<String> = conn
            .query_row(
                "SELECT role FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
                params![vault_id, target_user_id],
                |row| row.get(0),
            )
            .optional()?;
        if matches!(target_role.as_deref().and_then(VaultRole::parse), Some(VaultRole::Owner)) {
            return Err(ApiError::BadRequest(
                "cannot remove the vault owner; delete the vault instead".into(),
            ));
        }
        conn.execute(
            "DELETE FROM vault_members WHERE vault_id = ?1 AND user_id = ?2",
            params![vault_id, target_user_id],
        )?;
        Ok(())
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

    fn fresh() -> Storage {
        Storage::open_in_memory().unwrap()
    }

    fn mk_user(s: &Storage, id: &str, email: &str) {
        s.create_user(id, email, "h", "salt", "wmk-wmk-wmk-wmk-wmk", "rec", "", "")
            .unwrap();
    }

    #[test]
    fn create_and_find_user_round_trips() {
        let s = fresh();
        mk_user(&s, "uid-1", "a@b.test");
        let u = s.find_user_by_email("a@b.test").unwrap().unwrap();
        assert_eq!(u.id, "uid-1");
    }

    #[test]
    fn duplicate_email_rejected() {
        let s = fresh();
        mk_user(&s, "uid-1", "a@b.test");
        let err = s
            .create_user("uid-2", "a@b.test", "h", "s", "wmk", "rec", "", "")
            .unwrap_err();
        assert!(matches!(err, ApiError::UserExists));
    }

    #[test]
    fn create_vault_inserts_owner_membership() {
        let s = fresh();
        mk_user(&s, "u", "e@x");
        let v = s
            .create_vault("u", "Personal", "ct", "etag-1", "wvk")
            .unwrap();
        assert_eq!(v.name, "Personal");

        let list = s.list_vaults_for_user("u").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].vault.id, v.id);
        assert_eq!(list[0].role, VaultRole::Owner);
        assert_eq!(list[0].wrapped_vault_key_b64, "wvk");
    }

    #[test]
    fn list_vaults_returns_only_caller_memberships() {
        let s = fresh();
        mk_user(&s, "alice", "a@x");
        mk_user(&s, "bob", "b@x");
        s.create_vault("alice", "Alice's", "ct", "e", "wvk").unwrap();
        let bob = s.list_vaults_for_user("bob").unwrap();
        assert!(bob.is_empty());
        let alice = s.list_vaults_for_user("alice").unwrap();
        assert_eq!(alice.len(), 1);
    }

    #[test]
    fn update_vault_requires_membership_and_etag() {
        let s = fresh();
        mk_user(&s, "u", "e@x");
        let v = s.create_vault("u", "v", "ct1", "e1", "wvk").unwrap();
        let updated = s
            .update_vault_ciphertext("u", &v.id, "ct2", "e2", Some("e1"))
            .unwrap();
        assert_eq!(updated.etag, "e2");

        // Stale etag — conflict.
        let err = s
            .update_vault_ciphertext("u", &v.id, "ct3", "e3", Some("e1"))
            .unwrap_err();
        assert!(matches!(err, ApiError::VaultConflict));

        // Non-member — not found, never reaches conflict.
        mk_user(&s, "stranger", "s@x");
        let err = s
            .update_vault_ciphertext("stranger", &v.id, "ct4", "e4", Some("e2"))
            .unwrap_err();
        assert!(matches!(err, ApiError::NotFound));
    }

    #[test]
    fn reader_cannot_write() {
        let s = fresh();
        mk_user(&s, "owner", "o@x");
        mk_user(&s, "viewer", "v@x");
        let v = s.create_vault("owner", "v", "ct", "e1", "wvk").unwrap();
        s.share_vault(
            "owner",
            &VaultMembership {
                vault_id: v.id.clone(),
                user_id: "viewer".into(),
                role: VaultRole::Reader,
                wrapped_vault_key_b64: "wvk-for-viewer".into(),
                invited_at: 0,
                accepted_at: Some(0),
            },
        )
        .unwrap();
        let err = s
            .update_vault_ciphertext("viewer", &v.id, "ct2", "e2", Some("e1"))
            .unwrap_err();
        assert!(matches!(err, ApiError::Unauthorized));
    }

    #[test]
    fn only_owner_can_rename_or_delete() {
        let s = fresh();
        mk_user(&s, "owner", "o@x");
        mk_user(&s, "editor", "e@x");
        let v = s.create_vault("owner", "Old", "ct", "e1", "wvk").unwrap();
        s.share_vault(
            "owner",
            &VaultMembership {
                vault_id: v.id.clone(),
                user_id: "editor".into(),
                role: VaultRole::Editor,
                wrapped_vault_key_b64: "wvk-e".into(),
                invited_at: 0,
                accepted_at: Some(0),
            },
        )
        .unwrap();
        assert!(matches!(
            s.rename_vault("editor", &v.id, "New").unwrap_err(),
            ApiError::Unauthorized
        ));
        s.rename_vault("owner", &v.id, "New").unwrap();
        assert!(matches!(
            s.delete_vault("editor", &v.id).unwrap_err(),
            ApiError::Unauthorized
        ));
        s.delete_vault("owner", &v.id).unwrap();
        assert!(s.list_vaults_for_user("owner").unwrap().is_empty());
        assert!(s.list_vaults_for_user("editor").unwrap().is_empty());
    }
}
