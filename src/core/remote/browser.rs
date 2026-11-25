use std::sync::Arc;

use crate::core::remote::client::{RemoteDirEntry, RemoteTransport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Size,
    Mtime,
    Permissions,
}

pub struct SftpFileBrowser {
    transport: Arc<dyn RemoteTransport>,
    current_dir: String,
    entries: Vec<RemoteDirEntry>,
    search_query: String,
    show_hidden: bool,
    sort_column: SortColumn,
    sort_ascending: bool,
    max_entries_limit: usize,
}

impl SftpFileBrowser {
    pub fn new(transport: Arc<dyn RemoteTransport>, initial_dir: &str) -> Self {
        let mut browser = Self {
            transport,
            current_dir: initial_dir.to_string(),
            entries: Vec::new(),
            search_query: String::new(),
            show_hidden: false,
            sort_column: SortColumn::Name,
            sort_ascending: true,
            max_entries_limit: 5000,
        };
        let _ = browser.refresh();
        browser
    }

    pub fn current_directory(&self) -> &str {
        &self.current_dir
    }

    pub fn set_search_query(&mut self, query: &str) {
        self.search_query = query.to_string();
    }

    pub fn set_show_hidden(&mut self, show: bool) {
        self.show_hidden = show;
    }

    pub fn set_sort(&mut self, col: SortColumn, ascending: bool) {
        self.sort_column = col;
        self.sort_ascending = ascending;
        self.apply_sort();
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        let raw_entries = self.transport.list_dir(&self.current_dir)?;
        self.entries = raw_entries
            .into_iter()
            .take(self.max_entries_limit)
            .collect();
        self.apply_sort();
        Ok(())
    }

    pub fn navigate_to(&mut self, path: &str) -> Result<(), String> {
        self.current_dir = path.to_string();
        self.refresh()
    }

    pub fn navigate_up(&mut self) -> Result<(), String> {
        if self.current_dir == "/" || self.current_dir.is_empty() {
            return Ok(());
        }
        let parent = {
            let clean = self.current_dir.trim_end_matches('/');
            if let Some(pos) = clean.rfind('/') {
                if pos == 0 {
                    "/".to_string()
                } else {
                    clean[..pos].to_string()
                }
            } else {
                "/".to_string()
            }
        };
        self.navigate_to(&parent)
    }

    pub fn filtered_entries(&self) -> Vec<&RemoteDirEntry> {
        let query_lower = self.search_query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                if !self.show_hidden && e.name.starts_with('.') {
                    return false;
                }
                if !query_lower.is_empty() && !e.name.to_lowercase().contains(&query_lower) {
                    return false;
                }
                true
            })
            .collect()
    }

    fn apply_sort(&mut self) {
        self.entries.sort_by(|a, b| {
            // Directories always on top
            if a.is_dir != b.is_dir {
                return b.is_dir.cmp(&a.is_dir);
            }
            let ordering = match self.sort_column {
                SortColumn::Name => a.name.cmp(&b.name),
                SortColumn::Size => a.size.cmp(&b.size),
                SortColumn::Mtime => a.mtime.cmp(&b.mtime),
                SortColumn::Permissions => a.permissions.cmp(&b.permissions),
            };
            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
    }
}
