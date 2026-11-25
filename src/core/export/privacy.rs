use crate::core::domain::ip::CanonicalIp;

/// Masking options for sensitive data sanitization in exported reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivacyMaskingOptions {
    pub mask_ip_addresses: bool,
    pub mask_auth_tokens: bool,
    pub mask_passwords: bool,
    pub mask_emails: bool,
}

impl Default for PrivacyMaskingOptions {
    fn default() -> Self {
        Self {
            mask_ip_addresses: true,
            mask_auth_tokens: true,
            mask_passwords: true,
            mask_emails: true,
        }
    }
}

pub struct PrivacyFilter;

impl PrivacyFilter {
    /// Sanitizes a string for report export without modifying source files (RSK-15).
    pub fn mask_line(line: &str, opts: PrivacyMaskingOptions) -> String {
        let mut result = line.to_string();

        if opts.mask_auth_tokens {
            // Mask Bearer tokens: Bearer <token>
            if let Some(pos) = result.to_ascii_lowercase().find("bearer ") {
                let token_start = pos + 7;
                let token_end = result[token_start..]
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .map(|p| token_start + p)
                    .unwrap_or(result.len());
                let prefix = &result[..token_start];
                let suffix = &result[token_end..];
                result = format!("{}***MASKED_TOKEN***{}", prefix, suffix);
            }

            // Mask Basic auth
            if let Some(pos) = result.to_ascii_lowercase().find("basic ") {
                let token_start = pos + 6;
                let token_end = result[token_start..]
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .map(|p| token_start + p)
                    .unwrap_or(result.len());
                let prefix = &result[..token_start];
                let suffix = &result[token_end..];
                result = format!("{}***MASKED_AUTH***{}", prefix, suffix);
            }
        }

        if opts.mask_passwords {
            // Mask password= or pwd=
            for key in &["password=", "pwd=", "secret=", "api_key="] {
                if let Some(pos) = result.to_ascii_lowercase().find(key) {
                    let val_start = pos + key.len();
                    let val_end = result[val_start..]
                        .find(|c: char| c == '&' || c.is_whitespace() || c == '"' || c == '\'')
                        .map(|p| val_start + p)
                        .unwrap_or(result.len());
                    let prefix = &result[..val_start];
                    let suffix = &result[val_end..];
                    result = format!("{}***MASKED***{}", prefix, suffix);
                }
            }
        }

        if opts.mask_emails {
            // Simple email masking: x@y.z
            let words: Vec<&str> = result.split_whitespace().collect();
            let mut masked_words = Vec::with_capacity(words.len());
            for w in words {
                if w.contains('@') && w.contains('.') && !w.starts_with('@') && !w.ends_with('@') {
                    let parts: Vec<&str> = w.split('@').collect();
                    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                        let first_char = parts[0].chars().next().unwrap_or('*');
                        masked_words.push(format!("{}***@{}", first_char, parts[1]));
                        continue;
                    }
                }
                masked_words.push(w.to_string());
            }
            result = masked_words.join(" ");
        }

        if opts.mask_ip_addresses {
            // Mask IPv4 addresses in the string
            let words: Vec<&str> = result.split_whitespace().collect();
            let mut masked_words = Vec::with_capacity(words.len());
            for w in words {
                let clean_w = w.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
                if let Some(canonical) = CanonicalIp::parse(clean_w) {
                    let masked_ip = canonical.mask_privacy();
                    masked_words.push(w.replace(clean_w, &masked_ip));
                } else {
                    masked_words.push(w.to_string());
                }
            }
            result = masked_words.join(" ");
        }

        result
    }
}
