use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Document state serialized inside a Tylogger session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDocument {
    pub id: usize,
    pub path: Option<String>,
    pub title: String,
    pub is_multifile: bool,
    pub constituent_files: Vec<String>,
    pub is_filtered_tab: bool,
    pub parent_doc_id: Option<usize>,
    pub columnizer_id: String,
    pub selected_line: usize,
    pub top_line: usize,
    pub follow_tail: bool,
    pub encoding: String,
    pub search_query: Option<String>,
    pub filter_pattern: Option<String>,
    pub bookmarks: Vec<usize>,
    // Phase 5 additions
    pub timeshift_offset_nanos: i64,
    pub sync_group_id: Option<String>,
    pub is_merged_source: bool,
}

/// Versioned Tylogger session state (.tysession and .tyworkspace v1 and v2).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    pub schema_version: String,
    pub app_version: String,
    pub workspace_name: String,
    pub active_tab_idx: usize,
    pub split_view_active: bool,
    pub split_orientation: String,
    pub split_secondary_doc_id: Option<usize>,
    pub split_ratio: f32,
    pub documents: Vec<SessionDocument>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            schema_version: "2.0".to_string(),
            app_version: "0.5.0".to_string(),
            workspace_name: "Default Workspace".to_string(),
            active_tab_idx: 0,
            split_view_active: false,
            split_orientation: "Horizontal".to_string(),
            split_secondary_doc_id: None,
            split_ratio: 0.5,
            documents: Vec::new(),
        }
    }
}

/// Resolution action when a referenced log file in a session is missing on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingSourceAction {
    Locate(PathBuf),
    Retry,
    Skip,
    Remove,
}

impl SessionState {
    pub fn new(name: &str) -> Self {
        Self {
            schema_version: "1.0".to_string(),
            app_version: "0.5.0".to_string(),
            workspace_name: name.to_string(),
            active_tab_idx: 0,
            split_view_active: false,
            split_orientation: "Horizontal".to_string(),
            split_secondary_doc_id: None,
            split_ratio: 0.5,
            documents: Vec::new(),
        }
    }

    pub fn new_v2(name: &str) -> Self {
        Self {
            schema_version: "2.0".to_string(),
            app_version: "0.5.0".to_string(),
            workspace_name: name.to_string(),
            active_tab_idx: 0,
            split_view_active: false,
            split_orientation: "Horizontal".to_string(),
            split_secondary_doc_id: None,
            split_ratio: 0.5,
            documents: Vec::new(),
        }
    }

    pub fn add_document(&mut self, doc: SessionDocument) {
        self.documents.push(doc);
    }

    /// Sorts documents to guarantee dependency order:
    /// Parent documents are restored before child Filtered Tabs.
    pub fn dependency_ordered_documents(&self) -> Vec<SessionDocument> {
        let mut parents = Vec::new();
        let mut children = Vec::new();

        for doc in &self.documents {
            if doc.is_filtered_tab {
                children.push(doc.clone());
            } else {
                parents.push(doc.clone());
            }
        }

        parents.extend(children);
        parents
    }

    /// Serializes session state to structured JSON.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        out.push_str(&format!(
            "  \"schema_version\": \"{}\",\n",
            self.schema_version
        ));
        out.push_str(&format!("  \"app_version\": \"{}\",\n", self.app_version));
        out.push_str(&format!(
            "  \"workspace_name\": \"{}\",\n",
            self.workspace_name
        ));
        out.push_str(&format!("  \"active_tab_idx\": {},\n", self.active_tab_idx));
        out.push_str(&format!(
            "  \"split_view_active\": {},\n",
            self.split_view_active
        ));
        out.push_str(&format!(
            "  \"split_orientation\": \"{}\",\n",
            self.split_orientation
        ));
        if let Some(sec_id) = self.split_secondary_doc_id {
            out.push_str(&format!("  \"split_secondary_doc_id\": {},\n", sec_id));
        } else {
            out.push_str("  \"split_secondary_doc_id\": null,\n");
        }
        out.push_str(&format!("  \"split_ratio\": {:.2},\n", self.split_ratio));
        out.push_str("  \"documents\": [\n");

        for (i, doc) in self.documents.iter().enumerate() {
            if i > 0 {
                out.push_str(",\n");
            }
            out.push_str("    {\n");
            out.push_str(&format!("      \"id\": {},\n", doc.id));
            if let Some(p) = &doc.path {
                out.push_str(&format!(
                    "      \"path\": \"{}\",\n",
                    p.replace('\\', "\\\\")
                ));
            } else {
                out.push_str("      \"path\": null,\n");
            }
            out.push_str(&format!("      \"title\": \"{}\",\n", doc.title));
            out.push_str(&format!("      \"is_multifile\": {},\n", doc.is_multifile));
            out.push_str("      \"constituent_files\": [");
            for (cf_i, cf) in doc.constituent_files.iter().enumerate() {
                if cf_i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("\"{}\"", cf.replace('\\', "\\\\")));
            }
            out.push_str("],\n");
            out.push_str(&format!(
                "      \"is_filtered_tab\": {},\n",
                doc.is_filtered_tab
            ));
            if let Some(pid) = doc.parent_doc_id {
                out.push_str(&format!("      \"parent_doc_id\": {},\n", pid));
            } else {
                out.push_str("      \"parent_doc_id\": null,\n");
            }
            out.push_str(&format!(
                "      \"columnizer_id\": \"{}\",\n",
                doc.columnizer_id
            ));
            out.push_str(&format!(
                "      \"selected_line\": {},\n",
                doc.selected_line
            ));
            out.push_str(&format!("      \"top_line\": {},\n", doc.top_line));
            out.push_str(&format!("      \"follow_tail\": {},\n", doc.follow_tail));
            out.push_str(&format!("      \"encoding\": \"{}\",\n", doc.encoding));
            if let Some(sq) = &doc.search_query {
                out.push_str(&format!("      \"search_query\": \"{}\",\n", sq));
            } else {
                out.push_str("      \"search_query\": null,\n");
            }
            if let Some(fp) = &doc.filter_pattern {
                out.push_str(&format!("      \"filter_pattern\": \"{}\",\n", fp));
            } else {
                out.push_str("      \"filter_pattern\": null,\n");
            }
            out.push_str(&format!(
                "      \"timeshift_offset_nanos\": {},\n",
                doc.timeshift_offset_nanos
            ));
            if let Some(sg) = &doc.sync_group_id {
                out.push_str(&format!("      \"sync_group_id\": \"{}\",\n", sg));
            } else {
                out.push_str("      \"sync_group_id\": null,\n");
            }
            out.push_str(&format!(
                "      \"is_merged_source\": {},\n",
                doc.is_merged_source
            ));
            out.push_str("      \"bookmarks\": [");
            for (b_i, b) in doc.bookmarks.iter().enumerate() {
                if b_i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&b.to_string());
            }
            out.push_str("]\n    }");
        }

        out.push_str("\n  ]\n}");
        out
    }

    /// Deserializes a session from JSON text.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut state = SessionState::default();

        if let Some(name) = extract_string_val(json, "\"workspace_name\":") {
            state.workspace_name = name;
        }
        if let Some(schema) = extract_string_val(json, "\"schema_version\":") {
            state.schema_version = schema;
        }
        if let Some(app) = extract_string_val(json, "\"app_version\":") {
            state.app_version = app;
        }
        if let Some(tab_idx) = extract_usize_val(json, "\"active_tab_idx\":") {
            state.active_tab_idx = tab_idx;
        }
        if let Some(sv) = extract_bool_val(json, "\"split_view_active\":") {
            state.split_view_active = sv;
        }
        if let Some(so) = extract_string_val(json, "\"split_orientation\":") {
            state.split_orientation = so;
        }
        if let Some(sec_id) = extract_usize_val(json, "\"split_secondary_doc_id\":") {
            state.split_secondary_doc_id = Some(sec_id);
        }
        if let Some(sr) = extract_f32_val(json, "\"split_ratio\":") {
            state.split_ratio = sr;
        }

        // Parse documents array
        if let Some(docs_start) = json.find("\"documents\":") {
            let docs_section = &json[docs_start..];
            for doc_block in docs_section.split('{').skip(1) {
                if let Some(end) = doc_block.find('}') {
                    let block = &doc_block[..end];
                    if let Some(id) = extract_usize_val(block, "\"id\":") {
                        let path = extract_string_val(block, "\"path\":");
                        let title = extract_string_val(block, "\"title\":")
                            .unwrap_or_else(|| "Log".to_string());
                        let is_mf = extract_bool_val(block, "\"is_multifile\":").unwrap_or(false);
                        let is_ft =
                            extract_bool_val(block, "\"is_filtered_tab\":").unwrap_or(false);
                        let parent_id = extract_usize_val(block, "\"parent_doc_id\":");
                        let col_id = extract_string_val(block, "\"columnizer_id\":")
                            .unwrap_or_else(|| "plain".to_string());
                        let sel_line = extract_usize_val(block, "\"selected_line\":").unwrap_or(0);
                        let top_line = extract_usize_val(block, "\"top_line\":").unwrap_or(0);
                        let follow = extract_bool_val(block, "\"follow_tail\":").unwrap_or(true);
                        let enc = extract_string_val(block, "\"encoding\":")
                            .unwrap_or_else(|| "UTF-8".to_string());
                        let sq = extract_string_val(block, "\"search_query\":");
                        let fp = extract_string_val(block, "\"filter_pattern\":");
                        let ts_offset =
                            extract_i64_val(block, "\"timeshift_offset_nanos\":").unwrap_or(0);
                        let sync_group = extract_string_val(block, "\"sync_group_id\":");
                        let is_merged =
                            extract_bool_val(block, "\"is_merged_source\":").unwrap_or(false);

                        // Extract bookmarks array
                        let mut bm = Vec::new();
                        if let Some(bm_start) = block.find("\"bookmarks\":") {
                            if let Some(open) = block[bm_start..].find('[') {
                                if let Some(close) = block[bm_start + open..].find(']') {
                                    let bm_str =
                                        &block[bm_start + open + 1..bm_start + open + close];
                                    for part in bm_str.split(',') {
                                        if let Ok(num) = part.trim().parse::<usize>() {
                                            bm.push(num);
                                        }
                                    }
                                }
                            }
                        }

                        state.documents.push(SessionDocument {
                            id,
                            path,
                            title,
                            is_multifile: is_mf,
                            constituent_files: Vec::new(),
                            is_filtered_tab: is_ft,
                            parent_doc_id: parent_id,
                            columnizer_id: col_id,
                            selected_line: sel_line,
                            top_line,
                            follow_tail: follow,
                            encoding: enc,
                            search_query: sq,
                            filter_pattern: fp,
                            bookmarks: bm,
                            timeshift_offset_nanos: ts_offset,
                            sync_group_id: sync_group,
                            is_merged_source: is_merged,
                        });
                    }
                }
            }
        }

        Ok(state)
    }

    /// Atomically saves session state to disk using temporary file, sync flush, and replace.
    /// Also maintains last-known-good backup (.bak).
    pub fn save_atomic(&self, target_path: &Path) -> Result<(), String> {
        let json = self.to_json();
        let parent_dir = target_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_path = parent_dir.join(format!(
            ".{}_{}.tmp",
            target_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            std::process::id()
        ));

        // 1. Write to temp file
        {
            let mut file =
                File::create(&temp_path).map_err(|e| format!("Create temp session file: {}", e))?;
            file.write_all(json.as_bytes())
                .map_err(|e| format!("Write session temp: {}", e))?;
            file.sync_all()
                .map_err(|e| format!("Flush session disk buffer: {}", e))?;
        }

        // 2. Backup existing file to .bak if present
        if target_path.exists() {
            let bak_path = target_path.with_extension("bak");
            let _ = fs::copy(target_path, bak_path);
        }

        // 3. Atomic rename/replace
        fs::rename(&temp_path, target_path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            format!("Atomic replace session file: {}", e)
        })?;

        Ok(())
    }

    /// Loads session from file with atomic validation.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Read session: {}", e))?;
        Self::from_json(&content)
    }

    /// Verifies all referenced files exist. Returns list of (doc_id, missing_path).
    pub fn check_missing_sources(&self) -> Vec<(usize, PathBuf)> {
        let mut missing = Vec::new();
        for doc in &self.documents {
            if let Some(p) = &doc.path {
                let pb = PathBuf::from(p);
                if !pb.exists() {
                    missing.push((doc.id, pb));
                }
            }
        }
        missing
    }

    /// Safe importer for legacy LogExpert (.lxp / .lxj) session formats.
    /// Strictly ignores external DTDs, XXE entities, and executable objects.
    pub fn import_logexpert(content: &str) -> Result<Self, String> {
        let mut state = SessionState::new("Imported LogExpert Workspace");

        // Parse XML tags if .lxp format
        if content.contains("<LogExpertSession") || content.contains("<session") {
            let mut id_counter = 1;
            for file_block in content.split("<file>").skip(1) {
                if let Some(end) = file_block.find("</file>") {
                    let file_path = &file_block[..end].trim();
                    if !file_path.is_empty() {
                        let path_buf = PathBuf::from(file_path);
                        let title = path_buf
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();

                        state.documents.push(SessionDocument {
                            id: id_counter,
                            path: Some(file_path.to_string()),
                            title,
                            is_multifile: false,
                            constituent_files: Vec::new(),
                            is_filtered_tab: false,
                            parent_doc_id: None,
                            columnizer_id: "plain".to_string(),
                            selected_line: 0,
                            top_line: 0,
                            follow_tail: true,
                            encoding: "UTF-8".to_string(),
                            search_query: None,
                            filter_pattern: None,
                            bookmarks: Vec::new(),
                            timeshift_offset_nanos: 0,
                            sync_group_id: None,
                            is_merged_source: false,
                        });
                        id_counter += 1;
                    }
                }
            }
            return Ok(state);
        }

        // Otherwise parse JSON (.lxj)
        Self::from_json(content)
    }
}

/// Crash Recovery Manager tracking clean termination and recovery snapshots.
pub struct CrashRecoveryTracker {
    lock_file: PathBuf,
    snapshot_file: PathBuf,
}

impl CrashRecoveryTracker {
    pub fn new(appdata_dir: &Path) -> Self {
        Self {
            lock_file: appdata_dir.join(".tylogger_active"),
            snapshot_file: appdata_dir.join("recovery_snapshot.tysession"),
        }
    }

    /// Checks if previous session terminated abnormally (unclean shutdown).
    pub fn is_unclean_shutdown(&self) -> bool {
        self.lock_file.exists() && self.snapshot_file.exists()
    }

    /// Marks session as active on startup.
    pub fn mark_session_active(&self) -> Result<(), String> {
        File::create(&self.lock_file).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Marks session as cleanly exited on shutdown.
    pub fn mark_clean_shutdown(&self) {
        let _ = fs::remove_file(&self.lock_file);
    }

    /// Saves periodic background snapshot.
    pub fn save_snapshot(&self, state: &SessionState) -> Result<(), String> {
        state.save_atomic(&self.snapshot_file)
    }

    /// Loads recovery snapshot if present.
    pub fn load_snapshot(&self) -> Result<SessionState, String> {
        SessionState::load(&self.snapshot_file)
    }
}

// Helpers for JSON parsing without third-party crates
fn extract_string_val(json: &str, key: &str) -> Option<String> {
    if let Some(pos) = json.find(key) {
        let rem = &json[pos + key.len()..];
        if let Some(first_quote) = rem.find('"') {
            if let Some(end_quote) = rem[first_quote + 1..].find('"') {
                return Some(
                    rem[first_quote + 1..first_quote + 1 + end_quote].replace("\\\\", "\\"),
                );
            }
        }
    }
    None
}

fn extract_usize_val(json: &str, key: &str) -> Option<usize> {
    if let Some(pos) = json.find(key) {
        let rem = &json[pos + key.len()..];
        let num_str: String = rem
            .chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        return num_str.parse::<usize>().ok();
    }
    None
}

fn extract_bool_val(json: &str, key: &str) -> Option<bool> {
    if let Some(pos) = json.find(key) {
        let rem = &json[pos + key.len()..];
        let trimmed = rem.trim_start();
        if trimmed.starts_with("true") {
            return Some(true);
        } else if trimmed.starts_with("false") {
            return Some(false);
        }
    }
    None
}

fn extract_i64_val(json: &str, key: &str) -> Option<i64> {
    if let Some(pos) = json.find(key) {
        let rem = &json[pos + key.len()..];
        let num_str: String = rem
            .chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        return num_str.parse::<i64>().ok();
    }
    None
}

fn extract_f32_val(json: &str, key: &str) -> Option<f32> {
    if let Some(pos) = json.find(key) {
        let rem = &json[pos + key.len()..];
        let num_str: String = rem
            .chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        return num_str.parse::<f32>().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_serialization_and_deserialization() {
        let mut session = SessionState::new("Incident Investigation");
        session.active_tab_idx = 1;
        session.split_view_active = true;
        session.split_orientation = "Vertical".to_string();
        session.split_ratio = 0.6;

        let doc1 = SessionDocument {
            id: 1,
            path: Some("C:\\logs\\app.log".to_string()),
            title: "app.log".to_string(),
            is_multifile: false,
            constituent_files: Vec::new(),
            is_filtered_tab: false,
            parent_doc_id: None,
            columnizer_id: "clf".to_string(),
            selected_line: 42,
            top_line: 10,
            follow_tail: true,
            encoding: "UTF-8".to_string(),
            search_query: Some("ERROR".to_string()),
            filter_pattern: None,
            bookmarks: vec![5, 42, 100],
            timeshift_offset_nanos: 1_500_000_000,
            sync_group_id: Some("prod-sync".to_string()),
            is_merged_source: false,
        };

        let doc2 = SessionDocument {
            id: 2,
            path: None,
            title: "app.log [Filter: ERROR]".to_string(),
            is_multifile: false,
            constituent_files: Vec::new(),
            is_filtered_tab: true,
            parent_doc_id: Some(1),
            columnizer_id: "clf".to_string(),
            selected_line: 0,
            top_line: 0,
            follow_tail: false,
            encoding: "UTF-8".to_string(),
            search_query: None,
            filter_pattern: Some("ERROR".to_string()),
            bookmarks: vec![0],
            timeshift_offset_nanos: 0,
            sync_group_id: None,
            is_merged_source: false,
        };

        session.add_document(doc1);
        session.add_document(doc2);

        // Verify dependency order
        let ordered = session.dependency_ordered_documents();
        assert_eq!(ordered[0].id, 1); // Parent first
        assert_eq!(ordered[1].id, 2); // Filtered tab second

        let json = session.to_json();
        assert!(json.contains("\"Incident Investigation\""));
        assert!(json.contains("\"active_tab_idx\": 1"));
        assert!(json.contains("\"split_view_active\": true"));
        assert!(json.contains("\"timeshift_offset_nanos\": 1500000000"));

        let restored = SessionState::from_json(&json).expect("Parse session JSON");
        assert_eq!(restored.workspace_name, "Incident Investigation");
        assert_eq!(restored.documents.len(), 2);
        assert_eq!(restored.documents[0].bookmarks, vec![5, 42, 100]);
        assert_eq!(restored.documents[0].timeshift_offset_nanos, 1_500_000_000);
        assert_eq!(restored.split_view_active, true);
        assert_eq!(
            restored.documents[1].filter_pattern,
            Some("ERROR".to_string())
        );
    }

    #[test]
    fn test_schema_v1_to_v2_migration() {
        // Legacy v1.0 session JSON without split_view or timeshift fields
        let v1_json = r#"{
  "schema_version": "1.0",
  "app_version": "0.4.0",
  "workspace_name": "Legacy Session",
  "active_tab_idx": 0,
  "documents": [
    {
      "id": 1,
      "path": "C:\\logs\\legacy.log",
      "title": "legacy.log",
      "is_multifile": false,
      "constituent_files": [],
      "is_filtered_tab": false,
      "parent_doc_id": null,
      "columnizer_id": "plain",
      "selected_line": 0,
      "top_line": 0,
      "follow_tail": true,
      "encoding": "UTF-8",
      "search_query": null,
      "filter_pattern": null,
      "bookmarks": [10, 20]
    }
  ]
}"#;

        let migrated = SessionState::from_json(v1_json).expect("Migrate v1.0 session");
        assert_eq!(migrated.workspace_name, "Legacy Session");
        assert_eq!(migrated.schema_version, "1.0");
        assert_eq!(migrated.documents.len(), 1);
        // Default safe values populated
        assert_eq!(migrated.split_view_active, false);
        assert_eq!(migrated.documents[0].timeshift_offset_nanos, 0);
        assert_eq!(migrated.documents[0].sync_group_id, None);
        assert_eq!(migrated.documents[0].bookmarks, vec![10, 20]);
    }

    #[test]
    fn test_atomic_session_save_and_backup() {
        let temp_dir =
            std::env::temp_dir().join(format!("tylogger_session_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let session_file = temp_dir.join("test_workspace.tysession");

        let mut session = SessionState::new("Atomic Test");
        session.save_atomic(&session_file).expect("Initial save");
        assert!(session_file.exists());

        // Update session and save again -> should create .bak
        session.workspace_name = "Updated Workspace".to_string();
        session.save_atomic(&session_file).expect("Second save");

        let bak_file = session_file.with_extension("bak");
        assert!(bak_file.exists());

        let restored = SessionState::load(&session_file).expect("Load session");
        assert_eq!(restored.workspace_name, "Updated Workspace");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_crash_recovery_tracker() {
        let temp_dir =
            std::env::temp_dir().join(format!("tylogger_crash_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let tracker = CrashRecoveryTracker::new(&temp_dir);
        assert!(!tracker.is_unclean_shutdown());

        // Mark active and save snapshot
        tracker.mark_session_active().unwrap();
        let session = SessionState::new("Crash Session");
        tracker.save_snapshot(&session).unwrap();

        // Simulate crash: lock file still exists
        assert!(tracker.is_unclean_shutdown());

        let restored = tracker.load_snapshot().unwrap();
        assert_eq!(restored.workspace_name, "Crash Session");

        // Clean exit
        tracker.mark_clean_shutdown();
        assert!(!tracker.is_unclean_shutdown());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_safe_logexpert_xml_import() {
        let lxp_sample = r#"<?xml version="1.0" encoding="utf-8"?>
<LogExpertSession>
  <file>C:\logs\server.log</file>
  <file>D:\app\audit.log</file>
</LogExpertSession>"#;

        let imported = SessionState::import_logexpert(lxp_sample).unwrap();
        assert_eq!(imported.documents.len(), 2);
        assert_eq!(
            imported.documents[0].path,
            Some(r#"C:\logs\server.log"#.to_string())
        );
        assert_eq!(
            imported.documents[1].path,
            Some(r#"D:\app\audit.log"#.to_string())
        );
    }
}
