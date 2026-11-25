use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, SelectObject, SetBkMode, SetTextColor, TextOutW, HDC,
    HGDIOBJ, TRANSPARENT,
};

use crate::core::workspace::Document;
use crate::gui::theme::ThemeTokens;

pub const DEFAULT_PANEL_HEIGHT: i32 = 150;
pub const PANEL_HEADER_HEIGHT: i32 = 26;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanelTab {
    FilterResults,
    Bookmarks,
    LineInspector,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PanelAction {
    SwitchTab(ActivePanelTab),
    JumpToLine(usize),
    ClosePanel,
    None,
}

pub struct InspectionPanels {
    pub is_visible: bool,
    pub height: i32,
    pub active_tab: ActivePanelTab,
    pub scroll_top: usize,
}

impl Default for InspectionPanels {
    fn default() -> Self {
        Self::new()
    }
}

impl InspectionPanels {
    pub fn new() -> Self {
        Self {
            is_visible: false,
            height: DEFAULT_PANEL_HEIGHT,
            active_tab: ActivePanelTab::Bookmarks,
            scroll_top: 0,
        }
    }

    pub fn toggle_visibility(&mut self) -> bool {
        self.is_visible = !self.is_visible;
        self.is_visible
    }

    /// Hit-tests mouse click inside inspection panels area (y relative to panel top: 0..height).
    pub fn hit_test(&mut self, x: i32, y: i32, width: i32, doc: Option<&Document>) -> PanelAction {
        if y < 0 || y >= self.height {
            return PanelAction::None;
        }

        // Header click (switching panel tabs or close button)
        if y < PANEL_HEADER_HEIGHT {
            // Close 'x' button on far right
            if x >= width - 28 {
                return PanelAction::ClosePanel;
            }

            // Tab 1: Filter Results (x: 8..140)
            if x >= 8 && x < 140 {
                self.active_tab = ActivePanelTab::FilterResults;
                return PanelAction::SwitchTab(ActivePanelTab::FilterResults);
            }
            // Tab 2: Bookmarks (x: 144..260)
            if x >= 144 && x < 260 {
                self.active_tab = ActivePanelTab::Bookmarks;
                return PanelAction::SwitchTab(ActivePanelTab::Bookmarks);
            }
            // Tab 3: Line Inspector (x: 264..380)
            if x >= 264 && x < 380 {
                self.active_tab = ActivePanelTab::LineInspector;
                return PanelAction::SwitchTab(ActivePanelTab::LineInspector);
            }
            return PanelAction::None;
        }

        // Content area click (selecting a line to jump to)
        let row_height = 20;
        let content_y = y - PANEL_HEADER_HEIGHT;
        let clicked_row = self.scroll_top + (content_y / row_height) as usize;

        let doc = match doc {
            Some(d) => d,
            None => return PanelAction::None,
        };

        match self.active_tab {
            ActivePanelTab::FilterResults => {
                if clicked_row < doc.filter_matches.len() {
                    let source_line = doc.filter_matches[clicked_row];
                    return PanelAction::JumpToLine(source_line);
                }
            }
            ActivePanelTab::Bookmarks => {
                let bms = doc.bookmarks();
                if clicked_row < bms.len() {
                    let source_line = bms[clicked_row];
                    return PanelAction::JumpToLine(source_line);
                }
            }
            ActivePanelTab::LineInspector => {}
        }

        PanelAction::None
    }

    /// Renders the inspection panels container and active tab contents.
    pub unsafe fn draw(
        &self,
        hdc: HDC,
        width: i32,
        y: i32,
        doc: Option<&mut Document>,
        theme: &ThemeTokens,
        font: HGDIOBJ,
    ) {
        if !self.is_visible {
            return;
        }

        // Panel container background
        let bg_brush = CreateSolidBrush(theme.surface_primary.to_colorref());
        let panel_rect = RECT {
            left: 0,
            top: y,
            right: width,
            bottom: y + self.height,
        };
        FillRect(hdc, &panel_rect, bg_brush);
        DeleteObject(bg_brush as _);

        // Header bar background
        let header_brush = CreateSolidBrush(theme.surface_secondary.to_colorref());
        let header_rect = RECT {
            left: 0,
            top: y,
            right: width,
            bottom: y + PANEL_HEADER_HEIGHT,
        };
        FillRect(hdc, &header_rect, header_brush);
        DeleteObject(header_brush as _);

        // Top splitter border (indicates draggable splitter)
        let splitter_brush = CreateSolidBrush(theme.border_control.to_colorref());
        let splitter_rect = RECT {
            left: 0,
            top: y,
            right: width,
            bottom: y + 2,
        };
        FillRect(hdc, &splitter_rect, splitter_brush);
        DeleteObject(splitter_brush as _);

        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as _);

        // Header Tabs: Filter Results, Bookmarks, Line Inspector
        let filter_count = doc.as_ref().map(|d| d.filter_matches.len()).unwrap_or(0);
        let bm_count = doc.as_ref().map(|d| d.bookmarks().len()).unwrap_or(0);

        let tab1_label = format!("Filter Results ({})", filter_count);
        let tab2_label = format!("Bookmarks ({})", bm_count);
        let tab3_label = "Line Inspector".to_string();

        let tabs = [
            (
                &tab1_label,
                8,
                140,
                self.active_tab == ActivePanelTab::FilterResults,
            ),
            (
                &tab2_label,
                144,
                260,
                self.active_tab == ActivePanelTab::Bookmarks,
            ),
            (
                &tab3_label,
                264,
                380,
                self.active_tab == ActivePanelTab::LineInspector,
            ),
        ];

        for (label, left_x, right_x, is_active) in tabs {
            let t_rect = RECT {
                left: left_x,
                top: y + 2,
                right: right_x,
                bottom: y + PANEL_HEADER_HEIGHT,
            };

            if is_active {
                let active_brush = CreateSolidBrush(theme.surface_primary.to_colorref());
                FillRect(hdc, &t_rect, active_brush);
                DeleteObject(active_brush as _);

                let stripe_brush = CreateSolidBrush(theme.accent_primary.to_colorref());
                let stripe_rect = RECT {
                    left: left_x,
                    top: y + 1,
                    right: right_x,
                    bottom: y + 3,
                };
                FillRect(hdc, &stripe_rect, stripe_brush);
                DeleteObject(stripe_brush as _);

                SetTextColor(hdc, theme.text_primary.to_colorref());
            } else {
                SetTextColor(hdc, theme.text_secondary.to_colorref());
            }

            let wide: Vec<u16> = std::ffi::OsStr::new(label).encode_wide().collect();
            TextOutW(hdc, left_x + 8, y + 6, wide.as_ptr(), wide.len() as _);
        }

        // Close 'x' button
        SetTextColor(hdc, theme.text_muted.to_colorref());
        let close_x: Vec<u16> = "×\0".encode_utf16().collect();
        TextOutW(hdc, width - 20, y + 4, close_x.as_ptr(), 1);

        // Content Area Rendering
        let content_y_start = y + PANEL_HEADER_HEIGHT;
        let content_height = self.height - PANEL_HEADER_HEIGHT;

        let doc = match doc {
            Some(d) => d,
            None => {
                SelectObject(hdc, old_font);
                return;
            }
        };

        match self.active_tab {
            ActivePanelTab::FilterResults => {
                if doc.filter_matches.is_empty() {
                    SetTextColor(hdc, theme.text_disabled.to_colorref());
                    let empty_msg: Vec<u16> =
                        "No filter applied. Use Filter -> Filter Results (Ctrl+Shift+F).\0"
                            .encode_utf16()
                            .collect();
                    TextOutW(
                        hdc,
                        16,
                        content_y_start + 12,
                        empty_msg.as_ptr(),
                        (empty_msg.len() - 1) as _,
                    );
                } else {
                    let row_h = 20;
                    let visible_rows = (content_height / row_h) as usize;
                    for r in 0..visible_rows {
                        let filter_idx = self.scroll_top + r;
                        if filter_idx >= doc.filter_matches.len() {
                            break;
                        }
                        let source_line = doc.filter_matches[filter_idx];
                        let py = content_y_start + (r as i32 * row_h);

                        // Line number
                        SetTextColor(hdc, theme.accent_primary.to_colorref());
                        let num_str = format!("Line {:<6}", source_line + 1);
                        let wide_num: Vec<u16> =
                            std::ffi::OsStr::new(&num_str).encode_wide().collect();
                        TextOutW(hdc, 12, py + 2, wide_num.as_ptr(), wide_num.len() as _);

                        // Line text
                        if let Ok(line_text) = doc.get_line(source_line) {
                            SetTextColor(hdc, theme.text_primary.to_colorref());
                            let wide_txt: Vec<u16> =
                                std::ffi::OsStr::new(&line_text).encode_wide().collect();
                            TextOutW(
                                hdc,
                                96,
                                py + 2,
                                wide_txt.as_ptr(),
                                wide_txt.len().min(120) as _,
                            );
                        }
                    }
                }
            }
            ActivePanelTab::Bookmarks => {
                let bms = doc.bookmarks().to_vec();
                if bms.is_empty() {
                    SetTextColor(hdc, theme.text_disabled.to_colorref());
                    let empty_msg: Vec<u16> =
                        "No bookmarks yet. Press F2 or click the left gutter to bookmark a line.\0"
                            .encode_utf16()
                            .collect();
                    TextOutW(
                        hdc,
                        16,
                        content_y_start + 12,
                        empty_msg.as_ptr(),
                        (empty_msg.len() - 1) as _,
                    );
                } else {
                    let row_h = 20;
                    let visible_rows = (content_height / row_h) as usize;
                    for r in 0..visible_rows {
                        let bm_idx = self.scroll_top + r;
                        if bm_idx >= bms.len() {
                            break;
                        }
                        let source_line = bms[bm_idx];
                        let py = content_y_start + (r as i32 * row_h);

                        SetTextColor(hdc, theme.accent_primary.to_colorref());
                        let num_str = format!("Bookmark on Line {:<6}", source_line + 1);
                        let wide_num: Vec<u16> =
                            std::ffi::OsStr::new(&num_str).encode_wide().collect();
                        TextOutW(hdc, 12, py + 2, wide_num.as_ptr(), wide_num.len() as _);

                        if let Ok(line_text) = doc.get_line(source_line) {
                            SetTextColor(hdc, theme.text_primary.to_colorref());
                            let wide_txt: Vec<u16> =
                                std::ffi::OsStr::new(&line_text).encode_wide().collect();
                            TextOutW(
                                hdc,
                                180,
                                py + 2,
                                wide_txt.as_ptr(),
                                wide_txt.len().min(120) as _,
                            );
                        }
                    }
                }
            }
            ActivePanelTab::LineInspector => {
                let sel_line = doc.selected_line;
                if let Ok(line_text) = doc.get_line(sel_line) {
                    SetTextColor(hdc, theme.text_secondary.to_colorref());
                    let meta_str = format!(
                        "Line {}: {} characters, {} bytes",
                        sel_line + 1,
                        line_text.chars().count(),
                        line_text.len()
                    );
                    let wide_meta: Vec<u16> =
                        std::ffi::OsStr::new(&meta_str).encode_wide().collect();
                    TextOutW(
                        hdc,
                        16,
                        content_y_start + 8,
                        wide_meta.as_ptr(),
                        wide_meta.len() as _,
                    );

                    SetTextColor(hdc, theme.text_primary.to_colorref());
                    let wide_txt: Vec<u16> =
                        std::ffi::OsStr::new(&line_text).encode_wide().collect();
                    TextOutW(
                        hdc,
                        16,
                        content_y_start + 32,
                        wide_txt.as_ptr(),
                        wide_txt.len().min(180) as _,
                    );
                }
            }
        }

        SelectObject(hdc, old_font);
    }
}
