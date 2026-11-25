use std::ffi::c_void;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush,
    DeleteDC, DeleteObject, EndPaint, FillRect, GetDC, GetTextMetricsW, InvalidateRect, ReleaseDC,
    SelectObject, SetBkMode, SetTextColor, TextOutW, UpdateWindow, COLOR_WINDOW, HGDIOBJ,
    OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, TEXTMETRICW, TRANSPARENT, VARIABLE_PITCH,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::SetScrollInfo;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_B, VK_DOWN, VK_END, VK_F2, VK_F7, VK_HOME, VK_NEXT, VK_PRIOR, VK_T, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, GetScrollInfo,
    GetWindowLongPtrW, KillTimer, PostQuitMessage, RegisterClassW, SetTimer, SetWindowLongPtrW,
    SetWindowTextW, ShowWindow, TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, GWLP_USERDATA, SB_LINEDOWN, SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION,
    SB_THUMBTRACK, SB_VERT, SCROLLINFO, SIF_ALL, SIF_PAGE, SIF_POS, SIF_RANGE, SW_SHOW, WM_CREATE,
    WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_MOUSEWHEEL, WM_PAINT, WM_SIZE, WM_TIMER, WM_VSCROLL,
    WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VSCROLL,
};

use crate::config::WINDOW_CLASS_NAME;
use crate::core::{NonLockingLogReader, UpdateEvent};
use crate::gui::theme::ThemeMode;

pub struct AppState {
    pub reader: NonLockingLogReader,
    pub top_line: usize,
    pub visible_lines: usize,
    pub line_height: i32,
    pub font: HGDIOBJ,
    pub follow_tail: bool,
    pub selected_line: usize,
    pub theme_mode: ThemeMode,
}

impl AppState {
    pub fn new(reader: NonLockingLogReader) -> Self {
        Self {
            reader,
            top_line: 0,
            visible_lines: 25,
            line_height: 18,
            font: null_mut(),
            follow_tail: true,
            selected_line: 0,
            theme_mode: ThemeMode::Light,
        }
    }
}

pub fn run_gui_app(mut state: AppState) -> Result<(), String> {
    unsafe {
        let instance = GetModuleHandleW(null_mut());
        let class_name: Vec<u16> = format!("{}\0", WINDOW_CLASS_NAME).encode_utf16().collect();
        let title: Vec<u16> = "Tylogger\0".encode_utf16().collect();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: (COLOR_WINDOW + 1) as _,
            lpszMenuName: null_mut(),
            lpszClassName: class_name.as_ptr(),
        };

        RegisterClassW(&wc);

        let state_ptr = &mut state as *mut AppState;

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VSCROLL,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1024,
            720,
            null_mut(),
            null_mut(),
            instance,
            state_ptr as *const c_void,
        );

        if hwnd.is_null() {
            return Err("Failed to create Win32 main window".to_string());
        }

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        Ok(())
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;

    match msg {
        WM_CREATE => {
            let cs = lparam as *const CREATESTRUCTW;
            let state_ptr = (*cs).lpCreateParams as *mut AppState;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

            let font_name: Vec<u16> = "Consolas\0".encode_utf16().collect();
            let font = CreateFontW(
                -15,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                0,
                OUT_DEFAULT_PRECIS as u32,
                0,
                0,
                VARIABLE_PITCH as u32,
                font_name.as_ptr(),
            );

            if !state_ptr.is_null() {
                (*state_ptr).font = font as HGDIOBJ;

                // Measure actual font line height
                let hdc = GetDC(hwnd);
                let old_font = SelectObject(hdc, font as HGDIOBJ);
                let mut tm: TEXTMETRICW = std::mem::zeroed();
                GetTextMetricsW(hdc, &mut tm);
                (*state_ptr).line_height = (tm.tmHeight + tm.tmExternalLeading).max(16);
                SelectObject(hdc, old_font);
                ReleaseDC(hwnd, hdc);

                // Initial tail position
                let count = (*state_ptr).reader.line_count();
                if (*state_ptr).follow_tail && count > (*state_ptr).visible_lines {
                    (*state_ptr).top_line = count - (*state_ptr).visible_lines;
                }
            }

            // 250ms polling timer for live tailing
            SetTimer(hwnd, 1, 250, None);
            0
        }

        WM_SIZE => {
            if !state_ptr.is_null() {
                let height = (lparam >> 16) as i32;
                let lh = (*state_ptr).line_height.max(1);
                (*state_ptr).visible_lines = (height / lh).max(1) as usize;
                update_scroll_info(hwnd, &*state_ptr);
                InvalidateRect(hwnd, null_mut(), 0);
            }
            0
        }

        WM_VSCROLL => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let code = (wparam & 0xFFFF) as i32;
                let total = state.reader.line_count();

                if code == SB_LINEUP {
                    state.top_line = state.top_line.saturating_sub(1);
                    state.follow_tail = false;
                } else if code == SB_LINEDOWN {
                    if state.top_line + state.visible_lines < total {
                        state.top_line += 1;
                    }
                } else if code == SB_PAGEUP {
                    state.top_line = state.top_line.saturating_sub(state.visible_lines);
                    state.follow_tail = false;
                } else if code == SB_PAGEDOWN {
                    state.top_line = (state.top_line + state.visible_lines)
                        .min(total.saturating_sub(state.visible_lines));
                } else if code == SB_THUMBTRACK || code == SB_THUMBPOSITION {
                    let mut si: SCROLLINFO = std::mem::zeroed();
                    si.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
                    si.fMask = SIF_ALL;
                    GetScrollInfo(hwnd, SB_VERT, &mut si);
                    state.top_line = (si.nTrackPos as usize).min(total.saturating_sub(1));
                    state.follow_tail = false;
                }

                update_scroll_info(hwnd, state);
                update_title(hwnd, state);
                InvalidateRect(hwnd, null_mut(), 0);
            }
            0
        }

        WM_MOUSEWHEEL => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let delta = (wparam >> 16) as i16;
                let lines_to_scroll = (delta / 40).unsigned_abs() as usize;
                let total = state.reader.line_count();

                if delta > 0 {
                    state.top_line = state.top_line.saturating_sub(lines_to_scroll);
                    state.follow_tail = false;
                } else if state.top_line + state.visible_lines < total {
                    state.top_line = (state.top_line + lines_to_scroll)
                        .min(total.saturating_sub(state.visible_lines));
                }

                update_scroll_info(hwnd, state);
                update_title(hwnd, state);
                InvalidateRect(hwnd, null_mut(), 0);
            }
            0
        }

        WM_KEYDOWN => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let total = state.reader.line_count();
                let key = wparam as u16;

                if key == VK_UP {
                    state.top_line = state.top_line.saturating_sub(1);
                    state.selected_line = state.selected_line.saturating_sub(1);
                    state.follow_tail = false;
                } else if key == VK_DOWN {
                    if state.top_line + state.visible_lines < total {
                        state.top_line += 1;
                    }
                    if state.selected_line + 1 < total {
                        state.selected_line += 1;
                    }
                } else if key == VK_PRIOR {
                    state.top_line = state.top_line.saturating_sub(state.visible_lines);
                    state.follow_tail = false;
                } else if key == VK_NEXT {
                    state.top_line = (state.top_line + state.visible_lines)
                        .min(total.saturating_sub(state.visible_lines));
                } else if key == VK_HOME {
                    state.top_line = 0;
                    state.selected_line = 0;
                    state.follow_tail = false;
                } else if key == VK_END {
                    state.top_line = total.saturating_sub(state.visible_lines);
                    state.selected_line = total.saturating_sub(1);
                    state.follow_tail = true;
                } else if key == VK_T {
                    state.follow_tail = !state.follow_tail;
                    if state.follow_tail {
                        state.top_line = total.saturating_sub(state.visible_lines);
                    }
                } else if key == VK_B {
                    let target = state.selected_line;
                    state.reader.toggle_bookmark(target);
                } else if key == VK_F2 {
                    // Jump to next bookmark
                    let curr = state.top_line;
                    if let Some(&next_bm) = state.reader.bookmarks().iter().find(|&&b| b > curr) {
                        state.top_line = next_bm;
                        state.selected_line = next_bm;
                        state.follow_tail = false;
                    }
                } else if key == VK_F7 {
                    // Cycle theme (Light -> Dark -> Soft Dark)
                    state.theme_mode = state.theme_mode.cycle();
                }

                update_scroll_info(hwnd, state);
                update_title(hwnd, state);
                InvalidateRect(hwnd, null_mut(), 0);
            }
            0
        }

        WM_TIMER => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if let Ok(event) = state.reader.check_for_updates() {
                    match event {
                        UpdateEvent::Appended { new_lines } if new_lines > 0 => {
                            let total = state.reader.line_count();
                            if state.follow_tail && total > state.visible_lines {
                                state.top_line = total - state.visible_lines;
                            }
                            update_scroll_info(hwnd, state);
                            update_title(hwnd, state);
                            InvalidateRect(hwnd, null_mut(), 0);
                        }
                        UpdateEvent::Truncated => {
                            state.top_line = 0;
                            state.selected_line = 0;
                            update_scroll_info(hwnd, state);
                            update_title(hwnd, state);
                            InvalidateRect(hwnd, null_mut(), 0);
                        }
                        _ => {}
                    }
                }
            }
            0
        }

        WM_ERASEBKGND => {
            // Prevent flicker: background is drawn in double-buffered WM_PAINT
            1
        }

        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let tokens = state.theme_mode.tokens();

                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;

                // Double buffer
                let mem_dc = CreateCompatibleDC(hdc);
                let mem_bmp = CreateCompatibleBitmap(hdc, width, height);
                let old_bmp = SelectObject(mem_dc, mem_bmp as HGDIOBJ);

                // Background (surface.primary)
                let bg_brush = CreateSolidBrush(tokens.surface_primary.to_colorref());
                FillRect(mem_dc, &rect, bg_brush);
                DeleteObject(bg_brush as HGDIOBJ);

                // Gutter background
                let gutter_width = 72;
                let gutter_rect = RECT {
                    left: 0,
                    top: 0,
                    right: gutter_width,
                    bottom: height,
                };
                let gutter_brush = CreateSolidBrush(tokens.row_gutter.to_colorref());
                FillRect(mem_dc, &gutter_rect, gutter_brush);
                DeleteObject(gutter_brush as HGDIOBJ);

                // Gutter divider (border.subtle)
                let border_rect = RECT {
                    left: gutter_width - 1,
                    top: 0,
                    right: gutter_width,
                    bottom: height,
                };
                let border_brush = CreateSolidBrush(tokens.border_subtle.to_colorref());
                FillRect(mem_dc, &border_rect, border_brush);
                DeleteObject(border_brush as HGDIOBJ);

                let old_font = if !state.font.is_null() {
                    SelectObject(mem_dc, state.font)
                } else {
                    null_mut()
                };

                SetBkMode(mem_dc, TRANSPARENT as i32);

                let total_lines = state.reader.line_count();
                for i in 0..state.visible_lines {
                    let line_idx = state.top_line + i;
                    if line_idx >= total_lines {
                        break;
                    }

                    let y = (i as i32) * state.line_height;
                    let is_selected = line_idx == state.selected_line;
                    let is_bm = state.reader.is_bookmarked(line_idx);

                    // Row background: selection or alternate
                    if is_selected {
                        let sel_rect = RECT {
                            left: gutter_width,
                            top: y,
                            right: width,
                            bottom: y + state.line_height,
                        };
                        let sel_brush = CreateSolidBrush(tokens.selection_bg.to_colorref());
                        FillRect(mem_dc, &sel_rect, sel_brush);
                        DeleteObject(sel_brush as HGDIOBJ);
                    } else if line_idx % 2 == 1 {
                        let alt_rect = RECT {
                            left: gutter_width,
                            top: y,
                            right: width,
                            bottom: y + state.line_height,
                        };
                        let alt_brush = CreateSolidBrush(tokens.row_alternate.to_colorref());
                        FillRect(mem_dc, &alt_rect, alt_brush);
                        DeleteObject(alt_brush as HGDIOBJ);
                    }

                    // Bookmark star in gutter
                    if is_bm {
                        SetTextColor(mem_dc, tokens.accent_primary.to_colorref());
                        let star: Vec<u16> = "*\0".encode_utf16().collect();
                        TextOutW(mem_dc, 6, y, star.as_ptr(), 1);
                    }

                    // Line number
                    let num_str = format!("{:>6}", line_idx + 1);
                    let wide_num: Vec<u16> = num_str.encode_utf16().collect();
                    SetTextColor(mem_dc, tokens.text_muted.to_colorref());
                    TextOutW(mem_dc, 16, y, wide_num.as_ptr(), wide_num.len() as i32);

                    // Line content with semantic severity coloring
                    if let Ok(line_text) = state.reader.get_line(line_idx) {
                        let text_color = if is_selected {
                            tokens.selection_fg.to_colorref()
                        } else if line_text.contains("CRITICAL") || line_text.contains("FATAL") {
                            tokens.severity_critical.fg.to_colorref()
                        } else if line_text.contains("ERROR") {
                            tokens.severity_error.fg.to_colorref()
                        } else if line_text.contains("WARN") {
                            tokens.severity_warning.fg.to_colorref()
                        } else if line_text.contains("NOTICE") {
                            tokens.severity_notice.fg.to_colorref()
                        } else if line_text.contains("INFO") || line_text.contains("SUCCESS") {
                            tokens.severity_info.fg.to_colorref()
                        } else if line_text.contains("DEBUG") || line_text.contains("TRACE") {
                            tokens.severity_trace.fg.to_colorref()
                        } else {
                            tokens.text_primary.to_colorref()
                        };

                        SetTextColor(mem_dc, text_color);
                        let wide_text: Vec<u16> = line_text.encode_utf16().collect();
                        TextOutW(
                            mem_dc,
                            gutter_width + 8,
                            y,
                            wide_text.as_ptr(),
                            wide_text.len() as i32,
                        );
                    }
                }

                // Blit buffer to screen
                BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

                // Cleanup GDI objects
                if !old_font.is_null() {
                    SelectObject(mem_dc, old_font);
                }
                SelectObject(mem_dc, old_bmp);
                DeleteObject(mem_bmp as HGDIOBJ);
                DeleteDC(mem_dc);
            }

            EndPaint(hwnd, &ps);
            0
        }

        WM_DESTROY => {
            KillTimer(hwnd, 1);
            if !state_ptr.is_null() && !(*state_ptr).font.is_null() {
                DeleteObject((*state_ptr).font);
            }
            PostQuitMessage(0);
            0
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn update_scroll_info(hwnd: HWND, state: &AppState) {
    let total = state.reader.line_count();
    let mut si: SCROLLINFO = std::mem::zeroed();
    si.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
    si.fMask = SIF_RANGE | SIF_PAGE | SIF_POS;
    si.nMin = 0;
    si.nMax = total.saturating_sub(1) as i32;
    si.nPage = state.visible_lines as u32;
    si.nPos = state.top_line as i32;
    SetScrollInfo(hwnd, SB_VERT, &si, 1);
}

unsafe fn update_title(hwnd: HWND, state: &AppState) {
    let title_str = format!(
        "Tylogger [{}] - Lines: {} | Pos: {} | Tail: {} | Bookmarks: {}\0",
        state.theme_mode.name(),
        state.reader.line_count(),
        state.top_line + 1,
        if state.follow_tail { "ON" } else { "OFF" },
        state.reader.bookmarks().len()
    );
    let wide_title: Vec<u16> = title_str.encode_utf16().collect();
    SetWindowTextW(hwnd, wide_title.as_ptr());
}
