use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    Password,
    PrivateKey { path: String, has_passphrase: bool },
    EncryptedKey { path: String },
    SshAgent,
}

impl AuthMethod {
    pub fn name(&self) -> &'static str {
        match self {
            AuthMethod::Password => "Password",
            AuthMethod::PrivateKey { .. } => "Private Key",
            AuthMethod::EncryptedKey { .. } => "Encrypted Key",
            AuthMethod::SshAgent => "SSH Agent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: u32,
    pub jitter: bool,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 2000,
            max_delay_ms: 30000,
            backoff_factor: 2,
            jitter: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HostKeySetting {
    Strict,
    #[default]
    AskOnNew,
    PinnedSha256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
    pub default_dir: String,
    pub connect_timeout_secs: u32,
    pub read_timeout_secs: u32,
    pub keepalive_interval_secs: u32,
    pub reconnect_policy: ReconnectPolicy,
    pub host_key_setting: HostKeySetting,
    pub last_connected: Option<u64>,
    pub password_encrypted: Option<String>,
}

impl ConnectionProfile {
    pub fn new(id: &str, name: &str, host: &str, port: u16, username: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port,
            username: username.to_string(),
            auth_method: AuthMethod::Password,
            default_dir: "/var/log".to_string(),
            connect_timeout_secs: 10,
            read_timeout_secs: 15,
            keepalive_interval_secs: 30,
            reconnect_policy: ReconnectPolicy::default(),
            host_key_setting: HostKeySetting::default(),
            last_connected: None,
            password_encrypted: None,
        }
    }

    /// Serializes profile to a JSON string representation.
    pub fn to_json(&self, include_secrets: bool) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("{\n");
        out.push_str(&format!("  \"id\": \"{}\",\n", self.id));
        out.push_str(&format!("  \"name\": \"{}\",\n", self.name));
        out.push_str(&format!("  \"host\": \"{}\",\n", self.host));
        out.push_str(&format!("  \"port\": {},\n", self.port));
        out.push_str(&format!("  \"username\": \"{}\",\n", self.username));
        out.push_str(&format!(
            "  \"auth_method\": \"{}\",\n",
            self.auth_method.name()
        ));
        out.push_str(&format!("  \"default_dir\": \"{}\",\n", self.default_dir));
        out.push_str(&format!(
            "  \"connect_timeout_secs\": {},\n",
            self.connect_timeout_secs
        ));
        out.push_str(&format!(
            "  \"read_timeout_secs\": {},\n",
            self.read_timeout_secs
        ));
        out.push_str(&format!(
            "  \"keepalive_interval_secs\": {},\n",
            self.keepalive_interval_secs
        ));
        out.push_str(&format!(
            "  \"max_retries\": {},\n",
            self.reconnect_policy.max_retries
        ));
        out.push_str(&format!(
            "  \"initial_delay_ms\": {},\n",
            self.reconnect_policy.initial_delay_ms
        ));
        out.push_str(&format!(
            "  \"max_delay_ms\": {},\n",
            self.reconnect_policy.max_delay_ms
        ));
        if let Some(lc) = self.last_connected {
            out.push_str(&format!("  \"last_connected\": {},\n", lc));
        }
        if include_secrets {
            if let Some(ref cipher) = self.password_encrypted {
                out.push_str(&format!("  \"password_encrypted\": \"{}\"\n", cipher));
            } else {
                out.push_str("  \"password_encrypted\": null\n");
            }
        } else {
            out.push_str("  \"password_encrypted\": null\n");
        }
        out.push('}');
        out
    }

    /// Parses profile from a JSON string representation.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut profile = ConnectionProfile::new("", "", "", 22, "");
        for line in json.lines() {
            let trimmed = line.trim().trim_end_matches(',');
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().trim_matches('"');
                let val = v.trim().trim_matches('"');
                match key {
                    "id" => profile.id = val.to_string(),
                    "name" => profile.name = val.to_string(),
                    "host" => profile.host = val.to_string(),
                    "port" => profile.port = val.parse().unwrap_or(22),
                    "username" => profile.username = val.to_string(),
                    "auth_method" => {
                        profile.auth_method = match val {
                            "Private Key" => AuthMethod::PrivateKey {
                                path: String::new(),
                                has_passphrase: false,
                            },
                            "Encrypted Key" => AuthMethod::EncryptedKey {
                                path: String::new(),
                            },
                            "SSH Agent" => AuthMethod::SshAgent,
                            _ => AuthMethod::Password,
                        };
                    }
                    "default_dir" => profile.default_dir = val.to_string(),
                    "connect_timeout_secs" => {
                        profile.connect_timeout_secs = val.parse().unwrap_or(10)
                    }
                    "read_timeout_secs" => profile.read_timeout_secs = val.parse().unwrap_or(15),
                    "keepalive_interval_secs" => {
                        profile.keepalive_interval_secs = val.parse().unwrap_or(30)
                    }
                    "max_retries" => {
                        profile.reconnect_policy.max_retries = val.parse().unwrap_or(5)
                    }
                    "initial_delay_ms" => {
                        profile.reconnect_policy.initial_delay_ms = val.parse().unwrap_or(2000)
                    }
                    "max_delay_ms" => {
                        profile.reconnect_policy.max_delay_ms = val.parse().unwrap_or(30000)
                    }
                    "last_connected" => profile.last_connected = val.parse().ok(),
                    "password_encrypted" if val != "null" && !val.is_empty() => {
                        profile.password_encrypted = Some(val.to_string());
                    }
                    _ => {}
                }
            }
        }
        if profile.id.is_empty() {
            return Err("Missing profile id".to_string());
        }
        Ok(profile)
    }
}

pub struct ConnectionProfileManager {
    profiles: Vec<ConnectionProfile>,
    config_path: PathBuf,
}

impl ConnectionProfileManager {
    pub fn new(config_dir: &Path) -> Self {
        let config_path = config_dir.join("profiles.json");
        let mut mgr = Self {
            profiles: Vec::new(),
            config_path,
        };
        let _ = mgr.load();
        mgr
    }

    pub fn list_profiles(&self) -> &[ConnectionProfile] {
        &self.profiles
    }

    pub fn get_profile(&self, id: &str) -> Option<&ConnectionProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn get_profile_mut(&mut self, id: &str) -> Option<&mut ConnectionProfile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    pub fn add_profile(&mut self, profile: ConnectionProfile) -> Result<(), String> {
        if self.profiles.iter().any(|p| p.id == profile.id) {
            return Err(format!("Profile ID '{}' already exists", profile.id));
        }
        self.profiles.push(profile);
        self.save()
    }

    pub fn update_profile(&mut self, profile: ConnectionProfile) -> Result<(), String> {
        if let Some(pos) = self.profiles.iter().position(|p| p.id == profile.id) {
            self.profiles[pos] = profile;
            self.save()
        } else {
            Err(format!("Profile ID '{}' not found", profile.id))
        }
    }

    pub fn remove_profile(&mut self, id: &str) -> Result<bool, String> {
        if let Some(pos) = self.profiles.iter().position(|p| p.id == id) {
            self.profiles.remove(pos);
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn duplicate_profile(&mut self, id: &str) -> Result<String, String> {
        let orig = self
            .get_profile(id)
            .ok_or_else(|| "Profile not found".to_string())?
            .clone();
        let new_id = format!("{}_copy", orig.id);
        let new_name = format!("{} (Copy)", orig.name);
        let mut dup = orig;
        dup.id = new_id.clone();
        dup.name = new_name;
        self.add_profile(dup)?;
        Ok(new_id)
    }

    pub fn save(&self) -> Result<(), String> {
        let mut json = String::from("[\n");
        for (i, p) in self.profiles.iter().enumerate() {
            let p_json = p.to_json(true);
            json.push_str("  ");
            json.push_str(&p_json.replace('\n', "\n  "));
            if i + 1 < self.profiles.len() {
                json.push(',');
            }
            json.push('\n');
        }
        json.push(']');

        if let Some(parent) = self.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&self.config_path, json).map_err(|e| format!("Save profiles error: {}", e))
    }

    pub fn load(&mut self) -> Result<(), String> {
        if !self.config_path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| format!("Read profiles error: {}", e))?;

        // Simple JSON array of objects parser
        self.profiles.clear();
        let mut depth = 0;
        let mut current_obj = String::new();

        for ch in content.chars() {
            if ch == '{' {
                depth += 1;
                current_obj.push(ch);
            } else if ch == '}' {
                depth -= 1;
                current_obj.push(ch);
                if depth == 0 && !current_obj.is_empty() {
                    if let Ok(p) = ConnectionProfile::from_json(&current_obj) {
                        self.profiles.push(p);
                    }
                    current_obj.clear();
                }
            } else if depth > 0 {
                current_obj.push(ch);
            }
        }

        Ok(())
    }

    /// Exports all profiles to a JSON string, stripping encrypted credentials by default.
    pub fn export_profiles(&self, include_secrets: bool) -> String {
        let mut json = String::from("[\n");
        for (i, p) in self.profiles.iter().enumerate() {
            let p_json = p.to_json(include_secrets);
            json.push_str("  ");
            json.push_str(&p_json.replace('\n', "\n  "));
            if i + 1 < self.profiles.len() {
                json.push(',');
            }
            json.push('\n');
        }
        json.push(']');
        json
    }
}
