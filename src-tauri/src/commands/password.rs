use rand::Rng;

#[tauri::command]
pub fn generate_password(
    length: usize,
    use_uppercase: bool,
    use_lowercase: bool,
    use_numbers: bool,
    use_symbols: bool,
) -> Result<String, String> {
    if length == 0 || length > 256 {
        return Err("Invalid password length".to_string());
    }

    let mut charset = String::new();
    if use_uppercase {
        charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
    if use_lowercase {
        charset.push_str("abcdefghijklmnopqrstuvwxyz");
    }
    if use_numbers {
        charset.push_str("0123456789");
    }
    if use_symbols {
        charset.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?");
    }

    if charset.is_empty() {
        return Err("At least one character type must be selected".to_string());
    }

    let charset_vec: Vec<char> = charset.chars().collect();
    let mut rng = rand::rng();

    let password: String = (0..length)
        .map(|_| {
            let idx = rng.random_range(0..charset_vec.len());
            charset_vec[idx]
        })
        .collect();

    Ok(password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_length() {
        assert!(generate_password(0, true, true, true, true).is_err());
    }

    #[test]
    fn rejects_too_long() {
        assert!(generate_password(257, true, true, true, true).is_err());
    }

    #[test]
    fn rejects_empty_charset() {
        assert!(generate_password(16, false, false, false, false).is_err());
    }

    #[test]
    fn respects_length() {
        let p = generate_password(32, true, true, true, true).unwrap();
        assert_eq!(p.chars().count(), 32);

        let p2 = generate_password(1, false, true, false, false).unwrap();
        assert_eq!(p2.chars().count(), 1);

        let p3 = generate_password(256, true, true, true, true).unwrap();
        assert_eq!(p3.chars().count(), 256);
    }

    #[test]
    fn lowercase_only_charset() {
        let p = generate_password(64, false, true, false, false).unwrap();
        assert!(p.chars().all(|c| c.is_ascii_lowercase()), "got {}", p);
    }

    #[test]
    fn uppercase_only_charset() {
        let p = generate_password(64, true, false, false, false).unwrap();
        assert!(p.chars().all(|c| c.is_ascii_uppercase()), "got {}", p);
    }

    #[test]
    fn numbers_only_charset() {
        let p = generate_password(64, false, false, true, false).unwrap();
        assert!(p.chars().all(|c| c.is_ascii_digit()), "got {}", p);
    }

    #[test]
    fn symbols_only_charset() {
        let p = generate_password(64, false, false, false, true).unwrap();
        let symbols = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        assert!(p.chars().all(|c| symbols.contains(c)), "got {}", p);
    }

    #[test]
    fn two_calls_likely_differ() {
        // Two 32-char passwords from a 90-char alphabet colliding is ~ 1 in 90^32.
        let a = generate_password(32, true, true, true, true).unwrap();
        let b = generate_password(32, true, true, true, true).unwrap();
        assert_ne!(a, b);
    }
}
