use std::path::{Path, PathBuf};

use crate::core::reader::{NonLockingLogReader, UpdateEvent};
use crate::core::source::{allocate_source_id, LogSource, SourceLineRef};

/// Represents a single constituent file in a MultiFile series.
pub struct ConstituentFile {
    pub path: PathBuf,
    pub filename: String,
    pub reader: NonLockingLogReader,
    pub start_line_offset: usize,
    pub line_count: usize,
}

/// Unified MultiFile virtual log source for rotated log file sequences.
/// Combines rotated log files (e.g., `app.log.3`, `app.log.2`, `app.log.1`, `app.log`)
/// into a single continuous, sequentially ordered virtual document.
pub struct MultiFileSource {
    id: u32,
    generation: u64,
    title: String,
    base_path: PathBuf,
    files: Vec<ConstituentFile>,
    total_lines: usize,
    total_size: u64,
    bookmarks: Vec<usize>,
}

impl MultiFileSource {
    /// Discovers and opens all rotated files related to `primary_path`.
    /// Sorts files from oldest (highest numerical suffix) to newest (active file).
    pub fn open(primary_path: &Path) -> Result<Self, String> {
        let discovered_paths = Self::discover_rotated_files(primary_path);
        if discovered_paths.is_empty() {
            return Err(format!("No log files found matching {:?}", primary_path));
        }

        let mut files = Vec::new();
        let mut total_lines = 0;
        let mut total_size = 0;

        for path in discovered_paths {
            let filename = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let reader = NonLockingLogReader::open(&path)?;
            let count = reader.line_count();
            let size = reader.file_size();

            files.push(ConstituentFile {
                path,
                filename,
                reader,
                start_line_offset: total_lines,
                line_count: count,
            });

            total_lines += count;
            total_size += size;
        }

        let title = format!(
            "{} [MultiFile: {} files]",
            primary_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "MultiFile".to_string()),
            files.len()
        );

        Ok(Self {
            id: allocate_source_id(),
            generation: 1,
            title,
            base_path: primary_path.to_path_buf(),
            files,
            total_lines,
            total_size,
            bookmarks: Vec::new(),
        })
    }

    /// Discovers rotated log sequence files in the same directory.
    pub fn discover_rotated_files(primary_path: &Path) -> Vec<PathBuf> {
        let parent = match primary_path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };

        let file_stem = match primary_path.file_name().and_then(|f| f.to_str()) {
            Some(s) => s,
            None => return vec![primary_path.to_path_buf()],
        };

        let mut candidates: Vec<(i32, PathBuf)> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let name = match path.file_name().and_then(|f| f.to_str()) {
                    Some(n) => n,
                    None => continue,
                };

                // Pattern 1: access.log and access.log.N
                if name == file_stem {
                    candidates.push((0, path));
                } else if name.starts_with(file_stem) && name.len() > file_stem.len() {
                    let suffix = &name[file_stem.len()..];
                    if let Some(num_str) = suffix.strip_prefix('.') {
                        if let Ok(num) = num_str.parse::<i32>() {
                            candidates.push((num, path));
                        }
                    }
                } else if let Some((stem_prefix, stem_ext)) = file_stem.rsplit_once('.') {
                    // Pattern 2: app.1.log, app.2.log vs app.log
                    if name.starts_with(stem_prefix) && name.ends_with(stem_ext) {
                        let middle = &name[stem_prefix.len()..name.len() - stem_ext.len()];
                        let trimmed = middle.trim_matches('.');
                        if let Ok(num) = trimmed.parse::<i32>() {
                            candidates.push((num, path));
                        }
                    }
                }
            }
        }

        if candidates.is_empty() {
            return vec![primary_path.to_path_buf()];
        }

        // Sort descending by number (oldest first, e.g. 3, 2, 1, 0)
        candidates.sort_by_key(|b| std::cmp::Reverse(b.0));
        candidates.into_iter().map(|(_, p)| p).collect()
    }

    /// Internal helper: locates which constituent file contains `global_idx`
    fn locate_file(&self, global_idx: usize) -> Option<(usize, usize)> {
        if global_idx >= self.total_lines {
            return None;
        }

        for (file_idx, file) in self.files.iter().enumerate() {
            if global_idx >= file.start_line_offset
                && global_idx < file.start_line_offset + file.line_count
            {
                let local_idx = global_idx - file.start_line_offset;
                return Some((file_idx, local_idx));
            }
        }

        None
    }

    /// Re-indexes line offsets after updates or rotation.
    fn recalculate_offsets(&mut self) {
        let mut total = 0;
        let mut size = 0;
        for file in &mut self.files {
            file.start_line_offset = total;
            total += file.line_count;
            size += file.reader.file_size();
        }
        self.total_lines = total;
        self.total_size = size;
    }
}

impl LogSource for MultiFileSource {
    fn source_id(&self) -> u32 {
        self.id
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn display_name(&self) -> &str {
        &self.title
    }

    fn source_path(&self) -> Option<&Path> {
        Some(&self.base_path)
    }

    fn line_count(&self) -> usize {
        self.total_lines
    }

    fn file_size(&self) -> u64 {
        self.total_size
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        let (file_idx, local_idx) = self.locate_file(idx).ok_or_else(|| {
            format!(
                "Line {} out of bounds for MultiFile ({})",
                idx, self.total_lines
            )
        })?;

        self.files[file_idx].reader.get_line(local_idx)
    }

    fn get_line_ref(&self, idx: usize) -> SourceLineRef {
        let (_file_idx, local_idx) = self.locate_file(idx).unwrap_or((0, idx));
        SourceLineRef {
            source_id: self.id,
            generation: self.generation,
            line_idx: idx,
            byte_offset: local_idx as u64,
        }
    }

    fn check_for_updates(&mut self) -> Result<UpdateEvent, String> {
        let mut total_new_lines = 0;
        let mut truncated = false;

        // Check the active/newest file (last in our ordered list)
        if let Some(active_file) = self.files.last_mut() {
            match active_file.reader.check_for_updates()? {
                UpdateEvent::Appended { new_lines } => {
                    active_file.line_count += new_lines;
                    total_new_lines += new_lines;
                }
                UpdateEvent::Truncated => {
                    active_file.line_count = active_file.reader.line_count();
                    truncated = true;
                }
                UpdateEvent::NoChange => {}
            }
        }

        if truncated {
            self.generation += 1;
            self.recalculate_offsets();
            Ok(UpdateEvent::Truncated)
        } else if total_new_lines > 0 {
            self.total_lines += total_new_lines;
            self.total_size = self.files.iter().map(|f| f.reader.file_size()).sum();
            Ok(UpdateEvent::Appended {
                new_lines: total_new_lines,
            })
        } else {
            Ok(UpdateEvent::NoChange)
        }
    }

    fn toggle_bookmark(&mut self, idx: usize) -> bool {
        if idx >= self.total_lines {
            return false;
        }
        if let Some(pos) = self.bookmarks.iter().position(|&b| b == idx) {
            self.bookmarks.remove(pos);
            false
        } else {
            self.bookmarks.push(idx);
            self.bookmarks.sort_unstable();
            true
        }
    }

    fn is_bookmarked(&self, idx: usize) -> bool {
        self.bookmarks.binary_search(&idx).is_ok()
    }

    fn bookmarks(&self) -> &[usize] {
        &self.bookmarks
    }

    fn jump_to_prev_boundary(&self, current_idx: usize) -> Option<usize> {
        if self.files.is_empty() {
            return None;
        }

        // Find boundary before current_idx
        for file in self.files.iter().rev() {
            if file.start_line_offset < current_idx {
                return Some(file.start_line_offset);
            }
        }

        Some(0)
    }

    fn jump_to_next_boundary(&self, current_idx: usize) -> Option<usize> {
        for file in &self.files {
            if file.start_line_offset > current_idx {
                return Some(file.start_line_offset);
            }
        }

        None
    }

    fn get_constituent_filename(&self, idx: usize) -> Option<&str> {
        self.locate_file(idx)
            .map(|(file_idx, _)| self.files[file_idx].filename.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multifile_discovery_and_virtual_mapping() {
        let temp_dir =
            std::env::temp_dir().join(format!("tylogger_mf_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let f3 = temp_dir.join("access.log.3");
        let f2 = temp_dir.join("access.log.2");
        let f1 = temp_dir.join("access.log.1");
        let f0 = temp_dir.join("access.log");

        std::fs::write(&f3, "line 3-1\nline 3-2\n").unwrap();
        std::fs::write(&f2, "line 2-1\nline 2-2\n").unwrap();
        std::fs::write(&f1, "line 1-1\nline 1-2\n").unwrap();
        std::fs::write(&f0, "line 0-1\nline 0-2\n").unwrap();

        let mut mf = MultiFileSource::open(&f0).expect("Open MultiFile");
        assert_eq!(mf.line_count(), 8);

        // Check line order: oldest (3) to newest (0)
        assert_eq!(mf.get_line(0).unwrap(), "line 3-1");
        assert_eq!(mf.get_line(1).unwrap(), "line 3-2");
        assert_eq!(mf.get_line(2).unwrap(), "line 2-1");
        assert_eq!(mf.get_line(4).unwrap(), "line 1-1");
        assert_eq!(mf.get_line(6).unwrap(), "line 0-1");

        // Boundary navigation
        assert_eq!(mf.jump_to_next_boundary(0), Some(2));
        assert_eq!(mf.jump_to_next_boundary(2), Some(4));
        assert_eq!(mf.jump_to_next_boundary(4), Some(6));
        assert_eq!(mf.jump_to_next_boundary(6), None);

        assert_eq!(mf.jump_to_prev_boundary(6), Some(4));
        assert_eq!(mf.jump_to_prev_boundary(4), Some(2));

        // Constituent filename
        assert_eq!(mf.get_constituent_filename(0), Some("access.log.3"));
        assert_eq!(mf.get_constituent_filename(3), Some("access.log.2"));
        assert_eq!(mf.get_constituent_filename(5), Some("access.log.1"));
        assert_eq!(mf.get_constituent_filename(7), Some("access.log"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
