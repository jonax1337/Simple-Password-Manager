//! Runtime state for one linked cloud account.
//!
//! `master_key` lives for the whole session — it's what every vault_key
//! gets wrapped against. `active_vault` is set whenever the user has a
//! specific vault opened (the picker UI sets it, and push/pull use it).
//! Switching vaults swaps `active_vault`; logout drops both.
//!
//! Nothing here is persisted to disk by default. The cloud_persistence
//! module gates that behind the user's explicit "Remember me".

use super::crypto::SecretKey;

pub struct CloudSession {
    pub server_url: String,
    pub email: String,
    pub user_id: String,
    pub token: String,
    pub master_key: SecretKey,
    /// Curve25519 keypair. Populated for sharing-capable accounts (any
    /// signup since Stage C). Used to unseal shared `wrapped_vault_key`
    /// blobs received via /vaults/:id/share.
    pub account_keypair: Option<super::crypto::AccountKeypair>,
    pub active_vault: Option<ActiveVault>,
}

pub struct ActiveVault {
    pub id: String,
    pub name: String,
    pub vault_key: SecretKey,
    /// Last etag we observed on the server. Sent as `expected_etag` on the
    /// next PUT to detect concurrent writes from another device.
    pub last_known_etag: Option<String>,
}

impl CloudSession {
    pub fn new(
        server_url: String,
        email: String,
        user_id: String,
        token: String,
        master_key: SecretKey,
    ) -> Self {
        Self {
            server_url,
            email,
            user_id,
            token,
            master_key,
            account_keypair: None,
            active_vault: None,
        }
    }
}
