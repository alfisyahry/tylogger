# Tylogger Plugin SDK (Version 1.0)
## Developing Extensible Plugins for Tylogger

Welcome to the Tylogger Plugin SDK. Tylogger supports two modular plugin mechanisms:

1. **Declarative Columnizer Packages (In-Process Configuration):**
   - Configured via `manifest.json` and `columnizer.json`.
   - Supports delimited logs (CSV/TSV/custom delimiter), regular expression capture groups, and JSON Lines field mappings.
   - Evaluated purely inside Tylogger with zero binary execution and no external executable launch; input validation is still necessary.

2. **Trusted Executable Plugins (Out-of-Process, Windows Job Objects):**
   - Standalone binaries communicating over local Anonymous Pipes using length-prefixed JSON frames.
   - Isolated inside Windows Job Objects with strict RAM/CPU limits and automatic cleanup (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
   - Requires explicit user approval before execution.

---

## Directory Structure

```
sdk/
├── README.md                      # This guide
├── schemas/
│   ├── plugin-manifest.schema.json # JSON Schema for manifest.json
│   └── columnizer.schema.json     # JSON Schema for columnizer.json
├── protocol/
│   └── IPC_PROTOCOL.md            # Out-of-process IPC protocol specification
└── examples/
    ├── declarative-apache/        # Working declarative Apache Combined parser
    ├── custom-json-mapper/        # Working declarative JSON Lines field extractor
    └── executable-columnizer/     # Working standalone Rust IPC plugin host
```

---

## Plugin Discovery Paths

Tylogger automatically scans for plugin folders in:
- `%APPDATA%\Tylogger\plugins\` (Installed mode)
- `.\plugins\` (Portable mode relative to `tylogger.exe`)

Each plugin must reside in its own subdirectory containing `manifest.json`.
