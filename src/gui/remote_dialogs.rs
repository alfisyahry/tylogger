use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_ICONASTERISK, MB_ICONEXCLAMATION, MB_ICONINFORMATION, MB_ICONQUESTION,
    MB_OK, MB_YESNO,
};

use crate::core::archive::ZipMember;
use crate::core::legacy::LegacyImportSummary;
use crate::core::remote::client::RemoteDirEntry;
use crate::core::remote::profile::ConnectionProfile;

/// Displays the native Win32 Connection Manager dialog summary.
pub fn show_connection_manager_dialog(hwnd: HWND, profiles: &[ConnectionProfile]) {
    let title: Vec<u16> = "Connection Manager\0".encode_utf16().collect();
    let mut message = format!(
        "SSH & SFTP Remote Connection Profiles ({} configured):\n\n",
        profiles.len()
    );

    if profiles.is_empty() {
        message.push_str(
            "No remote connection profiles configured yet.\n\
            Click 'Add Profile' to configure a new SSH/SFTP server endpoint.\n\n\
            Credentials are encrypted via Windows user-scoped DPAPI.\n\
            Host-key verification is strictly enforced.\0",
        );
    } else {
        for (i, p) in profiles.iter().take(8).enumerate() {
            let pass_state = if p.password_encrypted.is_some() {
                "Saved (DPAPI)"
            } else {
                "Prompt"
            };
            message.push_str(&format!(
                "{}. [{}] {}@{}:{} (Auth: {}, Credential: {})\n",
                i + 1,
                p.name,
                p.username,
                p.host,
                p.port,
                p.auth_method.name(),
                pass_state
            ));
        }
        if profiles.len() > 8 {
            message.push_str(&format!("... and {} more profiles.\n", profiles.len() - 8));
        }
        message.push_str("\nActions: Connect | Browse Files | Add | Edit | Duplicate | Delete\0");
    }

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

/// Displays the strict SSH Host Key Verification Prompt on first connection (RSK-02).
/// Returns true if user explicitly accepts the host key.
pub fn show_host_key_verify_dialog(
    hwnd: HWND,
    host: &str,
    port: u16,
    key_type: &str,
    fingerprint_sha256: &str,
) -> bool {
    let title: Vec<u16> = "SSH Host Key Verification (New Host)\0"
        .encode_utf16()
        .collect();
    let message = format!(
        "The authenticity of host '{}:{}' can't be established.\n\n\
        Key algorithm: {}\n\
        SHA-256 Fingerprint:\n{}\n\n\
        Are you sure you want to continue connecting and record this key\n\
        in your trusted known_hosts store?\n\n\
        Press 'Yes' to accept and record, or 'No' to abort immediately.\0",
        host, port, key_type, fingerprint_sha256
    );
    let msg_wide: Vec<u16> = message.encode_utf16().collect();

    unsafe {
        let res = MessageBoxW(
            hwnd,
            msg_wide.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_ICONQUESTION,
        );
        res == IDYES
    }
}

/// Displays the Strict Host Key Mismatch Warning (MitM hazard detected).
pub fn show_host_key_mismatch_warning(
    hwnd: HWND,
    host: &str,
    recorded_fp: &str,
    received_fp: &str,
) {
    let title: Vec<u16> = "SECURITY WARNING: HOST KEY MISMATCH\0"
        .encode_utf16()
        .collect();
    let message = format!(
        "POTENTIAL SECURITY BREACH / MAN-IN-THE-MIDDLE HAZARD!\n\n\
        Host: {}\n\n\
        Recorded Fingerprint in known_hosts:\n{}\n\n\
        Received Fingerprint from Server:\n{}\n\n\
        The host key has changed since the previous connection.\n\
        Connection has been ABORTED immediately to protect your credentials.\n\
        Do not connect unless you have verified this key out-of-band with your server administrator.\0",
        host, recorded_fp, received_fp
    );
    let msg_wide: Vec<u16> = message.encode_utf16().collect();

    unsafe {
        MessageBoxW(
            hwnd,
            msg_wide.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONEXCLAMATION,
        );
    }
}

/// Displays the native Win32 SFTP Remote File Browser dialog summary.
pub fn show_remote_file_browser_dialog(hwnd: HWND, current_dir: &str, entries: &[&RemoteDirEntry]) {
    let title: Vec<u16> = "Remote File Browser (SFTP Read-Only)\0"
        .encode_utf16()
        .collect();
    let mut message = format!(
        "Remote Directory: {}\nListed Entries ({} files/folders):\n\n",
        current_dir,
        entries.len()
    );

    for (i, e) in entries.iter().take(10).enumerate() {
        let kind = if e.is_dir { "<DIR>" } else { "<FILE>" };
        let size_str = if e.is_dir {
            "-".to_string()
        } else {
            format!("{} B", e.size)
        };
        message.push_str(&format!(
            "{}. {} {:<24} {}\n",
            i + 1,
            kind,
            e.name,
            size_str
        ));
    }
    if entries.len() > 10 {
        message.push_str(&format!("... and {} more entries.\n", entries.len() - 10));
    }
    message.push_str("\nDouble-click a file to open as a remote log tab (Strictly Read-Only).\0");

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

/// Displays the native Win32 Archive Member Browser dialog summary.
pub fn show_archive_browser_dialog(hwnd: HWND, archive_name: &str, members: &[ZipMember]) {
    let title: Vec<u16> = "Archive Member Browser (ZIP / GZIP)\0"
        .encode_utf16()
        .collect();
    let mut message = format!(
        "Archive: {}\nTotal Members: {}\n\n",
        archive_name,
        members.len()
    );

    for (i, m) in members.iter().take(10).enumerate() {
        let kind = if m.is_dir { "<DIR>" } else { "<LOG>" };
        message.push_str(&format!(
            "{}. {} {:<24} ({} B uncompressed)\n",
            i + 1,
            kind,
            m.name,
            m.uncompressed_size
        ));
    }
    if members.len() > 10 {
        message.push_str(&format!("... and {} more members.\n", members.len() - 10));
    }
    message.push_str("\nSelect a member to open in a read-only tab (Anti-Zip-Bomb Protected).\0");

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

/// Displays the Legacy Import Summary dialog.
pub fn show_legacy_import_summary_dialog(hwnd: HWND, summary: &LegacyImportSummary) {
    let title: Vec<u16> = "LogExpert Legacy Import Report\0".encode_utf16().collect();
    let message = format!(
        "LogExpert Legacy Migration Completed:\n\n\
        • Session Name: {}\n\
        • Documents Imported: {}\n\
        • Bookmarks Restored: {}\n\
        • Filter Rules Imported: {}\n\
        • Highlight Rules Imported: {}\n\
        • Unsupported XML Elements Skipped: {}\n\n\
        All settings mapped safely without external DTD/XXE execution.\0",
        summary.session_title,
        summary.documents_imported,
        summary.bookmarks_imported,
        summary.filter_rules_imported,
        summary.highlight_rules_imported,
        summary.unsupported_elements_skipped
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
