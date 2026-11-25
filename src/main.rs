use std::env;
use std::path::{Path, PathBuf};

use tylogger::core::NonLockingLogReader;
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

    let reader = match NonLockingLogReader::open(&target_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[Error] {}", e);
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
            NonLockingLogReader::from_mock_lines(&mock_lines)
        }
    };

    let state = AppState::new(reader);
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

    println!("=== ALL 7 VERIFICATION CHECKS PASSED ===");
}
