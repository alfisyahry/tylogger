pub struct DynamicTokenizer;

impl DynamicTokenizer {
    /// Tokenizes dynamic parameters in a log message while keeping static text intact.
    pub fn tokenize(line: &str) -> String {
        let mut out = String::with_capacity(line.len());
        let words: Vec<&str> = line.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }

            // Check IP address format
            if word.contains('.')
                && word
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.' || c == ':')
            {
                let parts: Vec<&str> = word.split('.').collect();
                if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
                    out.push_str("<IP>");
                    continue;
                }
            }

            // Check UUID format (36 chars, 4 hyphens)
            let trimmed_word = word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
            if trimmed_word.len() == 36 && trimmed_word.chars().filter(|c| *c == '-').count() == 4 {
                out.push_str("<UUID>");
                continue;
            }

            // Check pure number
            let num_clean = word.trim_matches(|c: char| !c.is_ascii_digit());
            if !num_clean.is_empty() && num_clean == *word {
                out.push_str("<NUM>");
                continue;
            }

            // Check hex hash (e.g. 32-char md5 or 40-char sha1 or 64-char sha256)
            if (word.len() == 32 || word.len() == 40 || word.len() == 64)
                && word.chars().all(|c| c.is_ascii_hexdigit())
            {
                out.push_str("<HEX>");
                continue;
            }

            // Fallback word
            out.push_str(word);
        }

        out
    }
}
