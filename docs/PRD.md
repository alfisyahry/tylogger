# Product Requirements Document (PRD)
## LogExpert Rust Reimplementation

**Project:** Native Windows Log Analysis Tool (`logexpert-rust`)  
**Target:** 100% Feature Parity with Upstream LogExpert, High Performance, Low Memory Footprint  
**Platforms:** Windows 7 SP1 through Windows 11 (x86 & x64)  
**License:** MIT  

---

## 1. Executive Summary
LogExpert is a renowned, feature-rich GUI log analysis and file tailing tool originally written in C# (.NET Framework Windows Forms). While highly functional, the legacy .NET implementation suffers from high memory consumption on multi-gigabyte log files, occasional UI thread locking during intensive regex searches, and reliance on the .NET runtime.

The Rust reimplementation delivers a native Win32 desktop application that achieves complete functional parity with LogExpert while providing:
- **Instant startup** (< 200 ms).
- **Sub-50 MB memory baseline**, bounded even when tailing multi-gigabyte files.
- **Strictly non-locking read-only file access** allowing external loggers to write, rotate, and delete logs unhindered.
- **Zero web/runtime dependencies** (no Electron, no WebView, no Node.js, no .NET runtime).
- **Uncompromised backward compatibility** from Windows 7 SP1 to Windows 11 on both 32-bit and 64-bit architectures.

---

## 2. Target Users & Use Cases
1. **Systems & Site Reliability Engineers (SRE):** Monitoring live server, service, and container logs with low-latency follow-tail.
2. **Software Developers & QA:** Investigating multi-gigabyte crash logs, stack traces, and historical trace files with high-speed regex filtering.
3. **Embedded & Industrial Systems Operators:** Running diagnostic utilities on constrained legacy Windows 7 SP1 / Windows Embedded workstations without internet connectivity or modern runtimes.

---

## 3. Functional Requirements

### 3.1 Document & File Ingestion
- **FR-1.1:** Non-locking file access using `GENERIC_READ` and `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`.
- **FR-1.2:** Encoding auto-detection and manual override for UTF-8, UTF-16LE, UTF-16BE, and ANSI (Windows-1252 / ISO-8859-1).
- **FR-1.3:** Seamless follow-tail with configurable polling interval (default 250 ms) and real-time append notification.
- **FR-1.4:** Detection and graceful recovery from file truncation (size shrinkage) and file rotation.
- **FR-1.5:** MultiFile virtual concatenation (merging chronologically rotated log segments into a single contiguous virtual log stream).

### 3.2 Navigation & Rendering
- **FR-2.1:** Virtualized log grid capable of rendering millions of lines at constant 60 FPS without storing formatted text in memory.
- **FR-2.2:** Columnizer engine for structured log parsing (CSV, Apache/Log4j, XML, JSON, Timestamp + Payload).
- **FR-2.3:** Bookmark management with toggle, keyboard navigation (F2 / Shift+F2), and persistent user annotations/comments.
- **FR-2.4:** Visual marker bar on the right margin indicating relative positions of bookmarks, search hits, and highlight alerts.

### 3.3 Search & Filtering
- **FR-3.1:** Fast exact string and regular expression searching with immediate cancellation support.
- **FR-3.2:** Filter-to-Tab: Isolating matching log lines into a linked child tab that continues updating during live tailing.
- **FR-3.3:** Advanced filtering options: Invert match, Case sensitivity, Column-specific filter, Fore-spread and Back-spread context lines (surrounding lines around matches).
- **FR-3.4:** Highlight groups: Color-coded keyword and regex patterns applied across rows or specific columns.

### 3.4 Synchronization & Advanced Analytics
- **FR-4.1:** Multi-tab synchronized scrolling based on timestamp parsing across different log files.
- **FR-4.2:** Timeshift adjustment to reconcile time drift across disparate log sources.
- **FR-4.3:** External tool launching with dynamic arguments (e.g., launching editor or CLI at line offset).
- **FR-4.4:** Session persistence (`.lxp` workspace files saving open tabs, highlights, filters, and bookmarks).

---

## 4. Non-Functional Requirements

### 4.1 Performance & Resource Targets
- **Startup Latency:** Cold start < 250 ms; Warm start < 100 ms.
- **Memory Footprint:** Baseline < 30 MB; Max 150 MB when indexing a 10 GB log file (utilizing sparse 64-bit checkpoints).
- **Search Throughput:** > 250 MB/s for plain string search; > 100 MB/s for regex search.
- **UI Responsiveness:** GUI message loop latency < 16 ms (never blocked by background I/O or search).

### 4.2 Security, Privacy & Reliability
- **Offline Operation:** Zero external network requests; no telemetry, telemetry opt-in, or auto-updater phone-home.
- **File Safety:** Strictly read-only file handles; never modifies, overwrites, or truncates target log files.
- **Memory Safety:** 100% memory-safe Rust core; FFI boundary strictly isolated in Win32 platform layer.

---

## 5. Scope & Phasing
- **Phase 0 (Current):** Technical audit, upstream C# mapping, architecture specification, Win7 SP1 native GUI PoC, PE IAT import validation.
- **Phase 1:** Core engine (sparse indexing, windowed mmap, non-locking file reader, line cache).
- **Phase 2:** Virtualized Win32 log grid, bookmarking, and basic follow-tail.
- **Phase 3:** Search engine, filter-to-tab, and highlight groups.
- **Phase 4:** Columnizer engine, MultiFile, timesync, and session persistence.
- **Phase 5:** Plugin subsystem, edge-case hardening, and installer packaging.
