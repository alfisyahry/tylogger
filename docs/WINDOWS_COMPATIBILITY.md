# Windows Compatibility & Toolchain Guide
## Windows 7 SP1 through Windows 11 (x86 & x64)

**Core Mandate:** Guarantee flawless execution on Windows 7 SP1 (Kernel NT 6.1) up to Windows 11 without runtime errors, missing DLLs, or unsupported API entry points.

---

## 1. Supported Platform Matrix

| Operating System | NT Version | Architectures | Minimum Service Pack | Validation Method |
|---|---|---|---|---|
| Windows 7 | NT 6.1 | x86 (32-bit), x64 (64-bit) | Service Pack 1 (SP1) | PE Import Audit + NT 6.1 SDK Header Check |
| Windows 8 / 8.1 | NT 6.2 / 6.3 | x86, x64 | RTM / Update 1 | Backward compatible with Win7 subset |
| Windows 10 | NT 10.0 | x86, x64 | All builds (1507 to 22H2) | Direct Execution + DPI Scaling Check |
| Windows 11 | NT 10.0 | x64 | All builds | Host Development Environment (Verified) |

---

## 2. Prohibited Windows 8+ APIs (Win7 SP1 Hazard List)

Binary executables targeting Windows 7 SP1 will fail immediately at startup with dialogs like `"The procedure entry point ... could not be located in dynamic link library KERNEL32.dll"` if any of the following functions are linked:

| Prohibited API | Introduced In | Reason / Hazard | Safe Win7 SP1 Replacement |
|---|---|---|---|
| `GetSystemTimePreciseAsFileTime` | Windows 8 | High-precision time call used by modern runtimes | `GetSystemTimeAsFileTime` + `QueryPerformanceCounter` |
| `WaitOnAddress` / `WakeByAddress*` | Windows 8 | Futex-like synchronization API | Win32 Events (`CreateEventW`, `WaitForSingleObject`) or CriticalSection |
| `SetThreadDescription` | Windows 10 | Thread naming for debuggers | No-op or MSVC structured exception hack |
| `CreateFile2` | Windows 8 | Modern file creation wrapper | `CreateFileW` with standard Win32 flags |
| `VirtualAlloc2` | Windows 10 | Advanced memory mapping | `VirtualAlloc` / `CreateFileMappingW` |
| `api-ms-win-*` | Windows 8.1+ | Dynamic API sets (MinWin redirection) | Direct imports from `kernel32.dll`, `user32.dll`, `gdi32.dll` |
| `kernelbase.dll` | Windows 7 (internal) | Not intended for direct client linking | Link against `kernel32.dll` only |

---

## 3. Rust Toolchain & Linker Configuration

### 3.1 Cargo & Rust Profile (`Cargo.toml`)
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 3.2 Static C Runtime (CRT) Linking
To prevent dependencies on missing Visual C++ Redistributable DLLs (`MSVCP140.dll`, `VCRUNTIME140.dll`) on vanilla Windows 7 SP1 installations, compile with static CRT linking:
```
RUSTFLAGS="-C target-feature=+crt-static"
```

### 3.3 Subsystem Versioning
The Windows PE header specifies the minimum required subsystem version. For Windows 7 SP1:
- Major Subsystem Version: `6`
- Minor Subsystem Version: `1`

---

## 4. Import Address Table (IAT) Verification Procedure

To audit any generated binary for Windows 7 SP1 compliance:

```powershell
& "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.34.31933\bin\Hostx64\x64\dumpbin.exe" /imports target\release\logexpert-rust.exe
```

**Acceptance Criteria for IAT Audit:**
1. Imported DLLs must be restricted to standard Windows 7 core libraries:
   - `KERNEL32.dll`
   - `USER32.dll`
   - `GDI32.dll`
   - `COMCTL32.dll` (v6 enabled via manifest)
   - `SHELL32.dll`
   - `ADVAPI32.dll`
2. Zero imports from `api-ms-win-core-*` or `api-ms-win-crt-*` unless resolved by static CRT.
3. Zero unresolved symbols when cross-referenced against the Windows 7 SP1 SDK symbol manifest.
