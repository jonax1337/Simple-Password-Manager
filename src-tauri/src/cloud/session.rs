//! Runtime state for one linked cloud account.
//!
//! All secrets here are kept in RAM only — losing them means re-typing the
//! master password on the next push/pull, not data loss. We deliberately
//! don't persist `vault_key` or `token` to disk: that would re-create the
//! attack surface E2E was meant to remove.

use super::crypto::SecretKey;

pub struct CloudSession {
    pub server_url: String,
    pub email: String,
    pub user_id: String,
    pub token: String,
    pub vault_key: SecretKey,
    /// Last etag we observed on the server. Sent as `expected_etag` on the
    /// next PUT to detect concurrent writes from another device. `None`
    /// until the first successful GET or PUT.
    pub last_known_etag: Option<String>,
}

impl CloudSession {
    pub fn new(
        server_url: String,
        email: String,
        user_id: String,
        token: String,
        vault_key: SecretKey,
        last_known_etag: Option<String>,
    ) -> Self {
        Self {
            server_url,
            email,
            user_id,
            token,
            vault_key,
            last_known_etag,
        }
    }
}
