use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

/// 64-bit boundary threshold (2^32 = 4,294,967,296 bytes).
pub const BOUNDARY_4GB: u64 = 4 * 1024 * 1024 * 1024;

/// Generates deterministic synthetic log records of realistic variable lengths.
pub fn generate_deterministic_record(index: usize) -> String {
    let severities = ["INFO", "DEBUG", "WARN", "ERROR"];
    let sev = severities[index % severities.len()];
    let second = (index % 60) as u32;
    let minute = ((index / 60) % 60) as u32;
    let hour = ((index / 3600) % 24) as u32;
    format!(
        "2026-11-25 {:02}:{:02}:{:02}.{:03} [{}] [WorkerThread-{}] Event payload record #{} status=ok latency_us={}",
        hour,
        minute,
        second,
        index % 1000,
        sev,
        index % 16,
        index,
        (index * 13) % 50000
    )
}

/// Creates a test log file crossing the 4GB (2^32 byte) boundary deterministically
/// using sparse file seeking (SetFilePointerEx / File::seek) to test 64-bit offsets (AG-21, RSK-28).
pub fn create_64bit_boundary_test_file(path: &Path) -> std::io::Result<u64> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    // 1. Write header records near 0 offset
    writeln!(file, "2026-11-25 00:00:00.000 [INFO] System Initialized at offset 0")?;
    writeln!(file, "2026-11-25 00:00:01.000 [INFO] Sparse 64-bit boundary testing begins")?;

    // 2. Seek across 2^32 boundary (4GB + 1024 bytes)
    let target_offset = BOUNDARY_4GB + 1024;
    file.seek(SeekFrom::Start(target_offset))?;

    // 3. Write records specifically crossing 2^32
    writeln!(
        file,
        "2026-11-25 12:00:00.000 [WARN] Record located precisely at offset > 4GB: {}",
        target_offset
    )?;
    writeln!(
        file,
        "2026-11-25 12:00:01.000 [INFO] Record spanning across 64-bit virtual boundary successfully"
    )?;

    file.flush()?;
    let total_len = file.metadata()?.len();
    Ok(total_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::reader::NonLockingLogReader;

    #[test]
    fn test_deterministic_record_generation() {
        let r1 = generate_deterministic_record(0);
        let r2 = generate_deterministic_record(1);
        assert!(r1.contains("[INFO]"));
        assert!(r2.contains("[DEBUG]"));
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_64bit_boundary_file_and_sparse_reader() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("tylogger_64bit_boundary_{}.log", std::process::id()));

        let file_size = create_64bit_boundary_test_file(&test_file).expect("Create 64-bit boundary file");
        assert!(file_size > BOUNDARY_4GB, "File size must cross 4GB boundary");

        let mut reader = NonLockingLogReader::open(&test_file).expect("Open 64-bit reader");
        assert!(reader.file_size() > BOUNDARY_4GB);
        assert!(reader.line_count() >= 2);

        // Verify line 0 (near offset 0)
        let line_0 = reader.get_line(0).expect("Read line 0");
        assert!(line_0.contains("System Initialized at offset 0"));

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }
}
