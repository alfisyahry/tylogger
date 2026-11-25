use std::path::PathBuf;

/// Application User Model ID for Windows Taskbar grouping
pub const APP_ID: &str = "Tylogger";

/// Default application window class name
pub const WINDOW_CLASS_NAME: &str = "TyloggerWindowClass";

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
}
