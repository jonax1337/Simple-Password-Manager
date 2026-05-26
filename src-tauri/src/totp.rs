//! Minimal RFC 6238 TOTP implementation.
//!
//! Reads a KeePass entry's `otp` field (or any field whose value starts with
//! `otpauth://`) and produces a fresh 6-digit code plus a countdown.
//!
//! Supports only HMAC-SHA1 because that's all KeePassXC writes and all that
//! any real-world site we've seen needs. Extending to SHA256/SHA512 would be
//! a one-line `match` here plus a dep on `sha2`.

use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct TotpResult {
    pub code: String,
    pub period: u64,
    pub remaining_seconds: u64,
    pub algorithm: &'static str,
}

#[derive(Debug)]
pub enum TotpError {
    NoSecret,
    BadUri,
    UnsupportedAlgorithm,
    BadSecret,
}

pub fn from_otpauth_uri(uri: &str) -> Result<TotpResult, TotpError> {
    if !uri.starts_with("otpauth://totp/") {
        return Err(TotpError::BadUri);
    }
    let query_start = uri.find('?').ok_or(TotpError::BadUri)?;
    let query = &uri[query_start + 1..];

    let mut secret: Option<String> = None;
    let mut period: u64 = 30;
    let mut digits: u32 = 6;
    let mut algorithm: &'static str = "SHA1";

    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let key = it.next().unwrap_or("");
        let value = it.next().unwrap_or("");
        let value = percent_decode(value);
        match key.to_ascii_lowercase().as_str() {
            "secret" => secret = Some(value),
            "period" => {
                if let Ok(v) = value.parse::<u64>() {
                    period = v.max(1);
                }
            }
            "digits" => {
                if let Ok(v) = value.parse::<u32>() {
                    digits = v.clamp(6, 10);
                }
            }
            "algorithm" => {
                let upper = value.to_ascii_uppercase();
                if upper != "SHA1" {
                    return Err(TotpError::UnsupportedAlgorithm);
                }
                algorithm = "SHA1";
            }
            _ => {}
        }
    }

    let secret = secret.ok_or(TotpError::NoSecret)?;
    let secret_bytes = base32_decode(&secret).ok_or(TotpError::BadSecret)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let counter = now / period;

    let code = compute_totp(&secret_bytes, counter, digits);
    let remaining = period - (now % period);

    Ok(TotpResult {
        code,
        period,
        remaining_seconds: remaining,
        algorithm,
    })
}

/// If the field value looks like a raw base32 secret (legacy KeePass format),
/// build an otpauth URI on the fly and delegate.
pub fn from_raw_secret(secret: &str) -> Result<TotpResult, TotpError> {
    let cleaned = secret.replace(' ', "").to_ascii_uppercase();
    if cleaned.is_empty() {
        return Err(TotpError::NoSecret);
    }
    let secret_bytes = base32_decode(&cleaned).ok_or(TotpError::BadSecret)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let counter = now / 30;
    let code = compute_totp(&secret_bytes, counter, 6);
    Ok(TotpResult {
        code,
        period: 30,
        remaining_seconds: 30 - (now % 30),
        algorithm: "SHA1",
    })
}

fn compute_totp(secret: &[u8], counter: u64, digits: u32) -> String {
    type HmacSha1 = Hmac<Sha1>;
    let mut mac =
        HmacSha1::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&counter.to_be_bytes());
    let hash = mac.finalize().into_bytes();

    let offset = (hash[hash.len() - 1] & 0x0f) as usize;
    let binary = (u32::from(hash[offset] & 0x7f) << 24)
        | (u32::from(hash[offset + 1]) << 16)
        | (u32::from(hash[offset + 2]) << 8)
        | u32::from(hash[offset + 3]);

    let modulus = 10u32.pow(digits);
    format!("{:01$}", binary % modulus, digits as usize)
}

// ------------- Base32 decoder (RFC 4648, no padding required) -------------

fn base32_decode(input: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits: u64 = 0;
    let mut acc_bits: u32 = 0;
    let mut out = Vec::with_capacity(input.len() * 5 / 8);

    for c in input.chars() {
        if c == '=' || c.is_whitespace() {
            continue;
        }
        let c = c.to_ascii_uppercase() as u8;
        let value = ALPHABET.iter().position(|&b| b == c)? as u64;
        bits = (bits << 5) | value;
        acc_bits += 5;
        if acc_bits >= 8 {
            acc_bits -= 8;
            out.push(((bits >> acc_bits) & 0xff) as u8);
            bits &= (1u64 << acc_bits) - 1;
        }
    }
    Some(out)
}

// ------------- minimal percent-decoder for otpauth query values -------------

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) =
                (hex(bytes[i + 1]), hex(bytes[i + 2]))
            {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + b - b'a'),
        b'A'..=b'F' => Some(10 + b - b'A'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 test vector: secret "12345678901234567890" → at T=59 → code "94287082"
    // Secret in base32 of ASCII "12345678901234567890" is
    // "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".
    #[test]
    fn rfc6238_basic() {
        let secret = b"12345678901234567890";
        let code = compute_totp(secret, 59 / 30, 6);
        assert_eq!(code, "287082");
    }

    #[test]
    fn base32_roundtrip() {
        // "Hello!" → "JBSWY3DPEE======"
        let decoded = base32_decode("JBSWY3DPEE").unwrap();
        assert_eq!(&decoded, b"Hello!");
    }
}
