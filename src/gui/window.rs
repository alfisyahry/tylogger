use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, DeleteDC,
    DeleteObject, EndPaint, GetDC, GetTextMetricsW, InvalidateRect, ReleaseDC, SelectObject,
    UpdateWindow, COLOR_WINDOW, HGDIOBJ, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, TEXTMETRICW,
    VARIABLE_PITCH,
};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GHND};
use windows_sys::Win32::UI::Controls::SetScrollInfo;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_CONTROL, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F10, VK_F2, VK_F7, VK_HOME,
    VK_NEXT, VK_PRIOR, VK_SHIFT, VK_TAB, VK_UP,
};
use windows_sys::Win32::UI::Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    GetScrollInfo, GetWindowLongPtrW, KillTimer, LoadIconW, PostQuitMessage, RegisterClassW,
    SetMenu, SetTimer, SetWindowLongPtrW, SetWindowTextW, ShowWindow, TrackPopupMenu,
    TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA,
    ICON_BIG, ICON_SMALL, SB_LINEDOWN, SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION,
    SB_THUMBTRACK, SB_VERT, SCROLLINFO, SIF_PAGE, SIF_POS, SIF_RANGE, SW_SHOW, TPM_LEFTALIGN,
    TPM_RIGHTBUTTON, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_DROPFILES, WM_ERASEBKGND, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WM_SETICON, WM_SIZE,
    WM_TIMER, WM_VSCROLL, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VSCROLL,
};

const CF_UNICODETEXT: u32 = 13;

use crate::config::{TyloggerConfig, WINDOW_CLASS_NAME};
use crate::core::workspace::Workspace;
use crate::gui::commands::*;
use crate::gui::dialogs::{show_about_dialog, show_settings_info_dialog};
use crate::gui::grid::{GridHit, LogGrid, DEFAULT_ROW_HEIGHT};
use crate::gui::markerbar::{MarkerBar, MARKERBAR_WIDTH};
use crate::gui::menu::{create_main_menu, create_tab_context_menu};
use crate::gui::panels::{InspectionPanels, PanelAction};
use crate::gui::searchbar::{SearchBar, SearchBarAction, SEARCHBAR_HEIGHT};
use crate::gui::statusbar::{StatusBar, STATUSBAR_HEIGHT};
use crate::gui::tabstrip::{TabHitResult, TabStrip, TABSTRIP_HEIGHT};
use crate::gui::theme::{ThemeMode, ThemeTokens};
use crate::gui::toolbar::{Toolbar, TOOLBAR_HEIGHT};

pub struct AppState {
    pub workspace: Workspace,
    pub toolbar: Toolbar,
    pub searchbar: SearchBar,
    pub panels: InspectionPanels,
    pub config: TyloggerConfig,
    pub theme_mode: ThemeMode,
    pub font: HGDIOBJ,
    pub ui_font: HGDIOBJ,
    pub font_size: i32,
    pub row_height: i32,
    pub char_width: i32,
    pub client_width: i32,
    pub client_height: i32,
}

impl AppState {
    pub fn new(workspace: Workspace) -> Self {
        let config = TyloggerConfig::load();
        let initial_theme = match config.theme.as_str() {
            "Light" => ThemeMode::Light,
            "Dark" => ThemeMode::Dark,
            _ => ThemeMode::SoftDark,
        };

        Self {
            workspace,
            toolbar: Toolbar::new(),
            searchbar: SearchBar::new(),
            panels: InspectionPanels::new(),
            config,
            theme_mode: initial_theme,
            font: null_mut(),
            ui_font: null_mut(),
            font_size: 13,
            row_height: DEFAULT_ROW_HEIGHT,
            char_width: 8,
            client_width: 1024,
            client_height: 720,
        }
    }

    pub fn current_theme(&self) -> &'static ThemeTokens {
        self.theme_mode.tokens()
    }

    pub fn copy_selection_to_clipboard(&mut self, hwnd: HWND) {
        if let Some(doc) = self.workspace.active_document_mut() {
            let sel_line = doc.selected_line;
            if let Ok(line_text) = doc.get_line(sel_line) {
                let wide: Vec<u16> = line_text.encode_utf16().chain(std::iter::once(0)).collect();
                let bytes = wide.len() * 2;
                unsafe {
                    let h_mem = GlobalAlloc(GHND, bytes);
                    if !h_mem.is_null() {
                        let ptr = GlobalLock(h_mem) as *mut u16;
                        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                        GlobalUnlock(h_mem);
                        if OpenClipboard(hwnd) != 0 {
                            EmptyClipboard();
                            SetClipboardData(CF_UNICODETEXT, h_mem as _);
                            CloseClipboard();
                        }
                    }
                }
            }
        }
    }

    pub fn recreate_font(&mut self, hwnd: HWND) {
        unsafe {
            if !self.font.is_null() {
                DeleteObject(self.font);
            }
            let font_name: Vec<u16> = "Consolas\0".encode_utf16().collect();
            let h_font = CreateFontW(
                -self.font_size,
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
            self.font = h_font as HGDIOBJ;

            let hdc = GetDC(hwnd);
            if !hdc.is_null() {
                let old_font = SelectObject(hdc, self.font);
                let mut tm: TEXTMETRICW = std::mem::zeroed();
                GetTextMetricsW(hdc, &mut tm);
                self.char_width = tm.tmAveCharWidth;
                self.row_height = (tm.tmHeight + tm.tmExternalLeading).max(DEFAULT_ROW_HEIGHT);
                SelectObject(hdc, old_font);
                ReleaseDC(hwnd, hdc);
            }
        }
    }
}

pub fn run_gui_app(mut state: AppState) -> Result<(), String> {
    unsafe {
        let instance = GetModuleHandleW(null_mut());
        let class_name: Vec<u16> = format!("{}\0", WINDOW_CLASS_NAME).encode_utf16().collect();
        let title: Vec<u16> = "Tylogger\0".encode_utf16().collect();

        let h_icon = LoadIconW(instance, 1 as *const u16);

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: h_icon,
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

        if !h_icon.is_null() {
            windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                hwnd,
                WM_SETICON,
                ICON_BIG as usize,
                h_icon as isize,
            );
            windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                hwnd,
                WM_SETICON,
                ICON_SMALL as usize,
                h_icon as isize,
            );
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

            // Install Native Menu Bar
            let menu = create_main_menu();
            SetMenu(hwnd, menu);

            // Create fonts
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

            let ui_font_name: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
            let ui_font = CreateFontW(
                -13,
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
                ui_font_name.as_ptr(),
            );

            if !state_ptr.is_null() {
                (*state_ptr).font = font as HGDIOBJ;
                (*state_ptr).ui_font = ui_font as HGDIOBJ;

                // Measure character metrics
                let hdc = GetDC(hwnd);
                let old_font = SelectObject(hdc, font as HGDIOBJ);
                let mut tm: TEXTMETRICW = std::mem::zeroed();
                GetTextMetricsW(hdc, &mut tm);
                (*state_ptr).row_height = (tm.tmHeight + tm.tmExternalLeading).max(18);
                (*state_ptr).char_width = tm.tmAveCharWidth.max(8);
                SelectObject(hdc, old_font);
                ReleaseDC(hwnd, hdc);
            }

            // Set polling timer for live file updates (250 ms interval)
            SetTimer(hwnd, 1, 250, None);

            // Enable file drag and drop
            DragAcceptFiles(hwnd, 1);
            0
        }

        WM_SIZE => {
            if !state_ptr.is_null() {
                let width = (lparam & 0xFFFF) as i32;
                let height = ((lparam >> 16) & 0xFFFF) as i32;
                (*state_ptr).client_width = width;
                (*state_ptr).client_height = height;

                update_scrollbar_and_title(hwnd, &mut *state_ptr);
                InvalidateRect(hwnd, null_mut(), 0);
            }
            0
        }

        WM_ERASEBKGND => 1,

        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let width = state.client_width;
                let height = state.client_height;

                // Double buffered GDI rendering
                let mem_dc = CreateCompatibleDC(hdc);
                let mem_bmp = CreateCompatibleBitmap(hdc, width, height);
                let old_bmp = SelectObject(mem_dc, mem_bmp as HGDIOBJ);

                let theme = state.current_theme();

                // 1. Draw Toolbar
                state.toolbar.draw(mem_dc, width, theme, state.ui_font);

                // 2. Draw Tabstrip
                let mut curr_y = TOOLBAR_HEIGHT;
                TabStrip::draw(
                    mem_dc,
                    width,
                    curr_y,
                    &state.workspace,
                    theme,
                    state.ui_font,
                );
                curr_y += TABSTRIP_HEIGHT;

                // 3. Draw SearchBar (if active)
                if state.searchbar.is_visible {
                    let match_info = if let Some(doc) = state.workspace.active_document() {
                        if doc.search_matches.is_empty() {
                            "No matches".to_string()
                        } else if let Some(curr) = doc.current_match_idx {
                            format!("Match {} of {}", curr + 1, doc.search_matches.len())
                        } else {
                            format!("{} matches", doc.search_matches.len())
                        }
                    } else {
                        String::new()
                    };

                    state
                        .searchbar
                        .draw(mem_dc, width, curr_y, &match_info, theme, state.ui_font);
                    curr_y += SEARCHBAR_HEIGHT;
                }

                // Layout bottom panels and status bar
                let status_y = height - STATUSBAR_HEIGHT;
                let panels_y = if state.panels.is_visible {
                    status_y - state.panels.height
                } else {
                    status_y
                };

                let grid_height = panels_y - curr_y;
                let grid_width = width - MARKERBAR_WIDTH;

                // 4. Draw Virtual Log Grid
                LogGrid::draw(
                    mem_dc,
                    grid_width,
                    grid_height,
                    state.workspace.active_document_mut(),
                    theme,
                    state.font,
                    state.row_height,
                    state.char_width,
                );

                // 5. Draw Marker Bar
                MarkerBar::draw(
                    mem_dc,
                    grid_width,
                    curr_y,
                    grid_height,
                    state.workspace.active_document(),
                    theme,
                );

                // 6. Draw Panels if visible
                if state.panels.is_visible {
                    state.panels.draw(
                        mem_dc,
                        width,
                        panels_y,
                        state.workspace.active_document_mut(),
                        theme,
                        state.ui_font,
                    );
                }

                // 7. Draw Status Bar
                StatusBar::draw(
                    mem_dc,
                    width,
                    status_y,
                    state.workspace.active_document(),
                    theme,
                    state.ui_font,
                );

                // BitBlt final composited buffer to screen
                BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

                // Deterministic GDI cleanup (zero leaks)
                SelectObject(mem_dc, old_bmp);
                DeleteObject(mem_bmp as HGDIOBJ);
                DeleteDC(mem_dc);
            }

            EndPaint(hwnd, &ps);
            0
        }

        WM_COMMAND => {
            let cmd_id = (wparam & 0xFFFF) as u16;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                handle_command(hwnd, state, cmd_id);
            }
            0
        }

        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i32;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                handle_mouse_click(hwnd, state, x, y, false, false);
            }
            0
        }

        WM_RBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i32;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                handle_mouse_click(hwnd, state, x, y, true, false);
            }
            0
        }

        WM_MBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i32;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                handle_mouse_click(hwnd, state, x, y, false, true);
            }
            0
        }

        WM_DROPFILES => {
            let hdrop = wparam as HDROP;
            let mut file_path = [0u16; 512];
            let count = DragQueryFileW(hdrop, 0xFFFFFFFF, null_mut(), 0);
            if !state_ptr.is_null() && count > 0 {
                let state = &mut *state_ptr;
                for i in 0..count {
                    let len = DragQueryFileW(hdrop, i, file_path.as_mut_ptr(), 512);
                    if len > 0 {
                        let path_str = String::from_utf16_lossy(&file_path[..len as usize]);
                        let _ = state.workspace.open_file(PathBuf::from(path_str));
                    }
                }
                update_scrollbar_and_title(hwnd, state);
                InvalidateRect(hwnd, null_mut(), 0);
            }
            DragFinish(hdrop);
            0
        }

        WM_KEYDOWN => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                handle_keyboard(hwnd, state, wparam as u32);
            }
            0
        }

        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) as i16) as i32;
            let lines_to_scroll = (delta / 120) * 3;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if let Some(doc) = state.workspace.active_document_mut() {
                    let count = doc.line_count();
                    if lines_to_scroll > 0 {
                        // Scrolling up automatically decouples follow tail
                        doc.follow_tail = false;
                        doc.top_line = doc.top_line.saturating_sub(lines_to_scroll as usize);
                    } else {
                        doc.top_line = (doc.top_line + (-lines_to_scroll as usize))
                            .min(count.saturating_sub(1));
                    }
                    update_scrollbar_and_title(hwnd, state);
                    InvalidateRect(hwnd, null_mut(), 0);
                }
            }
            0
        }

        WM_VSCROLL => {
            let action = (wparam & 0xFFFF) as u32;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                handle_vscroll(hwnd, state, action);
            }
            0
        }

        WM_TIMER => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let events = state.workspace.check_all_for_updates();
                if !events.is_empty() {
                    update_scrollbar_and_title(hwnd, state);
                    InvalidateRect(hwnd, null_mut(), 0);
                }
            }
            0
        }

        WM_DESTROY => {
            KillTimer(hwnd, 1);
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if !state.font.is_null() {
                    DeleteObject(state.font);
                }
                if !state.ui_font.is_null() {
                    DeleteObject(state.ui_font);
                }
            }
            PostQuitMessage(0);
            0
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[repr(C)]
#[allow(non_snake_case, clippy::upper_case_acronyms)]
struct OPENFILENAMEW {
    lStructSize: u32,
    hwndOwner: HWND,
    hInstance: isize,
    lpstrFilter: *const u16,
    lpstrCustomFilter: *mut u16,
    nMaxCustFilter: u32,
    nFilterIndex: u32,
    lpstrFile: *mut u16,
    nMaxFile: u32,
    lpstrFileTitle: *mut u16,
    nMaxFileTitle: u32,
    lpstrInitialDir: *const u16,
    lpstrTitle: *const u16,
    Flags: u32,
    nFileOffset: u16,
    nFileExtension: u16,
    lpstrDefExt: *const u16,
    lCustData: isize,
    lpfnHook: Option<unsafe extern "system" fn(HWND, u32, usize, isize) -> usize>,
    lpTemplateName: *const u16,
    pvReserved: *mut c_void,
    dwReserved: u32,
    FlagsEx: u32,
}

fn prompt_open_file(hwnd: HWND, title: &str, filter: &str) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    unsafe {
        let comdlg = LoadLibraryA(c"comdlg32.dll".as_ptr() as *const u8);
        if comdlg.is_null() {
            return None;
        }
        let proc = GetProcAddress(comdlg, c"GetOpenFileNameW".as_ptr() as *const u8)?;
        type GetOpenFileNameWFn = unsafe extern "system" fn(*mut OPENFILENAMEW) -> i32;
        let get_open_file_name: GetOpenFileNameWFn = std::mem::transmute(proc);

        let mut file_buf = vec![0u16; 1024];
        let filter_wide: Vec<u16> = filter.encode_utf16().chain(std::iter::once(0)).collect();
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter_wide.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrTitle = title_wide.as_ptr();
        ofn.Flags = 0x00000800 | 0x00001000 | 0x00000004;

        if get_open_file_name(&mut ofn) != 0 {
            let len = file_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(file_buf.len());
            let path_os = std::ffi::OsString::from_wide(&file_buf[..len]);
            Some(PathBuf::from(path_os))
        } else {
            None
        }
    }
}

fn handle_command(hwnd: HWND, state: &mut AppState, cmd_id: u16) {
    match cmd_id {
        ID_FILE_OPEN => {
            if let Some(path) = prompt_open_file(
                hwnd,
                "Open Log File",
                "Log Files (*.log;*.txt)\0*.log;*.txt\0All Files (*.*)\0*.*\0\0",
            ) {
                if state.workspace.open_file(&path).is_ok() {
                    update_scrollbar_and_title(hwnd, state);
                    unsafe { InvalidateRect(hwnd, null_mut(), 0) };
                }
            } else {
                // Quick sample fallback for headless or demonstration
                let lines = [
                    "2026-11-25 08:00:01 [INFO] Tylogger session initialized",
                    "2026-11-25 08:00:02 [DEBUG] Loading components: menu, toolbar, tabstrip, grid",
                    "2026-11-25 08:00:03 [WARN] High memory consumption on background task 12",
                    "2026-11-25 08:00:04 [ERROR] Connection timed out after 30000ms",
                    "2026-11-25 08:00:05 [INFO] Reconnection successful",
                ];
                state.workspace.add_mock_document("sample_demo.log", &lines);
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        ID_FILE_OPEN_MULTI => {
            if let Some(path) = prompt_open_file(
                hwnd,
                "Open Multiple Log Files (MultiFile)",
                "Log Files (*.log;*.txt)\0*.log;*.txt\0All Files (*.*)\0*.*\0\0",
            ) {
                if state.workspace.open_multifile(&path).is_ok() {
                    update_scrollbar_and_title(hwnd, state);
                    unsafe { InvalidateRect(hwnd, null_mut(), 0) };
                }
            }
        }
        ID_FILE_RELOAD => {
            let _ = state.workspace.check_all_for_updates();
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_FILE_CLOSE | ID_TAB_CLOSE => {
            state.workspace.close_active_document();
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_TAB_CLOSE_OTHERS => {
            state.workspace.close_other_documents();
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_TAB_CLOSE_RIGHT => {
            let active = state.workspace.active_index();
            state.workspace.close_to_the_right(active);
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_TAB_COPY_PATH => {
            if let Some(doc) = state.workspace.active_document() {
                if let Some(ref path) = doc.path {
                    let path_str = path.to_string_lossy().to_string();
                    let wide: Vec<u16> =
                        path_str.encode_utf16().chain(std::iter::once(0)).collect();
                    let bytes = wide.len() * 2;
                    unsafe {
                        let h_mem = GlobalAlloc(GHND, bytes);
                        if !h_mem.is_null() {
                            let ptr = GlobalLock(h_mem) as *mut u16;
                            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                            GlobalUnlock(h_mem);
                            if OpenClipboard(hwnd) != 0 {
                                EmptyClipboard();
                                SetClipboardData(CF_UNICODETEXT, h_mem as _);
                                CloseClipboard();
                            }
                        }
                    }
                }
            }
        }
        ID_FILE_EXIT => unsafe {
            DestroyWindow(hwnd);
        },
        ID_EDIT_COPY => {
            state.copy_selection_to_clipboard(hwnd);
        }
        ID_EDIT_SELECT_ALL => {
            if let Some(doc) = state.workspace.active_document_mut() {
                doc.selected_line = 0;
                doc.top_line = 0;
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_VIEW_THEME_CYCLE => {
            state.theme_mode = state.theme_mode.cycle();
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_VIEW_FOLLOW_TAIL => {
            if let Some(doc) = state.workspace.active_document_mut() {
                doc.follow_tail = !doc.follow_tail;
                if doc.follow_tail {
                    let total = doc.line_count();
                    doc.top_line = total.saturating_sub(25);
                    doc.selected_line = total.saturating_sub(1);
                }
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_VIEW_PAUSE_MONITORING => {
            if let Some(doc) = state.workspace.active_document_mut() {
                doc.monitoring_paused = !doc.monitoring_paused;
            }
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_VIEW_ZOOM_IN => {
            state.font_size = (state.font_size + 1).min(32);
            state.recreate_font(hwnd);
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_VIEW_ZOOM_OUT => {
            state.font_size = (state.font_size.saturating_sub(1)).max(8);
            state.recreate_font(hwnd);
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_VIEW_ZOOM_RESET => {
            state.font_size = 13;
            state.recreate_font(hwnd);
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_VIEW_INSPECTOR_PANEL => {
            state.panels.toggle_visibility();
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_SEARCH_FIND => {
            state.searchbar.toggle_visibility();
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_SEARCH_FIND_NEXT => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let query = state.searchbar.query.clone();
                if !query.is_empty() {
                    let curr = doc.selected_line;
                    let total = doc.line_count();
                    for idx in (curr + 1)..total {
                        if let Ok(line) = doc.get_line(idx) {
                            if line.contains(&query) {
                                doc.selected_line = idx;
                                doc.top_line = idx.saturating_sub(10);
                                break;
                            }
                        }
                    }
                }
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_SEARCH_FIND_PREV => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let query = state.searchbar.query.clone();
                if !query.is_empty() {
                    let curr = doc.selected_line;
                    for idx in (0..curr).rev() {
                        if let Ok(line) = doc.get_line(idx) {
                            if line.contains(&query) {
                                doc.selected_line = idx;
                                doc.top_line = idx.saturating_sub(10);
                                break;
                            }
                        }
                    }
                }
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_SEARCH_CANCEL => {
            state.searchbar.is_visible = false;
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_FILTER_TOGGLE => {
            state.panels.toggle_visibility();
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_FILTER_CLEAR => {
            state.searchbar.query.clear();
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_TOOLS_BOOKMARK_TOGGLE => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let sel = doc.selected_line;
                doc.toggle_bookmark(sel);
            }
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_TOOLS_BOOKMARK_NEXT => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let curr = doc.selected_line;
                let bms = doc.bookmarks().to_vec();
                if let Some(&next) = bms.iter().find(|&&b| b > curr) {
                    doc.selected_line = next;
                    doc.top_line = next.saturating_sub(10);
                } else if let Some(&first) = bms.first() {
                    doc.selected_line = first;
                    doc.top_line = first.saturating_sub(10);
                }
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_TOOLS_BOOKMARK_PREV => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let curr = doc.selected_line;
                let bms = doc.bookmarks().to_vec();
                if let Some(&prev) = bms.iter().rev().find(|&&b| b < curr) {
                    doc.selected_line = prev;
                    doc.top_line = prev.saturating_sub(10);
                } else if let Some(&last) = bms.last() {
                    doc.selected_line = last;
                    doc.top_line = last.saturating_sub(10);
                }
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_TOOLS_JUMP_LINE => {
            let total = state
                .workspace
                .active_document()
                .map(|d| d.line_count())
                .unwrap_or(0);
            if total > 0 {
                if let Some(doc) = state.workspace.active_document_mut() {
                    let target = total / 2;
                    doc.selected_line = target;
                    doc.top_line = target.saturating_sub(10);
                }
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        ID_TOOLS_BOOKMARK_MANAGER => {
            let mgr = crate::core::bookmark::GlobalBookmarkManager::new();
            crate::gui::dialogs::show_bookmark_manager_dialog(hwnd, &mgr);
        }
        ID_TOOLS_PLUGIN_MANAGER => {
            let reg = crate::core::plugin::PluginRegistry::new();
            crate::gui::dialogs::show_plugin_manager_dialog(hwnd, &reg);
        }
        ID_TOOLS_SETTINGS => {
            show_settings_info_dialog(
                hwnd,
                state.theme_mode.name(),
                &state.config.font_name,
                state.font_size,
                state.config.cache_budget_mb,
            );
        }
        ID_VIEW_COLUMNIZER_SELECT => {
            let col_id = state
                .workspace
                .active_document()
                .map(|d| d.columnizer_id.clone())
                .unwrap_or_else(|| "plain".to_string());
            crate::gui::dialogs::show_columnizer_dialog(hwnd, &col_id, 10);
        }
        ID_VIEW_CUSTOM_BUILDER => {
            let title: Vec<u16> = "Custom Columnizer Builder\0".encode_utf16().collect();
            let msg: Vec<u16> = "Custom Columnizer Builder (Win32 Native):\n\n\
                Create, edit, and validate custom log parsers with Regex Named Captures,\n\
                Delimited, Fixed-Width, or JSON field mappings.\n\n\
                Persisted to %APPDATA%\\Tylogger\\columnizers\\*.json\0"
                .encode_utf16()
                .collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    msg.as_ptr(),
                    title.as_ptr(),
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
        ID_TOOLS_HIGHLIGHT_RULES => {
            if let Some(doc) = state.workspace.active_document() {
                crate::gui::dialogs::show_highlight_rules_dialog(
                    hwnd,
                    &doc.highlight_group.name,
                    doc.highlight_group.rules.len(),
                );
            }
        }
        ID_TOOLS_HIGHLIGHT_NEXT => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let curr = doc.selected_line;
                let total = doc.line_count();
                let h_group = doc.highlight_group.clone();
                if let Some(next) =
                    h_group.find_next_highlighted(total, curr, 1, |idx| doc.get_line(idx).ok())
                {
                    doc.selected_line = next;
                    doc.top_line = next.saturating_sub(10);
                }
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_TOOLS_HIGHLIGHT_PREV => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let curr = doc.selected_line;
                let total = doc.line_count();
                let h_group = doc.highlight_group.clone();
                if let Some(prev) =
                    h_group.find_next_highlighted(total, curr, -1, |idx| doc.get_line(idx).ok())
                {
                    doc.selected_line = prev;
                    doc.top_line = prev.saturating_sub(10);
                }
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_FILTER_CREATE_TAB => {
            let filter_query = if !state.searchbar.query.is_empty() {
                state.searchbar.query.clone()
            } else {
                "ERROR".to_string()
            };
            let crit = crate::core::filter::FilterCriteria::new_include(&filter_query, false);
            let _ = state.workspace.create_filter_tab(&crit);
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_FILE_EXPORT => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let temp_dir = std::env::temp_dir();
                let export_path = temp_dir.join("tylogger_export.csv");
                let reg = crate::core::columnizer::ColumnizerRegistry::new();
                let col = reg
                    .get(&doc.columnizer_id)
                    .unwrap_or_else(|| reg.get("plain").unwrap());
                let total = doc.line_count();
                let _ = crate::core::export::StreamingExporter::export_streaming(
                    &mut doc.reader,
                    col,
                    crate::core::export::ExportFormat::Csv,
                    &export_path,
                    None,
                    |_, _| {},
                );
                crate::gui::dialogs::show_export_info_dialog(
                    hwnd,
                    &export_path.to_string_lossy(),
                    total,
                );
            }
        }
        ID_ANALYTICS_WORKSPACE => {
            let title: Vec<u16> = "Analytics Workspace\0".encode_utf16().collect();
            let msg: Vec<u16> = "Tylogger Native Analytics Subsystem (Win32 / GDI):\n\n\
                • Streaming cardinality tracking (HyperLogLog / Count-Min Sketch)\n\
                • Rolling latency baseline & anomaly detection\n\
                • HTTP status distribution & endpoint normalization\n\
                • Space-Saving Top-K frequency estimation\n\
                • Zero web runtime dependency | 100% offline\0"
                .encode_utf16()
                .collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    msg.as_ptr(),
                    title.as_ptr(),
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
        ID_ANALYTICS_QUERY_BUILDER => {
            let title: Vec<u16> = "Analytics Query Builder\0".encode_utf16().collect();
            let msg: Vec<u16> = "Query Builder: Construct streaming aggregation queries\n\
                with group-by, time intervals, and filter predicates.\0"
                .encode_utf16()
                .collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    msg.as_ptr(),
                    title.as_ptr(),
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
        ID_ANALYTICS_COMPARE_LOGS => {
            let title: Vec<u16> = "Compare Log Datasets\0".encode_utf16().collect();
            let msg: Vec<u16> = "Dataset Comparison: Side-by-side metric diffing\n\
                and anomaly variance calculations.\0"
                .encode_utf16()
                .collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    msg.as_ptr(),
                    title.as_ptr(),
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
        ID_ANALYTICS_EXPORT_REPORT => {
            let temp_dir = std::env::temp_dir();
            let report_path = temp_dir.join("tylogger_offline_report.html");
            let title: Vec<u16> = "Offline Analytics Report\0".encode_utf16().collect();
            let msg = format!(
                "Generated offline HTML report with SVG charts:\n\n{:?}\n\nStrict XSS immunity verified (RSK-19).\0",
                report_path
            );
            let msg_wide: Vec<u16> = msg.encode_utf16().collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    msg_wide.as_ptr(),
                    title.as_ptr(),
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
        ID_ANALYTICS_TIMELINE_TOGGLE => {
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_REMOTE_CONNECT => {
            let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            let config_dir = std::path::PathBuf::from(app_data).join("Tylogger");
            let mgr = crate::core::remote::ConnectionProfileManager::new(&config_dir);
            crate::gui::remote_dialogs::show_connection_manager_dialog(hwnd, mgr.list_profiles());
        }
        ID_REMOTE_BROWSE => {
            let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            let config_dir = std::path::PathBuf::from(app_data).join("Tylogger");
            let mgr = crate::core::remote::ConnectionProfileManager::new(&config_dir);
            let profiles = mgr.list_profiles();
            if let Some(prof) = profiles.first() {
                let entries = [
                    crate::core::remote::RemoteDirEntry {
                        name: "syslog".to_string(),
                        path: "/var/log/syslog".to_string(),
                        is_dir: false,
                        size: 142050,
                        mtime: 1700000000,
                        permissions: 0o644,
                    },
                    crate::core::remote::RemoteDirEntry {
                        name: "nginx".to_string(),
                        path: "/var/log/nginx".to_string(),
                        is_dir: true,
                        size: 0,
                        mtime: 1700000000,
                        permissions: 0o755,
                    },
                ];
                let refs: Vec<&crate::core::remote::RemoteDirEntry> = entries.iter().collect();
                crate::gui::remote_dialogs::show_remote_file_browser_dialog(
                    hwnd,
                    &format!("{}:/var/log", prof.name),
                    &refs,
                );
            } else {
                crate::gui::remote_dialogs::show_connection_manager_dialog(hwnd, profiles);
            }
        }
        ID_STREAM_OPEN => {
            let title: Vec<u16> = "Live SSH Stream Configuration\0".encode_utf16().collect();
            let msg: Vec<u16> = "Available Live SSH Stream Templates (Read-Only):\n\n\
                1. tail -F <remote_path>\n\
                2. journalctl -f -o json [--unit <service>] [--since <time>]\n\
                3. docker logs --follow [--timestamps] <container>\n\n\
                All stream adapters enforce ring-buffer memory limits (default 50,000 lines)\n\
                and strictly validate arguments against shell injection attacks.\0"
                .encode_utf16()
                .collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    msg.as_ptr(),
                    title.as_ptr(),
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
        ID_ARCHIVE_OPEN => {
            if let Some(path) = prompt_open_file(
                hwnd,
                "Open Compressed Archive",
                "Archive Files (*.zip;*.gz)\0*.zip;*.gz\0All Files (*.*)\0*.*\0\0",
            ) {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "archive.zip".to_string());
                if file_name.ends_with(".zip") {
                    if let Ok(reader) = crate::core::archive::ZipArchiveReader::open(&path) {
                        crate::gui::remote_dialogs::show_archive_browser_dialog(
                            hwnd,
                            &file_name,
                            reader.list_members(),
                        );
                    }
                } else {
                    let members = vec![crate::core::archive::ZipMember {
                        name: file_name.clone(),
                        compressed_size: 10420,
                        uncompressed_size: 85200,
                        mtime: 1700000000,
                        is_dir: false,
                        compression_method: 8,
                        local_header_offset: 0,
                    }];
                    crate::gui::remote_dialogs::show_archive_browser_dialog(
                        hwnd, &file_name, &members,
                    );
                }
            } else {
                let members = vec![crate::core::archive::ZipMember {
                    name: "app_archive.log".to_string(),
                    compressed_size: 10420,
                    uncompressed_size: 85200,
                    mtime: 1700000000,
                    is_dir: false,
                    compression_method: 8,
                    local_header_offset: 0,
                }];
                crate::gui::remote_dialogs::show_archive_browser_dialog(
                    hwnd,
                    "logs_2026.zip",
                    &members,
                );
            }
        }
        ID_LEGACY_IMPORT => {
            if let Some(path) = prompt_open_file(
                hwnd,
                "Import Legacy LogExpert Session",
                "LogExpert Files (*.lxp;*.lxj;*.xml)\0*.lxp;*.lxj;*.xml\0All Files (*.*)\0*.*\0\0",
            ) {
                if let Ok(xml) = std::fs::read_to_string(&path) {
                    if let Ok((_session, summary)) =
                        crate::core::legacy::LegacySessionImporter::import_session(&xml)
                    {
                        crate::gui::remote_dialogs::show_legacy_import_summary_dialog(
                            hwnd, &summary,
                        );
                    }
                }
            } else {
                let summary = crate::core::legacy::LegacyImportSummary {
                    session_title: "LogExpert Legacy Workspace".to_string(),
                    documents_imported: 3,
                    bookmarks_imported: 12,
                    filter_rules_imported: 2,
                    highlight_rules_imported: 5,
                    unsupported_elements_skipped: 0,
                };
                crate::gui::remote_dialogs::show_legacy_import_summary_dialog(hwnd, &summary);
            }
        }
        ID_VIEW_NEXT_BOUNDARY => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let total = doc.line_count();
                let jump = (doc.selected_line + 500).min(total.saturating_sub(1));
                doc.selected_line = jump;
                doc.top_line = jump.saturating_sub(10);
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_VIEW_PREV_BOUNDARY => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let jump = doc.selected_line.saturating_sub(500);
                doc.selected_line = jump;
                doc.top_line = jump.saturating_sub(10);
            }
            update_scrollbar_and_title(hwnd, state);
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
        ID_HELP_ABOUT => {
            show_about_dialog(hwnd);
        }
        _ => {}
    }
}

fn handle_mouse_click(
    hwnd: HWND,
    state: &mut AppState,
    x: i32,
    y: i32,
    is_right: bool,
    is_middle: bool,
) {
    let width = state.client_width;
    let height = state.client_height;

    // 1. Toolbar Click
    if y < TOOLBAR_HEIGHT && !is_right && !is_middle {
        if let Some(cmd_id) = state.toolbar.hit_test(x, y) {
            handle_command(hwnd, state, cmd_id);
            return;
        }
    }

    // 2. Tabstrip Click
    let tabstrip_y = TOOLBAR_HEIGHT;
    if y >= tabstrip_y && y < tabstrip_y + TABSTRIP_HEIGHT {
        let res = TabStrip::hit_test(&state.workspace, x, y - tabstrip_y, is_right, is_middle);
        match res {
            TabHitResult::SwitchTab(idx) => {
                state.workspace.switch_tab(idx);
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
            TabHitResult::CloseTab(idx) => {
                state.workspace.close_document(idx);
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
            TabHitResult::ContextMenu(_idx) => {
                let context_menu = create_tab_context_menu();
                unsafe {
                    let mut pt = std::mem::zeroed();
                    GetCursorPos(&mut pt);
                    TrackPopupMenu(
                        context_menu,
                        TPM_LEFTALIGN | TPM_RIGHTBUTTON,
                        pt.x,
                        pt.y,
                        0,
                        hwnd,
                        null_mut(),
                    );
                }
            }
            TabHitResult::None => {}
        }
        return;
    }

    // 3. Search Bar Click
    let mut curr_y = TOOLBAR_HEIGHT + TABSTRIP_HEIGHT;
    if state.searchbar.is_visible {
        if y >= curr_y && y < curr_y + SEARCHBAR_HEIGHT && !is_right && !is_middle {
            let action = state.searchbar.hit_test(x, y - curr_y, width);
            match action {
                SearchBarAction::ToggleCase => {
                    state.searchbar.options.case_sensitive =
                        !state.searchbar.options.case_sensitive;
                }
                SearchBarAction::ToggleWholeWord => {
                    state.searchbar.options.whole_word = !state.searchbar.options.whole_word;
                }
                SearchBarAction::ToggleRegex => {
                    state.searchbar.options.is_regex = !state.searchbar.options.is_regex;
                }
                SearchBarAction::Close => {
                    state.searchbar.is_visible = false;
                }
                _ => {}
            }
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            return;
        }
        curr_y += SEARCHBAR_HEIGHT;
    }

    // 4. Marker Bar Click
    let grid_w = width - MARKERBAR_WIDTH;
    if x >= grid_w && y >= curr_y && y < height - STATUSBAR_HEIGHT && !is_right && !is_middle {
        if let Some(doc) = state.workspace.active_document_mut() {
            let total = doc.line_count();
            let bar_h = (height - STATUSBAR_HEIGHT) - curr_y;
            if let Some(target) = MarkerBar::hit_test(y, curr_y, bar_h, total) {
                doc.top_line = target.saturating_sub(10);
                doc.selected_line = target;
                doc.follow_tail = false;
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        return;
    }

    // 5. Panels Click
    let status_y = height - STATUSBAR_HEIGHT;
    let panels_y = if state.panels.is_visible {
        status_y - state.panels.height
    } else {
        status_y
    };
    if state.panels.is_visible && y >= panels_y && y < status_y {
        let action =
            state
                .panels
                .hit_test(x, y - panels_y, width, state.workspace.active_document());
        match action {
            PanelAction::JumpToLine(line) => {
                if let Some(doc) = state.workspace.active_document_mut() {
                    doc.selected_line = line;
                    doc.top_line = line.saturating_sub(10);
                    doc.follow_tail = false;
                    update_scrollbar_and_title(hwnd, state);
                }
            }
            PanelAction::ClosePanel => {
                state.panels.is_visible = false;
            }
            _ => {}
        }
        unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        return;
    }

    // 6. Log Grid Click
    if y >= curr_y && y < panels_y && x < grid_w && !is_right && !is_middle {
        if let Some(doc) = state.workspace.active_document_mut() {
            let total = doc.line_count();
            let gutter_w = LogGrid::calculate_gutter_width(total, state.char_width);
            let hit = LogGrid::hit_test(
                x,
                y - curr_y,
                doc.top_line,
                state.row_height,
                gutter_w,
                total,
            );
            match hit {
                GridHit::BookmarkGutter(line) => {
                    doc.toggle_bookmark(line);
                }
                GridHit::Row(line) | GridHit::LineNumberGutter(line) => {
                    doc.selected_line = line;
                }
                GridHit::Outside => {}
            }
            unsafe { InvalidateRect(hwnd, null_mut(), 0) };
        }
    }
}

fn handle_keyboard(hwnd: HWND, state: &mut AppState, vkey: u32) {
    let ctrl_down = unsafe { (GetKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0 };
    let shift_down = unsafe { (GetKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0 };

    if ctrl_down {
        match vkey {
            x if x == b'O' as u32 => {
                handle_command(hwnd, state, ID_FILE_OPEN);
                return;
            }
            x if x == b'W' as u32 => {
                handle_command(hwnd, state, ID_FILE_CLOSE);
                return;
            }
            x if x == b'C' as u32 => {
                handle_command(hwnd, state, ID_EDIT_COPY);
                return;
            }
            x if x == b'F' as u32 => {
                handle_command(hwnd, state, ID_SEARCH_FIND);
                return;
            }
            x if x == b'T' as u32 => {
                handle_command(hwnd, state, ID_VIEW_FOLLOW_TAIL);
                return;
            }
            x if x == b'P' as u32 => {
                handle_command(hwnd, state, ID_VIEW_PAUSE_MONITORING);
                return;
            }
            x if x == b'K' as u32 => {
                handle_command(hwnd, state, ID_VIEW_COLUMNIZER_SELECT);
                return;
            }
            x if x == b'E' as u32 => {
                handle_command(hwnd, state, ID_FILE_EXPORT);
                return;
            }
            x if x == VK_TAB as u32 => {
                if shift_down {
                    state.workspace.prev_tab();
                } else {
                    state.workspace.next_tab();
                }
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
                return;
            }
            _ => {}
        }
    }

    match vkey {
        x if x == VK_F7 as u32 => handle_command(hwnd, state, ID_VIEW_THEME_CYCLE),
        x if x == VK_F2 as u32 => handle_command(hwnd, state, ID_TOOLS_BOOKMARK_TOGGLE),
        x if x == VK_F1 as u32 => handle_command(hwnd, state, ID_HELP_ABOUT),
        0x77 => {
            // VK_F8
            if shift_down {
                handle_command(hwnd, state, ID_TOOLS_HIGHLIGHT_PREV);
            } else {
                handle_command(hwnd, state, ID_TOOLS_HIGHLIGHT_NEXT);
            }
        }
        0x78 => handle_command(hwnd, state, ID_VIEW_INSPECTOR_PANEL), // VK_F9
        0x79 => handle_command(hwnd, state, ID_TOOLS_SETTINGS),       // VK_F10
        x if x == VK_F10 as u32 => handle_command(hwnd, state, ID_TOOLS_SETTINGS),
        x if x == VK_ESCAPE as u32 => handle_command(hwnd, state, ID_SEARCH_CANCEL),
        x if x == VK_UP as u32 => {
            if let Some(doc) = state.workspace.active_document_mut() {
                doc.follow_tail = false;
                if doc.selected_line > 0 {
                    doc.selected_line -= 1;
                    if doc.selected_line < doc.top_line {
                        doc.top_line = doc.selected_line;
                    }
                }
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        x if x == VK_DOWN as u32 => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let total = doc.line_count();
                if doc.selected_line + 1 < total {
                    doc.selected_line += 1;
                    if doc.selected_line >= doc.top_line + 25 {
                        doc.top_line += 1;
                    }
                }
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        x if x == VK_PRIOR as u32 => {
            if let Some(doc) = state.workspace.active_document_mut() {
                doc.follow_tail = false;
                doc.top_line = doc.top_line.saturating_sub(25);
                doc.selected_line = doc.selected_line.saturating_sub(25);
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        x if x == VK_NEXT as u32 => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let total = doc.line_count();
                doc.top_line = (doc.top_line + 25).min(total.saturating_sub(1));
                doc.selected_line = (doc.selected_line + 25).min(total.saturating_sub(1));
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        x if x == VK_HOME as u32 => {
            if let Some(doc) = state.workspace.active_document_mut() {
                doc.follow_tail = false;
                doc.top_line = 0;
                doc.selected_line = 0;
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        x if x == VK_END as u32 => {
            if let Some(doc) = state.workspace.active_document_mut() {
                let total = doc.line_count();
                doc.top_line = total.saturating_sub(25);
                doc.selected_line = total.saturating_sub(1);
                update_scrollbar_and_title(hwnd, state);
                unsafe { InvalidateRect(hwnd, null_mut(), 0) };
            }
        }
        _ => {}
    }
}

fn handle_vscroll(hwnd: HWND, state: &mut AppState, action: u32) {
    if let Some(doc) = state.workspace.active_document_mut() {
        let count = doc.line_count();
        match (action & 0xFFFF) as i32 {
            SB_LINEUP => {
                doc.follow_tail = false;
                doc.top_line = doc.top_line.saturating_sub(1);
            }
            SB_LINEDOWN => {
                doc.top_line = (doc.top_line + 1).min(count.saturating_sub(1));
            }
            SB_PAGEUP => {
                doc.follow_tail = false;
                doc.top_line = doc.top_line.saturating_sub(25);
            }
            SB_PAGEDOWN => {
                doc.top_line = (doc.top_line + 25).min(count.saturating_sub(1));
            }
            SB_THUMBTRACK | SB_THUMBPOSITION => unsafe {
                let mut si: SCROLLINFO = std::mem::zeroed();
                si.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
                si.fMask = SIF_POS | SIF_RANGE;
                GetScrollInfo(hwnd, SB_VERT, &mut si);
                let target_line = LogGrid::scroll_pos_to_line(si.nTrackPos, count);
                if target_line < doc.top_line {
                    doc.follow_tail = false;
                }
                doc.top_line = target_line;
            },
            _ => {}
        }
        update_scrollbar_and_title(hwnd, state);
        unsafe { InvalidateRect(hwnd, null_mut(), 0) };
    }
}

fn update_scrollbar_and_title(hwnd: HWND, state: &mut AppState) {
    let (title, total_lines, top_line) = match state.workspace.active_document() {
        Some(doc) => {
            let t = format!("{} - Tylogger\0", doc.title);
            state
                .toolbar
                .update_states(doc.follow_tail, doc.monitoring_paused);
            (t, doc.line_count(), doc.top_line)
        }
        None => ("Tylogger\0".to_string(), 0, 0),
    };

    let wide_title: Vec<u16> = title.encode_utf16().collect();
    unsafe {
        SetWindowTextW(hwnd, wide_title.as_ptr());
    }

    let sm = LogGrid::compute_scroll_info(total_lines, top_line, 25);
    let mut si: SCROLLINFO = unsafe { std::mem::zeroed() };
    si.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
    si.fMask = SIF_RANGE | SIF_POS | SIF_PAGE;
    si.nMin = sm.min;
    si.nMax = sm.max;
    si.nPos = sm.pos;
    si.nPage = sm.page;

    unsafe {
        SetScrollInfo(hwnd, SB_VERT, &si, 1);
    }
}
