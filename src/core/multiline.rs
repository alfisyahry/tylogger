/// Represents a single logical log record that may span multiple physical lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalRecord {
    pub record_idx: usize,
    pub start_line: usize,
    pub end_line: usize, // inclusive
}

impl LogicalRecord {
    pub fn line_count(&self) -> usize {
        self.end_line - self.start_line + 1
    }
}

/// Engine that handles multiline record grouping (Java stack traces, Python tracebacks, etc.)
/// and maps between physical line indices and logical record indices.
#[derive(Debug, Default)]
pub struct MultilineEngine {
    records: Vec<LogicalRecord>,
    line_to_record: Vec<usize>,
}

impl MultilineEngine {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            line_to_record: Vec::new(),
        }
    }

    /// Evaluates whether a physical line is a continuation line of a previous record.
    pub fn is_continuation_line(line: &str) -> bool {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            return false;
        }

        // 1. Java stack trace continuation:
        //    \tat com.example.MyClass.method(MyClass.java:123)
        //    \t... 15 more
        //    Caused by: java.lang.NullPointerException
        if line.starts_with('\t') || line.starts_with("   at ") {
            return true;
        }
        if trimmed.starts_with("at ")
            || trimmed.starts_with("... ")
            || trimmed.starts_with("Caused by: ")
            || trimmed.starts_with("Suppressed: ")
        {
            return true;
        }

        // 2. Python traceback continuation:
        //    File "/app/main.py", line 42, in <module>
        //      func()
        if line.starts_with("  File \"") || line.starts_with("    ") {
            return true;
        }

        false
    }

    /// Builds logical record groupings from a list of physical log lines.
    pub fn index_lines(lines: &[&str]) -> Self {
        let mut records = Vec::new();
        let mut line_to_record = Vec::with_capacity(lines.len());

        if lines.is_empty() {
            return Self {
                records,
                line_to_record,
            };
        }

        let mut current_record_idx = 0;
        let mut record_start = 0;

        for (i, line) in lines.iter().enumerate() {
            if i > 0 && Self::is_continuation_line(line) {
                // Continuation line of the current record
                line_to_record.push(current_record_idx);
            } else {
                // New logical record begins
                if i > 0 {
                    records.push(LogicalRecord {
                        record_idx: current_record_idx,
                        start_line: record_start,
                        end_line: i - 1,
                    });
                    current_record_idx += 1;
                }
                record_start = i;
                line_to_record.push(current_record_idx);
            }
        }

        // Push the final record
        records.push(LogicalRecord {
            record_idx: current_record_idx,
            start_line: record_start,
            end_line: lines.len() - 1,
        });

        Self {
            records,
            line_to_record,
        }
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn get_record(&self, record_idx: usize) -> Option<&LogicalRecord> {
        self.records.get(record_idx)
    }

    pub fn physical_to_logical(&self, physical_line: usize) -> usize {
        if physical_line < self.line_to_record.len() {
            self.line_to_record[physical_line]
        } else {
            self.records.len().saturating_sub(1)
        }
    }

    /// Assembles the full multi-line text for a logical record.
    pub fn assemble_record_text<F>(&self, record_idx: usize, mut get_line: F) -> Option<String>
    where
        F: FnMut(usize) -> Option<String>,
    {
        let rec = self.records.get(record_idx)?;
        let mut buf = String::new();
        for line_idx in rec.start_line..=rec.end_line {
            if let Some(line) = get_line(line_idx) {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(&line);
            }
        }
        Some(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_stacktrace_grouping() {
        let lines = [
            "2026-11-25 10:00:00 [INFO] Starting request",
            "2026-11-25 10:00:01 [ERROR] Exception caught in handler",
            "\tat com.example.service.UserService.getUser(UserService.java:42)",
            "\tat com.example.controller.UserController.handle(UserController.java:18)",
            "Caused by: java.io.IOException: Connection reset",
            "\t... 12 more",
            "2026-11-25 10:00:02 [INFO] Request completed",
        ];

        let engine = MultilineEngine::index_lines(&lines);
        assert_eq!(engine.record_count(), 3);

        // Record 0: Line 0
        let r0 = engine.get_record(0).unwrap();
        assert_eq!(r0.start_line, 0);
        assert_eq!(r0.end_line, 0);
        assert_eq!(r0.line_count(), 1);

        // Record 1: Lines 1..5 (Error + 4 continuation lines)
        let r1 = engine.get_record(1).unwrap();
        assert_eq!(r1.start_line, 1);
        assert_eq!(r1.end_line, 5);
        assert_eq!(r1.line_count(), 5);

        // Record 2: Line 6
        let r2 = engine.get_record(2).unwrap();
        assert_eq!(r2.start_line, 6);
        assert_eq!(r2.end_line, 6);
        assert_eq!(r2.line_count(), 1);

        // Mapping
        assert_eq!(engine.physical_to_logical(0), 0);
        assert_eq!(engine.physical_to_logical(2), 1);
        assert_eq!(engine.physical_to_logical(5), 1);
        assert_eq!(engine.physical_to_logical(6), 2);

        // Assembly
        let text = engine
            .assemble_record_text(1, |idx| lines.get(idx).map(|s| s.to_string()))
            .unwrap();
        assert!(text.contains("Exception caught in handler"));
        assert!(text.contains("UserService.java:42"));
        assert!(text.contains("Caused by: java.io.IOException"));
    }
}
