use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::core::analytics::engine::AggregationResult;
use crate::core::export::privacy::{PrivacyFilter, PrivacyMaskingOptions};

pub struct OfflineReportGenerator;

impl OfflineReportGenerator {
    /// Escapes HTML special characters strictly to prevent XSS (RSK-04).
    pub fn escape_html(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        for c in input.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#39;"),
                _ => out.push(c),
            }
        }
        out
    }

    /// Generates a self-contained offline HTML report with inline CSS and inline SVG timeline.
    pub fn generate_html_report(
        result: &AggregationResult,
        destination_path: &Path,
        mask_opts: Option<PrivacyMaskingOptions>,
    ) -> Result<(), String> {
        let file = File::create(destination_path)
            .map_err(|e| format!("Failed to create HTML report: {}", e))?;
        let mut writer = BufWriter::new(file);

        let title = Self::escape_html(&result.query_title);

        writeln!(writer, "<!DOCTYPE html>").map_err(|e| e.to_string())?;
        writeln!(writer, "<html lang=\"en\">").map_err(|e| e.to_string())?;
        writeln!(writer, "<head>").map_err(|e| e.to_string())?;
        writeln!(writer, "<meta charset=\"utf-8\">").map_err(|e| e.to_string())?;
        writeln!(writer, "<title>Tylogger Analytics: {}</title>", title)
            .map_err(|e| e.to_string())?;
        writeln!(writer, "<style>").map_err(|e| e.to_string())?;
        writeln!(writer, "body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; margin: 0; padding: 24px; background: #1e1e1e; color: #d4d4d4; }}").map_err(|e| e.to_string())?;
        writeln!(
            writer,
            ".container {{ max-width: 1200px; margin: 0 auto; }}"
        )
        .map_err(|e| e.to_string())?;
        writeln!(writer, ".header {{ border-bottom: 2px solid #007acc; padding-bottom: 16px; margin-bottom: 24px; }}").map_err(|e| e.to_string())?;
        writeln!(
            writer,
            "h1 {{ margin: 0 0 8px 0; color: #ffffff; font-size: 24px; }}"
        )
        .map_err(|e| e.to_string())?;
        writeln!(writer, ".meta {{ color: #858585; font-size: 13px; }}")
            .map_err(|e| e.to_string())?;
        writeln!(
            writer,
            ".cards {{ display: flex; gap: 16px; margin-bottom: 24px; }}"
        )
        .map_err(|e| e.to_string())?;
        writeln!(writer, ".card {{ background: #252526; border: 1px solid #3c3c3c; border-radius: 4px; padding: 16px; flex: 1; }}").map_err(|e| e.to_string())?;
        writeln!(writer, ".card-title {{ font-size: 12px; color: #858585; text-transform: uppercase; margin-bottom: 8px; }}").map_err(|e| e.to_string())?;
        writeln!(
            writer,
            ".card-value {{ font-size: 24px; font-weight: bold; color: #ffffff; }}"
        )
        .map_err(|e| e.to_string())?;
        writeln!(writer, ".table-container {{ background: #252526; border: 1px solid #3c3c3c; border-radius: 4px; overflow: hidden; }}").map_err(|e| e.to_string())?;
        writeln!(writer, "table {{ width: 100%; border-collapse: collapse; text-align: left; font-size: 13px; }}").map_err(|e| e.to_string())?;
        writeln!(writer, "th {{ background: #2d2d30; padding: 10px 16px; color: #cccccc; border-bottom: 1px solid #3c3c3c; }}").map_err(|e| e.to_string())?;
        writeln!(
            writer,
            "td {{ padding: 8px 16px; border-bottom: 1px solid #2d2d30; color: #d4d4d4; }}"
        )
        .map_err(|e| e.to_string())?;
        writeln!(writer, "tr:hover td {{ background: #2a2d2e; }}").map_err(|e| e.to_string())?;
        writeln!(writer, ".svg-chart {{ margin-bottom: 24px; background: #252526; border: 1px solid #3c3c3c; border-radius: 4px; padding: 16px; }}").map_err(|e| e.to_string())?;
        writeln!(writer, "</style>").map_err(|e| e.to_string())?;
        writeln!(writer, "</head>").map_err(|e| e.to_string())?;
        writeln!(writer, "<body>").map_err(|e| e.to_string())?;
        writeln!(writer, "<div class=\"container\">").map_err(|e| e.to_string())?;

        // Header
        writeln!(writer, "<div class=\"header\">").map_err(|e| e.to_string())?;
        writeln!(writer, "<h1>{}</h1>", title).map_err(|e| e.to_string())?;
        let mode_label = if result.is_exact {
            "EXACT"
        } else {
            "APPROXIMATE"
        };
        writeln!(writer, "<div class=\"meta\">Mode: {} | Scanned: {} lines | Matched: {} records | Execution: {} ms</div>",
            mode_label, result.total_lines_scanned, result.matched_records, result.execution_millis).map_err(|e| e.to_string())?;
        writeln!(writer, "</div>").map_err(|e| e.to_string())?;

        // Cards
        writeln!(writer, "<div class=\"cards\">").map_err(|e| e.to_string())?;
        writeln!(writer, "<div class=\"card\"><div class=\"card-title\">Total Matched</div><div class=\"card-value\">{}</div></div>", result.matched_records).map_err(|e| e.to_string())?;
        writeln!(writer, "<div class=\"card\"><div class=\"card-title\">Distinct Groups</div><div class=\"card-value\">{}</div></div>", result.rows.len()).map_err(|e| e.to_string())?;
        writeln!(writer, "<div class=\"card\"><div class=\"card-title\">Scan Duration</div><div class=\"card-value\">{} ms</div></div>", result.execution_millis).map_err(|e| e.to_string())?;
        writeln!(writer, "</div>").map_err(|e| e.to_string())?;

        // Inline SVG Chart
        if !result.buckets.is_empty() {
            writeln!(writer, "<div class=\"svg-chart\">").map_err(|e| e.to_string())?;
            writeln!(writer, "<h3>Activity Timeline</h3>").map_err(|e| e.to_string())?;
            let svg_w = 1100;
            let svg_h = 160;
            let max_count = result
                .buckets
                .iter()
                .map(|(_, b)| b.count)
                .max()
                .unwrap_or(1)
                .max(1);
            let bar_w = ((svg_w as f64) / (result.buckets.len() as f64)).max(2.0);

            writeln!(
                writer,
                "<svg width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
                svg_w, svg_h, svg_w, svg_h
            )
            .map_err(|e| e.to_string())?;
            for (i, (_, b)) in result.buckets.iter().enumerate() {
                let bar_h = ((b.count as f64) / (max_count as f64) * 120.0).max(2.0);
                let x = (i as f64) * bar_w;
                let y = (svg_h as f64) - bar_h - 20.0;
                let fill = if b.error_count > 0 {
                    "#f48771"
                } else {
                    "#007acc"
                };
                writeln!(
                    writer,
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" />",
                    x,
                    y,
                    (bar_w - 1.0).max(1.0),
                    bar_h,
                    fill
                )
                .map_err(|e| e.to_string())?;
            }
            writeln!(writer, "</svg>").map_err(|e| e.to_string())?;
            writeln!(writer, "</div>").map_err(|e| e.to_string())?;
        }

        // Table
        writeln!(writer, "<div class=\"table-container\">").map_err(|e| e.to_string())?;
        writeln!(writer, "<table>").map_err(|e| e.to_string())?;
        writeln!(writer, "<thead><tr>").map_err(|e| e.to_string())?;
        for h in &result.headers {
            writeln!(writer, "<th>{}</th>", Self::escape_html(h)).map_err(|e| e.to_string())?;
        }
        writeln!(writer, "</tr></thead>").map_err(|e| e.to_string())?;
        writeln!(writer, "<tbody>").map_err(|e| e.to_string())?;

        for row in &result.rows {
            let key_str = if let Some(opts) = mask_opts {
                PrivacyFilter::mask_line(&row.key, opts)
            } else {
                row.key.clone()
            };

            writeln!(writer, "<tr>").map_err(|e| e.to_string())?;
            writeln!(writer, "<td>{}</td>", Self::escape_html(&key_str))
                .map_err(|e| e.to_string())?;
            writeln!(writer, "<td>{}</td>", row.count).map_err(|e| e.to_string())?;
            for val in &row.values {
                writeln!(writer, "<td>{:.2}</td>", val).map_err(|e| e.to_string())?;
            }
            writeln!(writer, "</tr>").map_err(|e| e.to_string())?;
        }

        writeln!(writer, "</tbody></table></div>").map_err(|e| e.to_string())?;
        writeln!(writer, "</div></body></html>").map_err(|e| e.to_string())?;

        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}
