use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, SelectObject, SetBkMode, SetTextColor, TextOutW,
    HDC, HGDIOBJ, TRANSPARENT,
};

use crate::core::workspace::Document;
use crate::gui::theme::ThemeTokens;

pub const STATUSBAR_HEIGHT: i32 = 24;

pub struct StatusBar;

impl StatusBar {
    /// Formats file size in human-readable bytes / KB / MB.
    fn format_size(bytes: u64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    /// Renders the status bar at the bottom of the window.
    pub unsafe fn draw(
        hdc: HDC,
        width: i32,
        y: i32,
        doc: Option<&Document>,
        theme: &ThemeTokens,
        font: HGDIOBJ,
    ) {
        // Status bar background
        let bg_brush = CreateSolidBrush(theme.surface_secondary.to_colorref());
        let bar_rect = RECT {
            left: 0,
            top: y,
            right: width,
            bottom: y + STATUSBAR_HEIGHT,
        };
        FillRect(hdc, &bar_rect, bg_brush);
        DeleteObject(bg_brush as _);

        // Top border divider
        let border_brush = CreateSolidBrush(theme.border_subtle.to_colorref());
        let border_rect = RECT {
            left: 0,
            top: y,
            right: width,
            bottom: y + 1,
        };
        FillRect(hdc, &border_rect, border_brush);
        DeleteObject(border_brush as _);

        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as _);
        SetTextColor(hdc, theme.text_secondary.to_colorref());

        let (status_str, enc_str, size_str, lines_str, cursor_str, search_str) = match doc {
            Some(d) => {
                let tail_status = if d.monitoring_paused {
                    "[PAUSED]"
                } else if d.follow_tail {
                    "[TAILING]"
                } else {
                    "[MONITORING]"
                };

                let size = Self::format_size(d.file_size());
                let lines = format!("Lines: {}", d.line_count());
                let cursor = format!("Line: {}", d.selected_line + 1);

                let search = if let Some(ref q) = d.search_query {
                    if d.search_matches.is_empty() {
                        format!("Search '{}': No matches", q)
                    } else if let Some(curr) = d.current_match_idx {
                        format!("Match {} of {}", curr + 1, d.search_matches.len())
                    } else {
                        format!("Matches: {}", d.search_matches.len())
                    }
                } else if d.filter_active {
                    format!("Filtered: {} lines", d.filter_matches.len())
                } else {
                    String::new()
                };

                (tail_status, d.encoding.as_str(), size, lines, cursor, search)
            }
            None => ("[IDLE]", "None", "0 B".to_string(), "No documents".to_string(), "".to_string(), "".to_string()),
        };

        // Draw segments with vertical separators
        let segments = [
            (status_str.to_string(), 100),
            (enc_str.to_string(), 80),
            (size_str, 90),
            (lines_str, 120),
            (cursor_str, 100),
            (search_str, 240),
        ];

        let mut curr_x = 10;
        for (text, seg_width) in &segments {
            if text.is_empty() {
                curr_x += seg_width;
                continue;
            }
            let wide: Vec<u16> = std::ffi::OsStr::new(text).encode_wide().collect();
            TextOutW(hdc, curr_x, y + 4, wide.as_ptr(), wide.len() as _);

            // Draw subtle vertical separator
            let sep_x = curr_x + seg_width - 8;
            let sep_brush = CreateSolidBrush(theme.border_subtle.to_colorref());
            let sep_rect = RECT {
                left: sep_x,
                top: y + 4,
                right: sep_x + 1,
                bottom: y + STATUSBAR_HEIGHT - 4,
            };
            FillRect(hdc, &sep_rect, sep_brush);
            DeleteObject(sep_brush as _);

            curr_x += seg_width;
        }

        SelectObject(hdc, old_font);
    }
}
