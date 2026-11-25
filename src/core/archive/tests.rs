#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use std::io::Write;

    use crate::core::archive::gzip::{check_zip_bomb, decompress_gzip, TempFileGuard};
    use crate::core::archive::zip::validate_safe_zip_entry_path;

    #[test]
    fn test_temp_file_guard_raii_cleanup() {
        let path;
        {
            let guard = TempFileGuard::new("test_raii").expect("Create guard");
            path = guard.path().to_path_buf();
            let mut f = std::fs::File::create(&path).expect("Create temp file");
            writeln!(f, "Temporary extracted data").expect("Write data");
            assert!(path.exists());
        }
        // Guard dropped here, file should be automatically deleted!
        assert!(
            !path.exists(),
            "Temporary file must be deleted upon TempFileGuard drop"
        );
    }

    #[test]
    fn test_zip_path_traversal_prevention() {
        // Safe relative names
        assert!(validate_safe_zip_entry_path("logs/access.log").is_ok());
        assert!(validate_safe_zip_entry_path("app_2026.log").is_ok());

        // Dangerous path traversal attempts
        assert!(validate_safe_zip_entry_path("../../etc/passwd").is_err());
        assert!(validate_safe_zip_entry_path("..\\..\\Windows\\System32\\cmd.exe").is_err());
        assert!(validate_safe_zip_entry_path("C:\\Windows\\win.ini").is_err());
        assert!(validate_safe_zip_entry_path("/etc/shadow").is_err());
        assert!(validate_safe_zip_entry_path("\\SystemRoot\\test").is_err());
    }

    #[test]
    fn test_gzip_synthetic_stored_block_decompression() {
        // Minimal RFC 1952 GZIP container wrapping uncompressed/stored Deflate block:
        // Header: 1F 8B (magic), 08 (Deflate), 00 (flags), 00 00 00 00 (mtime), 00 (xfl), 03 (OS=Unix)
        // Deflate Block: BFINAL=1, BTYPE=00 (Stored):
        // 01 (BFINAL=1, BTYPE=00)
        // 05 00 (LEN=5), FA FF (NLEN=~5)
        // "HELLO" (48 45 4C 4C 4F)
        // Trailer: CRC32 (4 bytes), ISIZE (4 bytes)
        let gzip_data: Vec<u8> = vec![
            0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // Header (10 bytes)
            0x01, 0x05, 0x00, 0xFA, 0xFF, // Deflate Stored header (len 5)
            b'H', b'E', b'L', b'L', b'O', // Payload
            0x00, 0x00, 0x00, 0x00, // CRC32 placeholder
            0x05, 0x00, 0x00, 0x00, // ISIZE=5
        ];

        let decompressed =
            decompress_gzip(&gzip_data, gzip_data.len() as u64).expect("Decompress stored GZIP");
        assert_eq!(decompressed, b"HELLO");
    }

    #[test]
    fn test_anti_zip_bomb_detection() {
        // Normal: 10KB -> 100KB (10:1 ratio)
        assert!(check_zip_bomb(100 * 1024, 10 * 1024).is_ok());

        // Archive bomb: 1KB -> 50MB (50000:1 ratio and > 10MB)
        assert!(check_zip_bomb(50 * 1024 * 1024, 1024).is_err());

        // Extreme size: > 2GB limit
        assert!(check_zip_bomb(3 * 1024 * 1024 * 1024, 100 * 1024 * 1024).is_err());
    }
}
