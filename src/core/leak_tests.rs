#[cfg(test)]
mod tests {
    use crate::core::reader::NonLockingLogReader;
    use crate::core::workspace::Workspace;
    use std::fs::File;
    use std::io::Write;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};

    #[test]
    fn test_1000x_open_close_zero_handle_leak() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("tylogger_leak_test_{}.log", std::process::id()));
        {
            let mut f = File::create(&test_file).unwrap();
            for i in 1..=50 {
                writeln!(f, "2026-11-25 10:00:00 [INFO] Line {}", i).unwrap();
            }
        }

        let process = unsafe { GetCurrentProcess() };
        let mut baseline_handles: u32 = 0;
        unsafe { GetProcessHandleCount(process, &mut baseline_handles) };

        // Warmup cycles
        for _ in 0..20 {
            let mut reader = NonLockingLogReader::open(&test_file).unwrap();
            let _ = reader.get_line(0);
        }

        unsafe { GetProcessHandleCount(process, &mut baseline_handles) };

        // Execute 1,000 open/read/close cycles (AG-19, RSK-20)
        for _ in 0..1000 {
            let mut reader = NonLockingLogReader::open(&test_file).unwrap();
            let _ = reader.get_line(10);
            let _ = reader.get_line(25);
        }

        let mut final_handles: u32 = 0;
        unsafe { GetProcessHandleCount(process, &mut final_handles) };

        let _ = std::fs::remove_file(&test_file);

        let drift = (final_handles as i64) - (baseline_handles as i64);
        // In multi-threaded test runners, worker thread pools fluctuate by ~20 handles.
        // A true leak across 1,000 open/close cycles would produce ~1,000 handles.
        assert!(
            drift <= 35,
            "Handle leak detected: baseline={}, final={}, drift={}",
            baseline_handles,
            final_handles,
            drift
        );
    }

    #[test]
    fn test_tab_cycle_zero_leak() {
        let mut ws = Workspace::new();
        for i in 0..100 {
            let title = format!("doc_{}.log", i);
            let idx = ws.add_mock_document(&title, &["Line A", "Line B"]);
            ws.close_document(idx);
        }
        assert_eq!(ws.document_count(), 0);
    }
}
