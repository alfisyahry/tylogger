# Feature Parity Matrix
## Upstream LogExpert C# vs. Rust Reimplementation

**Reference Upstream:** LogExpert 1.8.x (`https://github.com/LogExperts/LogExpert`)  
**Parity Target:** 100% Core & User Workflow Compatibility

---

## 1. Feature Parity Breakdown

| Feature Category | Upstream LogExpert Feature | Rust Implementation Approach | Status / Target |
|---|---|---|---|
| **Document Management** | Multi-tab document interface | Native Win32 Tab Control (`WC_TABCONTROL`) | Phase 2 |
| | Multi-window / Detachable tabs | Multi-HWND host with tab drag-and-drop | Phase 4 |
| | Session save / restore (`.lxp`) | Serde JSON / XML compatible format | Phase 4 |
| | Recent files list with persistence | Win32 MRU registry / config file | Phase 2 |
| **File I/O & Tailing** | Live Follow-Tail | Non-blocking background poller + tail mode | Phase 1 (PoC Ready) |
| | Non-locking file access | `FILE_SHARE_READ \| WRITE \| DELETE` | Phase 1 (PoC Ready) |
| | MultiFile (chronological log merge) | Virtual multi-source indexed stream | Phase 4 |
| | Rotation / Truncation recovery | Auto-detect size shrinkage and re-index | Phase 1 |
| | Encoding support (UTF8, UTF16, ANSI) | Streaming encoding converter | Phase 1 |
| **Grid & Navigation** | Virtualized row rendering | Custom GDI double-buffered grid | Phase 0 (PoC Ready) |
| | Column resizing & auto-fit | Win32 Header Control (`WC_HEADER`) | Phase 2 |
| | Smooth scrolling (keys & wheel) | Standard scrollbar message handlers | Phase 0 (PoC Ready) |
| | Go to line (`Ctrl+G`) | Direct sparse index jump (< 1ms) | Phase 2 |
| **Bookmarks & Annotations**| Toggle bookmark (`Ctrl+B`) | BitSet / interval tree bookmark store | Phase 2 |
| | Bookmark comments / notes | Hash map `line_id -> String` | Phase 3 |
| | Bookmark navigation (`F2`/`Shift+F2`) | Binary search in bookmark index | Phase 2 |
| | Right-side Marker Bar | Custom painted vertical margin strip | Phase 2 |
| **Search & Filtering** | Plain text search | Boyer-Moore / Knuth-Morris-Pratt | Phase 3 |
| | Regular expression search | Linear-time DFA `regex` crate | Phase 3 |
| | Filter-to-Tab | Virtual child document over filtered index | Phase 3 |
| | Invert match (exclude patterns) | Inverted filter predicate logic | Phase 3 |
| | Range filter (from pattern to pattern)| Multi-state filter machine | Phase 4 |
| | Fore-spread & Back-spread context | Window offset expansion around matches | Phase 3 |
| | Column-specific filtering | Predicate evaluated on parsed column | Phase 3 |
| **Highlighting** | Highlight Groups | Ordered regex/string match rules | Phase 3 |
| | Row and cell color overrides | GDI brush / text color selection | Phase 3 |
| | Bookmark triggers on pattern match | Auto-bookmark hook on indexer append | Phase 4 |
| **Columnizers** | Default (raw line) | Pass-through columnizer | Phase 2 |
| | CSV / Delimited columnizer | Configurable delimiter tokenizer | Phase 3 |
| | Timestamp + Message columnizer | Fast datetime parser | Phase 3 |
| | Log4j / Logback XML columnizer | Lightweight XML event pull parser | Phase 4 |
| | Custom C# / Native plugin API | C ABI dynamic link library (`cdylib`) | Phase 5 |
| **Time Sync** | Timestamp synchronization | Timestamp offset index across tabs | Phase 4 |
| | Timeshift adjustment | User delta offset applied to time column | Phase 4 |
| | Time spread calculation | Inter-line duration calculation | Phase 4 |
| **External Integration**| External Tools launcher | `CreateProcessW` with token macros | Phase 4 |
| | CLI command-line arguments | Argument parser (`--file`, `--line`, etc.)| Phase 2 |

---

## 2. Parity Testing Strategy
1. **Fixture Parity:** Replay recorded LogExpert test log fixtures (Apache access logs, log4net XML, multi-gigabyte traces).
2. **Behavioral Assertions:** Verify line counts, bookmark offsets, filter tab content, and search match indices match upstream LogExpert down to the exact row number.
3. **Regex Verification:** Unit tests matching .NET standard regex fixtures against Rust engine behavior.
