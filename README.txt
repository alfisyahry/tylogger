Tylogger v1.0.0 GA - High-Performance Native Windows Log Analysis Tool
======================================================================
Architecture: Native Win32/GDI, Rust core, 64-bit sparse index engine.
Supported OS: Windows 7 SP1 (with KB2533623), Windows 8/8.1, Windows 10, Windows 11 (x86 & x64).

Features:
- Non-locking live log reader (FILE_SHARE_READ|WRITE|DELETE)
- High-speed 64-bit sparse index for multi-gigabyte log files (>4GB)
- Linear-time DFA search engine with ReDoS immunity
- Modular columnizers (CLF, CSV, XML, JSON Lines, Syslog RFC 5424)
- Virtual Filter-to-Tab with micro-batching
- Time synchronization and multi-document sync loop guards
- Windows DPAPI credential vault for secure remote access

Portable Mode:
When 'portable.flag' is present in this directory, Tylogger stores all settings,
preferences, and bookmark files locally in './config/' without modifying the Windows registry.
