use std::fs;
use std::path::{Path, PathBuf};

/// Plugin category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PluginCategory {
    #[default]
    Columnizer,
    KeywordAction,
    ContextMenuAction,
}

/// Security and trust status of an external plugin.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PluginTrustState {
    #[default]
    Disabled,
    LocalApprovalRequired,
    Trusted,
    Error(String),
}

/// Metadata manifest of an external Tylogger plugin (`manifest.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub category: PluginCategory,
    pub api_version: String,
    pub min_app_version: String,
    pub package_dir: PathBuf,
    pub columnizer_config: Option<String>,
    pub executable_path: Option<String>,
    pub enabled: bool,
    pub trust_state: PluginTrustState,
}

impl PluginManifest {
    pub fn parse_manifest(package_dir: &Path, content: &str) -> Result<Self, String> {
        if content.len() > 65_536 {
            return Err("Manifest exceeds maximum size limit of 64KB".to_string());
        }

        let id = extract_val(content, "\"id\":")
            .ok_or_else(|| "Missing required 'id' field in manifest.json".to_string())?;
        let name = extract_val(content, "\"name\":")
            .ok_or_else(|| "Missing required 'name' field in manifest.json".to_string())?;
        let version = extract_val(content, "\"version\":").unwrap_or_else(|| "1.0.0".to_string());
        let author = extract_val(content, "\"author\":").unwrap_or_else(|| "Unknown".to_string());
        let desc = extract_val(content, "\"description\":").unwrap_or_default();
        let api_ver = extract_val(content, "\"api_version\":").unwrap_or_else(|| "1.0".to_string());
        let min_app =
            extract_val(content, "\"min_app_version\":").unwrap_or_else(|| "0.4.0".to_string());

        let cat_str =
            extract_val(content, "\"category\":").unwrap_or_else(|| "columnizer".to_string());
        let category = match cat_str.to_lowercase().as_str() {
            "keyword_action" | "keyword" => PluginCategory::KeywordAction,
            "context_menu" | "context" => PluginCategory::ContextMenuAction,
            _ => PluginCategory::Columnizer,
        };

        let col_cfg = extract_val(content, "\"columnizer_config\":");
        let exe_path = extract_val(content, "\"executable\":");

        // Executable plugins require local user approval by default
        let trust_state = if exe_path.is_some() {
            PluginTrustState::LocalApprovalRequired
        } else {
            PluginTrustState::Trusted
        };

        let is_declarative = exe_path.is_none();

        Ok(Self {
            id,
            name,
            version,
            author,
            description: desc,
            category,
            api_version: api_ver,
            min_app_version: min_app,
            package_dir: package_dir.to_path_buf(),
            columnizer_config: col_cfg,
            executable_path: exe_path,
            enabled: is_declarative, // Declarative plugins enabled by default
            trust_state,
        })
    }
}

/// Centralized Plugin Registry managing external plugins and security policies.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    plugins: Vec<PluginManifest>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    pub fn all(&self) -> &[PluginManifest] {
        &self.plugins
    }

    pub fn get(&self, id: &str) -> Option<&PluginManifest> {
        self.plugins.iter().find(|p| p.id == id)
    }

    pub fn register(&mut self, manifest: PluginManifest) -> Result<(), String> {
        if self.plugins.iter().any(|p| p.id == manifest.id) {
            return Err(format!(
                "Plugin with id '{}' is already registered",
                manifest.id
            ));
        }
        self.plugins.push(manifest);
        Ok(())
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
            p.enabled = enabled;
            if enabled && p.trust_state == PluginTrustState::Disabled {
                p.trust_state = PluginTrustState::Trusted;
            } else if !enabled {
                p.trust_state = PluginTrustState::Disabled;
            }
            true
        } else {
            false
        }
    }

    /// Safely discovers external plugin packages under `base_dir` by inspecting `manifest.json`.
    /// strictly reads JSON metadata without executing any binaries.
    pub fn discover_directory(&mut self, base_dir: &Path) -> usize {
        if !base_dir.exists() || !base_dir.is_dir() {
            return 0;
        }

        let mut discovered = 0;
        if let Ok(entries) = fs::read_dir(base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_file = path.join("manifest.json");
                    if manifest_file.exists() && manifest_file.is_file() {
                        if let Ok(content) = fs::read_to_string(&manifest_file) {
                            match PluginManifest::parse_manifest(&path, &content) {
                                Ok(manifest) => {
                                    if self.register(manifest).is_ok() {
                                        discovered += 1;
                                    }
                                }
                                Err(e) => {
                                    // Record malformed plugin with error badge without crashing
                                    let dir_name = path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string();
                                    let err_manifest = PluginManifest {
                                        id: format!("invalid.{}", dir_name),
                                        name: format!("Error: {}", dir_name),
                                        version: "0.0.0".to_string(),
                                        author: "Unknown".to_string(),
                                        description: e.clone(),
                                        category: PluginCategory::Columnizer,
                                        api_version: "1.0".to_string(),
                                        min_app_version: "0.4.0".to_string(),
                                        package_dir: path.clone(),
                                        columnizer_config: None,
                                        executable_path: None,
                                        enabled: false,
                                        trust_state: PluginTrustState::Error(e),
                                    };
                                    let _ = self.register(err_manifest);
                                }
                            }
                        }
                    }
                }
            }
        }

        discovered
    }
}

fn extract_val(json: &str, key: &str) -> Option<String> {
    if let Some(pos) = json.find(key) {
        let rem = &json[pos + key.len()..];
        if let Some(first_quote) = rem.find('"') {
            if let Some(end_quote) = rem[first_quote + 1..].find('"') {
                return Some(
                    rem[first_quote + 1..first_quote + 1 + end_quote].replace("\\\\", "\\"),
                );
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_discovery_and_metadata_validation() {
        let temp_dir =
            std::env::temp_dir().join(format!("tylogger_plugin_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let valid_pkg = temp_dir.join("apache-parser");
        let _ = fs::create_dir_all(&valid_pkg);
        let manifest_content = r#"{
  "id": "com.tylogger.apache",
  "name": "Apache Parser",
  "version": "1.2.0",
  "author": "LogExperts",
  "category": "columnizer",
  "columnizer_config": "columnizer.json"
}"#;
        fs::write(valid_pkg.join("manifest.json"), manifest_content).unwrap();

        // Malformed package
        let malformed_pkg = temp_dir.join("broken-parser");
        let _ = fs::create_dir_all(&malformed_pkg);
        fs::write(malformed_pkg.join("manifest.json"), "invalid json content").unwrap();

        let mut reg = PluginRegistry::new();
        let found = reg.discover_directory(&temp_dir);
        assert_eq!(found, 1);
        assert_eq!(reg.count(), 2); // 1 valid + 1 error entry

        let plugin = reg.get("com.tylogger.apache").unwrap();
        assert_eq!(plugin.name, "Apache Parser");
        assert_eq!(plugin.trust_state, PluginTrustState::Trusted);
        assert!(plugin.enabled);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
