use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, FrameRect, SelectObject, SetBkMode, SetTextColor,
    TextOutW, HDC, HGDIOBJ, TRANSPARENT,
};

use crate::gui::commands::*;
use crate::gui::theme::ThemeTokens;

pub const TOOLBAR_HEIGHT: i32 = 32;

#[derive(Debug, Clone)]
pub struct ToolbarButton {
    pub id: u16,
    pub label: &'static str,
    pub width: i32,
    pub is_toggle: bool,
    pub is_active: bool,
}

pub struct Toolbar {
    pub buttons: Vec<ToolbarButton>,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl Toolbar {
    pub fn new() -> Self {
        let buttons = vec![
            ToolbarButton { id: ID_FILE_OPEN, label: "Open", width: 56, is_toggle: false, is_active: false },
            ToolbarButton { id: ID_FILE_RELOAD, label: "Reload", width: 60, is_toggle: false, is_active: false },
            ToolbarButton { id: ID_FILE_CLOSE, label: "Close", width: 54, is_toggle: false, is_active: false },
            ToolbarButton { id: ID_SEARCH_FIND, label: "Find", width: 50, is_toggle: false, is_active: false },
            ToolbarButton { id: ID_FILTER_TOGGLE, label: "Filter", width: 54, is_toggle: false, is_active: false },
            ToolbarButton { id: ID_TOOLS_BOOKMARK_TOGGLE, label: "Bookmark", width: 74, is_toggle: false, is_active: false },
            ToolbarButton { id: ID_VIEW_FOLLOW_TAIL, label: "Follow Tail", width: 84, is_toggle: true, is_active: true },
            ToolbarButton { id: ID_VIEW_PAUSE_MONITORING, label: "Pause", width: 58, is_toggle: true, is_active: false },
            ToolbarButton { id: ID_TOOLS_SETTINGS, label: "Settings", width: 68, is_toggle: false, is_active: false },
        ];
        Self { buttons }
    }

    /// Updates active state of toggle buttons based on current document status.
    pub fn update_states(&mut self, follow_tail: bool, paused: bool) {
        for btn in &mut self.buttons {
            if btn.id == ID_VIEW_FOLLOW_TAIL {
                btn.is_active = follow_tail;
            } else if btn.id == ID_VIEW_PAUSE_MONITORING {
                btn.is_active = paused;
            }
        }
    }

    /// Hit-tests a point (x, y) relative to toolbar and returns command ID if clicked.
    pub fn hit_test(&self, x: i32, y: i32) -> Option<u16> {
        if y < 2 || y >= TOOLBAR_HEIGHT - 2 {
            return None;
        }
        let mut curr_x = 8;
        for btn in &self.buttons {
            if x >= curr_x && x < curr_x + btn.width {
                return Some(btn.id);
            }
            curr_x += btn.width + 6;
        }
        None
    }

    /// Renders the toolbar into the specified HDC and bounds.
    pub unsafe fn draw(&self, hdc: HDC, width: i32, theme: &ThemeTokens, font: HGDIOBJ) {
        let bg_brush = CreateSolidBrush(theme.surface_secondary.to_colorref());
        let rect = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: TOOLBAR_HEIGHT,
        };
        FillRect(hdc, &rect, bg_brush);
        DeleteObject(bg_brush as _);

        // Bottom border
        let border_brush = CreateSolidBrush(theme.border_subtle.to_colorref());
        let border_rect = RECT {
            left: 0,
            top: TOOLBAR_HEIGHT - 1,
            right: width,
            bottom: TOOLBAR_HEIGHT,
        };
        FillRect(hdc, &border_rect, border_brush);
        DeleteObject(border_brush as _);

        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as _);

        let mut curr_x = 8;
        for btn in &self.buttons {
            let btn_rect = RECT {
                left: curr_x,
                top: 4,
                right: curr_x + btn.width,
                bottom: TOOLBAR_HEIGHT - 4,
            };

            let (btn_bg, text_color, border_color) = if btn.is_active {
                (
                    theme.accent_primary.to_colorref(),
                    theme.accent_on_solid.to_colorref(),
                    theme.accent_primary.to_colorref(),
                )
            } else {
                (
                    theme.surface_primary.to_colorref(),
                    theme.text_primary.to_colorref(),
                    theme.border_control.to_colorref(),
                )
            };

            let b_brush = CreateSolidBrush(btn_bg);
            FillRect(hdc, &btn_rect, b_brush);
            DeleteObject(b_brush as _);

            let f_brush = CreateSolidBrush(border_color);
            FrameRect(hdc, &btn_rect, f_brush);
            DeleteObject(f_brush as _);

            SetTextColor(hdc, text_color);
            let wide_label: Vec<u16> = std::ffi::OsStr::new(btn.label)
                .encode_wide()
                .collect();
            // Center text inside button
            let text_x = curr_x + 8;
            let text_y = 6;
            TextOutW(hdc, text_x, text_y, wide_label.as_ptr(), wide_label.len() as _);

            curr_x += btn.width + 6;
        }

        SelectObject(hdc, old_font);
    }
}
