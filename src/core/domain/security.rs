use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityObservation {
    pub category: String,
    pub description: String,
    pub event_count: u64,
    pub first_seen_line: usize,
    pub sample_text: String,
}

pub struct SecurityPresetAnalyzer {
    pub wazuh_level_counts: HashMap<u8, u64>,
    pub suricata_signatures: HashMap<String, u64>,
    pub auth_failures: HashMap<String, u64>, // IP/User -> count
}

impl Default for SecurityPresetAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityPresetAnalyzer {
    pub fn new() -> Self {
        Self {
            wazuh_level_counts: HashMap::new(),
            suricata_signatures: HashMap::new(),
            auth_failures: HashMap::new(),
        }
    }

    pub fn inspect_line(&mut self, line: &str, line_idx: usize) -> Option<SecurityObservation> {
        // Wazuh alert check (e.g. JSON with "rule":{"level": 12})
        if line.contains("\"rule\"") && line.contains("\"level\"") {
            if let Some(pos) = line.find("\"level\":") {
                let rest = &line[pos + 8..].trim_start();
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(lvl) = digits.parse::<u8>() {
                    *self.wazuh_level_counts.entry(lvl).or_insert(0) += 1;
                    if lvl >= 10 {
                        return Some(SecurityObservation {
                            category: "Wazuh High Severity Alert".to_string(),
                            description: format!("Wazuh rule level {} triggered", lvl),
                            event_count: 1,
                            first_seen_line: line_idx,
                            sample_text: line.chars().take(120).collect(),
                        });
                    }
                }
            }
        }

        // Suricata signature check (e.g. "alert" ... "signature")
        if line.contains("\"event_type\":\"alert\"") || line.contains("\"alert\":") {
            if let Some(pos) = line.find("\"signature\":") {
                let rest = &line[pos + 12..].trim_start();
                if let Some(stripped) = rest.strip_prefix('"') {
                    let sig: String = stripped.chars().take_while(|c| *c != '"').collect();
                    *self.suricata_signatures.entry(sig.clone()).or_insert(0) += 1;
                    return Some(SecurityObservation {
                        category: "Suricata IDS Alert".to_string(),
                        description: format!("IDS signature: {}", sig),
                        event_count: 1,
                        first_seen_line: line_idx,
                        sample_text: line.chars().take(120).collect(),
                    });
                }
            }
        }

        // Authentication failure check (HTTP 401/403 or keyword failed login)
        if line.contains("401 ")
            || line.contains("403 ")
            || line.contains("Failed password")
            || line.contains("auth failed")
        {
            return Some(SecurityObservation {
                category: "Authentication Failure Observation".to_string(),
                description: "Access denial or authentication rejection logged".to_string(),
                event_count: 1,
                first_seen_line: line_idx,
                sample_text: line.chars().take(120).collect(),
            });
        }

        None
    }
}
