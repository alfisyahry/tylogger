use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    let mut header = [0u8; 4];

    loop {
        if stdin.read_exact(&mut header).is_err() {
            break;
        }

        let len = u32::from_le_bytes(header) as usize;
        let mut buf = vec![0u8; len];
        stdin.read_exact(&mut buf)?;

        let req_str = String::from_utf8_lossy(&buf);

        if req_str.contains("\"handshake\"") {
            let resp = "{\"type\":\"handshake_ack\",\"plugin_name\":\"Standalone IPC Parser\",\"columns\":[\"Field1\",\"Field2\",\"Field3\"]}";
            let r_len = resp.len() as u32;
            stdout.write_all(&r_len.to_le_bytes())?;
            stdout.write_all(resp.as_bytes())?;
            stdout.flush()?;
        } else if req_str.contains("\"parse_line\"") {
            let resp = "{\"type\":\"parse_response\",\"line_idx\":0,\"columns\":[\"TokenA\",\"TokenB\",\"TokenC\"]}";
            let r_len = resp.len() as u32;
            stdout.write_all(&r_len.to_le_bytes())?;
            stdout.write_all(resp.as_bytes())?;
            stdout.flush()?;
        } else if req_str.contains("\"shutdown\"") {
            break;
        }
    }

    Ok(())
}
