//! Windows Hello "Quick Unlock".
//!
//! Stores the master password in the Windows Credential Manager under a
//! per-database target name. Reads are gated by a UserConsentVerifier
//! prompt (Hello face / fingerprint / PIN). The stored secret is bound
//! to the current Windows user account and isn't roamed across machines.
//!
//! Threat model: someone with read access to your account's credential
//! store (admin, malware running as your user) can still extract the
//! password without Hello — we enforce the prompt at the application
//! layer, not the OS storage layer. Hello is a convenience for re-unlock,
//! not a hardening over what KDBX + master password already provides.

use sha1::{Digest, Sha1};

const TARGET_PREFIX: &str = "digital.laux.simple_password_manager:hello:";

fn target_for(db_path: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(db_path.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    // 20-byte SHA-1 → 40 hex chars; trim so the full target string fits in
    // CRED_MAX_GENERIC_TARGET_NAME_LENGTH (32767) with room to spare.
    format!("{}{}", TARGET_PREFIX, &hex[..32])
}

// ----------------------------- Windows impl -----------------------------

#[cfg(windows)]
mod imp {
    use super::target_for;
    use windows::core::HSTRING;
    use windows::Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
    };
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    /// Returns true if a Hello PIN, face, or fingerprint is set up on the
    /// device — i.e. the user could actually be prompted right now.
    pub async fn available() -> Result<bool, String> {
        let op = UserConsentVerifier::CheckAvailabilityAsync()
            .map_err(|e| format!("CheckAvailabilityAsync failed: {}", e))?;
        let avail = op
            .await
            .map_err(|e| format!("await CheckAvailability: {}", e))?;
        Ok(matches!(avail, UserConsentVerifierAvailability::Available))
    }

    pub async fn store(db_path: &str, password: &str) -> Result<(), String> {
        let target = target_for(db_path);
        let target_w: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();

        // The credential blob field is a byte buffer — we write the
        // password as UTF-8 and read it back the same way.
        let mut secret_bytes: Vec<u8> = password.as_bytes().to_vec();

        let cred = CREDENTIALW {
            Flags: CRED_FLAGS(0),
            Type: CRED_TYPE_GENERIC,
            TargetName: windows::core::PWSTR(target_w.as_ptr() as *mut _),
            Comment: windows::core::PWSTR::null(),
            LastWritten: windows::Win32::Foundation::FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: secret_bytes.len() as u32,
            CredentialBlob: secret_bytes.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: windows::core::PWSTR::null(),
            UserName: windows::core::PWSTR::null(),
        };

        unsafe {
            CredWriteW(&cred, 0).map_err(|e| format!("CredWriteW failed: {}", e))?;
        }
        Ok(())
    }

    /// Prompt Hello first. On success, read the stored password and return
    /// it. On user cancellation, return Err with a distinctive message so
    /// the frontend can fall back to the master-password input silently.
    pub async fn retrieve(db_path: &str) -> Result<String, String> {
        let title = HSTRING::from("Unlock your password database");
        let op = UserConsentVerifier::RequestVerificationAsync(&title)
            .map_err(|e| format!("RequestVerificationAsync failed: {}", e))?;
        let result = op
            .await
            .map_err(|e| format!("await RequestVerification: {}", e))?;

        match result {
            UserConsentVerificationResult::Verified => {}
            UserConsentVerificationResult::Canceled => return Err("cancelled".to_string()),
            _ => return Err(format!("Hello verification failed: {:?}", result)),
        }

        let target = target_for(db_path);
        let target_w: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();
            CredReadW(
                windows::core::PCWSTR(target_w.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
                &mut cred_ptr,
            )
            .map_err(|e| {
                if e.code() == ERROR_NOT_FOUND.to_hresult() {
                    "no stored credential".to_string()
                } else {
                    format!("CredReadW failed: {}", e)
                }
            })?;

            let cred = &*cred_ptr;
            let len = cred.CredentialBlobSize as usize;
            let blob = std::slice::from_raw_parts(cred.CredentialBlob, len);
            let password = String::from_utf8(blob.to_vec())
                .map_err(|_| "stored credential is not valid UTF-8".to_string());

            CredFree(cred_ptr as *mut _);
            password
        }
    }

    pub async fn clear(db_path: &str) -> Result<(), String> {
        let target = target_for(db_path);
        let target_w: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            match CredDeleteW(
                windows::core::PCWSTR(target_w.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
            ) {
                Ok(()) => Ok(()),
                Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
                Err(e) => Err(format!("CredDeleteW failed: {}", e)),
            }
        }
    }

    /// Probe whether a credential exists for this DB without prompting
    /// Hello. Lets the unlock screen decide whether to show the Hello
    /// button without burning a biometric scan on every render.
    pub async fn is_enrolled(db_path: &str) -> Result<bool, String> {
        let target = target_for(db_path);
        let target_w: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();
            match CredReadW(
                windows::core::PCWSTR(target_w.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
                &mut cred_ptr,
            ) {
                Ok(()) => {
                    CredFree(cred_ptr as *mut _);
                    Ok(true)
                }
                Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(false),
                Err(e) => Err(format!("CredReadW probe failed: {}", e)),
            }
        }
    }
}

// ----------------------------- Stubs -----------------------------

#[cfg(not(windows))]
mod imp {
    pub async fn available() -> Result<bool, String> {
        Ok(false)
    }
    pub async fn store(_db_path: &str, _password: &str) -> Result<(), String> {
        Err("Windows Hello is only available on Windows".to_string())
    }
    pub async fn retrieve(_db_path: &str) -> Result<String, String> {
        Err("Windows Hello is only available on Windows".to_string())
    }
    pub async fn clear(_db_path: &str) -> Result<(), String> {
        Ok(())
    }
    pub async fn is_enrolled(_db_path: &str) -> Result<bool, String> {
        Ok(false)
    }
}

// ----------------------------- Tauri commands -----------------------------

#[tauri::command]
pub async fn hello_available() -> Result<bool, String> {
    imp::available().await
}

#[tauri::command]
pub async fn hello_is_enrolled(db_path: String) -> Result<bool, String> {
    if db_path.is_empty() {
        return Ok(false);
    }
    imp::is_enrolled(&db_path).await
}

#[tauri::command]
pub async fn hello_store(db_path: String, password: String) -> Result<(), String> {
    if db_path.is_empty() {
        return Err("db_path is required".into());
    }
    if password.is_empty() {
        return Err("password is required".into());
    }
    imp::store(&db_path, &password).await
}

#[tauri::command]
pub async fn hello_retrieve(db_path: String) -> Result<String, String> {
    if db_path.is_empty() {
        return Err("db_path is required".into());
    }
    imp::retrieve(&db_path).await
}

#[tauri::command]
pub async fn hello_clear(db_path: String) -> Result<(), String> {
    if db_path.is_empty() {
        return Ok(());
    }
    imp::clear(&db_path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_for_is_stable() {
        // Same input → same target name, so we can find the credential later.
        let a = target_for("/tmp/foo.kdbx");
        let b = target_for("/tmp/foo.kdbx");
        assert_eq!(a, b);
    }

    #[test]
    fn target_for_differs_per_path() {
        let a = target_for("/tmp/foo.kdbx");
        let b = target_for("/tmp/bar.kdbx");
        assert_ne!(a, b);
    }

    #[test]
    fn target_for_carries_prefix() {
        let t = target_for("/tmp/x.kdbx");
        assert!(t.starts_with(TARGET_PREFIX), "prefix missing in {}", t);
    }
}
