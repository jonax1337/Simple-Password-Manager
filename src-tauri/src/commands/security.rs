use crate::state::AppState;
use crate::mutex_utils::safe_lock;
use sha1::{Sha1, Digest};
use tauri::State;

#[cfg(test)]
#[path = "security_tests.rs"]
mod security_tests;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct BreachedEntry {
    pub uuid: String,
    pub title: String,
    pub username: String,
    pub breach_count: u32,
}

// Type alias to reduce complexity for Clippy
type PrefixDelayTuple = (String, Vec<(String, String, String, String)>, u64);

#[tauri::command]
pub async fn check_breached_passwords(state: State<'_, AppState>) -> Result<Vec<BreachedEntry>, String> {
    use std::collections::HashMap;
    use rand::Rng;
    
    // Extract all entries while holding the lock, then release it
    let all_entries = {
        let database_lock = safe_lock(&state.database, "check_breached_passwords")?;
        
        if let Some(db) = database_lock.as_ref() {
            db.get_all_entries()
        } else {
            return Err("No database loaded".to_string());
        }
    }; // Lock is dropped here
    
    // Group entries by hash prefix to batch API calls and reduce metadata leakage
    let mut prefix_to_entries: HashMap<String, Vec<(String, String, String, String)>> = HashMap::new();
    
    for entry in all_entries {
        if entry.password.is_empty() {
            continue;
        }
        
        let mut hasher = Sha1::new();
        hasher.update(entry.password.as_bytes());
        let hash = hasher.finalize();
        let hash_hex = format!("{:X}", hash);
        let prefix = hash_hex[..5].to_string();
        let suffix = hash_hex[5..].to_string();
        
        prefix_to_entries
            .entry(prefix)
            .or_default()
            .push((entry.uuid, entry.title, entry.username, suffix));
    }
    
    // Cache for API responses to avoid duplicate requests
    let mut prefix_cache: HashMap<String, String> = HashMap::new();
    let mut breached_entries = Vec::new();
    let mut error_count = 0;
    
    // Pre-generate random delays for each prefix to avoid holding non-Send RNG across await
    let delays: Vec<PrefixDelayTuple> = {
        let mut rng = rand::thread_rng();
        prefix_to_entries
            .into_iter()
            .map(|(prefix, entries)| {
                let delay_ms = rng.gen_range(50..200);
                (prefix, entries, delay_ms)
            })
            .collect()
    };
    
    // Process each unique prefix
    for (prefix, entries, delay_ms) in delays {
        // Add random delay between requests to prevent timing analysis (50-200ms)
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        
        // Check if we already have this prefix cached
        let response_body = if let Some(cached) = prefix_cache.get(&prefix) {
            cached.clone()
        } else {
            match fetch_hibp_range(&prefix).await {
                Ok(body) => {
                    prefix_cache.insert(prefix.clone(), body.clone());
                    body
                }
                Err(_) => {
                    error_count += entries.len();
                    continue;
                }
            }
        };
        
        // Check each entry against the response
        for (uuid, title, username, suffix) in entries {
            if let Some(count) = parse_hibp_response(&response_body, &suffix) {
                if count > 0 {
                    breached_entries.push(BreachedEntry {
                        uuid,
                        title,
                        username,
                        breach_count: count,
                    });
                }
            }
        }
    }
    
    if error_count > 0 {
        eprintln!("HIBP breach check: {} password(s) could not be checked due to API errors", error_count);
    }
    
    Ok(breached_entries)
}

// Fetch HIBP range data for a given prefix
async fn fetch_hibp_range(prefix: &str) -> Result<String, String> {
    let url = format!("https://api.pwnedpasswords.com/range/{}", prefix);
    let client = reqwest::Client::builder()
        .user_agent("Simple-Password-Manager")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to query HIBP API: {}", e))?;
    
    // Handle rate limiting
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown");
        return Err(format!("HIBP rate limited (Retry-After: {})", retry_after));
    }
    
    if !response.status().is_success() {
        return Err(format!("HIBP API returned status: {}", response.status()));
    }
    
    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
}

// Parse HIBP response to find breach count for a specific hash suffix
fn parse_hibp_response(body: &str, suffix: &str) -> Option<u32> {
    for line in body.lines() {
        if let Some((hash_part, count_str)) = line.split_once(':') {
            if hash_part == suffix {
                return count_str.trim().parse::<u32>().ok();
            }
        }
    }
    None
}

// Tauri commands for managing dismissed breach warnings
// Store dismissed breaches in AppState instead of client-side localStorage for better security

#[tauri::command]
pub fn save_dismissed_breach(state: State<AppState>, db_path: String, entry_uuid: String) -> Result<(), String> {
    use std::collections::HashSet;
    
    // Validate inputs
    if db_path.is_empty() {
        eprintln!("save_dismissed_breach: Empty database path provided");
        return Err("Database path cannot be empty".to_string());
    }
    if entry_uuid.is_empty() {
        eprintln!("save_dismissed_breach: Empty entry UUID provided");
        return Err("Entry UUID cannot be empty".to_string());
    }
    
    // Acquire lock with error handling
    let mut dismissed = safe_lock(&state.dismissed_breaches, "save_dismissed_breach")?;
    
    dismissed
        .entry(db_path.clone())
        .or_insert_with(HashSet::new)
        .insert(entry_uuid.clone());
    
    Ok(())
}

#[tauri::command]
pub fn get_dismissed_breaches(state: State<AppState>, db_path: String) -> Result<Vec<String>, String> {
    // Validate input
    if db_path.is_empty() {
        eprintln!("get_dismissed_breaches: Empty database path provided");
        return Ok(Vec::new()); // Return empty vec instead of error for graceful degradation
    }
    
    // Acquire lock with error handling
    let dismissed = safe_lock(&state.dismissed_breaches, "get_dismissed_breaches")?;
    
    if let Some(dismissed_set) = dismissed.get(&db_path) {
        Ok(dismissed_set.iter().cloned().collect())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn clear_dismissed_breach(state: State<AppState>, db_path: String, entry_uuid: String) -> Result<(), String> {
    // Validate inputs
    if db_path.is_empty() {
        eprintln!("clear_dismissed_breach: Empty database path provided");
        return Err("Database path cannot be empty".to_string());
    }
    if entry_uuid.is_empty() {
        eprintln!("clear_dismissed_breach: Empty entry UUID provided");
        return Err("Entry UUID cannot be empty".to_string());
    }
    
    // Acquire lock with error handling
    let mut dismissed = safe_lock(&state.dismissed_breaches, "clear_dismissed_breach")?;
    
    if let Some(dismissed_set) = dismissed.get_mut(&db_path) {
        dismissed_set.remove(&entry_uuid);
    }
    
    Ok(())
}
