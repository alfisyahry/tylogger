#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tempfile_crate_replacement::create_test_dir;

    use crate::core::reader::UpdateEvent;
    use crate::core::remote::browser::SftpFileBrowser;
    use crate::core::remote::client::MockRemoteTransport;
    use crate::core::remote::connection::{
        calculate_backoff_delay, BoundedConnectionPool, CancellationToken,
    };
    use crate::core::remote::credential::{
        base64_decode, base64_encode, protect_secret_dpapi, unprotect_secret_dpapi, CredentialStore,
    };
    use crate::core::remote::known_hosts::{HostKeyVerificationResult, KnownHostsStore};
    use crate::core::remote::multifile::RemoteMultiFileScanner;
    use crate::core::remote::profile::{ConnectionProfile, ConnectionProfileManager};
    use crate::core::remote::sftp_reader::SftpLogReader;
    use crate::core::source::LogSource;

    mod tempfile_crate_replacement {
        use std::path::PathBuf;
        pub fn create_test_dir(name: &str) -> PathBuf {
            let p =
                std::env::temp_dir().join(format!("tylogger_test_{}_{}", name, std::process::id()));
            let _ = std::fs::create_dir_all(&p);
            p
        }
    }

    #[test]
    fn test_profile_manager_lifecycle_and_export() {
        let test_dir = create_test_dir("profiles");
        let mut mgr = ConnectionProfileManager::new(&test_dir);

        let mut p1 = ConnectionProfile::new("srv1", "Production Web", "192.168.1.10", 22, "admin");
        p1.password_encrypted = Some("DPAPI:TESTCIPHER==".to_string());
        mgr.add_profile(p1).expect("Add profile");

        assert_eq!(mgr.list_profiles().len(), 1);
        let exported_safe = mgr.export_profiles(false);
        assert!(!exported_safe.contains("DPAPI:TESTCIPHER=="));
        assert!(exported_safe.contains("\"password_encrypted\": null"));

        let dup_id = mgr.duplicate_profile("srv1").expect("Duplicate");
        assert_eq!(dup_id, "srv1_copy");
        assert_eq!(mgr.list_profiles().len(), 2);

        assert!(mgr.remove_profile("srv1").unwrap());
        assert_eq!(mgr.list_profiles().len(), 1);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_credential_dpapi_and_store() {
        let store = CredentialStore::new();
        store.set_session_password("srv_temp", "secret123");
        assert_eq!(
            store.get_session_password("srv_temp"),
            Some("secret123".to_string())
        );
        store.clear();
        assert_eq!(store.get_session_password("srv_temp"), None);

        // Base64 helper test
        let raw = b"Hello Tylogger Remote Log Access!";
        let encoded = base64_encode(raw);
        let decoded = base64_decode(&encoded).expect("Decode base64");
        assert_eq!(decoded, raw);

        // DPAPI protection test
        let secret = "SuperSafeSSHPassword!#2026";
        let protected = protect_secret_dpapi(secret).expect("Protect DPAPI");
        assert_ne!(protected, secret);
        let recovered = unprotect_secret_dpapi(&protected).expect("Unprotect DPAPI");
        assert_eq!(recovered, secret);
    }

    #[test]
    fn test_known_hosts_strict_mitm_abort() {
        let test_dir = create_test_dir("known_hosts");
        let hosts_file = test_dir.join("known_hosts");
        let mut store = KnownHostsStore::new(&hosts_file);

        let host = "logserver.internal";
        let port = 22;
        let fp1 = "SHA256:U1BFQ0lBTFNIQTI1NkZJTkdFUlBSSU5UMTIzNA==";

        // 1. Initial check: NewHost
        assert_eq!(
            store.verify_host(host, port, "ssh-ed25519", fp1),
            HostKeyVerificationResult::NewHost
        );

        // 2. Record
        store
            .record_host(host, port, "ssh-ed25519", fp1)
            .expect("Record host");

        // 3. Match
        assert_eq!(
            store.verify_host(host, port, "ssh-ed25519", fp1),
            HostKeyVerificationResult::Match
        );

        // 4. Changed key -> Mismatch (MitM hazard)
        let fp2 = "SHA256:QUxURVJFRFNISVRFVEtFWUZJTkdFUlBSSU5UMTIzNA==";
        match store.verify_host(host, port, "ssh-ed25519", fp2) {
            HostKeyVerificationResult::Mismatch { recorded, actual } => {
                assert_eq!(recorded, fp1);
                assert_eq!(actual, fp2);
            }
            other => panic!("Expected Mismatch, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_backoff_and_pool_concurrency() {
        // Backoff: 2s, 4s, 8s, 16s, capped at 30s
        let d1 = calculate_backoff_delay(1, 2000, 30000, 2, false);
        assert_eq!(d1, 2000);
        let d2 = calculate_backoff_delay(2, 2000, 30000, 2, false);
        assert_eq!(d2, 4000);
        let d3 = calculate_backoff_delay(3, 2000, 30000, 2, false);
        assert_eq!(d3, 8000);
        let d4 = calculate_backoff_delay(4, 2000, 30000, 2, false);
        assert_eq!(d4, 16000);
        let d5 = calculate_backoff_delay(5, 2000, 30000, 2, false);
        assert_eq!(d5, 30000);

        // Connection pool
        let pool = BoundedConnectionPool::new(2);
        let lease1 = pool.acquire().expect("Acquire 1");
        assert_eq!(pool.active_connections(), 1);
        let lease2 = pool.acquire().expect("Acquire 2");
        assert_eq!(pool.active_connections(), 2);
        assert!(pool.acquire().is_err()); // Capacity exceeded!
        drop(lease1);
        assert_eq!(pool.active_connections(), 1);
        drop(lease2);
        assert_eq!(pool.active_connections(), 0);

        // Cancellation token
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_sftp_reader_and_tailing() {
        let transport = Arc::new(MockRemoteTransport::new());
        let sample = b"2026-11-25 [INFO] Log remote line 1\n2026-11-25 [INFO] Log remote line 2\n2026-11-25 [WARN] Log remote line 3\n";
        transport.add_file("/var/log/syslog", sample, 1700000000);

        let mut reader =
            SftpLogReader::open(transport.clone(), "srv1", "SHA256:abc", "/var/log/syslog")
                .expect("Open remote reader");

        assert_eq!(reader.line_count(), 3);
        assert!(reader.is_early_preview_ready());
        assert_eq!(
            reader.get_line(0).unwrap(),
            "2026-11-25 [INFO] Log remote line 1"
        );
        assert_eq!(
            reader.get_line(2).unwrap(),
            "2026-11-25 [WARN] Log remote line 3"
        );

        // Bookmark
        assert!(reader.toggle_bookmark(1));
        assert!(reader.is_bookmarked(1));

        // Test remote append
        transport.append_file(
            "/var/log/syslog",
            b"2026-11-25 [ERROR] Log remote line 4\n",
            1700000010,
        );
        let ev = reader.check_for_updates().expect("Check updates");
        assert_eq!(ev, UpdateEvent::Appended { new_lines: 1 });
        assert_eq!(reader.line_count(), 4);
        assert_eq!(
            reader.get_line(3).unwrap(),
            "2026-11-25 [ERROR] Log remote line 4"
        );

        // Test remote truncation
        transport.truncate_file(
            "/var/log/syslog",
            b"2026-11-25 [INFO] New truncated cycle line 1\n",
            1700000020,
        );
        let ev_trunc = reader.check_for_updates().expect("Check updates truncate");
        assert_eq!(ev_trunc, UpdateEvent::Truncated);
        assert_eq!(reader.line_count(), 1);
        assert_eq!(
            reader.get_line(0).unwrap(),
            "2026-11-25 [INFO] New truncated cycle line 1"
        );
    }

    #[test]
    fn test_remote_multifile_and_browser() {
        let transport = Arc::new(MockRemoteTransport::new());
        transport.add_dir("/var/log", 1700000000);
        transport.add_file("/var/log/app.log", b"active", 1700000050);
        transport.add_file("/var/log/app.log.1", b"rot1", 1700000040);
        transport.add_file("/var/log/app.log.2", b"rot2", 1700000030);

        let rotated = RemoteMultiFileScanner::discover_rotated_files(
            transport.clone(),
            "/var/log",
            "app.log",
        )
        .expect("Discover rotated");

        assert_eq!(rotated.len(), 3);
        // Ordered oldest to newest: app.log.2, app.log.1, app.log
        assert_eq!(rotated[0].filename, "app.log.2");
        assert_eq!(rotated[1].filename, "app.log.1");
        assert_eq!(rotated[2].filename, "app.log");

        // Browser
        let mut browser = SftpFileBrowser::new(transport.clone(), "/var/log");
        let entries = browser.filtered_entries();
        assert_eq!(entries.len(), 3);
        browser.set_search_query("log.1");
        assert_eq!(browser.filtered_entries().len(), 1);
        assert_eq!(browser.filtered_entries()[0].name, "app.log.1");
    }
}
