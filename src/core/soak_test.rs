#[cfg(test)]
mod tests {
    use crate::core::reader::{NonLockingLogReader, UpdateEvent};
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::time::{Duration, Instant};

    #[derive(Debug)]
    #[allow(dead_code)]
    pub struct SoakMetrics {
        pub total_appends: usize,
        pub elapsed: Duration,
        pub memory_usage_bytes: usize,
    }

    #[test]
    fn test_sustained_tailing_soak_harness() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("tylogger_soak_{}.log", std::process::id()));

        {
            let mut f = std::fs::File::create(&test_file).unwrap();
            writeln!(f, "2026-11-25 00:00:00 [SOAK] Initial baseline record").unwrap();
        }

        let mut reader = NonLockingLogReader::open(&test_file).expect("Open reader");
        let start = Instant::now();
        let simulation_duration = Duration::from_millis(500); // Fast deterministic soak cycle in unit suite

        let mut appender = OpenOptions::new()
            .append(true)
            .open(&test_file)
            .expect("Open appender");

        let mut appends_count = 0;
        let mut cycle = 0;

        while start.elapsed() < simulation_duration {
            cycle += 1;
            for j in 0..50 {
                writeln!(
                    appender,
                    "2026-11-25 12:00:00.000 [INFO] [SoakTail] Cycle {} entry #{}",
                    cycle, j
                )
                .unwrap();
                appends_count += 1;
            }
            appender.flush().unwrap();

            let update = reader.check_for_updates().unwrap();
            match update {
                UpdateEvent::Appended { .. } => {}
                _ => panic!("Expected appended update event"),
            }

            // Read random lines to exercise LRU line cache under continuous tailing
            let count = reader.line_count();
            if count > 10 {
                let _ = reader.get_line(count / 2);
                let _ = reader.get_line(count - 1);
            }
        }

        let total_lines = reader.line_count();
        assert_eq!(total_lines, appends_count + 1);

        // Memory cache must remain strictly bounded within configured budget
        assert!(reader.cache_bytes() <= 64 * 1024 * 1024);

        let _ = std::fs::remove_file(&test_file);
    }
}
