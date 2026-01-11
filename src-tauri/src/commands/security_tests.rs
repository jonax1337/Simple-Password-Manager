#[cfg(test)]
mod tests {
    use sha1::{Sha1, Digest};

    fn sha1_hash(password: &str) -> String {
        let mut hasher = Sha1::new();
        hasher.update(password.as_bytes());
        let result = hasher.finalize();
        format!("{:X}", result)
    }

    #[test]
    fn test_sha1_hash_known_values() {
        // Test known SHA1 hashes
        let hash1 = sha1_hash("password");
        assert_eq!(hash1, "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8");

        let hash2 = sha1_hash("123456");
        assert_eq!(hash2, "7C4A8D09CA3762AF61E59520943DC26494F8941B");
    }

    #[test]
    fn test_sha1_hash_prefix() {
        let hash = sha1_hash("password");
        let prefix = &hash[0..5];
        assert_eq!(prefix, "5BAA6");
    }

    #[test]
    fn test_sha1_hash_suffix() {
        let hash = sha1_hash("password");
        let suffix = &hash[5..];
        assert_eq!(suffix, "1E4C9B93F3F0682250B6CF8331B7EE68FD8");
    }


    #[test]
    fn test_k_anonymity_prefix() {
        // K-anonymity uses first 5 characters of SHA1
        let hash1 = sha1_hash("password123");
        let hash2 = sha1_hash("password456");
        
        // Different passwords should have different hashes
        assert_ne!(hash1, hash2);

        // But we only send first 5 chars to API
        let prefix1 = &hash1[0..5];
        let prefix2 = &hash2[0..5];
        
        // These might be same or different - depends on collision
        println!("Prefix1: {}, Prefix2: {}", prefix1, prefix2);
    }
}
