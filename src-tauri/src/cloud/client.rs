//! Typed HTTP client mirroring the server's REST surface. All wire types live
//! here so the server can evolve them in tandem (mirror copies; not a shared
//! crate yet — that's a later refactor if the server moves out of-repo).

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

#[derive(Debug, Serialize)]
struct SignupReq<'a> {
    email: &'a str,
    kdf_salt_b64: &'a str,
    client_auth_hash: &'a str,
    initial_vault_blob_b64: &'a str,
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
}

#[derive(Debug, Serialize)]
struct EmailOnlyReq<'a> {
    email: &'a str,
    // Server's kdf_params handler reuses LoginReq shape; we send a dummy auth
    // hash that gets ignored. Keeps the server's deserialization happy.
    client_auth_hash: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct KdfParamsResp {
    pub kdf_salt_b64: String,
}

#[derive(Debug, Deserialize)]
pub struct GetVaultResp {
    pub etag: String,
    pub ciphertext_b64: String,
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

    pub async fn signup(
        &self,
        email: &str,
        kdf_salt_b64: &str,
        client_auth_hash: &str,
        initial_vault_blob_b64: &str,
    ) -> Result<SignupResp, CloudError> {
        let res = self
            .http
            .post(format!("{}/auth/signup", self.base_url))
            .json(&SignupReq {
                email,
                kdf_salt_b64,
                client_auth_hash,
                initial_vault_blob_b64,
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

    pub async fn get_vault(&self, token: &str) -> Result<GetVaultResp, CloudError> {
        let res = self
            .http
            .get(format!("{}/vault", self.base_url))
            .bearer_auth(token)
            .send()
            .await?;
        self.parse_json(res).await
    }

    pub async fn put_vault(
        &self,
        token: &str,
        ciphertext_b64: &str,
        expected_etag: Option<&str>,
    ) -> Result<PutVaultResp, CloudError> {
        let res = self
            .http
            .put(format!("{}/vault", self.base_url))
            .bearer_auth(token)
            .json(&PutVaultReq {
                ciphertext_b64,
                expected_etag,
            })
            .send()
            .await?;
        self.parse_json(res).await
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
        match status {
            StatusCode::UNAUTHORIZED => Err(CloudError::Unauthorized),
            StatusCode::CONFLICT => Err(CloudError::Conflict),
            StatusCode::NOT_FOUND => Err(CloudError::NotFound),
            _ => Err(CloudError::Http {
                status: status.as_u16(),
                body,
            }),
        }
    }
}
