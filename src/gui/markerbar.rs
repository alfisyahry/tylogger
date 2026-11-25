use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, FrameRect, HDC,
};

use crate::core::workspace::Document;
use crate::gui::theme::ThemeTokens;

pub const MARKERBAR_WIDTH: i32 = 14;

pub struct MarkerBar;

impl MarkerBar {
    /// Hit-tests a click at y-coordinate inside marker bar (top: y_start, height: total_height).
    /// Returns the target 64-bit logical line index to scroll to.
    pub fn hit_test(y_click: i32, y_start: i32, height: i32, total_lines: usize) -> Option<usize> {
        if total_lines == 0 || height <= 0 {
            return None;
        }
        let rel_y = y_click - y_start;
        if rel_y < 0 || rel_y >= height {
            return None;
        }
        let ratio = rel_y as f64 / height as f64;
        let target = (ratio * total_lines as f64).round() as usize;
        Some(target.min(total_lines - 1))
    }

    /// Renders the overview marker bar.
    pub unsafe fn draw(
        hdc: HDC,
        x: i32,
        y: i32,
        height: i32,
        doc: Option<&Document>,
        theme: &ThemeTokens,
    ) {
        if height <= 0 {
            return;
        }

        // Marker bar background
        let bg_brush = CreateSolidBrush(theme.surface_secondary.to_colorref());
        let bar_rect = RECT {
            left: x,
            top: y,
            right: x + MARKERBAR_WIDTH,
            bottom: y + height,
        };
        FillRect(hdc, &bar_rect, bg_brush);
        DeleteObject(bg_brush as _);

        // Left border
        let border_brush = CreateSolidBrush(theme.border_subtle.to_colorref());
        let border_rect = RECT {
            left: x,
            top: y,
            right: x + 1,
            bottom: y + height,
        };
        FillRect(hdc, &border_rect, border_brush);
        DeleteObject(border_brush as _);

        let doc = match doc {
            Some(d) => d,
            None => return,
        };

        let total_lines = doc.line_count();
        if total_lines == 0 {
            return;
        }

        // 1. Draw Search Matches (Amber ticks)
        if !doc.search_matches.is_empty() {
            let match_brush = CreateSolidBrush(theme.search_match_bg.to_colorref());
            for &match_line in &doc.search_matches {
                let tick_y = y + ((match_line as f64 / total_lines as f64) * height as f64) as i32;
                let tick_rect = RECT {
                    left: x + 2,
                    top: tick_y.max(y).min(y + height - 2),
                    right: x + MARKERBAR_WIDTH - 2,
                    bottom: (tick_y + 2).min(y + height),
                };
                FillRect(hdc, &tick_rect, match_brush);
            }
            DeleteObject(match_brush as _);
        }

        // 2. Draw Bookmarks (Vibrant blue/cyan markers)
        let bookmarks = doc.bookmarks();
        if !bookmarks.is_empty() {
            let bm_brush = CreateSolidBrush(theme.accent_primary.to_colorref());
            for &bm_line in bookmarks {
                let tick_y = y + ((bm_line as f64 / total_lines as f64) * height as f64) as i32;
                let tick_rect = RECT {
                    left: x + 1,
                    top: tick_y.max(y).min(y + height - 3),
                    right: x + MARKERBAR_WIDTH - 1,
                    bottom: (tick_y + 3).min(y + height),
                };
                FillRect(hdc, &tick_rect, bm_brush);
            }
            DeleteObject(bm_brush as _);
        }

        // 3. Viewport thumb overlay (indicates current scroll position and visible fraction)
        let visible_lines = 30.min(total_lines);
        let thumb_top = y + ((doc.top_line as f64 / total_lines as f64) * height as f64) as i32;
        let thumb_height = ((visible_lines as f64 / total_lines as f64) * height as f64).max(6.0) as i32;

        let thumb_rect = RECT {
            left: x + 1,
            top: thumb_top.min(y + height - thumb_height),
            right: x + MARKERBAR_WIDTH - 1,
            bottom: (thumb_top + thumb_height).min(y + height),
        };

        let thumb_border = CreateSolidBrush(theme.border_control.to_colorref());
        FrameRect(hdc, &thumb_rect, thumb_border);
        DeleteObject(thumb_border as _);
    }
}
