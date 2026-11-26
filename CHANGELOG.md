# Tylogger Changelog

All notable changes to this project are documented in this file.
The format is based on Keep a Changelog, adhering to Semantic Versioning.

---

## [1.0.0] - 2026-11-26 (General Availability)

### Added
- **Universal Binary Architecture:** Unified MSVC static CRT compilation with Windows subsystem version 6.01, allowing single universal executables for both x86 (PE32) and x64 (PE32+) across Windows 7 SP1 through Windows 11.
- **Portable Distribution Packages:** Automated creation of standalone portable zip archives (`Tylogger-1.0.0-windows-x86-portable.zip` and `Tylogger-1.0.0-windows-x64-portable.zip`) with `portable.flag` detection for zero-registry footprint.
- **Native Windows Inno Setup Installer (`installer/tylogger.iss`):** Dual install modes supporting per-user (`%LOCALAPPDATA%\Programs\Tylogger` without UAC) and all-users (`%ProgramFiles%\Tylogger` elevated).
- **Application Instance Mutex (`AppMutex`):** Single-instance detection with `TyloggerSingleInstanceMutex` to protect active running sessions during installation and upgrades (RSK-31).
- **Non-Intrusive Shell Integration:** Optional file associations for `.tysession` and `.tyworkspace` and explorer context menu integration ("Open with Tylogger") for `.log` and `.txt` via `SystemFileAssociations` without hijacking default text handlers.
- **Safe Uninstallation (RSK-30):** Guaranteed preservation of user configuration (`%APPDATA%\Tylogger`), bookmarks, and monitored log files upon software removal.
- **Authenticode Signing Specification (`docs/CODE_SIGNING.md`):** Complete Microsoft SignTool workflow with SHA-256 digest and RFC 3161 compliant timestamping.
- **Package Integrity Test Harness:** Deterministic validation script (`scripts/test_package_integrity.ps1`) verifying archive structure, hashes, and security flags.

### Changed
- Bumped version to `1.0.0 GA` across `Cargo.toml`, resource manifest `tylogger.rc`, `src/gui/dialogs.rs`, `src/core/session.rs`, and CLI usage banners.
- Reconciled all 40 Acceptance Gates (AG-01 through AG-40) in `docs/ACCEPTANCE_TRACEABILITY.md`.
- Expanded Security Risk Register to 32 entries (RSK-01 through RSK-32) covering installer and packaging attack vectors.

---

## [0.9.0] - 2026-11-25 (Release Candidate)

### Added
- Quad-target distribution artifacts in `dist/` (`legacy-x86`, `legacy-x64`, `modern-x86`, `modern-x64`) with cryptographic `build-manifest.json`.
- Windows 7 SP1 runtime compatibility shims in `src/core/compat.rs` for `WaitOnAddress`, CryptoAPI fallback, and `SetDefaultDllDirectories`.
- Bounded byte-accounting ring buffer (`BoundedStreamStorage`) enforcing `max_memory_bytes` with FIFO eviction (RSK-27).
- Tri-tier memory profile switcher: Low Memory (64MB), Balanced (256MB), High Performance (1024MB).
- Virtual FilterTab micro-batch critical sections narrowing mutex lock duration < 2ms.
- Reusable GDI brush caching in `LogGrid::draw` eliminating redraw handle churn.
- 64-bit sparse index verification across the 4GB boundary.

---

## [0.8.0] - 2026-11-25 (Feature Parity & Hardening)

### Added
- Complete parity with upstream LogExpert across 18 functional categories (74 verified features, 18 extensions).
- Full Win32 command integration covering all 48 dispatch commands.
- Elimination of placeholder code, stubs, and mocks across production source trees.
- Comprehensive security risk register mitigating RSK-01 through RSK-25.

---

## [0.7.0] - 2026-11-25 (Remote Logs, Archives & Plugins)
- SSH/SFTP streaming reader with DPAPI credential vault.
- Strict host-key verification and MitM abort logic.
- Compressed GZIP and ZIP archive stream reading with anti-zip-bomb and path-traversal guards.
- Safe legacy session import (`.lxp`/`.lxj`) with strict anti-XXE defense.

---

## [0.6.0] - 2026-11-25 (Advanced Streaming Analytics)
- HyperLogLog cardinality estimation, Space-Saving Top-K, and Count-Min sketch.
- Rolling anomaly detection and offline HTML analytics export.

---

## [0.5.0] - 2026-11-25 (Time Intelligence & Automation)
- Integer nanoseconds timestamp model and timeshift synchronization.
- Multi-document sync loop guards and automated incoming-only triggers.

---

## [0.4.0] - 2026-11-25 (Unified Search & ReDoS Defense)
- Linear-time DFA regular expression engine immune to catastrophic backtracking.
- Damerau-Levenshtein fuzzy search and atomic session persistence.

---

## [0.3.0] - 2026-11-25 (Advanced Filter & Highlighting)
- Virtual Filter-to-Tab creation and context spread deduplication.
- WCAG AAA contrast-compliant highlight rendering.

---

## [0.2.0] - 2026-11-25 (Multi-Document & Sparse Index)
- Multi-tab document workspace and 64-bit sparse checkpoint indexing.
- Modular columnizers (CLF, CSV, XML, JSON Lines, Syslog).

---

## [0.1.0] - 2026-11-25 (Non-Locking File Engine)
- High-performance non-locking reader (`FILE_SHARE_READ|WRITE|DELETE`).
- Real-time tailing and automated log rotation detection.

---

## [0.0.1] - 2026-11-25 (Win32 Native PoC)
- Double-buffered virtual GDI log grid rendering at 60 FPS.
