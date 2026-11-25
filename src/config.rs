use std::path::PathBuf;

/// Application User Model ID for Windows Taskbar grouping
pub const APP_ID: &str = "Tylogger";

/// Default application window class name
pub const WINDOW_CLASS_NAME: &str = "TyloggerWindowClass";

/// Persistent user configuration for Tylogger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyloggerConfig {
    pub theme: String,
    pub font_name: String,
    pub font_size: i32,
    pub row_height: i32,
    pub follow_tail_default: bool,
    pub cache_budget_mb: usize,
    pub poll_interval_ms: u32,
    pub portable_mode: bool,
}

impl Default for TyloggerConfig {
    fn default() -> Self {
        Self {
            theme: "Soft Dark".to_string(),
            font_name: "Consolas".to_string(),
            font_size: 13,
            row_height: 20,
            follow_tail_default: true,
            cache_budget_mb: 64,
            poll_interval_ms: 250,
            portable_mode: false,
        }
    }
}

impl TyloggerConfig {
    /// Loads configuration from the config file, returning defaults if not found or corrupted.
    pub fn load() -> Self {
        let dir = get_config_dir();
        let config_file = dir.join("config.json");
        if !config_file.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&config_file) {
            Ok(content) => Self::parse_json(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Saves configuration to disk using an atomic replace technique.
    pub fn save(&self) -> Result<(), String> {
        let dir = get_config_dir();
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
        }

        let target_file = dir.join("config.json");
        let temp_file = dir.join("config.json.tmp");

        let json = self.to_json();
        std::fs::write(&temp_file, json)
            .map_err(|e| format!("Failed to write temp config: {}", e))?;

        // Atomic swap / replace
        if target_file.exists() {
            let _ = std::fs::remove_file(&target_file);
        }
        std::fs::rename(&temp_file, &target_file)
            .map_err(|e| format!("Failed to atomically rename config file: {}", e))?;

        Ok(())
    }

    /// Serializes settings to formatted JSON string.
    pub fn to_json(&self) -> String {
        format!(
            "{{\n  \"theme\": \"{}\",\n  \"font_name\": \"{}\",\n  \"font_size\": {},\n  \"row_height\": {},\n  \"follow_tail_default\": {},\n  \"cache_budget_mb\": {},\n  \"poll_interval_ms\": {},\n  \"portable_mode\": {}\n}}\n",
            self.theme, self.font_name, self.font_size, self.row_height,
            self.follow_tail_default, self.cache_budget_mb, self.poll_interval_ms, self.portable_mode
        )
    }

    /// Defensive JSON parser for Tylogger preferences without heavy external dependencies.
    pub fn parse_json(json: &str) -> Option<Self> {
        let mut cfg = Self::default();

        for line in json.lines() {
            let trimmed = line.trim().trim_end_matches(',');
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().trim_matches('"');
                let val = v.trim().trim_matches('"');

                match key {
                    "theme" => cfg.theme = val.to_string(),
                    "font_name" => cfg.font_name = val.to_string(),
                    "font_size" => {
                        if let Ok(num) = val.parse::<i32>() {
                            cfg.font_size = num.clamp(9, 32);
                        }
                    }
                    "row_height" => {
                        if let Ok(num) = val.parse::<i32>() {
                            cfg.row_height = num.clamp(14, 48);
                        }
                    }
                    "follow_tail_default" => {
                        if let Ok(b) = val.parse::<bool>() {
                            cfg.follow_tail_default = b;
                        }
                    }
                    "cache_budget_mb" => {
                        if let Ok(num) = val.parse::<usize>() {
                            cfg.cache_budget_mb = num.clamp(8, 2048);
                        }
                    }
                    "poll_interval_ms" => {
                        if let Ok(num) = val.parse::<u32>() {
                            cfg.poll_interval_ms = num.clamp(50, 5000);
                        }
                    }
                    "portable_mode" => {
                        if let Ok(b) = val.parse::<bool>() {
                            cfg.portable_mode = b;
                        }
                    }
                    _ => {}
                }
            }
        }

        Some(cfg)
    }
}

/// Returns the configuration directory for Tylogger.
/// Follows standard portable mode priority:
/// 1. Portable: If `.\config\` exists beside the executable or current dir, use it.
/// 2. Installed: `%APPDATA%\Tylogger`.
pub fn get_config_dir() -> PathBuf {
    // 1. Portable check: relative to current executable or working dir
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let portable_cfg = exe_dir.join("config");
            if portable_cfg.exists() && portable_cfg.is_dir() {
                return portable_cfg;
            }
        }
    }

    let local_cfg = PathBuf::from("config");
    if local_cfg.exists() && local_cfg.is_dir() {
        return local_cfg;
    }

    // 2. Standard %APPDATA%\Tylogger
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("Tylogger");
        return p;
    }

    // Fallback if APPDATA is unset
    PathBuf::from(".tylogger")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_id_and_class_name() {
        assert_eq!(APP_ID, "Tylogger");
        assert_eq!(WINDOW_CLASS_NAME, "TyloggerWindowClass");
    }

    #[test]
    fn test_config_dir_resolution() {
        let cfg = get_config_dir();
        assert!(!cfg.as_os_str().is_empty());
    }

    #[test]
    fn test_config_serialization_and_parsing() {
        let original = TyloggerConfig {
            theme: "Dark".to_string(),
            font_size: 15,
            cache_budget_mb: 128,
            follow_tail_default: false,
            ..Default::default()
        };

        let json = original.to_json();
        let parsed = TyloggerConfig::parse_json(&json).expect("Parse JSON");
        assert_eq!(parsed, original);
    }
}
