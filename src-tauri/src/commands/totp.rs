//! Tauri commands for the in-app TOTP setup UX.
//!
//! These let the entry editor validate a pasted otpauth URI (or raw base32
//! secret), preview the current 6-digit code, and persist the value as a
//! custom field on the entry without the user having to know that's how
//! TOTP is stored under the hood.

use serde::Serialize;

#[derive(Serialize)]
pub struct TotpPreview {
    pub code: String,
    pub period: u64,
    pub remaining_seconds: u64,
    pub algorithm: String,
    /// The canonical otpauth:// URI we'd store. If the user pasted a raw
    /// secret we normalise it into a URI so the field is portable to any
    /// KeePassXC-compatible app.
    pub otpauth_uri: String,
}

#[tauri::command]
pub fn preview_totp(input: String) -> Result<TotpPreview, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("empty".into());
    }

    let (result, uri) = if trimmed.starts_with("otpauth://") {
        (
            simple_password_manager::totp::from_otpauth_uri(trimmed).map_err(totp_error_message)?,
            trimmed.to_string(),
        )
    } else {
        // Treat the input as a raw base32 secret. We also build a synthetic
        // otpauth URI so saving stores a portable value.
        let cleaned: String = trimmed
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_uppercase();
        let result = simple_password_manager::totp::from_raw_secret(&cleaned).map_err(totp_error_message)?;
        let uri = format!(
            "otpauth://totp/Password%20Manager?secret={}&period=30&digits=6&algorithm=SHA1",
            cleaned
        );
        (result, uri)
    };

    Ok(TotpPreview {
        code: result.code,
        period: result.period,
        remaining_seconds: result.remaining_seconds,
        algorithm: result.algorithm.to_string(),
        otpauth_uri: uri,
    })
}

fn totp_error_message(e: simple_password_manager::totp::TotpError) -> String {
    match e {
        simple_password_manager::totp::TotpError::NoSecret => "secret missing".into(),
        simple_password_manager::totp::TotpError::BadUri => "not a valid otpauth:// URI".into(),
        simple_password_manager::totp::TotpError::BadSecret => "secret is not valid base32".into(),
        simple_password_manager::totp::TotpError::UnsupportedAlgorithm => "only SHA1 is supported".into(),
    }
}
