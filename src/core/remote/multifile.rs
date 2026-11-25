use std::sync::Arc;

use crate::core::remote::client::RemoteTransport;

#[derive(Debug, Clone)]
pub struct RemoteRotatedFile {
    pub filename: String,
    pub path: String,
    pub size: u64,
    pub mtime: u64,
}

pub struct RemoteMultiFileScanner;

impl RemoteMultiFileScanner {
    /// Discovers and chronologically orders rotated files matching a base log name on remote host.
    /// E.g. server.log, server.log.1, server.log.2 -> sorted oldest to newest.
    pub fn discover_rotated_files(
        transport: Arc<dyn RemoteTransport>,
        dir_path: &str,
        base_name: &str,
    ) -> Result<Vec<RemoteRotatedFile>, String> {
        let entries = transport.list_dir(dir_path)?;
        let mut matching = Vec::new();

        for e in entries {
            if !e.is_dir && (e.name == base_name || e.name.starts_with(&format!("{}.", base_name)))
            {
                matching.push(RemoteRotatedFile {
                    filename: e.name,
                    path: e.path,
                    size: e.size,
                    mtime: e.mtime,
                });
            }
        }

        // Sort by rotation index / mtime: older files first, active file last
        matching.sort_by(|a, b| {
            let index_a = parse_rotation_index(&a.filename, base_name);
            let index_b = parse_rotation_index(&b.filename, base_name);

            match (index_a, index_b) {
                // If both have explicit numeric suffixes like .1, .2: higher number is older
                (Some(idx_a), Some(idx_b)) => idx_b.cmp(&idx_a),
                // Numbered file is older than un-numbered active file
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                // Fallback to mtime
                (None, None) => a.mtime.cmp(&b.mtime),
            }
        });

        Ok(matching)
    }
}

fn parse_rotation_index(filename: &str, base: &str) -> Option<u32> {
    if filename.len() > base.len() && filename.starts_with(base) {
        let suffix = &filename[base.len()..];
        let trimmed = suffix.trim_start_matches('.');
        trimmed.parse::<u32>().ok()
    } else {
        None
    }
}
