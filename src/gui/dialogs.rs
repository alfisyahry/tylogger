use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONASTERISK, MB_ICONINFORMATION, MB_OK,
};

/// Displays the official About Tylogger dialog.
pub fn show_about_dialog(hwnd: HWND) {
    let title: Vec<u16> = "About Tylogger\0".encode_utf16().collect();
    let message = "Tylogger — High-Performance Native Log Analysis Tool\n\
        Version 0.2.0 (Phase 2 Native Desktop Experience)\n\
        Target: Windows 7 SP1 through Windows 11 (x86 & x64)\n\n\
        Architecture: Win32 native GDI / 64-bit sparse index engine\n\
        Zero CLR/.NET dependency | Zero browser/WebView runtime\n\n\
        Copyright (c) 2026 Tylogger Contributors.\n\
        Based on LogExpert architecture (MIT License).\n\
        Upstream commit: a147ed316cca14f4d90a41186079a116f91ef9ee\0"
        .to_string();
    let msg_wide: Vec<u16> = message.encode_utf16().collect();

    unsafe {
        MessageBoxW(
            hwnd,
            msg_wide.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

/// Displays the Settings summary dialog with current preferences.
pub fn show_settings_info_dialog(
    hwnd: HWND,
    theme_name: &str,
    font_name: &str,
    font_size: i32,
    cache_mb: usize,
) {
    let title: Vec<u16> = "Tylogger Preferences\0".encode_utf16().collect();
    let message = format!(
        "Current Preferences:\n\n\
        • Active Theme: {}\n\
        • Interface Font: {}\n\
        • Log Font Size: {} DIP\n\
        • LRU Cache Byte Budget: {} MB\n\
        • Follow Tail Default: Enabled\n\n\
        Configuration is saved atomically to %APPDATA%\\Tylogger\\config.json\0",
        theme_name, font_name, font_size, cache_mb
    );
    let msg_wide: Vec<u16> = message.encode_utf16().collect();

    unsafe {
        MessageBoxW(
            hwnd,
            msg_wide.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONASTERISK,
        );
    }
}

/// Parses and validates a line number for Jump to Line navigation.
pub fn parse_jump_target(input: &str, max_lines: usize) -> Option<usize> {
    if let Ok(num) = input.trim().parse::<usize>() {
        if num >= 1 && num <= max_lines {
            return Some(num - 1);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jump_target_validation() {
        assert_eq!(parse_jump_target("42", 100), Some(41));
        assert_eq!(parse_jump_target("1", 100), Some(0));
        assert_eq!(parse_jump_target("100", 100), Some(99));
        assert_eq!(parse_jump_target("0", 100), None);
        assert_eq!(parse_jump_target("101", 100), None);
        assert_eq!(parse_jump_target("abc", 100), None);
    }
}
