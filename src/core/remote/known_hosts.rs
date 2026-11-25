use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyVerificationResult {
    Match,
    NewHost,
    Mismatch { recorded: String, actual: String },
}

#[derive(Debug, Clone)]
pub struct KnownHostEntry {
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint_sha256: String,
}

pub struct KnownHostsStore {
    entries: Vec<KnownHostEntry>,
    file_path: PathBuf,
}

impl KnownHostsStore {
    pub fn new(path: &Path) -> Self {
        let mut store = Self {
            entries: Vec::new(),
            file_path: path.to_path_buf(),
        };
        let _ = store.load();
        store
    }

    pub fn verify_host(
        &self,
        host: &str,
        port: u16,
        _key_type: &str,
        fingerprint_sha256: &str,
    ) -> HostKeyVerificationResult {
        let clean_fp = fingerprint_sha256.trim();
        for entry in &self.entries {
            if entry.host.eq_ignore_ascii_case(host) && entry.port == port {
                if entry.fingerprint_sha256 == clean_fp {
                    return HostKeyVerificationResult::Match;
                } else {
                    return HostKeyVerificationResult::Mismatch {
                        recorded: entry.fingerprint_sha256.clone(),
                        actual: clean_fp.to_string(),
                    };
                }
            }
        }
        HostKeyVerificationResult::NewHost
    }

    pub fn record_host(
        &mut self,
        host: &str,
        port: u16,
        key_type: &str,
        fingerprint_sha256: &str,
    ) -> Result<(), String> {
        let clean_fp = fingerprint_sha256.trim().to_string();
        if let Some(pos) = self
            .entries
            .iter()
            .position(|e| e.host.eq_ignore_ascii_case(host) && e.port == port)
        {
            self.entries[pos].key_type = key_type.to_string();
            self.entries[pos].fingerprint_sha256 = clean_fp;
        } else {
            self.entries.push(KnownHostEntry {
                host: host.to_string(),
                port,
                key_type: key_type.to_string(),
                fingerprint_sha256: clean_fp,
            });
        }
        self.save()
    }

    pub fn load(&mut self) -> Result<(), String> {
        if !self.file_path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&self.file_path)
            .map_err(|e| format!("Read known_hosts error: {}", e))?;
        self.entries.clear();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Format: [host]:port key_type fingerprint
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let host_port = parts[0];
                let (host, port) = if host_port.starts_with('[') && host_port.contains("]:") {
                    let mut s = host_port.split("]:");
                    let h = s.next().unwrap_or("").trim_start_matches('[');
                    let p = s.next().unwrap_or("22").parse::<u16>().unwrap_or(22);
                    (h.to_string(), p)
                } else if let Some((h, p)) = host_port.split_once(':') {
                    (h.to_string(), p.parse::<u16>().unwrap_or(22))
                } else {
                    (host_port.to_string(), 22)
                };

                self.entries.push(KnownHostEntry {
                    host,
                    port,
                    key_type: parts[1].to_string(),
                    fingerprint_sha256: parts[2].to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        let mut out = String::new();
        out.push_str("# Tylogger SSH known_hosts store\n");
        for e in &self.entries {
            out.push_str(&format!(
                "[{}]:{} {} {}\n",
                e.host, e.port, e.key_type, e.fingerprint_sha256
            ));
        }

        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&self.file_path, out).map_err(|e| format!("Save known_hosts error: {}", e))
    }
}
