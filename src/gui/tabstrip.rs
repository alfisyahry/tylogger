use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, FrameRect, SelectObject, SetBkMode, SetTextColor,
    TextOutW, HDC, HGDIOBJ, TRANSPARENT,
};

use crate::core::workspace::Workspace;
use crate::gui::theme::ThemeTokens;

pub const TABSTRIP_HEIGHT: i32 = 28;

#[derive(Debug, PartialEq, Eq)]
pub enum TabHitResult {
    SwitchTab(usize),
    CloseTab(usize),
    ContextMenu(usize),
    None,
}

pub struct TabStrip;

impl TabStrip {
    /// Hit-tests a point (x, y) relative to the tab strip.
    pub fn hit_test(
        workspace: &Workspace,
        x: i32,
        y: i32,
        is_right_click: bool,
        is_middle_click: bool,
    ) -> TabHitResult {
        if y < 0 || y >= TABSTRIP_HEIGHT || workspace.is_empty() {
            return TabHitResult::None;
        }

        let mut curr_x = 4;
        let tab_width = 160;

        for (idx, _doc) in (0..workspace.document_count()).enumerate() {
            let tab_right = curr_x + tab_width;
            if x >= curr_x && x < tab_right {
                if is_middle_click {
                    return TabHitResult::CloseTab(idx);
                }
                if is_right_click {
                    return TabHitResult::ContextMenu(idx);
                }
                // Check if close 'x' clicked (rightmost 20px of tab)
                if x >= tab_right - 22 {
                    return TabHitResult::CloseTab(idx);
                }
                return TabHitResult::SwitchTab(idx);
            }
            curr_x += tab_width + 4;
        }

        TabHitResult::None
    }

    /// Renders the document tab strip.
    pub unsafe fn draw(
        hdc: HDC,
        width: i32,
        y_offset: i32,
        workspace: &Workspace,
        theme: &ThemeTokens,
        font: HGDIOBJ,
    ) {
        // Tabstrip background
        let bg_brush = CreateSolidBrush(theme.window_bg.to_colorref());
        let rect = RECT {
            left: 0,
            top: y_offset,
            right: width,
            bottom: y_offset + TABSTRIP_HEIGHT,
        };
        FillRect(hdc, &rect, bg_brush);
        DeleteObject(bg_brush as _);

        // Bottom divider
        let divider_brush = CreateSolidBrush(theme.border_subtle.to_colorref());
        let div_rect = RECT {
            left: 0,
            top: y_offset + TABSTRIP_HEIGHT - 1,
            right: width,
            bottom: y_offset + TABSTRIP_HEIGHT,
        };
        FillRect(hdc, &div_rect, divider_brush);
        DeleteObject(divider_brush as _);

        if workspace.is_empty() {
            return;
        }

        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as _);

        let mut curr_x = 4;
        let tab_width = 160;
        let active_idx = workspace.active_index();

        for idx in 0..workspace.document_count() {
            let doc = match workspace.document(idx) {
                Some(d) => d,
                None => continue,
            };

            let is_active = idx == active_idx;
            let tab_rect = RECT {
                left: curr_x,
                top: y_offset + 3,
                right: curr_x + tab_width,
                bottom: y_offset + TABSTRIP_HEIGHT,
            };

            let (tab_bg, text_color) = if is_active {
                (
                    theme.surface_primary.to_colorref(),
                    theme.text_primary.to_colorref(),
                )
            } else {
                (
                    theme.surface_secondary.to_colorref(),
                    theme.text_secondary.to_colorref(),
                )
            };

            let t_brush = CreateSolidBrush(tab_bg);
            FillRect(hdc, &tab_rect, t_brush);
            DeleteObject(t_brush as _);

            let border_brush = CreateSolidBrush(theme.border_subtle.to_colorref());
            FrameRect(hdc, &tab_rect, border_brush);
            DeleteObject(border_brush as _);

            // Active tab top indicator stripe
            if is_active {
                let stripe_brush = CreateSolidBrush(theme.accent_primary.to_colorref());
                let stripe_rect = RECT {
                    left: curr_x,
                    top: y_offset + 1,
                    right: curr_x + tab_width,
                    bottom: y_offset + 4,
                };
                FillRect(hdc, &stripe_rect, stripe_brush);
                DeleteObject(stripe_brush as _);
            }

            // Draw Activity Dot if new lines arrived in background
            if doc.has_new_activity && !is_active {
                let dot_brush = CreateSolidBrush(theme.severity_warning.fg.to_colorref());
                let dot_rect = RECT {
                    left: curr_x + 6,
                    top: y_offset + 10,
                    right: curr_x + 12,
                    bottom: y_offset + 16,
                };
                FillRect(hdc, &dot_rect, dot_brush);
                DeleteObject(dot_brush as _);
            }

            // Draw Tab Title (truncated if long)
            SetTextColor(hdc, text_color);
            let title_prefix = if doc.has_new_activity && !is_active {
                "  "
            } else {
                ""
            };
            let display_title = format!("{}{}", title_prefix, doc.title);
            let wide_title: Vec<u16> = std::ffi::OsStr::new(&display_title).encode_wide().collect();
            let title_len = wide_title.len().min(14);
            TextOutW(
                hdc,
                curr_x + 8,
                y_offset + 6,
                wide_title.as_ptr(),
                title_len as _,
            );

            // Draw Close 'x' button
            SetTextColor(hdc, theme.text_muted.to_colorref());
            let close_x: Vec<u16> = "×\0".encode_utf16().collect();
            TextOutW(
                hdc,
                curr_x + tab_width - 16,
                y_offset + 5,
                close_x.as_ptr(),
                1,
            );

            curr_x += tab_width + 4;
        }

        SelectObject(hdc, old_font);
    }
}
