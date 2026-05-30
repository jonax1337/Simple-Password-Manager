//! Client-side crypto primitives for cloud sync.
//!
//! Derivation chain:
//!     master_key = Argon2id(master_password, kdf_salt, m=64MB, t=3, p=4)  → 32B
//!     client_auth_hash = SHA-256(master_key || "auth")                    → 32B
//!     vault_key        = SHA-256(master_key || "vault")                   → 32B
//!
//! Vault sealing: AES-256-GCM with a fresh 12-byte nonce per encryption,
//! packed as `nonce || ciphertext || tag` (the tag is included in the
//! ciphertext by the aes-gcm crate). Base64-encoded for transport.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng as AeadOsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use argon2_pure::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::RngCore;
use ::sha2::{Digest, Sha256};
use zeroize::Zeroize;

const KDF_MEMORY_KIB: u32 = 64 * 1024; // 64 MiB
const KDF_ITERATIONS: u32 = 3;
const KDF_PARALLELISM: u32 = 4;
const KDF_OUTPUT_LEN: usize = 32;

/// A 32-byte secret. Zeroed on drop so derived keys don't linger in heap
/// pages after the session ends.
#[derive(Clone)]
pub struct SecretKey([u8; 32]);

impl SecretKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Reconstruct a key from raw bytes — used when re-hydrating a session
    /// from cached vault-key material (e.g. between push and pull on the
    /// same session). The caller is responsible for the bytes being a
    /// legitimately derived key.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("KDF failed: {0}")]
    Kdf(String),
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed (wrong key or tampered blob)")]
    Decrypt,
    #[error("invalid base64: {0}")]
    Base64(String),
    #[error("blob too short: expected nonce + ciphertext")]
    BlobTooShort,
}

/// Generate a fresh 16-byte salt for first-time signup. Public — gets stored
/// on the server so future logins from new devices can reproduce the KDF.
pub fn new_kdf_salt() -> Vec<u8> {
    let mut salt = vec![0u8; 16];
    rand::rng().fill_bytes(&mut salt);
    salt
}

pub fn b64_encode(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

pub fn b64_decode(s: &str) -> Result<Vec<u8>, CryptoError> {
    B64.decode(s).map_err(|e| CryptoError::Base64(e.to_string()))
}

/// Run the slow first-stage KDF. This is the only place the master password
/// is consumed.
/// Derive the password-wrapping key. This is the only place the master
/// password is consumed; the result is used to AES-GCM-wrap/unwrap the
/// (random) master_key.
pub fn derive_password_key(password: &str, salt: &[u8]) -> Result<SecretKey, CryptoError> {
    let params = Params::new(
        KDF_MEMORY_KIB,
        KDF_ITERATIONS,
        KDF_PARALLELISM,
        Some(KDF_OUTPUT_LEN),
    )
    .map_err(|e| CryptoError::Kdf(e.to_string()))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|e| CryptoError::Kdf(e.to_string()))?;
    Ok(SecretKey(out))
}

/// Generate the canonical 32-byte master_key at signup. Random; never
/// derived from a password. Persistence: wrapped versions in cloud, never
/// stored plaintext.
pub fn generate_master_key() -> SecretKey {
    let mut out = [0u8; 32];
    rand::rng().fill_bytes(&mut out);
    SecretKey(out)
}

/// Encrypt a SecretKey (master_key) under another SecretKey (password_key or
/// recovery_key). Output layout matches `seal_vault`: 12-byte nonce ||
/// ciphertext-with-tag.
pub fn wrap_key(wrap_key: &SecretKey, key_to_wrap: &SecretKey) -> Result<Vec<u8>, CryptoError> {
    seal_vault(wrap_key, key_to_wrap.as_bytes())
}

/// Inverse of `wrap_key`. Returns a fresh SecretKey holding the 32 plaintext
/// bytes, with the usual zeroize-on-drop semantics.
pub fn unwrap_key(wrap_key: &SecretKey, wrapped: &[u8]) -> Result<SecretKey, CryptoError> {
    let plaintext = open_vault(wrap_key, wrapped)?;
    if plaintext.len() != 32 {
        return Err(CryptoError::Decrypt);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&plaintext);
    Ok(SecretKey(out))
}

/// Cheap hash-based subkey derivation. We don't need HKDF here — the master
/// key is already uniform random from Argon2.
fn derive_subkey(master: &SecretKey, label: &[u8]) -> SecretKey {
    let mut hasher = Sha256::new();
    hasher.update(master.as_bytes());
    hasher.update(label);
    let out: [u8; 32] = hasher.finalize().into();
    SecretKey(out)
}

pub fn derive_auth_hash(master: &SecretKey) -> SecretKey {
    derive_subkey(master, b"auth")
}

pub fn derive_vault_key(master: &SecretKey) -> SecretKey {
    derive_subkey(master, b"vault")
}

/// Encrypt `plaintext` with `vault_key`. The returned blob is
/// `nonce || ciphertext_with_tag`, ready to base64 and ship.
pub fn seal_vault(vault_key: &SecretKey, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let key = Key::<Aes256Gcm>::from_slice(vault_key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut AeadOsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| CryptoError::Encrypt)?;
    let mut out = Vec::with_capacity(nonce.len() + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Generate a 20-character recovery code: 16 random bytes encoded as
/// Crockford-friendly base32 with dashes every 4 chars
/// (`XXXX-XXXX-XXXX-XXXX-XXXX`). 80 bits of entropy — enough to resist
/// offline brute-force when also gated by Argon2id derivation.
pub fn generate_recovery_code() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    // Crockford base32 alphabet (no 0/O/I/L confusion). Manual encoding
    // because we only have base64 in deps and we want the human-readable
    // alphabet.
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut out = String::with_capacity(20 + 4); // 5 groups of 4 + 4 dashes
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    let mut emitted = 0;
    for b in bytes.iter().copied() {
        buffer = (buffer << 8) | (b as u32);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((buffer >> bits) & 0x1F) as usize;
            out.push(ALPHABET[idx] as char);
            emitted += 1;
            if emitted % 4 == 0 && emitted < 20 {
                out.push('-');
            }
            if emitted == 20 {
                return out;
            }
        }
    }
    out
}

/// Derive the wrapping key from a recovery code. Same Argon2id parameters
/// as the password path, with a deterministic salt = SHA256(email) so the
/// client can reproduce it later without storing anything alongside the
/// printed code.
pub fn derive_recovery_key(code: &str, email: &str) -> Result<SecretKey, CryptoError> {
    // Normalize: strip dashes + whitespace, uppercase — humans transcribe
    // codes inconsistently and "Aa-Bb cc" should map to the same key as
    // "AABBCC".
    let normalized: String = code
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if normalized.is_empty() {
        return Err(CryptoError::Kdf("empty recovery code".into()));
    }
    let salt = {
        let mut h = Sha256::new();
        h.update(b"recovery:");
        h.update(email.as_bytes());
        h.finalize()
    };
    derive_password_key(&normalized, &salt[..16])
}

pub fn open_vault(vault_key: &SecretKey, blob: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if blob.len() < 12 + 16 {
        return Err(CryptoError::BlobTooShort);
    }
    let (nonce_bytes, ct) = blob.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(vault_key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ct)
        .map_err(|_| CryptoError::Decrypt)
}

