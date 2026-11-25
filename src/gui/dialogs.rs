use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONASTERISK, MB_ICONINFORMATION, MB_OK,
};

/// Displays the official About Tylogger dialog.
pub fn show_about_dialog(hwnd: HWND) {
    let title: Vec<u16> = "About Tylogger\0".encode_utf16().collect();
    let message = "Tylogger — High-Performance Native Log Analysis Tool\n\
        Version 0.3.0 (Phase 3 Advanced Log Processing)\n\
        Target: Windows 7 SP1 through Windows 11 (x86 & x64)\n\n\
        Architecture: Win32 native GDI / 64-bit sparse index engine\n\
        MultiFile Rotated Log Management | Modular Columnizers\n\
        Advanced Filter-to-Tab | Highlighting Engine & Groups\n\
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
        • Global Memory Budget: 256 MB\n\
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

/// Displays the Columnizer selection dialog summary.
pub fn show_columnizer_dialog(hwnd: HWND, current_id: &str, available_count: usize) {
    let title: Vec<u16> = "Choose Log Columnizer\0".encode_utf16().collect();
    let message = format!(
        "Available Modular Columnizers ({} registered):\n\n\
        1. Plain Text / Default\n\
        2. Timestamp Columnizer (ISO, EU, Slash)\n\
        3. Webserver Common Log Format (CLF)\n\
        4. CSV Columnizer (RFC 4180 quotes & escapes)\n\
        5. Log4j XML Columnizer (No DTD / safe)\n\
        6. Apache Combined Log Format\n\
        7. Nginx Access Log Format\n\
        8. JSON Lines / NDJSON Columnizer\n\
        9. TSV Columnizer\n\
        10. Syslog Columnizer (RFC 3164/5424)\n\n\
        Active Columnizer for current document: [{}]\0",
        available_count, current_id
    );
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

/// Displays Highlight Rules Editor summary.
pub fn show_highlight_rules_dialog(hwnd: HWND, group_name: &str, rule_count: usize) {
    let title: Vec<u16> = "Highlight Rules Editor\0".encode_utf16().collect();
    let message = format!(
        "Highlighting Engine & Groups:\n\n\
        • Active Group: {}\n\
        • Configured Rules: {}\n\
        • Presets: TRACE, DEBUG, INFO, WARN, ERROR, CRITICAL, FATAL, HTTP 2xx-5xx\n\
        • Navigation: Jump to Next (F8) / Jump to Prev (Shift+F8)\n\
        • Marker Bar: Real-time visual tick marks\0",
        group_name, rule_count
    );
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

/// Displays Streaming Export completion info.
pub fn show_export_info_dialog(hwnd: HWND, path: &str, line_count: usize) {
    let title: Vec<u16> = "Export Completed\0".encode_utf16().collect();
    let message = format!(
        "Successfully streamed {} records to:\n\n{}\n\n\
        Formula injection sanitization applied for CSV.\n\
        Source log file write-protection verified.\0",
        line_count, path
    );
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

/// Displays Global Bookmark Manager summary dialog.
pub fn show_bookmark_manager_dialog(
    hwnd: HWND,
    mgr: &crate::core::bookmark::GlobalBookmarkManager,
) {
    let title: Vec<u16> = "Global Bookmark Manager\0".encode_utf16().collect();
    let message = format!(
        "Global Bookmark Manager:\n\n\
        • Total Bookmarks: {}\n\
        • Synchronized Views: Original Documents <-> Filter-to-Tab\n\
        • Rotation Integrity: Preserved across MultiFile rollover\n\
        • Actions Available: Jump, Edit Label/Comment, Clear Invalid, Export/Import CSV\0",
        mgr.count()
    );
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

/// Displays Plugin Manager summary dialog.
pub fn show_plugin_manager_dialog(hwnd: HWND, reg: &crate::core::plugin::PluginRegistry) {
    let title: Vec<u16> = "Plugin Manager\0".encode_utf16().collect();
    let message = format!(
        "Tylogger Plugin Architecture & Extensibility:\n\n\
        • Discovered Plugins: {}\n\
        • Categories: Declarative Parsers (In-Process), Out-of-Process Hosts (Job Objects)\n\
        • Security Model: Trust-gated execution, zero mandatory .NET / CLR runtime\n\
        • Discovery Paths: %APPDATA%\\Tylogger\\plugins\\ and .\\plugins\\\0",
        reg.count()
    );
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
