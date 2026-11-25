#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::time::Instant;
    use crate::core::reader::{NonLockingLogReader, UpdateEvent};

    #[test]
    fn test_high_volume_synthetic_ingestion() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("tylogger_ingest_{}.log", std::process::id()));

        // Create initial file
        {
            let mut f = std::fs::File::create(&test_file).unwrap();
            writeln!(f, "2026-11-25 00:00:00 [INIT] Producer started").unwrap();
        }

        let mut reader = NonLockingLogReader::open(&test_file).expect("Open reader for ingestion");
        assert_eq!(reader.line_count(), 1);

        let mut appender = OpenOptions::new()
            .append(true)
            .open(&test_file)
            .expect("Open appender");

        // Simulate high-volume burst ingestion: 10,000 lines (AG-23)
        let total_burst_lines = 10_000;
        let start = Instant::now();

        for i in 1..=total_burst_lines {
            writeln!(
                appender,
                "2026-11-25 12:00:{:02}.{:03} [INFO] [SyntheticProducer] High-rate ingested record #{}",
                (i / 1000) % 60,
                i % 1000,
                i
            )
            .unwrap();
        }
        appender.flush().unwrap();

        let elapsed = start.elapsed();
        let update = reader.check_for_updates().expect("Check for appended updates");
        match update {
            UpdateEvent::Appended { new_lines } => {
                assert_eq!(new_lines, total_burst_lines);
            }
            _ => panic!("Expected UpdateEvent::Appended"),
        }

        assert_eq!(reader.line_count(), total_burst_lines + 1);

        // Verify random access retrieval under high volume
        let mid_line = reader.get_line(5000).expect("Read middle line");
        assert!(mid_line.contains("record #5000"));

        let last_line = reader.get_line(10000).expect("Read last line");
        assert!(last_line.contains("record #10000"));

        let _ = std::fs::remove_file(&test_file);

        // Ingestion rate throughput check (> 1,000 lines/sec)
        let lines_per_sec = (total_burst_lines as f64) / elapsed.as_secs_f64().max(0.001);
        assert!(lines_per_sec >= 1000.0, "Throughput must exceed 1000 lines/sec (actual: {:.1})", lines_per_sec);
    }
}
