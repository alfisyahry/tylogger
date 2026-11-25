//! Centralized command registry for Tylogger.
//! Ensures consistent IDs, labels, shortcuts, and handlers across
//! Menu Bar, Toolbar, Context Menus, and Keyboard Accelerators.

pub const ID_FILE_OPEN: u16 = 1001;
pub const ID_FILE_OPEN_MULTI: u16 = 1002;
pub const ID_FILE_RELOAD: u16 = 1003;
pub const ID_FILE_CLOSE: u16 = 1004;
pub const ID_FILE_EXIT: u16 = 1005;

pub const ID_EDIT_COPY: u16 = 1010;
pub const ID_EDIT_SELECT_ALL: u16 = 1011;

pub const ID_VIEW_THEME_CYCLE: u16 = 1020;
pub const ID_VIEW_FOLLOW_TAIL: u16 = 1021;
pub const ID_VIEW_PAUSE_MONITORING: u16 = 1022;
pub const ID_VIEW_ZOOM_IN: u16 = 1023;
pub const ID_VIEW_ZOOM_OUT: u16 = 1024;
pub const ID_VIEW_ZOOM_RESET: u16 = 1025;

pub const ID_SEARCH_FIND: u16 = 1030;
pub const ID_SEARCH_FIND_NEXT: u16 = 1031;
pub const ID_SEARCH_FIND_PREV: u16 = 1032;
pub const ID_SEARCH_CANCEL: u16 = 1033;

pub const ID_FILTER_TOGGLE: u16 = 1040;
pub const ID_FILTER_CLEAR: u16 = 1041;

pub const ID_TOOLS_BOOKMARK_TOGGLE: u16 = 1050;
pub const ID_TOOLS_BOOKMARK_NEXT: u16 = 1051;
pub const ID_TOOLS_BOOKMARK_PREV: u16 = 1052;
pub const ID_TOOLS_JUMP_LINE: u16 = 1053;
pub const ID_TOOLS_SETTINGS: u16 = 1054;

pub const ID_HELP_ABOUT: u16 = 1060;

// Context Menu IDs for Tabs
pub const ID_TAB_CLOSE: u16 = 2001;
pub const ID_TAB_CLOSE_OTHERS: u16 = 2002;
pub const ID_TAB_CLOSE_RIGHT: u16 = 2003;
pub const ID_TAB_COPY_PATH: u16 = 2004;

#[derive(Debug, Clone)]
pub struct CommandDef {
    pub id: u16,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub enabled: bool,
    pub checked: bool,
}

impl CommandDef {
    pub const fn new(id: u16, label: &'static str, shortcut: &'static str) -> Self {
        Self {
            id,
            label,
            shortcut,
            enabled: true,
            checked: false,
        }
    }
}
