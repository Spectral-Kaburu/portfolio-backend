//! Redaction module - removes sensitive data from text.
//!
//! RUST NOTE: This is a pure function that returns cleaned text and a count
//! of redactions. False positives are acceptable; leaks are not.

use regex::Regex;

/// Redacted text and count of replacements made.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct Redacted {
    pub text: String,
    pub count: usize,
}

/// Compile all regex patterns for redaction, lazily with proper error handling.
/// Patterns that fail to compile are skipped.
fn get_patterns() -> Vec<Regex> {
    let mut patterns = Vec::new();

    // URLs with credentials: https://user:pass@host
    if let Ok(p) = Regex::new(r"https://[^/:]+:[^@]+@") {
        patterns.push(p);
    }
    // Bearer tokens: Bearer abc123...
    if let Ok(p) = Regex::new(r"Bearer\s+[a-zA-Z0-9_\-\.]+") {
        patterns.push(p);
    }
    // AWS Access Key IDs: AKIAxxxxxxxxxxxxxxxx
    if let Ok(p) = Regex::new(r"AKIA[0-9A-Z]{16}") {
        patterns.push(p);
    }
    // Generic API key patterns: key="value" or key: "value"
    if let Ok(p) = Regex::new(r#"key["\']?\s*[:=]\s*["\']?[a-zA-Z0-9_\-]{16,}"#) {
        patterns.push(p);
    }
    // OpenAI-style keys: sk-...
    if let Ok(p) = Regex::new(r"sk-[a-zA-Z0-9_]{20,}") {
        patterns.push(p);
    }
    // Private keys (multi-line) - using # raw string to avoid \s issues
    if let Ok(p) = Regex::new(
        r#"-----BEGIN (RSA |DSA |EC |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END (RSA |DSA |EC |OPENSSH )?PRIVATE KEY-----"#,
    ) {
        patterns.push(p);
    }
    // Long hex strings (32+ consecutive hex chars - potential hashes/keys)
    // Excludes exactly 40-char hex strings which are git commit hashes (evidence-replaced)
    // Matches: 32-39 chars OR 41+ chars, but NOT exactly 40 chars
    if let Ok(p) = Regex::new(r"\b([a-fA-F0-9]{32,39}|[a-fA-F0-9]{41,})\b") {
        patterns.push(p);
    }
    // Email addresses
    if let Ok(p) = Regex::new(r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+") {
        patterns.push(p);
    }
    // Generic API key env vars: SOMETHING_API_KEY="value" or SOMETHING_API_KEY=value
    if let Ok(p) = Regex::new(r#"[A-Z_]+_API_KEY["\']?\s*[:=]\s*["\']?[a-zA-Z0-9_\-]{8,}"#) {
        patterns.push(p);
    }

    patterns
}

/// Redact sensitive patterns from text.
/// Returns the cleaned text and count of redactions.
#[allow(dead_code)]
pub fn redact(text: &str) -> Redacted {
    let patterns = get_patterns();
    let mut redacted = text.to_string();
    let mut count = 0;

    for pattern in patterns {
        let matches: Vec<_> = pattern.find_iter(text).collect();
        count += matches.len();

        // Replace all matches with [REDACTED]
        redacted = pattern.replace_all(&redacted, "[REDACTED]").to_string();
    }

    Redacted {
        text: redacted,
        count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sentinel values used to verify redactions work
    const SENTINEL_EMAIL: &str = "testuser@example.com";
    const SENTINEL_API_KEY: &str = "sk-test_api_key_12345678901234567890";
    const SENTINEL_AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
    const SENTINEL_BEARER: &str = "Bearer secret_token_12345";
    const SENTINEL_URL_CREDS: &str = "https://user:pass@example.com/repo";
    const SENTINEL_LONG_HEX: &str = "0123456789abcdef0123456789abcdef";

    const SENTINEL_KEY_PATTERN: &str = r#"api_key="sk_test_12345678901234567890""#;

    // Text that should survive redaction
    const PRESERVED_TEXT: &str = r#"
        This is a commit from 2024-01-15.
        Version 1.2.3 was released.
        The repo is at /home/user/projects/my-project.
        Last updated on 2024-02-20.
        # Title is preserved.
    "#;

    #[test]
    fn test_email_redaction() {
        let result = redact(SENTINEL_EMAIL);
        assert!(
            result.text.contains("[REDACTED]"),
            "Email should be redacted"
        );
        assert!(
            !result.text.contains("@example.com"),
            "Email domain should not appear"
        );
        assert_eq!(result.count, 1);
    }

    #[test]
    fn test_api_key_redaction() {
        let result = redact(SENTINEL_API_KEY);
        assert!(
            result.text.contains("[REDACTED]"),
            "OpenAI key should be redacted"
        );
        assert!(
            !result.text.contains("sk-test"),
            "Key value should not appear in output"
        );
        assert!(result.count >= 1, "Should have at least one redaction");
    }

    #[test]
    fn test_aws_key_redaction() {
        let result = redact(SENTINEL_AWS_KEY);
        assert!(
            result.text.contains("[REDACTED]"),
            "AWS key should be redacted"
        );
        assert!(
            !result.text.contains("AKIA"),
            "AWS key prefix should not appear"
        );
        assert_eq!(result.count, 1);
    }

    #[test]
    fn test_bearer_token_redaction() {
        let result = redact(SENTINEL_BEARER);
        assert!(
            result.text.contains("[REDACTED]"),
            "Bearer token should be redacted"
        );
        assert!(
            !result.text.contains("secret_token"),
            "Token value should not appear"
        );
        assert_eq!(result.count, 1);
    }

    #[test]
    fn test_url_with_credentials_redaction() {
        let result = redact(SENTINEL_URL_CREDS);
        assert!(
            result.text.contains("[REDACTED]"),
            "URL with credentials should be redacted"
        );
        assert!(
            !result.text.contains("user:pass@"),
            "Credentials should not appear in output"
        );
        assert!(result.count >= 1, "Should have at least one redaction");
    }

    #[test]
    fn test_long_hex_redaction() {
        let result = redact(SENTINEL_LONG_HEX);
        assert!(
            result.text.contains("[REDACTED]"),
            "Long hex should be redacted"
        );
        assert!(
            !result.text.contains("0123456789abcdef"),
            "Hex content should not appear"
        );
        assert_eq!(result.count, 1);
    }

    #[test]
    fn test_short_hex_preserved() {
        // 8 hex chars should NOT be redacted
        let result = redact("deadbeef");
        assert!(
            result.text.contains("deadbeef"),
            "Short hex should be preserved"
        );
        assert_eq!(result.count, 0);
    }

    #[test]
    fn test_commit_hash_preserved() {
        // 40-char hex strings (git commit hashes) should NOT be redacted
        // They will be evidence-replaced elsewhere in the pipeline
        let commit_hash = "0123456789abcdef0123456789abcdef01234567";
        let result = redact(commit_hash);
        assert!(
            result
                .text
                .contains("0123456789abcdef0123456789abcdef01234567"),
            "40-char commit hash should be preserved"
        );
        assert_eq!(result.count, 0, "Commit hash should not be redacted");
    }

    #[test]
    fn test_api_key_pattern_redaction() {
        let result = redact(SENTINEL_KEY_PATTERN);
        assert!(
            result.text.contains("[REDACTED]"),
            "API key pattern should be redacted"
        );
        assert!(
            !result.text.contains("sk_test"),
            "Key value should not appear"
        );
    }

    #[test]
    fn test_preserved_text_unchanged() {
        let result = redact(PRESERVED_TEXT);
        // Dates should be preserved
        assert!(
            result.text.contains("2024-01-15"),
            "Date should be preserved"
        );
        assert!(
            result.text.contains("2024-02-20"),
            "Another date should be preserved"
        );
        // Version strings should be preserved
        assert!(result.text.contains("1.2.3"), "Version should be preserved");
        // Title should be preserved
        assert!(
            result.text.contains("Title is preserved"),
            "Title should be preserved"
        );
    }

    #[test]
    fn test_private_key_redaction() {
        let key_with_text = r#"-----BEGIN RSA PRIVATE KEY-----
Secret key content here
-----END RSA PRIVATE KEY-----
Some normal text after"#;
        let result = redact(key_with_text);
        assert!(
            result.text.contains("[REDACTED]"),
            "Private key should be redacted"
        );
        assert!(
            !result.text.contains("Secret key content"),
            "Private key content should not appear"
        );
    }

    #[test]
    fn test_multiple_patterns() {
        let combined = format!(
            "Contact {}, key=sk_test_12345678901234567890, url: {}",
            SENTINEL_EMAIL, SENTINEL_URL_CREDS
        );
        let result = redact(&combined);
        assert!(
            result.text.contains("[REDACTED]"),
            "Should contain redaction marker"
        );
        // Should have redactions for email, API key, and URL
        assert!(result.count >= 3, "Should have at least 3 redactions");
    }

    #[test]
    fn test_api_key_env_var_redaction() {
        // Test generic API key env var pattern
        let text = r#"DATABASE_API_KEY="super_secret_key_12345"
ANOTHER_API_KEY=another_secret_value_01234567"#;
        let result = redact(text);
        assert!(
            result.text.contains("[REDACTED]"),
            "API key env var should be redacted"
        );
        assert!(
            !result.text.contains("super_secret_key"),
            "First key value should not appear"
        );
        assert!(
            !result.text.contains("another_secret_value"),
            "Second key value should not appear"
        );
    }
}
