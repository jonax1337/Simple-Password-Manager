//! Typed HTTP client mirroring the multi-vault server's REST surface.

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CloudError {
    #[error("network: {0}")]
    Network(String),
    #[error("server returned {status}: {body}")]
    Http { status: u16, body: String },
    #[error("unauthorized (wrong credentials or expired token)")]
    Unauthorized,
    #[error("vault conflict — remote has been updated; pull first")]
    Conflict,
    #[error("not found")]
    NotFound,
}

impl From<reqwest::Error> for CloudError {
    fn from(e: reqwest::Error) -> Self {
        CloudError::Network(e.to_string())
    }
}

// ----- auth -----

#[derive(Debug, Serialize)]
struct SignupReq<'a> {
    email: &'a str,
    kdf_salt_b64: &'a str,
    client_auth_hash: &'a str,
    wrapped_master_key_b64: &'a str,
    recovery_blob_b64: &'a str,
    account_pubkey_b64: &'a str,
    wrapped_account_privkey_b64: &'a str,
    initial_vault_name: &'a str,
    initial_vault_blob_b64: &'a str,
    initial_wrapped_vault_key_b64: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct SignupResp {
    pub token: String,
    pub user_id: String,
}

#[derive(Debug, Serialize)]
struct LoginReq<'a> {
    email: &'a str,
    client_auth_hash: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct LoginResp {
    pub token: String,
    pub user_id: String,
    pub kdf_salt_b64: String,
    pub wrapped_master_key_b64: String,
    #[serde(default)]
    pub wrapped_account_privkey_b64: String,
    #[serde(default)]
    pub account_pubkey_b64: String,
}

#[derive(Debug, Serialize)]
struct EmailOnlyReq<'a> {
    email: &'a str,
    client_auth_hash: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct KdfParamsResp {
    pub kdf_salt_b64: String,
    pub wrapped_master_key_b64: String,
}

#[derive(Debug, Serialize)]
struct RecoveryInitReq<'a> {
    email: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct RecoveryInitResp {
    pub recovery_blob_b64: String,
}

#[derive(Debug, Serialize)]
struct ResetReq<'a> {
    email: &'a str,
    client_auth_hash: &'a str,
    new_kdf_salt_b64: &'a str,
    new_wrapped_master_key_b64: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct ResetResp {
    pub token: String,
}

// ----- vaults -----

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VaultSummary {
    pub id: String,
    pub name: String,
    pub role: String,
    pub owner_user_id: String,
    pub etag: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub wrapped_vault_key_b64: String,
}

#[derive(Debug, Deserialize)]
pub struct ListVaultsResp {
    pub vaults: Vec<VaultSummary>,
}

#[derive(Debug, Deserialize)]
pub struct VaultFull {
    pub id: String,
    pub name: String,
    pub role: String,
    pub owner_user_id: String,
    pub etag: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub wrapped_vault_key_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Debug, Serialize)]
struct CreateVaultReq<'a> {
    name: &'a str,
    ciphertext_b64: &'a str,
    wrapped_vault_key_b64: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct CreateVaultResp {
    pub id: String,
    pub etag: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
struct PutVaultReq<'a> {
    ciphertext_b64: &'a str,
    expected_etag: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct PutVaultResp {
    pub etag: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
struct RenameVaultReq<'a> {
    name: &'a str,
}

#[derive(Clone)]
pub struct CloudClient {
    http: Client,
    base_url: String,
}

impl CloudClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn signup(&self, req: SignupArgs<'_>) -> Result<SignupResp, CloudError> {
        let res = self
            .http
            .post(format!("{}/auth/signup", self.base_url))
            .json(&SignupReq {
                email: req.email,
                kdf_salt_b64: req.kdf_salt_b64,
                client_auth_hash: req.client_auth_hash,
                wrapped_master_key_b64: req.wrapped_master_key_b64,
                recovery_blob_b64: req.recovery_blob_b64,
                account_pubkey_b64: req.account_pubkey_b64,
                wrapped_account_privkey_b64: req.wrapped_account_privkey_b64,
                initial_vault_name: req.initial_vault_name,
                initial_vault_blob_b64: req.initial_vault_blob_b64,
                initial_wrapped_vault_key_b64: req.initial_wrapped_vault_key_b64,
            })
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn login(
        &self,
        email: &str,
        client_auth_hash: &str,
    ) -> Result<LoginResp, CloudError> {
        let res = self
            .http
            .post(format!("{}/auth/login", self.base_url))
            .json(&LoginReq {
                email,
                client_auth_hash,
            })
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn kdf_params(&self, email: &str) -> Result<KdfParamsResp, CloudError> {
        let res = self
            .http
            .post(format!("{}/auth/kdf-params", self.base_url))
            .json(&EmailOnlyReq {
                email,
                client_auth_hash: "x",
            })
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn recovery_init(&self, email: &str) -> Result<RecoveryInitResp, CloudError> {
        let res = self
            .http
            .post(format!("{}/auth/recovery-init", self.base_url))
            .json(&RecoveryInitReq { email })
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn reset_password(
        &self,
        email: &str,
        client_auth_hash: &str,
        new_kdf_salt_b64: &str,
        new_wrapped_master_key_b64: &str,
    ) -> Result<ResetResp, CloudError> {
        let res = self
            .http
            .post(format!("{}/auth/reset", self.base_url))
            .json(&ResetReq {
                email,
                client_auth_hash,
                new_kdf_salt_b64,
                new_wrapped_master_key_b64,
            })
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn list_vaults(&self, token: &str) -> Result<ListVaultsResp, CloudError> {
        let res = self
            .http
            .get(format!("{}/vaults", self.base_url))
            .bearer_auth(token)
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn get_vault(&self, token: &str, vault_id: &str) -> Result<VaultFull, CloudError> {
        let res = self
            .http
            .get(format!("{}/vaults/{}", self.base_url, vault_id))
            .bearer_auth(token)
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn put_vault(
        &self,
        token: &str,
        vault_id: &str,
        ciphertext_b64: &str,
        expected_etag: Option<&str>,
    ) -> Result<PutVaultResp, CloudError> {
        let res = self
            .http
            .put(format!("{}/vaults/{}", self.base_url, vault_id))
            .bearer_auth(token)
            .json(&PutVaultReq {
                ciphertext_b64,
                expected_etag,
            })
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn create_vault(
        &self,
        token: &str,
        name: &str,
        ciphertext_b64: &str,
        wrapped_vault_key_b64: &str,
    ) -> Result<CreateVaultResp, CloudError> {
        let res = self
            .http
            .post(format!("{}/vaults", self.base_url))
            .bearer_auth(token)
            .json(&CreateVaultReq {
                name,
                ciphertext_b64,
                wrapped_vault_key_b64,
            })
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn rename_vault(
        &self,
        token: &str,
        vault_id: &str,
        name: &str,
    ) -> Result<(), CloudError> {
        let res = self
            .http
            .patch(format!("{}/vaults/{}", self.base_url, vault_id))
            .bearer_auth(token)
            .json(&RenameVaultReq { name })
            .send()
            .await?;
        self.parse_unit(res).await
    }

    pub async fn delete_vault(
        &self,
        token: &str,
        vault_id: &str,
    ) -> Result<(), CloudError> {
        let res = self
            .http
            .delete(format!("{}/vaults/{}", self.base_url, vault_id))
            .bearer_auth(token)
            .send()
            .await?;
        self.parse_unit(res).await
    }

    pub async fn user_lookup(
        &self,
        token: &str,
        email: &str,
    ) -> Result<UserLookupResp, CloudError> {
        let res = self
            .http
            .post(format!("{}/users/lookup", self.base_url))
            .bearer_auth(token)
            .json(&EmailOnlyReq {
                email,
                client_auth_hash: "x",
            })
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn share_vault(
        &self,
        token: &str,
        vault_id: &str,
        recipient_user_id: &str,
        wrapped_vault_key_b64: &str,
        role: &str,
    ) -> Result<(), CloudError> {
        let res = self
            .http
            .post(format!("{}/vaults/{}/share", self.base_url, vault_id))
            .bearer_auth(token)
            .json(&ShareReq {
                recipient_user_id,
                wrapped_vault_key_b64,
                role,
            })
            .send()
            .await?;
        self.parse_unit(res).await
    }

    pub async fn unshare_vault(
        &self,
        token: &str,
        vault_id: &str,
        user_id: &str,
    ) -> Result<(), CloudError> {
        let res = self
            .http
            .delete(format!("{}/vaults/{}/share", self.base_url, vault_id))
            .bearer_auth(token)
            .json(&UnshareReq { user_id })
            .send()
            .await?;
        self.parse_unit(res).await
    }

    async fn parse_json<T: for<'de> Deserialize<'de>>(
        &self,
        res: reqwest::Response,
    ) -> Result<T, CloudError> {
        let status = res.status();
        if status.is_success() {
            return Ok(res.json().await?);
        }
        let body = res.text().await.unwrap_or_default();
        Err(self.status_to_err(status, body))
    }

    async fn parse_unit(&self, res: reqwest::Response) -> Result<(), CloudError> {
        let status = res.status();
        if status.is_success() {
            return Ok(());
        }
        let body = res.text().await.unwrap_or_default();
        Err(self.status_to_err(status, body))
    }

    fn status_to_err(&self, status: StatusCode, body: String) -> CloudError {
        match status {
            StatusCode::UNAUTHORIZED => CloudError::Unauthorized,
            StatusCode::CONFLICT => CloudError::Conflict,
            StatusCode::NOT_FOUND => CloudError::NotFound,
            _ => CloudError::Http {
                status: status.as_u16(),
                body,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UserLookupResp {
    pub user_id: String,
    pub account_pubkey_b64: String,
}

#[derive(Debug, Serialize)]
struct ShareReq<'a> {
    recipient_user_id: &'a str,
    wrapped_vault_key_b64: &'a str,
    role: &'a str,
}

#[derive(Debug, Serialize)]
struct UnshareReq<'a> {
    user_id: &'a str,
}

/// Signup needs 10 wire fields. Bundling them into a struct keeps the
/// call site readable and the field-name correspondence with the server
/// obvious.
pub struct SignupArgs<'a> {
    pub email: &'a str,
    pub kdf_salt_b64: &'a str,
    pub client_auth_hash: &'a str,
    pub wrapped_master_key_b64: &'a str,
    pub recovery_blob_b64: &'a str,
    pub account_pubkey_b64: &'a str,
    pub wrapped_account_privkey_b64: &'a str,
    pub initial_vault_name: &'a str,
    pub initial_vault_blob_b64: &'a str,
    pub initial_wrapped_vault_key_b64: &'a str,
}
