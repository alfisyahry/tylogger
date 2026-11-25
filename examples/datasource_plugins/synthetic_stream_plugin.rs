//! Example 2: Synthetic Live Event Stream Data Source Plugin
//! Demonstrates an active streaming log source that emits continuous synthetic events.

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--info") {
        println!("Tylogger Synthetic Live Event Stream Plugin v1.0 (Continuous DataSource)");
        return Ok(());
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut line = String::new();
    while stdin.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.contains("\"ds_open\"") {
            let resp = "{\"type\":\"ds_open_ack\",\"source_id\":102,\"initial_lines\":10,\"is_continuous\":true}\n";
            stdout.write_all(resp.as_bytes())?;
            stdout.flush()?;
        } else if trimmed.contains("\"ds_read_line\"") {
            let resp = "{\"type\":\"ds_read_line_ack\",\"line_idx\":0,\"text\":\"2026-11-25 10:00:00 [STREAM] Synthetic telemetry event batch\\n\"}\n";
            stdout.write_all(resp.as_bytes())?;
            stdout.flush()?;
        } else if trimmed.contains("\"ds_close\"") {
            break;
        }
        line.clear();
    }

    Ok(())
}
