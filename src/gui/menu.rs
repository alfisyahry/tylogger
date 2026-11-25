use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateMenu, CreatePopupMenu, HMENU, MF_POPUP, MF_SEPARATOR, MF_STRING,
};

use crate::gui::commands::*;

/// Encodes a string as a null-terminated UTF-16 wide string for Win32 API.
fn to_wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Builds the main Win32 Menu Bar for Tylogger.
pub fn create_main_menu() -> HMENU {
    unsafe {
        let menu_bar = CreateMenu();

        // 1. File Menu
        let file_menu = CreatePopupMenu();
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_FILE_OPEN as usize,
            to_wide("&Open File...\tCtrl+O").as_ptr(),
        );
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_FILE_OPEN_MULTI as usize,
            to_wide("Open &Multiple Files...\tCtrl+Shift+O").as_ptr(),
        );
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_FILE_RELOAD as usize,
            to_wide("&Reload\tF5").as_ptr(),
        );
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_FILE_EXPORT as usize,
            to_wide("&Export...\tCtrl+E").as_ptr(),
        );
        AppendMenuW(file_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_FILE_CLOSE as usize,
            to_wide("&Close Document\tCtrl+W").as_ptr(),
        );
        AppendMenuW(file_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_FILE_EXIT as usize,
            to_wide("E&xit\tAlt+F4").as_ptr(),
        );
        AppendMenuW(
            menu_bar,
            MF_POPUP,
            file_menu as usize,
            to_wide("&File").as_ptr(),
        );

        // 2. Edit Menu
        let edit_menu = CreatePopupMenu();
        AppendMenuW(
            edit_menu,
            MF_STRING,
            ID_EDIT_COPY as usize,
            to_wide("&Copy\tCtrl+C").as_ptr(),
        );
        AppendMenuW(
            edit_menu,
            MF_STRING,
            ID_EDIT_SELECT_ALL as usize,
            to_wide("Select &All\tCtrl+A").as_ptr(),
        );
        AppendMenuW(
            menu_bar,
            MF_POPUP,
            edit_menu as usize,
            to_wide("&Edit").as_ptr(),
        );

        // 3. View Menu
        let view_menu = CreatePopupMenu();
        AppendMenuW(
            view_menu,
            MF_STRING,
            ID_VIEW_THEME_CYCLE as usize,
            to_wide("Cycle &Theme\tF7").as_ptr(),
        );
        AppendMenuW(view_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            view_menu,
            MF_STRING,
            ID_VIEW_FOLLOW_TAIL as usize,
            to_wide("&Follow Tail\tCtrl+T").as_ptr(),
        );
        AppendMenuW(
            view_menu,
            MF_STRING,
            ID_VIEW_PAUSE_MONITORING as usize,
            to_wide("&Pause Monitoring\tCtrl+Space").as_ptr(),
        );
        AppendMenuW(view_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            view_menu,
            MF_STRING,
            ID_VIEW_COLUMNIZER_SELECT as usize,
            to_wide("Choose &Columnizer...\tCtrl+K").as_ptr(),
        );
        AppendMenuW(
            view_menu,
            MF_STRING,
            ID_VIEW_CUSTOM_BUILDER as usize,
            to_wide("Custom Columnizer &Builder...\tCtrl+Shift+K").as_ptr(),
        );
        AppendMenuW(
            view_menu,
            MF_STRING,
            ID_VIEW_INSPECTOR_PANEL as usize,
            to_wide("Toggle Structured Log &Inspector\tF9").as_ptr(),
        );
        AppendMenuW(view_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            view_menu,
            MF_STRING,
            ID_VIEW_NEXT_BOUNDARY as usize,
            to_wide("Jump to Next File &Boundary\tAlt+Down").as_ptr(),
        );
        AppendMenuW(
            view_menu,
            MF_STRING,
            ID_VIEW_PREV_BOUNDARY as usize,
            to_wide("Jump to Prev File Boundary\tAlt+Up").as_ptr(),
        );
        AppendMenuW(
            menu_bar,
            MF_POPUP,
            view_menu as usize,
            to_wide("&View").as_ptr(),
        );

        // 4. Search & Filter Menu
        let search_menu = CreatePopupMenu();
        AppendMenuW(
            search_menu,
            MF_STRING,
            ID_SEARCH_FIND as usize,
            to_wide("&Find in Current Document...\tCtrl+F").as_ptr(),
        );
        AppendMenuW(
            search_menu,
            MF_STRING,
            ID_SEARCH_FIND_NEXT as usize,
            to_wide("Find &Next\tF3").as_ptr(),
        );
        AppendMenuW(
            search_menu,
            MF_STRING,
            ID_SEARCH_FIND_PREV as usize,
            to_wide("Find &Previous\tShift+F3").as_ptr(),
        );
        AppendMenuW(
            search_menu,
            MF_STRING,
            ID_SEARCH_CANCEL as usize,
            to_wide("Cancel Current Search\tEsc").as_ptr(),
        );
        AppendMenuW(search_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            search_menu,
            MF_STRING,
            ID_FILTER_TOGGLE as usize,
            to_wide("Toggle &Live Filter Panel\tCtrl+Shift+F").as_ptr(),
        );
        AppendMenuW(
            search_menu,
            MF_STRING,
            ID_FILTER_CREATE_TAB as usize,
            to_wide("Create &Filter-to-Tab...\tCtrl+Alt+F").as_ptr(),
        );
        AppendMenuW(
            search_menu,
            MF_STRING,
            ID_FILTER_CLEAR as usize,
            to_wide("Clear Filter Query").as_ptr(),
        );
        AppendMenuW(
            menu_bar,
            MF_POPUP,
            search_menu as usize,
            to_wide("&Search").as_ptr(),
        );

        // 5. Tools Menu
        let tools_menu = CreatePopupMenu();
        AppendMenuW(
            tools_menu,
            MF_STRING,
            ID_TOOLS_BOOKMARK_TOGGLE as usize,
            to_wide("Toggle &Bookmark\tF2").as_ptr(),
        );
        AppendMenuW(
            tools_menu,
            MF_STRING,
            ID_TOOLS_BOOKMARK_NEXT as usize,
            to_wide("Next Bookmark\tShift+F2").as_ptr(),
        );
        AppendMenuW(
            tools_menu,
            MF_STRING,
            ID_TOOLS_BOOKMARK_PREV as usize,
            to_wide("Previous Bookmark\tAlt+F2").as_ptr(),
        );
        AppendMenuW(tools_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            tools_menu,
            MF_STRING,
            ID_TOOLS_HIGHLIGHT_RULES as usize,
            to_wide("&Highlight Rules Editor...\tCtrl+H").as_ptr(),
        );
        AppendMenuW(
            tools_menu,
            MF_STRING,
            ID_TOOLS_HIGHLIGHT_NEXT as usize,
            to_wide("Jump to Next Highlight\tF8").as_ptr(),
        );
        AppendMenuW(
            tools_menu,
            MF_STRING,
            ID_TOOLS_HIGHLIGHT_PREV as usize,
            to_wide("Jump to Previous Highlight\tShift+F8").as_ptr(),
        );
        AppendMenuW(tools_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            tools_menu,
            MF_STRING,
            ID_TOOLS_JUMP_LINE as usize,
            to_wide("&Jump to Line Number...\tCtrl+G").as_ptr(),
        );
        AppendMenuW(
            tools_menu,
            MF_STRING,
            ID_TOOLS_SETTINGS as usize,
            to_wide("Application &Settings...\tF10").as_ptr(),
        );
        AppendMenuW(
            menu_bar,
            MF_POPUP,
            tools_menu as usize,
            to_wide("&Tools").as_ptr(),
        );

        // 6. Help Menu
        let help_menu = CreatePopupMenu();
        AppendMenuW(
            help_menu,
            MF_STRING,
            ID_HELP_ABOUT as usize,
            to_wide("&About Tylogger...\tF1").as_ptr(),
        );
        AppendMenuW(
            menu_bar,
            MF_POPUP,
            help_menu as usize,
            to_wide("&Help").as_ptr(),
        );

        menu_bar
    }
}

/// Creates the Tab context menu for right-click on tab items.
pub fn create_tab_context_menu() -> HMENU {
    unsafe {
        let menu = CreatePopupMenu();
        AppendMenuW(
            menu,
            MF_STRING,
            ID_TAB_CLOSE as usize,
            to_wide("&Close Tab\tCtrl+W").as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            ID_TAB_CLOSE_OTHERS as usize,
            to_wide("Close &Other Tabs").as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            ID_TAB_CLOSE_RIGHT as usize,
            to_wide("Close Tabs to the &Right").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            menu,
            MF_STRING,
            ID_TAB_COPY_PATH as usize,
            to_wide("Copy &File Path").as_ptr(),
        );
        menu
    }
}
