# Tylogger — Command Registry & Shortcut Mapping

This document provides the authoritative mapping between command IDs, display labels, default keyboard shortcuts, and UI invocation points (Menu Bar, Toolbar, Context Menus).

| Command ID | Technical Identifier | Menu Path | Toolbar | Shortcut | Description |
|---|---|---|---|---|---|
| 1001 | `ID_FILE_OPEN` | File -> Open File... | Open | `Ctrl+O` | Open a log file in a new tab |
| 1002 | `ID_FILE_OPEN_MULTI` | File -> Open Multiple Files... | — | `Ctrl+Shift+O` | Open multiple log files concurrently |
| 1003 | `ID_FILE_RELOAD` | File -> Reload | Reload | `F5` | Reload and re-index the active document |
| 1004 | `ID_FILE_CLOSE` | File -> Close Document | Close | `Ctrl+W` | Close the active document tab |
| 1005 | `ID_FILE_EXIT` | File -> Exit | — | `Alt+F4` | Exit application |
| 1010 | `ID_EDIT_COPY` | Edit -> Copy | — | `Ctrl+C` | Copy selected line(s) to Windows clipboard |
| 1011 | `ID_EDIT_SELECT_ALL` | Edit -> Select All | — | `Ctrl+A` | Select all rows in active document |
| 1020 | `ID_VIEW_THEME_CYCLE` | View -> Cycle Theme | — | `F7` | Cycle through Light, Dark, Soft Dark palettes |
| 1021 | `ID_VIEW_FOLLOW_TAIL` | View -> Follow Tail | Follow Tail | `Ctrl+T` | Toggle auto-scroll to latest incoming line |
| 1022 | `ID_VIEW_PAUSE_MONITORING` | View -> Pause Monitoring | Pause | `Ctrl+P` | Suspend background file append checks |
| 1023 | `ID_VIEW_ZOOM_IN` | View -> Zoom In | — | `Ctrl++` | Increase font size |
| 1024 | `ID_VIEW_ZOOM_OUT` | View -> Zoom Out | — | `Ctrl+-` | Decrease font size |
| 1025 | `ID_VIEW_ZOOM_RESET` | View -> Reset Zoom | — | `Ctrl+0` | Reset font size to default (13 DIP) |
| 1030 | `ID_SEARCH_FIND` | Search -> Find... | Find | `Ctrl+F` | Toggle contextual search bar |
| 1031 | `ID_SEARCH_FIND_NEXT` | Search -> Find Next | — | `F3` | Jump to next search match |
| 1032 | `ID_SEARCH_FIND_PREV` | Search -> Find Previous | — | `Shift+F3` | Jump to previous search match |
| 1033 | `ID_SEARCH_CANCEL` | Search -> Cancel Search | — | `Esc` | Cancel search and close search bar |
| 1040 | `ID_FILTER_TOGGLE` | Filter -> Filter Results... | Filter | `Ctrl+Shift+F` | Toggle inspection panel & filter results |
| 1041 | `ID_FILTER_CLEAR` | Filter -> Clear Filter | — | — | Reset filter criteria |
| 1050 | `ID_TOOLS_BOOKMARK_TOGGLE` | Tools -> Toggle Bookmark | Bookmark | `F2` | Toggle bookmark on selected line |
| 1051 | `ID_TOOLS_BOOKMARK_NEXT` | Tools -> Next Bookmark | — | `Shift+F2` | Jump to next bookmark |
| 1052 | `ID_TOOLS_BOOKMARK_PREV` | Tools -> Previous Bookmark | — | `Ctrl+F2` | Jump to previous bookmark |
| 1053 | `ID_TOOLS_JUMP_LINE` | Tools -> Jump to Line... | — | `Ctrl+G` | Jump to specific 64-bit line number |
| 1054 | `ID_TOOLS_SETTINGS` | Tools -> Settings... | Settings | `F10` | Display user preferences dialog |
| 1060 | `ID_HELP_ABOUT` | Help -> About Tylogger | — | `F1` | Show official About Tylogger dialog |
| 2401 | `ID_REMOTE_CONNECT` | File -> Remote -> Connect to SSH/SFTP... | Connect | `Ctrl+Shift+R` | Open SSH/SFTP Connection Manager |
| 2402 | `ID_REMOTE_BROWSE` | File -> Remote -> Browse SFTP Files... | Browse | `Ctrl+Shift+B` | Open SFTP Remote File Browser |
| 2403 | `ID_STREAM_OPEN` | File -> Stream -> Open SSH Stream... | Stream | `Ctrl+Shift+S` | Open SSH Live Stream Adapter (tail, journald, docker) |
| 2404 | `ID_ARCHIVE_OPEN` | File -> Archive -> Open Compressed Log... | Archive | `Ctrl+Shift+Z` | Open Compressed Archive (GZIP/ZIP) |
| 2405 | `ID_LEGACY_IMPORT` | File -> Import -> Legacy LogExpert Session... | Import | `Ctrl+Shift+I` | Import LogExpert Legacy Session (.lxp/.lxj) |

## Tab Context Menu Commands

| Command ID | Label | Action |
|---|---|---|
| 2001 | Close Tab | Closes the right-clicked tab (`Ctrl+W`) |
| 2002 | Close Other Tabs | Closes all document tabs except the target tab |
| 2003 | Close Tabs to the Right | Closes all document tabs positioned to the right |
| 2004 | Copy File Path | Copies full canonical file path to Windows clipboard |
