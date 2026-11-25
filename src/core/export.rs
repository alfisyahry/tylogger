use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::core::columnizer::Columnizer;
use crate::core::source::LogSource;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    PlainText,
    JsonLines,
}

pub struct StreamingExporter;

impl StreamingExporter {
    /// Streams lines from `source` to `destination_path` using `format`.
    /// `line_indices`: optional subset of line indices (e.g. from filter or selection).
    /// If None, exports all lines in `source`.
    pub fn export_streaming<F>(
        source: &mut dyn LogSource,
        columnizer: &dyn Columnizer,
        format: ExportFormat,
        destination_path: &Path,
        line_indices: Option<&[usize]>,
        mut progress_cb: F,
    ) -> Result<usize, String>
    where
        F: FnMut(usize, usize), // (current_exported, total)
    {
        // 1. Safety check: Prevent overwriting source log file
        if let Some(src_path) = source.source_path() {
            if let (Ok(canon_src), Ok(canon_dst)) = (
                std::fs::canonicalize(src_path),
                std::fs::canonicalize(destination_path),
            ) {
                if canon_src == canon_dst {
                    return Err(format!(
                        "Security Violation: Target export path {:?} is identical to the active source file.",
                        destination_path
                    ));
                }
            } else if src_path == destination_path {
                return Err("Security Violation: Cannot overwrite active source file.".to_string());
            }
        }

        let out_file = File::create(destination_path)
            .map_err(|e| format!("Failed to create export file {:?}: {}", destination_path, e))?;
        let mut writer = BufWriter::with_capacity(64 * 1024, out_file);

        let total_lines = line_indices
            .map(|idx| idx.len())
            .unwrap_or_else(|| source.line_count());

        // Optional CSV/TSV header row
        if format == ExportFormat::Csv || format == ExportFormat::Tsv {
            let cols = columnizer.column_names();
            let delim = if format == ExportFormat::Csv {
                ","
            } else {
                "\t"
            };
            let header = cols
                .iter()
                .map(|c| Self::sanitize_csv_field(c))
                .collect::<Vec<_>>()
                .join(delim);
            writeln!(writer, "{}", header).map_err(|e| e.to_string())?;
        }

        let mut exported_count = 0;

        for step in 0..total_lines {
            let line_idx = line_indices.map(|slice| slice[step]).unwrap_or(step);

            let raw_line = source.get_line(line_idx)?;

            match format {
                ExportFormat::PlainText => {
                    writeln!(writer, "{}", raw_line).map_err(|e| e.to_string())?;
                }
                ExportFormat::Csv => {
                    let cols = columnizer.parse_line(&raw_line);
                    let row = cols
                        .iter()
                        .map(|c| Self::sanitize_csv_field(c))
                        .collect::<Vec<_>>()
                        .join(",");
                    writeln!(writer, "{}", row).map_err(|e| e.to_string())?;
                }
                ExportFormat::Tsv => {
                    let cols = columnizer.parse_line(&raw_line);
                    let row = cols.join("\t");
                    writeln!(writer, "{}", row).map_err(|e| e.to_string())?;
                }
                ExportFormat::JsonLines => {
                    let cols = columnizer.parse_line(&raw_line);
                    let col_names = columnizer.column_names();
                    let mut json_obj = String::from("{");
                    for (i, name) in col_names.iter().enumerate() {
                        let val = cols.get(i).cloned().unwrap_or_default();
                        if i > 0 {
                            json_obj.push(',');
                        }
                        json_obj.push_str(&format!(
                            "\"{}\":\"{}\"",
                            name,
                            val.replace('\\', "\\\\").replace('"', "\\\"")
                        ));
                    }
                    json_obj.push('}');
                    writeln!(writer, "{}", json_obj).map_err(|e| e.to_string())?;
                }
            }

            exported_count += 1;
            if exported_count % 1000 == 0 {
                progress_cb(exported_count, total_lines);
            }
        }

        writer.flush().map_err(|e| e.to_string())?;
        progress_cb(exported_count, total_lines);

        Ok(exported_count)
    }

    /// Sanitizes CSV field against formula injection and escapes embedded quotes.
    /// Prefixes '=', '+', '-', '@' with a single-quote ' so spreadsheets render as text.
    pub fn sanitize_csv_field(val: &str) -> String {
        let mut text = val.to_string();

        // 1. Formula injection neutralization
        if text.starts_with('=')
            || text.starts_with('+')
            || text.starts_with('-')
            || text.starts_with('@')
        {
            text = format!("'{}", text);
        }

        // 2. RFC 4180 quote escaping if contains comma, quote, or newline
        if text.contains(',') || text.contains('"') || text.contains('\n') || text.contains('\r') {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::columnizer::WebserverClfColumnizer;
    use crate::core::source::MockSource;

    #[test]
    fn test_csv_formula_injection_sanitization() {
        assert_eq!(
            StreamingExporter::sanitize_csv_field("normal_text"),
            "normal_text"
        );
        assert_eq!(
            StreamingExporter::sanitize_csv_field("=cmd|' /C calc'!A0"),
            "'=cmd|' /C calc'!A0"
        );
        assert_eq!(StreamingExporter::sanitize_csv_field("+12345"), "'+12345");
        assert_eq!(
            StreamingExporter::sanitize_csv_field("-discount"),
            "'-discount"
        );
        assert_eq!(StreamingExporter::sanitize_csv_field("@admin"), "'@admin");
        assert_eq!(
            StreamingExporter::sanitize_csv_field(r#"field, with "quotes""#),
            r#""field, with ""quotes""""#
        );
    }

    #[test]
    fn test_streaming_export_csv_and_tsv() {
        let sample = [
            r#"192.168.1.1 - admin [25/Nov/2026:10:00:00 +0700] "GET /index.html HTTP/1.1" 200 1024"#,
            r#"192.168.1.2 - guest [25/Nov/2026:10:00:01 +0700] "POST /login HTTP/1.1" 403 512"#,
        ];
        let mut source = MockSource::new("test.log", &sample);
        let col = WebserverClfColumnizer;

        let temp_dir = std::env::temp_dir();
        let csv_dst = temp_dir.join(format!("test_export_{}.csv", std::process::id()));

        let count = StreamingExporter::export_streaming(
            &mut source,
            &col,
            ExportFormat::Csv,
            &csv_dst,
            None,
            |_, _| {},
        )
        .expect("Export CSV");

        assert_eq!(count, 2);
        let content = std::fs::read_to_string(&csv_dst).unwrap();
        assert!(content.contains("Host,Ident,User"));
        assert!(content.contains("192.168.1.1"));
        assert!(content.contains("192.168.1.2"));

        let _ = std::fs::remove_file(&csv_dst);
    }
}
