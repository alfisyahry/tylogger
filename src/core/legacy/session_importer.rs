use crate::core::session::{SessionDocument, SessionState};

#[derive(Debug, Clone)]
pub struct LegacyImportSummary {
    pub session_title: String,
    pub documents_imported: usize,
    pub bookmarks_imported: usize,
    pub filter_rules_imported: usize,
    pub highlight_rules_imported: usize,
    pub unsupported_elements_skipped: usize,
}

pub struct LegacySessionImporter;

impl LegacySessionImporter {
    /// Safely imports LogExpert upstream `.lxp` or `.lxj` XML session files.
    /// Strictly protects against XML External Entity (XXE) and SSRF attacks (RSK-10).
    pub fn import_session(
        xml_content: &str,
    ) -> Result<(SessionState, LegacyImportSummary), String> {
        // Anti-XXE defense: reject any DTD or ENTITY declarations
        if xml_content.contains("<!DOCTYPE") || xml_content.contains("<!ENTITY") {
            return Err("Security Violation (XXE Hazard): DOCTYPE and ENTITY definitions are strictly prohibited".to_string());
        }

        let mut summary = LegacyImportSummary {
            session_title: "Imported LogExpert Legacy Session".to_string(),
            documents_imported: 0,
            bookmarks_imported: 0,
            filter_rules_imported: 0,
            highlight_rules_imported: 0,
            unsupported_elements_skipped: 0,
        };

        let mut session = SessionState::new_v3("Legacy Import Workspace");

        // Parse XML elements using safe minimal tokenizer
        let mut in_file = false;
        let mut cur_path = String::new();
        let mut cur_columnizer = None;
        let mut cur_bookmarks = Vec::new();
        let mut cur_scroll = 0;
        let mut cur_filter = None;

        for line in xml_content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("<file>") || trimmed.starts_with("<document>") {
                in_file = true;
                cur_path.clear();
                cur_columnizer = None;
                cur_bookmarks.clear();
                cur_scroll = 0;
                cur_filter = None;
            } else if trimmed.starts_with("</file>") || trimmed.starts_with("</document>") {
                if in_file && !cur_path.is_empty() {
                    let title = cur_path
                        .rsplit('\\')
                        .next()
                        .or_else(|| cur_path.rsplit('/').next())
                        .unwrap_or(&cur_path)
                        .to_string();

                    summary.documents_imported += 1;
                    summary.bookmarks_imported += cur_bookmarks.len();
                    if cur_filter.is_some() {
                        summary.filter_rules_imported += 1;
                    }

                    session.documents.push(SessionDocument {
                        id: summary.documents_imported,
                        path: Some(cur_path.clone()),
                        title,
                        is_multifile: false,
                        constituent_files: Vec::new(),
                        is_filtered_tab: false,
                        parent_doc_id: None,
                        columnizer_id: cur_columnizer.take().unwrap_or_else(|| "plain".to_string()),
                        selected_line: cur_scroll,
                        top_line: cur_scroll,
                        follow_tail: true,
                        encoding: "UTF-8".to_string(),
                        search_query: None,
                        filter_pattern: cur_filter.take(),
                        bookmarks: cur_bookmarks.clone(),
                        timeshift_offset_nanos: 0,
                        sync_group_id: None,
                        is_merged_source: false,
                    });
                }
                in_file = false;
            } else if let Some(val) = extract_xml_tag_content(trimmed, "fileName") {
                cur_path = val;
            } else if let Some(val) = extract_xml_tag_content(trimmed, "path") {
                cur_path = val;
            } else if let Some(val) = extract_xml_tag_content(trimmed, "columnizer") {
                cur_columnizer = Some(val);
            } else if let Some(val) = extract_xml_tag_content(trimmed, "currentLine") {
                cur_scroll = val.parse().unwrap_or(0);
            } else if let Some(val) = extract_xml_tag_content(trimmed, "bookmark") {
                if let Ok(line_num) = val.parse::<usize>() {
                    cur_bookmarks.push(line_num);
                }
            } else if let Some(val) = extract_xml_tag_content(trimmed, "filterText") {
                if !val.is_empty() {
                    cur_filter = Some(val);
                }
            } else if trimmed.starts_with("<highlight") {
                summary.highlight_rules_imported += 1;
            } else if trimmed.starts_with('<') && !trimmed.starts_with("</") {
                summary.unsupported_elements_skipped += 1;
            }
        }

        // If single document format without explicit <file> wrapper
        if summary.documents_imported == 0 && !cur_path.is_empty() {
            let title = cur_path
                .rsplit('\\')
                .next()
                .or_else(|| cur_path.rsplit('/').next())
                .unwrap_or(&cur_path)
                .to_string();

            summary.documents_imported = 1;
            summary.bookmarks_imported = cur_bookmarks.len();
            if cur_filter.is_some() {
                summary.filter_rules_imported = 1;
            }

            session.documents.push(SessionDocument {
                id: 1,
                path: Some(cur_path),
                title,
                is_multifile: false,
                constituent_files: Vec::new(),
                is_filtered_tab: false,
                parent_doc_id: None,
                columnizer_id: cur_columnizer.unwrap_or_else(|| "plain".to_string()),
                selected_line: cur_scroll,
                top_line: cur_scroll,
                follow_tail: true,
                encoding: "UTF-8".to_string(),
                search_query: None,
                filter_pattern: cur_filter,
                bookmarks: cur_bookmarks,
                timeshift_offset_nanos: 0,
                sync_group_id: None,
                is_merged_source: false,
            });
        }

        Ok((session, summary))
    }
}

fn extract_xml_tag_content(line: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);

    if let Some(start_idx) = line.find(&open) {
        if let Some(end_idx) = line.find(&close) {
            let content = &line[start_idx + open.len()..end_idx];
            return Some(content.trim().to_string());
        }
    }
    None
}
