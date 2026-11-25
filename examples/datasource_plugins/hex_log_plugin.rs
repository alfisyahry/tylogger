//! Example 1: Hex Log Decoder Data Source Plugin
//! Demonstrates a custom file-source adapter that decodes raw binary data into formatted hex/ASCII log lines.

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--info") {
        println!("Tylogger Hex Log Decoder Plugin v1.0 (DataSource)");
        return Ok(());
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut line = String::new();
    while stdin.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.contains("\"ds_open\"") {
            // Respond with OpenAck
            let resp = "{\"type\":\"ds_open_ack\",\"source_id\":101,\"initial_lines\":64,\"is_continuous\":false}\n";
            stdout.write_all(resp.as_bytes())?;
            stdout.flush()?;
        } else if trimmed.contains("\"ds_read_line\"") {
            // Format line as Hex dump
            let resp = "{\"type\":\"ds_read_line_ack\",\"line_idx\":0,\"text\":\"00000000: 48 65 78 4C 6F 67 20 50 61 79 6C 6F 61 64 21 0A  HexLog Payload!.\\n\"}\n";
            stdout.write_all(resp.as_bytes())?;
            stdout.flush()?;
        } else if trimmed.contains("\"ds_close\"") {
            break;
        }
        line.clear();
    }

    Ok(())
}
