# Tylogger Session Format & Legacy Migration Specification

This document defines the normative JSON schema for Tylogger session workspaces (Schema v3.0) and specifies the security and mapping semantics when migrating legacy LogExpert XML session files (`.lxp` / `.lxj`).

---

## 1. Session Schema v3.0 (Native JSON Format)

Tylogger sessions are serialized as UTF-8 encoded JSON with atomic write guarantees (writing to a temporary file before atomic renaming) to prevent corruption during system crashes.

```json
{
  "version": 3,
  "documents": [
    {
      "source_type": "LocalFile",
      "canonical_path": "C:\\logs\\app.log",
      "columnizer_name": "Webserver CLF",
      "bookmarks": [42, 108, 512],
      "active_line": 108,
      "follow_tail": true
    },
    {
      "source_type": "SftpFile",
      "canonical_path": "/var/log/syslog",
      "profile_id": "prod_web_01",
      "host_fingerprint": "SHA256:dGhpc2lzYXNhZmVmaW5nZXJwcmludGtleWZvcjIwMjY=",
      "columnizer_name": "Syslog RFC 5424",
      "bookmarks": [12],
      "active_line": 50,
      "follow_tail": true
    }
  ],
  "active_tab_index": 0,
  "highlight_groups": [
    {
      "name": "Server Errors",
      "rules": [
        {
          "id": 1,
          "name": "FATAL",
          "pattern": "FATAL",
          "is_regex": false,
          "case_sensitive": true,
          "fg_color": 16777215,
          "bg_color": 136,
          "bold": true,
          "priority": 100,
          "marker_bar": true,
          "enabled": true
        }
      ]
    }
  ],
  "filter_presets": [
    {
      "name": "Errors Only",
      "pattern": "ERR",
      "is_regex": false,
      "case_sensitive": false,
      "include": true
    }
  ],
  "analytics_workspace_open": false,
  "active_analytics_query": null,
  "analytics_preset_id": null
}
```

---

## 2. Legacy LogExpert Migration (.lxp / .lxj)

Legacy LogExpert session files use an XML schema storing file paths, bookmarks, columnizers, and external tool definitions.

### 2.1 Anti-XXE Defense (RSK-10)
When parsing `.lxp` and `.lxj` files:
- External DTDs (`<!DOCTYPE`) and entity references (`<!ENTITY`) are strictly rejected with an immediate error (`XXE Hazard: External entities prohibited`).
- XML parsing is handled using safe non-evaluating streaming techniques.

### 2.2 Component Mapping
1. **Documents (`<fileList>` / `<file>`):**
   - File paths are validated and resolved into `SessionDocument` entries.
   - Associated columnizers are matched against Tylogger's built-in columnizers (`Webserver CLF`, `CSV RFC 4180`, `Log4j XML`, `JSON Lines`, `Syslog`).
2. **Bookmarks (`<bookmarkList>` / `<bookmark>`):**
   - 0-based and 1-based line index offsets are normalized to Tylogger's 0-based 64-bit line indexing.
3. **Highlight Rules (`<highlightGroup>`):**
   - Rules are imported with descending priority order to preserve LogExpert's top-first precedence.
   - Patterns containing `*`, `^`, or `$` are automatically flagged as regular expressions.
4. **External Tools (`<externalTool>` - RSK-11):**
   - Imported tool configurations are flagged as **Untrusted** (`is_trusted: false`) and **Disabled** by default.
   - Users must explicitly inspect and trust migrated external tool commands before execution to prevent malicious command execution from untrusted session files.
