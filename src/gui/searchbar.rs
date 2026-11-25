use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, FrameRect, SelectObject, SetBkMode, SetTextColor,
    TextOutW, HDC, HGDIOBJ, TRANSPARENT,
};

use crate::core::search::SearchOptions;
use crate::gui::theme::ThemeTokens;

pub const SEARCHBAR_HEIGHT: i32 = 32;

#[derive(Debug, PartialEq, Eq)]
pub enum SearchBarAction {
    ToggleCase,
    ToggleWholeWord,
    ToggleRegex,
    FindPrevious,
    FindNext,
    Close,
    InputClicked,
    None,
}

pub struct SearchBar {
    pub is_visible: bool,
    pub query: String,
    pub options: SearchOptions,
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            is_visible: false,
            query: String::new(),
            options: SearchOptions::default(),
        }
    }

    pub fn toggle_visibility(&mut self) -> bool {
        self.is_visible = !self.is_visible;
        self.is_visible
    }

    /// Hit-tests a click at (x, y) relative to search bar.
    pub fn hit_test(&self, x: i32, y: i32, width: i32) -> SearchBarAction {
        if y < 2 || y >= SEARCHBAR_HEIGHT - 2 {
            return SearchBarAction::None;
        }

        // Search input area: x = 60 to (width - 240)
        let input_right = (width - 240).max(120);
        if x >= 60 && x < input_right {
            return SearchBarAction::InputClicked;
        }

        // Buttons on right: [Aa: 32px] [W: 30px] [.*: 32px] [^: 28px] [v: 28px] [x: 28px]
        let mut curr_x = input_right + 8;

        // [Aa]
        if x >= curr_x && x < curr_x + 32 {
            return SearchBarAction::ToggleCase;
        }
        curr_x += 36;

        // [W]
        if x >= curr_x && x < curr_x + 30 {
            return SearchBarAction::ToggleWholeWord;
        }
        curr_x += 34;

        // [.*]
        if x >= curr_x && x < curr_x + 32 {
            return SearchBarAction::ToggleRegex;
        }
        curr_x += 36;

        // [Prev]
        if x >= curr_x && x < curr_x + 28 {
            return SearchBarAction::FindPrevious;
        }
        curr_x += 32;

        // [Next]
        if x >= curr_x && x < curr_x + 28 {
            return SearchBarAction::FindNext;
        }
        curr_x += 32;

        // [Close]
        if x >= curr_x && x < curr_x + 28 {
            return SearchBarAction::Close;
        }

        SearchBarAction::None
    }

    /// Renders the contextual search bar.
    pub unsafe fn draw(
        &self,
        hdc: HDC,
        width: i32,
        y: i32,
        match_info: &str,
        theme: &ThemeTokens,
        font: HGDIOBJ,
    ) {
        if !self.is_visible {
            return;
        }

        // Background
        let bg_brush = CreateSolidBrush(theme.surface_secondary.to_colorref());
        let bar_rect = RECT {
            left: 0,
            top: y,
            right: width,
            bottom: y + SEARCHBAR_HEIGHT,
        };
        FillRect(hdc, &bar_rect, bg_brush);
        DeleteObject(bg_brush as _);

        // Bottom border
        let border_brush = CreateSolidBrush(theme.border_subtle.to_colorref());
        let border_rect = RECT {
            left: 0,
            top: y + SEARCHBAR_HEIGHT - 1,
            right: width,
            bottom: y + SEARCHBAR_HEIGHT,
        };
        FillRect(hdc, &border_rect, border_brush);
        DeleteObject(border_brush as _);

        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT as _);

        // "Find:" label
        SetTextColor(hdc, theme.text_secondary.to_colorref());
        let label_wide: Vec<u16> = "Find:\0".encode_utf16().collect();
        TextOutW(hdc, 12, y + 7, label_wide.as_ptr(), 5);

        // Search Input Box
        let input_right = (width - 240).max(120);
        let input_rect = RECT {
            left: 56,
            top: y + 4,
            right: input_right,
            bottom: y + SEARCHBAR_HEIGHT - 4,
        };

        let in_brush = CreateSolidBrush(theme.surface_primary.to_colorref());
        FillRect(hdc, &input_rect, in_brush);
        DeleteObject(in_brush as _);

        let in_border = CreateSolidBrush(theme.border_control.to_colorref());
        FrameRect(hdc, &input_rect, in_border);
        DeleteObject(in_border as _);

        // Input text or placeholder
        if self.query.is_empty() {
            SetTextColor(hdc, theme.text_disabled.to_colorref());
            let ph: Vec<u16> = "Type text to search (Enter to find next, Esc to close)...\0"
                .encode_utf16()
                .collect();
            TextOutW(hdc, 62, y + 6, ph.as_ptr(), (ph.len() - 1) as _);
        } else {
            SetTextColor(hdc, theme.text_primary.to_colorref());
            let q_wide: Vec<u16> = std::ffi::OsStr::new(&self.query).encode_wide().collect();
            TextOutW(hdc, 62, y + 6, q_wide.as_ptr(), q_wide.len() as _);
        }

        // Draw buttons: [Aa] [W] [.*] [^] [v] [x]
        let mut curr_x = input_right + 8;
        let buttons = [
            ("Aa", 32, self.options.case_sensitive),
            ("W", 30, self.options.whole_word),
            (".*", 32, self.options.is_regex),
            ("▲", 28, false),
            ("▼", 28, false),
            ("×", 28, false),
        ];

        for (btn_text, btn_w, is_active) in buttons {
            let b_rect = RECT {
                left: curr_x,
                top: y + 4,
                right: curr_x + btn_w,
                bottom: y + SEARCHBAR_HEIGHT - 4,
            };

            let (btn_bg, text_color) = if is_active {
                (theme.accent_primary.to_colorref(), theme.accent_on_solid.to_colorref())
            } else {
                (theme.surface_primary.to_colorref(), theme.text_primary.to_colorref())
            };

            let b_brush = CreateSolidBrush(btn_bg);
            FillRect(hdc, &b_rect, b_brush);
            DeleteObject(b_brush as _);

            let f_brush = CreateSolidBrush(theme.border_control.to_colorref());
            FrameRect(hdc, &b_rect, f_brush);
            DeleteObject(f_brush as _);

            SetTextColor(hdc, text_color);
            let wide: Vec<u16> = std::ffi::OsStr::new(btn_text).encode_wide().collect();
            let tx = curr_x + (btn_w - (wide.len() as i32 * 8)).max(4) / 2;
            TextOutW(hdc, tx, y + 6, wide.as_ptr(), wide.len() as _);

            curr_x += btn_w + 4;
        }

        // Draw match count info if present
        if !match_info.is_empty() {
            SetTextColor(hdc, theme.text_secondary.to_colorref());
            let m_wide: Vec<u16> = std::ffi::OsStr::new(match_info).encode_wide().collect();
            TextOutW(hdc, curr_x + 8, y + 7, m_wide.as_ptr(), m_wide.len() as _);
        }

        SelectObject(hdc, old_font);
    }
}
