# Tylogger Out-of-Process IPC Protocol Specification
Version: 1.0

## Transport Layer
- Communication occurs via standard anonymous pipes (stdin/stdout) spawned by Tylogger.
- Windows Job Objects constrain child processes with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
- Tylogger enforces a watchdog timeout (default: 3000 ms) per request.

## Framing
Every IPC message is encoded as:
1. `length`: 4 bytes (u32 little-endian) representing payload byte count.
2. `payload`: UTF-8 JSON text.

## Message Types

### 1. Handshake (Tylogger -> Plugin)
```json
{
  "type": "handshake",
  "api_version": "1.0",
  "plugin_id": "com.example.parser"
}
```

### 2. Handshake Acknowledgment (Plugin -> Tylogger)
```json
{
  "type": "handshake_ack",
  "plugin_name": "My Custom Parser",
  "columns": ["Timestamp", "Severity", "Message"]
}
```

### 3. Parse Line Request (Tylogger -> Plugin)
```json
{
  "type": "parse_line",
  "line_idx": 105,
  "line": "2026-11-25 10:00:00 [ERROR] Out of memory"
}
```

### 4. Parse Line Response (Plugin -> Tylogger)
```json
{
  "type": "parse_response",
  "line_idx": 105,
  "columns": ["2026-11-25 10:00:00", "ERROR", "Out of memory"]
}
```

### 5. Shutdown (Tylogger -> Plugin)
```json
{
  "type": "shutdown"
}
```
