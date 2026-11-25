# UI/UX Design Specification
## LogExpert Rust Reimplementation

**Design Philosophy:** Clean, professional, high-density native Windows workstation aesthetic.  
**Style Target:** Native Win32 classic/modern look (matching Sysinternals, Notepad++, and classic LogExpert).  
**Zero Bloat:** No decorative subtitles, web card containers, animations, gradients, or glassmorphism.

---

## 1. Visual Hierarchy & Window Layout

```
+-------------------------------------------------------------------------+
| [Menu Bar] File  Edit  View  Filter  Options  Tools  Help               |
+-------------------------------------------------------------------------+
| [Main Toolbar] Open | Tail [x] | Bookmark | Search | Filter | Clear     |
+-------------------------------------------------------------------------+
| [Tab Strip] [app.log x] [service.log x] [Filter: error* x] [+]          |
+-------------------------------------------------------------------+-----+
| Line | Timestamp           | Level | Message                      | M   |
|------|---------------------|-------|------------------------------| A   |
| 1    | 2026-11-25 08:00:01 | INFO  | System startup sequence...   | R   |
| 2    | 2026-11-25 08:00:02 | DEBUG | Connecting to database...    | K   |
| 3 [*]| 2026-11-25 08:00:04 | ERROR | Connection timeout [retry 1] | E   |
| 4    | 2026-11-25 08:00:05 | WARN  | Slow query execution: 320ms  | R   |
| ...  | ...                 | ...   | ...                          |     |
+-------------------------------------------------------------------+-----+
| [Filter Dock Panel - Collapsible]                                       |
| Search: [ ERROR|FATAL                  ] [x] Regex [ ] Case [Filter-Tab]|
+-------------------------------------------------------------------------+
| [Status Bar] Lines: 1,420,890 | Size: 184.2 MB | UTF-8 | Pos: 3 | Tail: ON|
+-------------------------------------------------------------------------+
```

---

## 2. Typography & Fonts
- **Interface & Chrome Controls:** `Segoe UI` (default), 9 pt on 96 DPI (scaled dynamically per monitor DPI). Fallback: `Tahoma` on legacy Windows installations.
- **Log Data & Monospace Grids:** `Consolas` (default), 10 pt. Fallback: `Lucida Console` or `Courier New`.
- **User Customization:** Font face and size must be freely configurable via settings with instant live preview.

---

## 3. Color Palette & Theming

### 3.1 Classic Light Theme (Default Windows Native)
- **Window Background:** `COLOR_WINDOW` (`#FFFFFF` / `#F0F0F0`).
- **Text Color:** `COLOR_WINDOWTEXT` (`#000000`).
- **Selection Background:** `COLOR_HIGHLIGHT` (`#3399FF`).
- **Selection Text:** `COLOR_HIGHLIGHTTEXT` (`#FFFFFF`).
- **Bookmark Margin:** `#FFF8DC` (Cornsilk) with `#D9534F` bookmark star indicator.

### 3.2 Soft Dark Theme (Developer Optimized)
- **Window Background:** `#1E1E1E` (Dark Charcoal).
- **Panel & Chrome:** `#252526`.
- **Text Color:** `#D4D4D4` (Off-white).
- **Selection Background:** `#264F78` (Subdued Blue).
- **Border / Splitters:** `#3E3E42`.

### 3.3 Log Level Standard Colors
- **ERROR / CRITICAL:** Text `#D9534F`, Soft Dark `#F48771`.
- **WARN / WARNING:** Text `#F0AD4E`, Soft Dark `#CCA700`.
- **INFO:** Standard text color.
- **DEBUG / TRACE:** Muted gray `#888888`.

---

## 4. Key Components & Interactions

### 4.1 Virtualized Log Grid
- **Custom Double-Buffered Rendering:** `WM_PAINT` renders only rows currently in the client viewport + 1 overscan row.
- **Column Resizing:** Draggable split headers with double-click auto-fit.
- **Row Selection:** Single-click row selection, Shift-click range, Ctrl-click multi-select.
- **Cell Copying:** Ctrl+C copies selected line(s) or specific cell value.

### 4.2 Marker Bar (Right Margin Overview)
- A narrow 16px vertical strip beside the scrollbar.
- Represents the entire file height proportionally.
- Renders colored horizontal ticks:
  - Red tick = Error / Highlight match.
  - Blue/Yellow tick = Bookmark.
  - Green tick = Active search match.
- Clicking any tick instantly scrolls the grid to that row.

### 4.3 Collapsible Filter Dock Panel
- Positioned at the bottom or docked to the side.
- Hotkey `Ctrl+F` focuses search input; `Esc` clears or hides panel.
- Live search highlights matching rows in-place; "Filter to Tab" creates a persistent virtual filtered view.

---

## 5. Keyboard Navigation & Shortcuts
| Action | Shortcut | Description |
|---|---|---|
| Open File | `Ctrl+O` | Standard file picker dialog |
| Close Tab | `Ctrl+W` | Close current document tab |
| Find / Filter | `Ctrl+F` | Open/focus filter panel |
| Find Next | `F3` | Navigate to next search hit |
| Find Previous | `Shift+F3` | Navigate to previous search hit |
| Toggle Bookmark | `Ctrl+B` or Double-click margin | Set/clear bookmark on current row |
| Next Bookmark | `F2` | Jump to next bookmark |
| Prev Bookmark | `Shift+F2` | Jump to previous bookmark |
| Follow Tail | `Ctrl+T` | Toggle continuous scroll-to-bottom |
| Go to Line | `Ctrl+G` | Jump to line number |
| Refresh | `F5` | Re-read and re-index file from disk |
| Copy Row | `Ctrl+C` | Copy selected line(s) to clipboard |
