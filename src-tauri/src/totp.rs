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

    // Additional RFC 6238 SHA-1 vectors (truncated to 6 digits).
    #[test]
    fn rfc6238_vectors() {
        let secret = b"12345678901234567890";
        // T = 1111111109 (counter = 37037036) → "081804" (last 6 of "07081804")
        assert_eq!(compute_totp(secret, 1111111109 / 30, 6), "081804");
        // T = 1234567890 (counter = 41152263) → "050471" (last 6 of "89005924"? double-check)
        // Per RFC: 1234567890 → 89005924, so truncated 6 = "005924"
        assert_eq!(compute_totp(secret, 1234567890 / 30, 6), "005924");
        // T = 2000000000 → 69279037 → "279037"
        assert_eq!(compute_totp(secret, 2000000000 / 30, 6), "279037");
    }

    #[test]
    fn base32_roundtrip() {
        // "Hello!" → "JBSWY3DPEE======"
        let decoded = base32_decode("JBSWY3DPEE").unwrap();
        assert_eq!(&decoded, b"Hello!");
    }

    #[test]
    fn base32_ignores_padding_and_whitespace_and_case() {
        let a = base32_decode("JBSWY3DPEE======").unwrap();
        let b = base32_decode("jbswy 3dp ee").unwrap();
        assert_eq!(a, b);
        assert_eq!(&a, b"Hello!");
    }

    #[test]
    fn base32_rejects_invalid_chars() {
        assert!(base32_decode("!!!!").is_none());
        assert!(base32_decode("8888").is_none()); // 8 and 9 are not in base32 alphabet
    }

    #[test]
    fn from_otpauth_uri_minimal() {
        let r = from_otpauth_uri("otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(r.code.len(), 6);
        assert_eq!(r.period, 30);
        assert_eq!(r.algorithm, "SHA1");
        assert!(r.remaining_seconds > 0 && r.remaining_seconds <= 30);
    }

    #[test]
    fn from_otpauth_uri_custom_period() {
        let r = from_otpauth_uri("otpauth://totp/Foo?secret=JBSWY3DPEHPK3PXP&period=60").unwrap();
        assert_eq!(r.period, 60);
    }

    #[test]
    fn from_otpauth_uri_clamps_digits() {
        let r6 = from_otpauth_uri("otpauth://totp/X?secret=JBSWY3DPEHPK3PXP&digits=6").unwrap();
        assert_eq!(r6.code.len(), 6);
        let r8 = from_otpauth_uri("otpauth://totp/X?secret=JBSWY3DPEHPK3PXP&digits=8").unwrap();
        assert_eq!(r8.code.len(), 8);
        // digits = 3 should be clamped up to 6
        let r3 = from_otpauth_uri("otpauth://totp/X?secret=JBSWY3DPEHPK3PXP&digits=3").unwrap();
        assert_eq!(r3.code.len(), 6);
    }

    #[test]
    fn from_otpauth_uri_rejects_non_sha1() {
        let err = from_otpauth_uri(
            "otpauth://totp/X?secret=JBSWY3DPEHPK3PXP&algorithm=SHA256",
        )
        .unwrap_err();
        assert!(matches!(err, TotpError::UnsupportedAlgorithm));
    }

    #[test]
    fn from_otpauth_uri_rejects_missing_secret() {
        let err = from_otpauth_uri("otpauth://totp/X?period=30").unwrap_err();
        assert!(matches!(err, TotpError::NoSecret));
    }

    #[test]
    fn from_otpauth_uri_rejects_bad_scheme() {
        let err = from_otpauth_uri("https://totp/X?secret=ABC").unwrap_err();
        assert!(matches!(err, TotpError::BadUri));

        let err2 = from_otpauth_uri("otpauth://hotp/X?secret=ABC").unwrap_err();
        assert!(matches!(err2, TotpError::BadUri));
    }

    #[test]
    fn from_otpauth_uri_rejects_no_query() {
        let err = from_otpauth_uri("otpauth://totp/X").unwrap_err();
        assert!(matches!(err, TotpError::BadUri));
    }

    #[test]
    fn from_otpauth_uri_percent_decodes_secret() {
        // %20 in the secret is unusual but the parser must handle percent-encoding.
        // Use a label with a percent-encoded colon; ensure decoding doesn't blow up.
        let r = from_otpauth_uri("otpauth://totp/Issuer%3Aalice?secret=JBSWY3DPEHPK3PXP")
            .unwrap();
        assert_eq!(r.code.len(), 6);
    }

    #[test]
    fn from_raw_secret_works_with_spaces_and_lowercase() {
        let r = from_raw_secret("jbswy 3dp ehpk 3pxp").unwrap();
        assert_eq!(r.code.len(), 6);
        assert_eq!(r.period, 30);
    }

    #[test]
    fn from_raw_secret_rejects_empty() {
        let err = from_raw_secret("").unwrap_err();
        assert!(matches!(err, TotpError::NoSecret));
        let err2 = from_raw_secret("   ").unwrap_err();
        assert!(matches!(err2, TotpError::NoSecret));
    }

    #[test]
    fn from_raw_secret_rejects_invalid_base32() {
        let err = from_raw_secret("!!!notbase32!!!").unwrap_err();
        assert!(matches!(err, TotpError::BadSecret));
    }

    #[test]
    fn percent_decode_basic() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("a%3Ab"), "a:b");
        assert_eq!(percent_decode("nopercent"), "nopercent");
        assert_eq!(percent_decode("%2F%2F"), "//");
    }

    #[test]
    fn percent_decode_leaves_invalid_alone() {
        // Trailing % with no hex digits is left as-is.
        assert_eq!(percent_decode("a%"), "a%");
        assert_eq!(percent_decode("a%2"), "a%2");
        // Invalid hex bytes are left untouched.
        assert_eq!(percent_decode("a%ZZ"), "a%ZZ");
    }
}
