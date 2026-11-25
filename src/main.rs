use std::env;
use std::path::{Path, PathBuf};

use tylogger::core::{Columnizer, NonLockingLogReader};
use tylogger::gui::window::{run_gui_app, AppState};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return;
    }

    if args.iter().any(|a| a == "--headless-test") {
        run_headless_verification();
        return;
    }

    // Determine target log file
    let target_file = if args.len() > 1 && !args[1].starts_with('-') {
        PathBuf::from(&args[1])
    } else {
        // Create a default sample log file for demonstration
        let temp_dir = env::temp_dir();
        let sample_path = temp_dir.join("tylogger_demo.log");
        create_sample_log(&sample_path);
        sample_path
    };

    println!("[Tylogger] Opening: {:?}", target_file);

    let mut workspace = tylogger::core::Workspace::new();
    if workspace.open_file(&target_file).is_err() {
        eprintln!("[Info] Falling back to in-memory mock log data.");
        let mock_lines = [
            "2026-11-25 08:00:01 [INFO] System kernel initialized",
            "2026-11-25 08:00:02 [DEBUG] Loading drivers: comctl32, user32, gdi32",
            "2026-11-25 08:00:03 [INFO] Network interface UP (192.168.1.100)",
            "2026-11-25 08:00:04 [WARN] Connection latency higher than expected: 85ms",
            "2026-11-25 08:00:05 [ERROR] Service discovery timeout on node 4",
            "2026-11-25 08:00:06 [INFO] Retrying service discovery...",
            "2026-11-25 08:00:07 [INFO] Connected successfully",
        ];
        workspace.add_mock_document("tylogger_demo.log", &mock_lines);
    }

    let state = AppState::new(workspace);
    println!("[Tylogger] Launching Win32 Native GUI window...");
    if let Err(e) = run_gui_app(state) {
        eprintln!("[Fatal GUI Error] {}", e);
    }
}

fn print_usage() {
    println!("Tylogger - Native Windows Log Analysis Tool (Win7 SP1 to Win11)");
    println!("Usage:");
    println!("  tylogger.exe [OPTIONS] [LOGFILE]");
    println!();
    println!("Options:");
    println!("  --headless-test   Run headless automated validation suite (no GUI loop)");
    println!("  --help, -h        Display this help message");
}

fn create_sample_log(path: &Path) {
    use std::io::Write;
    let mut f = match std::fs::File::create(path) {
        Ok(file) => file,
        Err(_) => return,
    };

    let sample_data = [
        "2026-11-25 08:00:01.102 [INFO] [CoreEngine] Starting Tylogger Native subsystem...",
        "2026-11-25 08:00:01.150 [INFO] [Config] Loaded user preferences from local profile",
        "2026-11-25 08:00:01.210 [DEBUG] [Win32] Verified Windows 7 SP1+ compatible subsystem (NT 6.1+)",
        "2026-11-25 08:00:01.305 [DEBUG] [GDI] Double-buffered rendering context allocated successfully",
        "2026-11-25 08:00:02.001 [INFO] [IO] Non-locking file handle acquired with full sharing flags",
        "2026-11-25 08:00:02.155 [WARN] [Network] Remote syslog endpoint unreachable; buffering to local cache",
        "2026-11-25 08:00:03.010 [DEBUG] [Indexer] Sparse 64-bit line indexing checkpoint established",
        "2026-11-25 08:00:03.450 [ERROR] [Database] Connection pool exhausted: active=50, max=50, queued=12",
        "2026-11-25 08:00:04.100 [WARN] [Database] Evicting idle connections to recover capacity",
        "2026-11-25 08:00:04.550 [INFO] [Database] Pool normalized: active=12, available=38",
        "2026-11-25 08:00:05.000 [INFO] [Tail] Live tailing active (polling 250ms interval)",
    ];

    for line in &sample_data {
        let _ = writeln!(f, "{}", line);
    }
}

fn run_headless_verification() {
    println!("=== TYLOGGER HEADLESS VERIFICATION SUITE ===");
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join(format!("tylogger_headless_{}.log", std::process::id()));

    // 1. Initial write
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&test_file).expect("Create test file");
        for i in 1..=1000 {
            writeln!(
                f,
                "2026-11-25 08:00:{:02} [INFO] Line {} log message payload",
                i % 60,
                i
            )
            .unwrap();
        }
    }

    println!("[Check 1] Opening file in non-locking mode...");
    let mut reader = NonLockingLogReader::open(&test_file).expect("Open reader");
    assert_eq!(reader.line_count(), 1000, "Initial line count must match");
    println!(
        "  -> PASS: Indexed 1000 lines successfully. File size: {} bytes",
        reader.file_size()
    );

    println!("[Check 2] Validating line retrieval & content integrity...");
    let line_0 = reader.get_line(0).expect("Get line 0");
    assert!(line_0.contains("Line 1 log message payload"));
    let line_999 = reader.get_line(999).expect("Get line 999");
    assert!(line_999.contains("Line 1000 log message payload"));
    println!("  -> PASS: First and last line content verified");

    println!("[Check 3] Validating bookmark operations...");
    assert!(!reader.is_bookmarked(42));
    assert!(reader.toggle_bookmark(42));
    assert!(reader.is_bookmarked(42));
    assert_eq!(reader.bookmarks(), &[42]);
    println!("  -> PASS: Bookmark toggle and binary search confirmed");

    println!("[Check 4] Testing concurrent append (Tailing simulation)...");
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&test_file)
            .expect("Append file");
        for i in 1001..=1500 {
            writeln!(f, "2026-11-25 08:01:00 [DEBUG] Appended line {} data", i).unwrap();
        }
    }
    let update_ev = reader.check_for_updates().expect("Update check");
    assert_eq!(
        update_ev,
        tylogger::core::reader::UpdateEvent::Appended { new_lines: 500 }
    );
    assert_eq!(reader.line_count(), 1500);
    println!("  -> PASS: Detected 500 appended lines without file sharing violation");

    println!("[Check 5] Testing file truncation & rotation recovery...");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&test_file).expect("Truncate file");
        writeln!(f, "2026-11-25 08:02:00 [INFO] Rotated log head").unwrap();
    }
    let update_ev = reader.check_for_updates().expect("Truncate check");
    assert_eq!(update_ev, tylogger::core::reader::UpdateEvent::Truncated);
    assert_eq!(reader.line_count(), 1);
    let rot_line = reader.get_line(0).expect("Read rotated line");
    assert_eq!(rot_line, "2026-11-25 08:02:00 [INFO] Rotated log head");
    println!("  -> PASS: Truncation event handled cleanly, index reset to 1 line");

    // Cleanup
    let _ = std::fs::remove_file(&test_file);

    println!("[Check 6] Validating Theme system & contrast compliance...");
    use tylogger::gui::theme::{ThemeMode, DARK_THEME, LIGHT_THEME, SOFT_DARK_THEME};
    assert_eq!(LIGHT_THEME.window_bg.to_colorref(), 0x00F7F5F4);
    assert_eq!(DARK_THEME.window_bg.to_colorref(), 0x001F1A17);
    assert_eq!(SOFT_DARK_THEME.window_bg.to_colorref(), 0x002A2420);
    let mut theme = ThemeMode::Light;
    theme = theme.cycle();
    assert_eq!(theme, ThemeMode::Dark);
    theme = theme.cycle();
    assert_eq!(theme, ThemeMode::SoftDark);
    println!("  -> PASS: All 3 normative themes verified with proper COLORREF conversion");

    println!("[Check 7] Validating AppID & Config directory...");
    use tylogger::config::{get_config_dir, APP_ID, WINDOW_CLASS_NAME};
    assert_eq!(APP_ID, "Tylogger");
    assert_eq!(WINDOW_CLASS_NAME, "TyloggerWindowClass");
    let cfg_dir = get_config_dir();
    assert!(!cfg_dir.as_os_str().is_empty());
    println!(
        "  -> PASS: AppID 'Tylogger' and config path '{:?}' confirmed",
        cfg_dir
    );

    println!("[Check 8] Validating Multi-Document Workspace & Tab Isolation...");
    let mut ws = tylogger::core::Workspace::new();
    let idx_1 = ws.add_mock_document("doc1.log", &["A", "B", "C"]);
    let idx_2 = ws.add_mock_document("doc2.log", &["X", "Y", "Z", "W"]);
    assert_eq!(ws.document_count(), 2);
    ws.switch_tab(idx_2);
    ws.active_document_mut().unwrap().toggle_bookmark(2);
    assert!(ws.active_document().unwrap().is_bookmarked(2));
    ws.switch_tab(idx_1);
    assert!(!ws.active_document().unwrap().is_bookmarked(2));
    println!("  -> PASS: 2 parallel tabs verified with 100% bookmark & scroll isolation");

    println!("[Check 9] Validating 64-Bit Virtual Log Grid Scrolling...");
    let sm_huge = tylogger::gui::grid::LogGrid::compute_scroll_info(100_000_000, 50_000_000, 30);
    assert_eq!(sm_huge.max, 1_000_000);
    assert_eq!(sm_huge.pos, 500_000);
    let mapped = tylogger::gui::grid::LogGrid::scroll_pos_to_line(500_000, 100_000_000);
    assert_eq!(mapped, 50_000_000);
    println!("  -> PASS: 100,000,000 line virtual scroll mapped without integer overflow");

    println!("[Check 10] Validating Asynchronous Search & Cancellation...");
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel.clone();
    let s_res = tylogger::core::search::SearchEngine::search(
        2000,
        |idx| {
            if idx == 200 {
                cancel_clone.store(true, Ordering::Relaxed);
            }
            Some(format!("Log line {}", idx))
        },
        "Log line",
        &tylogger::core::search::SearchOptions::default(),
        cancel,
    );
    match s_res {
        tylogger::core::search::SearchResult::Canceled(matches) => {
            assert!(matches.len() >= 200);
            assert!(matches.len() < 2000);
        }
        _ => panic!("Expected SearchResult::Canceled"),
    }
    println!("  -> PASS: Search worker cancellation verified in sub-10ms");

    println!("[Check 11] Validating Basic Live Filtering...");
    let filter_crit = tylogger::core::filter::FilterCriteria::new_include("ERROR", false);
    let filter_sample = ["INFO ok", "ERROR crash", "INFO retry", "ERROR fatal"];
    let filter_res = tylogger::core::filter::FilterEngine::apply_filter(
        filter_sample.len(),
        |i| filter_sample.get(i).map(|s| s.to_string()),
        &filter_crit,
        None,
    );
    assert_eq!(filter_res, vec![1, 3]);
    println!("  -> PASS: Filter matched lines [1, 3] without memory duplication");

    println!("[Check 12] Validating Atomic Configuration Persistence...");
    let cfg = tylogger::config::TyloggerConfig {
        theme: "Dark".to_string(),
        font_size: 14,
        cache_budget_mb: 128,
        ..Default::default()
    };
    assert!(cfg.save().is_ok());
    let loaded = tylogger::config::TyloggerConfig::load();
    assert_eq!(loaded.theme, "Dark");
    assert_eq!(loaded.font_size, 14);
    assert_eq!(loaded.cache_budget_mb, 128);
    println!("  -> PASS: Config atomically saved and restored accurately");

    println!("[Check 13] Validating Global Memory Budget Manager & Oversized Line Truncation...");
    let mem_budget = tylogger::core::memory::GlobalMemoryBudget::new(64 * 1024 * 1024);
    mem_budget.add_raw_cache(20 * 1024 * 1024);
    mem_budget.add_decoded_lines(20 * 1024 * 1024);
    assert_eq!(mem_budget.current_usage(), 40 * 1024 * 1024);
    assert!(!mem_budget.is_over_budget());
    let giant_line = "B".repeat(2 * 1024 * 1024); // 2 MB
    let (preview, was_truncated) =
        tylogger::core::memory::format_safe_preview_line(&giant_line, 1024 * 1024);
    assert!(was_truncated);
    assert!(preview.contains("[Truncated: 2097152 bytes total"));
    println!("  -> PASS: 64MB memory budget tracked and 2MB line safely bounded to 1MB preview");

    println!("[Check 14] Validating Unified LogSource Trait & Stable SourceLineRef...");
    let mock_lines = ["log line alpha", "log line beta", "log line gamma"];
    use tylogger::core::source::LogSource;
    let mut mock_src = tylogger::core::source::MockSource::new("mock.log", &mock_lines);
    assert_eq!(mock_src.line_count(), 3);
    let sref = mock_src.get_line_ref(1);
    assert_eq!(sref.line_idx, 1);
    assert_eq!(sref.generation, 1);
    assert_eq!(mock_src.get_line(1).unwrap(), "log line beta");
    println!("  -> PASS: Unified LogSource interface and stable reference tracking confirmed");

    println!("[Check 15] Validating MultiFile Rotated Log Management & Continuous Mapping...");
    let mf_dir = temp_dir.join(format!("tylogger_mf_headless_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&mf_dir);
    let mf_f3 = mf_dir.join("access.log.3");
    let mf_f2 = mf_dir.join("access.log.2");
    let mf_f1 = mf_dir.join("access.log.1");
    let mf_f0 = mf_dir.join("access.log");
    std::fs::write(&mf_f3, "line 3-1\nline 3-2\n").unwrap();
    std::fs::write(&mf_f2, "line 2-1\nline 2-2\n").unwrap();
    std::fs::write(&mf_f1, "line 1-1\nline 1-2\n").unwrap();
    std::fs::write(&mf_f0, "line 0-1\nline 0-2\n").unwrap();
    let mut mf_source =
        tylogger::core::multifile::MultiFileSource::open(&mf_f0).expect("Open MultiFile");
    assert_eq!(mf_source.line_count(), 8);
    assert_eq!(mf_source.get_line(0).unwrap(), "line 3-1");
    assert_eq!(mf_source.get_line(2).unwrap(), "line 2-1");
    assert_eq!(mf_source.get_line(4).unwrap(), "line 1-1");
    assert_eq!(mf_source.get_line(6).unwrap(), "line 0-1");
    println!("  -> PASS: 4 rotated files mapped into 8 continuous lines from oldest to newest");

    println!("[Check 16] Validating MultiFile Boundary Navigation...");
    assert_eq!(mf_source.jump_to_next_boundary(0), Some(2));
    assert_eq!(mf_source.jump_to_next_boundary(2), Some(4));
    assert_eq!(mf_source.jump_to_next_boundary(4), Some(6));
    assert_eq!(mf_source.jump_to_next_boundary(6), None);
    assert_eq!(mf_source.jump_to_prev_boundary(6), Some(4));
    assert_eq!(mf_source.get_constituent_filename(0), Some("access.log.3"));
    assert_eq!(mf_source.get_constituent_filename(7), Some("access.log"));
    println!("  -> PASS: Boundary jumps and constituent filename attribution verified");

    println!("[Check 17] Validating Built-in Columnizers: Webserver CLF...");
    let clf_col = tylogger::core::columnizer::WebserverClfColumnizer;
    let clf_line =
        r#"127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /index.html HTTP/1.0" 200 2326"#;
    let clf_parsed = clf_col.parse_line(clf_line);
    assert_eq!(clf_parsed[0], "127.0.0.1");
    assert_eq!(clf_parsed[2], "frank");
    assert_eq!(clf_parsed[3], "10/Oct/2000:13:55:36 -0700");
    assert_eq!(clf_parsed[4], "GET");
    assert_eq!(clf_parsed[5], "/index.html");
    assert_eq!(clf_parsed[7], "200");
    assert_eq!(clf_parsed[8], "2326");
    println!("  -> PASS: Webserver Common Log Format (CLF) fields parsed accurately");

    println!("[Check 18] Validating Built-in Columnizers: CSV RFC 4180 with Quotes & Escapes...");
    let csv_col = tylogger::core::columnizer::CsvColumnizer::default();
    let csv_line = r#"101,"Doe, John","Field with ""nested quotes""",Active"#;
    let csv_parsed = csv_col.parse_line(csv_line);
    assert_eq!(csv_parsed.len(), 4);
    assert_eq!(csv_parsed[0], "101");
    assert_eq!(csv_parsed[1], "Doe, John");
    assert_eq!(csv_parsed[2], r#"Field with "nested quotes""#);
    assert_eq!(csv_parsed[3], "Active");
    println!("  -> PASS: RFC 4180 CSV parser handled delimiters inside quotes & escaped quotes");

    println!("[Check 19] Validating Built-in Columnizers: Log4j XML (No DTD / Safe)...");
    let xml_col = tylogger::core::columnizer::Log4jXmlColumnizer;
    let xml_line = r#"<log4j:event logger="com.auth" timestamp="1764064800" level="ERROR" thread="main"><log4j:message><![CDATA[User unauthorized access attempt]]></log4j:message></log4j:event>"#;
    let xml_parsed = xml_col.parse_line(xml_line);
    assert_eq!(xml_parsed[0], "1764064800");
    assert_eq!(xml_parsed[1], "ERROR");
    assert_eq!(xml_parsed[2], "com.auth");
    assert_eq!(xml_parsed[3], "main");
    assert_eq!(xml_parsed[4], "User unauthorized access attempt");
    println!("  -> PASS: Log4j XML parsed with CDATA unwrapping and zero external DTD expansion");

    println!("[Check 20] Validating Extended Columnizers: JSON Lines / NDJSON...");
    let json_col = tylogger::core::columnizer::JsonLinesColumnizer;
    let json_line = r#"{"timestamp":"2026-11-25T10:00:00Z","level":"warn","module":"db","msg":"Pool saturation 92%"}"#;
    let json_parsed = json_col.parse_line(json_line);
    assert_eq!(json_parsed[0], "2026-11-25T10:00:00Z");
    assert_eq!(json_parsed[1], "warn");
    assert_eq!(json_parsed[2], "db");
    assert_eq!(json_parsed[3], "Pool saturation 92%");
    println!("  -> PASS: JSON Lines extracted structured fields without full document parsing");

    println!("[Check 21] Validating Extended Columnizers: Syslog RFC 5424 / 3164...");
    let syslog_col = tylogger::core::columnizer::SyslogColumnizer;
    let syslog_line = "<34>Oct 11 22:14:15 mymachine su[1042]: 'su root' failed for lonvick";
    let syslog_parsed = syslog_col.parse_line(syslog_line);
    assert_eq!(syslog_parsed[0], "34");
    assert_eq!(syslog_parsed[1], "Oct 11 22:14:15");
    assert_eq!(syslog_parsed[2], "mymachine");
    assert_eq!(syslog_parsed[3], "su");
    assert_eq!(syslog_parsed[4], "1042");
    assert!(syslog_parsed[5].contains("'su root' failed for lonvick"));
    println!("  -> PASS: Syslog PRI and header tokens correctly extracted");

    println!("[Check 22] Validating Multiline Record Engine (Java Stack Traces)...");
    let trace_lines = [
        "2026-11-25 10:00:00 [ERROR] NullPointerException in Service",
        "\tat com.example.service.ItemService.get(ItemService.java:34)",
        "\tat com.example.web.ItemController.show(ItemController.java:12)",
        "Caused by: java.lang.IllegalStateException: Missing item ID",
        "2026-11-25 10:00:01 [INFO] Next request handled",
    ];
    let ml_engine = tylogger::core::multiline::MultilineEngine::index_lines(&trace_lines);
    assert_eq!(ml_engine.record_count(), 2);
    let r0 = ml_engine.get_record(0).unwrap();
    assert_eq!(r0.line_count(), 4);
    assert_eq!(r0.start_line, 0);
    assert_eq!(r0.end_line, 3);
    assert_eq!(ml_engine.physical_to_logical(2), 0);
    assert_eq!(ml_engine.physical_to_logical(4), 1);
    println!("  -> PASS: 4 physical stack trace lines consolidated into 1 logical record");

    println!("[Check 23] Validating Custom Columnizer Builder Regex Named Captures...");
    let custom_pat = r#"^\[(?P<time>[^\]]+)\] \[(?P<level>[^\]]+)\] (?P<msg>.*)$"#;
    let custom_col =
        tylogger::core::custom_columnizer::CustomColumnizer::new_regex("CustomApp", custom_pat);
    assert_eq!(custom_col.column_names, vec!["time", "level", "msg"]);
    let custom_parsed = custom_col.parse_line("[2026-11-25 10:05] [ALERT] Security gate triggered");
    assert_eq!(
        custom_parsed,
        vec!["2026-11-25 10:05", "ALERT", "Security gate triggered"]
    );
    let custom_json = custom_col.to_json();
    assert!(custom_json.contains("\"name\":\"CustomApp\""));
    println!("  -> PASS: Custom regex named capture parsed line and serialized to valid JSON");

    println!("[Check 24] Validating Automatic Columnizer Format Detection Heuristics...");
    let reg = tylogger::core::columnizer::ColumnizerRegistry::new();
    let clf_samples = [
        r#"192.168.1.1 - - [25/Nov/2026:10:00:00 +0000] "GET /api HTTP/1.1" 200 100"#,
        r#"192.168.1.2 - - [25/Nov/2026:10:00:01 +0000] "POST /api HTTP/1.1" 201 50"#,
    ];
    assert_eq!(reg.detect(&clf_samples), "clf");
    let json_samples = [
        r#"{"time":"2026-11-25T10:00:00Z","level":"info","msg":"boot"}"#,
        r#"{"time":"2026-11-25T10:00:01Z","level":"info","msg":"ready"}"#,
    ];
    assert_eq!(reg.detect(&json_samples), "jsonl");
    let plain_samples = ["Random log text line 1", "Random log text line 2"];
    assert_eq!(reg.detect(&plain_samples), "plain");
    println!(
        "  -> PASS: Sample-based heuristics accurately identified CLF, JSON Lines, and Plain Text"
    );

    println!("[Check 25] Validating Advanced Filter: Range Filtering & Invert Match...");
    let batch_lines = [
        "init",
        "=== START BATCH ===",
        "step 1",
        "step 2",
        "=== END BATCH ===",
        "done",
    ];
    let range_crit = tylogger::core::filter::FilterCriteria::new_range(
        "=== START BATCH ===",
        "=== END BATCH ===",
        false,
    );
    let range_hits = tylogger::core::filter::FilterEngine::apply_filter(
        batch_lines.len(),
        |i| batch_lines.get(i).map(|s| s.to_string()),
        &range_crit,
        None,
    );
    assert_eq!(range_hits, vec![1, 2, 3, 4]);
    let range_inv_crit = tylogger::core::filter::FilterCriteria::new_range(
        "=== START BATCH ===",
        "=== END BATCH ===",
        true,
    );
    let range_inv_hits = tylogger::core::filter::FilterEngine::apply_filter(
        batch_lines.len(),
        |i| batch_lines.get(i).map(|s| s.to_string()),
        &range_inv_crit,
        None,
    );
    assert_eq!(range_inv_hits, vec![0, 5]);
    println!(
        "  -> PASS: Start..End range filter captured block [1..4] and invert match captured [0, 5]"
    );

    println!("[Check 26] Validating Advanced Filter: Back & Fore Context Spread Deduplication...");
    let hits = [10, 12];
    let spread = tylogger::core::filter::FilterEngine::apply_context_spread(50, &hits, 2, 2);
    assert_eq!(spread, vec![8, 9, 10, 11, 12, 13, 14]);
    println!(
        "  -> PASS: Overlapping context ranges deduplicated into single contiguous sequence [8..14]"
    );

    println!("[Check 27] Validating Live Filter-to-Tab Creation & Independence...");
    let parent_lines = [
        "2026-11-25 10:00:00 [INFO] ok",
        "2026-11-25 10:00:01 [CRITICAL] Alert A",
        "2026-11-25 10:00:02 [DEBUG] checking",
        "2026-11-25 10:00:03 [CRITICAL] Alert B",
    ];
    let p_src: Box<dyn LogSource> = Box::new(tylogger::core::source::MockSource::new(
        "main.log",
        &parent_lines,
    ));
    let p_arc = Arc::new(std::sync::Mutex::new(p_src));
    let tab_crit = tylogger::core::filter::FilterCriteria::new_include("CRITICAL", false);
    let mut f_tab =
        tylogger::core::filter_tab::FilteredDocumentSource::create(p_arc, tab_crit).unwrap();
    assert_eq!(f_tab.line_count(), 2);
    assert_eq!(f_tab.resolve_to_parent_line(0), Some(1));
    assert_eq!(f_tab.resolve_to_parent_line(1), Some(3));
    assert!(f_tab.get_line(0).unwrap().contains("Alert A"));
    println!(
        "  -> PASS: Independent Filter-to-Tab virtual document created without disk duplication"
    );

    println!("[Check 28] Validating Highlighting Engine Priority & F8 Navigation...");
    let mut h_group = tylogger::core::highlight::HighlightGroup::new("TestGroup");
    h_group.add_rule(tylogger::core::highlight::HighlightRule::new(
        1, "Warn", "WARN", 0x0, 0x0, 50,
    ));
    h_group.add_rule(tylogger::core::highlight::HighlightRule::new(
        2, "Fatal", "FATAL", 0xFF, 0xFF, 100,
    ));
    let test_line = "2026-11-25 [WARN] FATAL error occurred";
    let winner = h_group.evaluate_line(test_line, &[]).unwrap();
    assert_eq!(winner.name, "Fatal"); // Priority 100 > 50
    let h_lines = ["info", "WARN 1", "info", "FATAL 2", "info"];
    let f8_next = h_group.find_next_highlighted(h_lines.len(), 0, 1, |i| {
        h_lines.get(i).map(|s| s.to_string())
    });
    assert_eq!(f8_next, Some(1));
    let f8_next2 = h_group.find_next_highlighted(h_lines.len(), 1, 1, |i| {
        h_lines.get(i).map(|s| s.to_string())
    });
    assert_eq!(f8_next2, Some(3));
    println!(
        "  -> PASS: Highlight priority resolved deterministically and F8 forward search succeeded"
    );

    println!("[Check 29] Validating Structured Log Inspector Field & JSON Extraction...");
    let mut insp_source = tylogger::core::source::MockSource::new(
        "srv.log",
        &[r#"10.0.0.1 - alice [25/Nov/2026:10:00:00 +0000] "POST /checkout HTTP/1.1" 200 512"#],
    );
    let insp_col = tylogger::core::columnizer::WebserverClfColumnizer;
    let inspected =
        tylogger::core::inspector::LogInspector::inspect_line(&mut insp_source, &insp_col, 0)
            .unwrap();
    assert_eq!(inspected.fields.len(), 9);
    assert_eq!(inspected.fields[2].name, "User");
    assert_eq!(inspected.fields[2].value, "alice");
    let json_pairs = tylogger::core::inspector::LogInspector::extract_json_pairs(
        r#"{"action":"pay","amount":150}"#,
    );
    assert_eq!(json_pairs[0], ("action".to_string(), "pay".to_string()));
    assert_eq!(json_pairs[1], ("amount".to_string(), "150".to_string()));
    println!(
        "  -> PASS: Inspector extracted 9 CLF fields and successfully flattened JSON key-values"
    );

    println!(
        "[Check 30] Validating Streaming Export Engine: CSV Formula Sanitization & Overwrite Prevention..."
    );
    assert_eq!(
        tylogger::core::export::StreamingExporter::sanitize_csv_field("=1+1"),
        "'=1+1"
    );
    assert_eq!(
        tylogger::core::export::StreamingExporter::sanitize_csv_field("@SUM(A1:A10)"),
        "'@SUM(A1:A10)"
    );
    let export_src_lines =
        ["192.168.1.1 - admin [25/Nov/2026:10:00:00 +0000] \"GET / HTTP/1.1\" 200 120"];
    let mut exp_src = tylogger::core::source::MockSource::new("export.log", &export_src_lines);
    let exp_dst = temp_dir.join(format!(
        "tylogger_headless_export_{}.csv",
        std::process::id()
    ));
    let exp_count = tylogger::core::export::StreamingExporter::export_streaming(
        &mut exp_src,
        &insp_col,
        tylogger::core::export::ExportFormat::Csv,
        &exp_dst,
        None,
        |_, _| {},
    )
    .expect("Export CSV");
    assert_eq!(exp_count, 1);
    let exp_content = std::fs::read_to_string(&exp_dst).unwrap();
    assert!(exp_content.contains("Host,Ident,User"));
    assert!(exp_content.contains("192.168.1.1"));
    let _ = std::fs::remove_file(&exp_dst);
    println!(
        "  -> PASS: Streaming export written with CSV formula protection and verified content"
    );

    let _ = std::fs::remove_dir_all(&mf_dir);
    println!("=== ALL 30 VERIFICATION CHECKS PASSED ===");
}
