# System Architecture Specification
## LogExpert Rust Reimplementation

**Architecture Model:** Multi-threaded, Modular Event-Driven Win32 Architecture  
**Guiding Principle:** Strict isolation between Core I/O / Indexing and UI Presentation. Zero blocking of the GUI thread.

---

## 1. System Architecture Diagram

```
+--------------------------------------------------------------------------------+
|                               Win32 GUI Layer                                  |
|  +-------------------+  +--------------------+  +---------------------------+  |
|  |  MainWindow Host  |  | Virtual Log Grid   |  | Filter & Search Dock      |  |
|  |  (Menu, Toolbar)  |  | (Custom GDI Paint) |  | (Controls & Inputs)       |  |
|  +-------------------+  +--------------------+  +---------------------------+  |
|            |                      |                           |                |
|            +----------------------+---------------------------+                |
|                                   | Win32 Message Pump (GetMessage/Dispatch)   |
|                                   v                                            |
|                  [ GUI Controller & Document Manager ]                         |
+-----------------------------------|--------------------------------------------+
                                    | Thread Channels & Win32 Events (WM_APP)
                                    v
+--------------------------------------------------------------------------------+
|                               Core Engine Layer                                |
|  +--------------------------------------------------------------------------+  |
|  |                              Document Session                            |  |
|  |  +--------------------+  +--------------------+  +--------------------+  |  |
|  |  | Sparse Line Index  |  | Bounded LRU Cache  |  | Columnizer Registry|  |  |
|  |  | (64-bit offsets)   |  | (Decoded Strings)  |  | (Parser Dispatch)  |  |  |
|  |  +--------------------+  +--------------------+  +--------------------+  |  |
|  +--------------------------------------------------------------------------+  |
|            |                                              |                    |
|            v                                              v                    |
|  +--------------------+                         +--------------------+         |
|  | File Reader Engine |                         | Background Workers |         |
|  | - Non-locking Read |                         | - Parallel Search  |         |
|  | - Windowed mmap    |                         | - Filter-to-Tab    |         |
|  | - Stream Fallback  |                         | - Cancellable DFA  |         |
|  | - Change Poller    |                         +--------------------+         |
|  +--------------------+                                                        |
+--------------------------------------------------------------------------------+
```

---

## 2. Module Breakdown

### 2.1 `logexpert-core`
- **`io::file_source`:** Encapsulates Windows file handles opened with:
  ```rust
  // Non-locking share attributes
  FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
  ```
  Provides windowed memory mapping for large files with graceful fallback to standard buffered streaming when memory mapping is unavailable (e.g., network shares).
- **`index::sparse_index`:** 64-bit byte-offset indexing. Stores line boundaries in sparse chunks (e.g., every 64th or 128th line stored explicitly; intervening lines resolved on demand). Ensures 10+ million lines consume < 20 MB RAM.
- **`cache::line_cache`:** Bounded LRU cache holding decoded, formatted line strings to prevent repeated disk reads during viewport scrolling.
- **`encoding::detector`:** Detects UTF-8, UTF-16LE/BE (BOM), and ANSI codepages. Decodes byte slices into UTF-8 strings.
- **`search::engine`:** Cancellable background regex and text search worker. Uses chunk-based scanning and cooperative preemption (checks cancellation atomic flag every 64 KB).

### 2.2 `logexpert-gui`
- **`window::main_window`:** Registers the Win32 window class, creates top-level overlapped window, creates menu, rebar toolbar, and status bar.
- **`window::log_grid`:** Custom custom-control implementing virtual grid rendering:
  - Responds to `WM_PAINT` by querying visible row range from vertical scrollbar position.
  - Draws text using GDI double-buffering (`CreateCompatibleDC`, `CreateCompatibleBitmap`, `BitBlt`) to guarantee zero flicker.
- **`window::marker_bar`:** Custom control rendering bookmark and highlight positions proportionally to total line count.
- **`dpi::scaling`:** High-DPI awareness logic supporting per-monitor DPI scaling on Windows 10/11 and system DPI on Windows 7.

### 2.3 `logexpert-plugins`
- **`columnizer`:** Pluggable interface for parsing raw lines into discrete tabular columns.
  - Built-in: Default (raw), CSV, Timestamp + Message, Log4j/Logback, XML parser.
- **`highlight`:** Rule evaluation engine applying foreground/background colors and bookmark triggers based on regex/string conditions.

---

## 3. Concurrency & Threading Model

1. **Main UI Thread:**
   - Sole owner of Win32 HWNDs and GDI handles.
   - Dispatches user input (keyboard, mouse, menus).
   - Receives completion/progress notifications from background threads via `PostMessage(hwnd, WM_APP_DATA_READY, ...)`.

2. **File Watcher & Indexer Thread:**
   - Dedicated worker per open document.
   - Monitors file size changes via `GetFileSizeEx` and directory change notifications.
   - Incrementally appends new line offsets to the sparse index when file grows.
   - Detects file truncation (`new_size < previous_size`) and triggers clean re-index.

3. **Search Worker Thread Pool:**
   - Spawns on-demand when user initiates Search or Filter-to-Tab.
   - Operates on immutable slices of the line index.
   - Yields results in batches to prevent flooding the GUI message queue.

---

## 4. Error Handling & Defensiveness
- **Zero Panic Guarantee:** File I/O errors (e.g., unexpected network disconnection on mapped drives) are caught and reported as status indicators rather than unwinding.
- **Graceful Degradation:** If `CreateFileMapping` fails due to address space exhaustion on 32-bit systems, the engine transparently falls back to `ReadFile` stream chunks.
