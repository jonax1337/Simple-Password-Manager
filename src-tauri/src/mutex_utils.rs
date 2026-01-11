/// Utility module for safe Mutex handling with poison recovery
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Safely locks a mutex, recovering from poison errors
/// 
/// If a thread panics while holding the mutex, the mutex becomes "poisoned".
/// This function automatically recovers by clearing the poison and proceeding.
/// 
/// # Arguments
/// * `mutex` - The mutex to lock
/// * `context` - A context string for error logging (e.g., function name)
/// 
/// # Returns
/// * `Ok(guard)` - Successfully locked mutex guard
/// * `Err(message)` - If the mutex is fundamentally broken (rare)
pub fn safe_lock<'a, T>(mutex: &'a Mutex<T>, context: &str) -> Result<MutexGuard<'a, T>, String> {
    mutex.lock()
        .or_else(|poison_err: PoisonError<MutexGuard<'a, T>>| -> Result<MutexGuard<'a, T>, std::sync::TryLockError<MutexGuard<'a, T>>> {
            // Log the poisoning event for debugging
            eprintln!("[WARN] Mutex poisoned in '{}', recovering...", context);
            
            // Return the guard from the poison error, effectively clearing the poison
            Ok(poison_err.into_inner())
        })
        .map_err(|_| {
            // This should never happen - only if mutex is fundamentally broken
            let err_msg = format!("Critical: Mutex in '{}' is fundamentally broken", context);
            eprintln!("[ERROR] {}", err_msg);
            err_msg
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_safe_lock_normal_case() {
        let mutex = Mutex::new(42);
        let guard = safe_lock(&mutex, "test").unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_safe_lock_poison_recovery() {
        let mutex = Arc::new(Mutex::new(0));
        let mutex_clone = Arc::clone(&mutex);

        // Deliberately poison the mutex
        let _ = thread::spawn(move || {
            let _guard = mutex_clone.lock().unwrap();
            panic!("Intentional panic to poison mutex");
        }).join();

        // Our safe_lock should recover from the poison
        let result = safe_lock(&mutex, "test_poison");
        assert!(result.is_ok(), "safe_lock should recover from poison");
        
        // Should be able to use the mutex normally
        let mut guard = result.unwrap();
        *guard = 42;
        assert_eq!(*guard, 42);
    }
}
