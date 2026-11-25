use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, SelectObject, SetBkMode, SetTextColor, TextOutW,
    HDC, HGDIOBJ, TRANSPARENT,
};

use crate::core::workspace::Document;
use crate::gui::theme::ThemeTokens;

pub const BOOKMARK_GUTTER_WIDTH: i32 = 24;
pub const DEFAULT_ROW_HEIGHT: i32 = 20;

#[derive(Debug, PartialEq, Eq)]
pub enum GridHit {
    BookmarkGutter(usize),
    LineNumberGutter(usize),
    Row(usize),
    Outside,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollMapping {
    pub min: i32,
    pub max: i32,
    pub pos: i32,
    pub page: u32,
}

pub struct LogGrid;

impl LogGrid {
    /// Computes dynamic gutter width based on total lines and character width.
    pub fn calculate_gutter_width(total_lines: usize, char_width: i32) -> i32 {
        let digits = if total_lines == 0 {
            1
        } else {
            (total_lines as f64).log10().floor() as i32 + 1
        };
        let line_num_width = (digits.max(3) * char_width.max(7)) + 12;
        BOOKMARK_GUTTER_WIDTH + line_num_width
    }

    /// Computes 64-bit to 32-bit proportional scrollbar parameters without integer overflow.
    pub fn compute_scroll_info(
        total_lines: usize,
        top_line: usize,
        visible_lines: usize,
    ) -> ScrollMapping {
        if total_lines == 0 {
            return ScrollMapping {
                min: 0,
                max: 0,
                pos: 0,
                page: 1,
            };
        }

        const MAX_SCROLL_RANGE: u64 = 1_000_000;

        if (total_lines as u64) <= MAX_SCROLL_RANGE {
            ScrollMapping {
                min: 0,
                max: total_lines.saturating_sub(1) as i32,
                pos: top_line.min(total_lines.saturating_sub(1)) as i32,
                page: (visible_lines as u32).max(1),
            }
        } else {
            // Proportional scaling for huge (>1M) lines
            let scale = total_lines as f64 / MAX_SCROLL_RANGE as f64;
            let scaled_pos = (top_line as f64 / scale).round() as i32;
            let scaled_page = ((visible_lines as f64 / scale).round() as u32).max(1);

            ScrollMapping {
                min: 0,
                max: MAX_SCROLL_RANGE as i32,
                pos: scaled_pos.min(MAX_SCROLL_RANGE as i32),
                page: scaled_page,
            }
        }
    }

    /// Converts 32-bit scrollbar position back to 64-bit logical line index.
    pub fn scroll_pos_to_line(pos: i32, total_lines: usize) -> usize {
        if total_lines == 0 {
            return 0;
        }

        const MAX_SCROLL_RANGE: u64 = 1_000_000;

        if (total_lines as u64) <= MAX_SCROLL_RANGE {
            (pos as usize).min(total_lines.saturating_sub(1))
        } else {
            let scale = total_lines as f64 / MAX_SCROLL_RANGE as f64;
            ((pos as f64 * scale).round() as usize).min(total_lines.saturating_sub(1))
        }
    }

    /// Hit-tests a coordinate inside the grid client area.
    pub fn hit_test(
        x: i32,
        y: i32,
        top_line: usize,
        row_height: i32,
        gutter_width: i32,
        total_lines: usize,
    ) -> GridHit {
        if y < 0 || total_lines == 0 || row_height <= 0 {
            return GridHit::Outside;
        }

        let clicked_row = top_line + (y / row_height) as usize;
        if clicked_row >= total_lines {
            return GridHit::Outside;
        }

        if x < BOOKMARK_GUTTER_WIDTH {
            GridHit::BookmarkGutter(clicked_row)
        } else if x < gutter_width {
            GridHit::LineNumberGutter(clicked_row)
        } else {
            GridHit::Row(clicked_row)
        }
    }

    /// Renders visible rows of the virtual log grid using double-buffered GDI.
    pub unsafe fn draw(
        hdc: HDC,
        width: i32,
        height: i32,
        doc: Option<&mut Document>,
        theme: &ThemeTokens,
        font: HGDIOBJ,
        row_height: i32,
        char_width: i32,
    ) {
        // Base viewport background
        let bg_brush = CreateSolidBrush(theme.row_bg.to_colorref());
        let full_rect = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        FillRect(hdc, &full_rect, bg_brush);
        DeleteObject(bg_brush as _);

        let doc = match doc {
            Some(d) => d,
            None => {
                // Empty state message
                let old_font = SelectObject(hdc, font);
                SetBkMode(hdc, TRANSPARENT as _);
                SetTextColor(hdc, theme.text_muted.to_colorref());
                let msg: Vec<u16> = "No log file open. Use File -> Open (Ctrl+O) or Drag & Drop a file here.\0"
                    .encode_utf16()
                    .collect();
                TextOutW(hdc, 24, 24, msg.as_ptr(), (msg.len() - 1) as _);
                SelectObject(hdc, old_font);
                return;
            }
        };

        let total_lines = doc.line_count();
        if total_lines == 0 {
            let old_font = SelectObject(hdc, font);
            SetBkMode(hdc, TRANSPARENT as _);
            SetTextColor(hdc, theme.text_muted.to_colorref());
            let msg: Vec<u16> = "Log file is empty (0 lines).\0".encode_utf16().collect();
            TextOutW(hdc, 24, 24, msg.as_ptr(), (msg.len() - 1) as _);
            SelectObject(hdc, old_font);
            return;
        }

        let gutter_width = Self::calculate_gutter_width(total_lines, char_width);
        let visible_count = ((height / row_height) + 2) as usize;
        let top_line = doc.top_line;
        let selected_line = doc.selected_line;

        // Draw gutter background
        let gutter_brush = CreateSolidBrush(theme.row_gutter.to_colorref());
        let gutter_rect = RECT {
            left: 0,
            top: 0,
            right: gutter_width,
            bottom: height,
        };
        FillRect(hdc, &gutter_rect, gutter_brush);
        DeleteObject(gutter_brush as _);

        // Gutter right border
        let border_brush = CreateSolidBrush(theme.border_subtle.to_colorref());
        let border_rect = RECT {
            left: gutter_width - 1,
            top: 0,
            right: gutter_width,
            bottom: height,
        };
        FillRect(hdc, &border_rect, border_brush);
        DeleteObject(border_brush as _);

        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as _);

        // Render visible rows
        for i in 0..visible_count {
            let line_idx = top_line + i;
            if line_idx >= total_lines {
                break;
            }

            let y_pos = (i as i32) * row_height;
            let is_selected = line_idx == selected_line;
            let is_bookmarked = doc.is_bookmarked(line_idx);

            // Row background
            let row_rect = RECT {
                left: gutter_width,
                top: y_pos,
                right: width,
                bottom: y_pos + row_height,
            };

            let row_bg = if is_selected {
                theme.selection_bg.to_colorref()
            } else if line_idx % 2 == 1 {
                theme.row_alternate.to_colorref()
            } else {
                theme.row_bg.to_colorref()
            };

            let r_brush = CreateSolidBrush(row_bg);
            FillRect(hdc, &row_rect, r_brush);
            DeleteObject(r_brush as _);

            // Bookmark icon / glyph in gutter
            if is_bookmarked {
                let bm_brush = CreateSolidBrush(theme.accent_primary.to_colorref());
                let bm_rect = RECT {
                    left: 6,
                    top: y_pos + 4,
                    right: 18,
                    bottom: y_pos + row_height - 4,
                };
                FillRect(hdc, &bm_rect, bm_brush);
                DeleteObject(bm_brush as _);
            }

            // Line number in gutter
            SetTextColor(hdc, theme.text_muted.to_colorref());
            let line_num_str = format!("{}", line_idx + 1);
            let wide_num: Vec<u16> = std::ffi::OsStr::new(&line_num_str).encode_wide().collect();
            let num_x = gutter_width - 10 - (wide_num.len() as i32 * char_width);
            TextOutW(hdc, num_x, y_pos + 2, wide_num.as_ptr(), wide_num.len() as _);

            // Line text content
            if let Ok(line_text) = doc.get_line(line_idx) {
                let text_color = if is_selected {
                    theme.selection_fg.to_colorref()
                } else {
                    theme.text_primary.to_colorref()
                };

                SetTextColor(hdc, text_color);
                let wide_text: Vec<u16> = std::ffi::OsStr::new(&line_text).encode_wide().collect();
                let text_x = gutter_width + 8 - doc.scroll_x;
                TextOutW(hdc, text_x, y_pos + 2, wide_text.as_ptr(), wide_text.len() as _);

                // Highlight search match if query exists and matches this line
                if let Some(ref query) = doc.search_query {
                    if !query.is_empty() {
                        if let Some(match_pos) = line_text.to_lowercase().find(&query.to_lowercase()) {
                            let match_x = text_x + (match_pos as i32 * char_width);
                            let match_w = query.len() as i32 * char_width;
                            let match_rect = RECT {
                                left: match_x,
                                top: y_pos + 1,
                                right: match_x + match_w,
                                bottom: y_pos + row_height - 1,
                            };
                            let m_brush = CreateSolidBrush(theme.search_match_bg.to_colorref());
                            FillRect(hdc, &match_rect, m_brush);
                            DeleteObject(m_brush as _);

                            // Redraw matched text in match color
                            SetTextColor(hdc, theme.search_match_fg.to_colorref());
                            let matched_str = &line_text[match_pos..match_pos + query.len()];
                            let wide_match: Vec<u16> = std::ffi::OsStr::new(matched_str).encode_wide().collect();
                            TextOutW(hdc, match_x, y_pos + 2, wide_match.as_ptr(), wide_match.len() as _);
                        }
                    }
                }
            }
        }

        SelectObject(hdc, old_font);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_mapping_32bit_and_64bit() {
        // Small file (1000 lines) -> 1:1 mapping
        let sm = LogGrid::compute_scroll_info(1000, 50, 25);
        assert_eq!(sm.min, 0);
        assert_eq!(sm.max, 999);
        assert_eq!(sm.pos, 50);
        assert_eq!(sm.page, 25);
        assert_eq!(LogGrid::scroll_pos_to_line(50, 1000), 50);

        // Huge file (100,000,000 lines) -> scaled mapping without overflow
        let sm_huge = LogGrid::compute_scroll_info(100_000_000, 50_000_000, 30);
        assert_eq!(sm_huge.min, 0);
        assert_eq!(sm_huge.max, 1_000_000);
        assert_eq!(sm_huge.pos, 500_000);
        let mapped_line = LogGrid::scroll_pos_to_line(500_000, 100_000_000);
        assert_eq!(mapped_line, 50_000_000);
    }

    #[test]
    fn test_gutter_width_calculation() {
        let w_small = LogGrid::calculate_gutter_width(500, 8);
        assert!(w_small >= 48);
        let w_large = LogGrid::calculate_gutter_width(10_000_000, 8);
        assert!(w_large > w_small);
    }
}
