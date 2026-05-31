//! Windows Hello "Quick Unlock".
//!
//! Stores the master password in the Windows Credential Manager under a
//! per-database target name. Reads are gated by a UserConsentVerifier
//! prompt (Hello face / fingerprint / PIN). The stored secret is bound
//! to the current Windows user account and isn't roamed across machines.
//!
//! On Win32 desktop the bare `UserConsentVerifier::RequestVerificationAsync`
//! call doesn't surface the system picker — we have to route the call
//! through `IUserConsentVerifierInterop::RequestVerificationForWindowAsync`
//! with the HWND of our main window. The WinRT-only path silently
//! resolves without UI on a desktop app, which is what made the original
//! implementation appear to do nothing.
//!
//! Threat model: someone with read access to your account's credential
//! store (admin, malware running as your user) can still extract the
//! password without Hello — we enforce the prompt at the application
//! layer, not the OS storage layer. Hello is a convenience for re-unlock,
//! not a hardening over what KDBX + master password already provides.

use sha1::{Digest, Sha1};

// Only the Windows impl uses these; on other platforms the non-test build
// has no references (the unit tests do exercise target_for cross-platform,
// but `cargo clippy` without --all-targets doesn't see test code).
#[cfg_attr(not(windows), allow(dead_code))]
const TARGET_PREFIX: &str = "digital.laux.simple_password_manager:hello:";

/// Cloud-side Hello bundle slot. Single, well-known target — assumes one
/// cloud account per Windows user (matches the cloud_persistence model).
/// The bundle is opaque JSON: server URL, email, cloud-pw, default vault
/// id + kdbx-pw. Frontend serializes; Rust just stores and retrieves.
#[cfg_attr(not(windows), allow(dead_code))]
const CLOUD_TARGET: &str = "digital.laux.simple_password_manager:hello:cloud-default";

#[cfg_attr(not(windows), allow(dead_code))]
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
    use windows::Win32::Foundation::{ERROR_NOT_FOUND, HWND};
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };
    use windows::Win32::System::WinRT::IUserConsentVerifierInterop;

    /// Returns true if a Hello PIN, face, or fingerprint is set up on the
    /// device — i.e. the user could actually be prompted right now.
    /// This is a pure query, no UI is shown.
    pub async fn available() -> Result<bool, String> {
        let op = UserConsentVerifier::CheckAvailabilityAsync()
            .map_err(|e| format!("CheckAvailabilityAsync failed: {}", e))?;
        let avail = op
            .await
            .map_err(|e| format!("await CheckAvailability: {}", e))?;
        Ok(matches!(avail, UserConsentVerifierAvailability::Available))
    }

    /// Trigger the system Hello picker (face / fingerprint / PIN). Returns
    /// Ok(()) if the user verified, Err("cancelled") if they cancelled,
    /// Err(other) for any device-side failure. Needs the HWND of a real
    /// app window — the consent dialog parents itself there.
    /// `hwnd_raw` is the HWND wrapped as a raw pointer value: it's
    /// `!Send` as a `HWND` struct but the underlying isize *is* Send,
    /// so we pass that across the async boundary and reconstitute the
    /// HWND inside the synchronous setup block. The actual `HWND` value
    /// only lives long enough to start the IAsyncOperation, and that
    /// operation is Send (via windows_future's AsyncFuture impl).
    pub async fn verify(hwnd_raw: isize, prompt: &str) -> Result<(), String> {
        let title = HSTRING::from(prompt);
        let op: windows_future::IAsyncOperation<UserConsentVerificationResult> = {
            let hwnd = HWND(hwnd_raw as *mut _);
            let interop: IUserConsentVerifierInterop =
                windows::core::factory::<UserConsentVerifier, IUserConsentVerifierInterop>()
                    .map_err(|e| format!("UserConsentVerifier interop factory: {}", e))?;
            unsafe {
                interop
                    .RequestVerificationForWindowAsync(hwnd, &title)
                    .map_err(|e| format!("RequestVerificationForWindowAsync failed: {}", e))?
            }
        };
        let result = op
            .await
            .map_err(|e| format!("await RequestVerification: {}", e))?;

        match result {
            UserConsentVerificationResult::Verified => Ok(()),
            UserConsentVerificationResult::Canceled => Err("cancelled".to_string()),
            UserConsentVerificationResult::DeviceNotPresent => {
                Err("Windows Hello device is not present".into())
            }
            UserConsentVerificationResult::NotConfiguredForUser => {
                Err("Windows Hello is not configured for this user".into())
            }
            UserConsentVerificationResult::DisabledByPolicy => {
                Err("Windows Hello is disabled by policy".into())
            }
            UserConsentVerificationResult::DeviceBusy => {
                Err("Windows Hello device is busy. Try again.".into())
            }
            UserConsentVerificationResult::RetriesExhausted => {
                Err("Windows Hello retry limit reached".into())
            }
            other => Err(format!("Hello verification failed: {:?}", other)),
        }
    }

    pub fn store(db_path: &str, password: &str) -> Result<(), String> {
        let target = target_for(db_path);
        let target_w: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();

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

    pub fn read_secret(db_path: &str) -> Result<String, String> {
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

    pub fn clear(db_path: &str) -> Result<(), String> {
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
    pub fn is_enrolled(db_path: &str) -> Result<bool, String> {
        let target = target_for(db_path);
        cred_exists(&target)
    }

    fn cred_exists(target: &str) -> Result<bool, String> {
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

    // ----- Cloud bundle: same CredManager mechanics, fixed target. -----

    pub fn store_cloud(json: &str) -> Result<(), String> {
        use super::CLOUD_TARGET;
        let target_w: Vec<u16> =
            CLOUD_TARGET.encode_utf16().chain(std::iter::once(0)).collect();
        let mut secret_bytes: Vec<u8> = json.as_bytes().to_vec();
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

    pub fn read_cloud() -> Result<String, String> {
        use super::CLOUD_TARGET;
        let target_w: Vec<u16> =
            CLOUD_TARGET.encode_utf16().chain(std::iter::once(0)).collect();
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
                    "no stored cloud bundle".to_string()
                } else {
                    format!("CredReadW failed: {}", e)
                }
            })?;
            let cred = &*cred_ptr;
            let len = cred.CredentialBlobSize as usize;
            let blob = std::slice::from_raw_parts(cred.CredentialBlob, len);
            let s = String::from_utf8(blob.to_vec())
                .map_err(|_| "stored cloud bundle is not valid UTF-8".to_string());
            CredFree(cred_ptr as *mut _);
            s
        }
    }

    pub fn clear_cloud() -> Result<(), String> {
        use super::CLOUD_TARGET;
        let target_w: Vec<u16> =
            CLOUD_TARGET.encode_utf16().chain(std::iter::once(0)).collect();
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

    pub fn cloud_is_enrolled() -> Result<bool, String> {
        use super::CLOUD_TARGET;
        cred_exists(CLOUD_TARGET)
    }
}

// ----------------------------- Stubs -----------------------------

// The non-Windows command bodies return the error directly without ever
// calling into imp::, but we keep the stub module so the platform-gated
// import paths in this file resolve uniformly. All items here are
// intentionally unused on non-Windows builds.
#[cfg(not(windows))]
#[allow(dead_code)]
mod imp {
    pub async fn available() -> Result<bool, String> {
        Ok(false)
    }
    pub async fn verify(_hwnd_raw: isize, _prompt: &str) -> Result<(), String> {
        Err("Windows Hello is only available on Windows".to_string())
    }
    pub fn store(_db_path: &str, _password: &str) -> Result<(), String> {
        Err("Windows Hello is only available on Windows".to_string())
    }
    pub fn read_secret(_db_path: &str) -> Result<String, String> {
        Err("Windows Hello is only available on Windows".to_string())
    }
    pub fn clear(_db_path: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn is_enrolled(_db_path: &str) -> Result<bool, String> {
        Ok(false)
    }
    pub fn store_cloud(_json: &str) -> Result<(), String> {
        Err("Windows Hello is only available on Windows".to_string())
    }
    pub fn read_cloud() -> Result<String, String> {
        Err("Windows Hello is only available on Windows".to_string())
    }
    pub fn clear_cloud() -> Result<(), String> {
        Ok(())
    }
    pub fn cloud_is_enrolled() -> Result<bool, String> {
        Ok(false)
    }
}

// ----------------------------- Tauri commands -----------------------------

/// Returns the main window's HWND as a Send-friendly isize so it can
/// cross async boundaries. The caller reconstitutes a `HWND` from it
/// only inside the synchronous Win32 setup section.
#[cfg(windows)]
fn main_window_hwnd_raw(app: &tauri::AppHandle) -> Result<isize, String> {
    use tauri::Manager;
    let window = app
        .get_webview_window("main")
        .ok_or("main window not available")?;
    let raw = window
        .hwnd()
        .map_err(|e| format!("failed to get HWND: {}", e))?;
    // Tauri's HWND wraps a `*mut c_void`; cast to isize so the value is
    // Send. We don't dereference it on Rust's side, just hand it back
    // to Windows on a single thread later.
    Ok(raw.0 as isize)
}

#[tauri::command]
pub async fn hello_available() -> Result<bool, String> {
    imp::available().await
}

#[tauri::command]
pub fn hello_is_enrolled(db_path: String) -> Result<bool, String> {
    if db_path.is_empty() {
        return Ok(false);
    }
    imp::is_enrolled(&db_path)
}

/// Storing a credential is gated by a live Hello prompt: the user has
/// to prove they're physically present before we write the secret. This
/// also doubles as a smoke-test of the Hello integration at setup time.
#[tauri::command]
#[allow(unused_variables)]
pub async fn hello_store(
    app: tauri::AppHandle,
    db_path: String,
    password: String,
) -> Result<(), String> {
    if db_path.is_empty() {
        return Err("db_path is required".into());
    }
    if password.is_empty() {
        return Err("password is required".into());
    }

    #[cfg(windows)]
    {
        let hwnd_raw = main_window_hwnd_raw(&app)?;
        imp::verify(hwnd_raw, "Confirm to save your master password for Windows Hello unlock")
            .await?;
        imp::store(&db_path, &password)
    }

    #[cfg(not(windows))]
    {
        Err("Windows Hello is only available on Windows".to_string())
    }
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn hello_retrieve(
    app: tauri::AppHandle,
    db_path: String,
) -> Result<String, String> {
    if db_path.is_empty() {
        return Err("db_path is required".into());
    }

    #[cfg(windows)]
    {
        let hwnd_raw = main_window_hwnd_raw(&app)?;
        imp::verify(hwnd_raw, "Unlock your password database").await?;
        imp::read_secret(&db_path)
    }

    #[cfg(not(windows))]
    {
        Err("Windows Hello is only available on Windows".to_string())
    }
}

#[tauri::command]
pub fn hello_clear(db_path: String) -> Result<(), String> {
    if db_path.is_empty() {
        return Ok(());
    }
    imp::clear(&db_path)
}

// ----- Cloud-side Hello bundle (opaque JSON) -----

/// Is there a cloud-Hello bundle currently saved? Probed by the unlock
/// screen to decide whether to render the prominent "Sign in with Hello"
/// button alongside the password form.
#[tauri::command]
pub fn hello_cloud_is_enrolled() -> Result<bool, String> {
    imp::cloud_is_enrolled()
}

/// Save a cloud-Hello bundle. The frontend serializes whatever payload
/// makes the unlock smooth on next launch (typically: cloud password +
/// the default vault id + that vault's KDBX password). Storing is gated
/// by a live Hello prompt so the user is physically present.
#[tauri::command]
#[allow(unused_variables)]
pub async fn hello_cloud_store(
    app: tauri::AppHandle,
    bundle_json: String,
) -> Result<(), String> {
    if bundle_json.is_empty() {
        return Err("bundle is empty".into());
    }
    #[cfg(windows)]
    {
        let hwnd_raw = main_window_hwnd_raw(&app)?;
        imp::verify(hwnd_raw, "Confirm to save your cloud account for Hello unlock").await?;
        imp::store_cloud(&bundle_json)
    }
    #[cfg(not(windows))]
    {
        Err("Windows Hello is only available on Windows".to_string())
    }
}

/// Reveal the cloud-Hello bundle after a successful Hello prompt. Returns
/// the same JSON the caller stored — frontend parses it and drives the
/// auto-login + auto-vault-open flow.
#[tauri::command]
#[allow(unused_variables)]
pub async fn hello_cloud_retrieve(app: tauri::AppHandle) -> Result<String, String> {
    #[cfg(windows)]
    {
        let hwnd_raw = main_window_hwnd_raw(&app)?;
        imp::verify(hwnd_raw, "Unlock your cloud vault").await?;
        imp::read_cloud()
    }
    #[cfg(not(windows))]
    {
        Err("Windows Hello is only available on Windows".to_string())
    }
}

#[tauri::command]
pub fn hello_cloud_clear() -> Result<(), String> {
    imp::clear_cloud()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_for_is_stable() {
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
