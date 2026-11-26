# Tylogger Windows Installer & Deployment Guide
## Inno Setup Native Packaging & Enterprise Silent Deployment Specification

**Target Milestone:** 1.0.0 GA  
**Script Path:** `installer/tylogger.iss`  
**Output Executable:** `dist/Tylogger-1.0.0-windows-setup.exe`  

---

## 1. Overview & Dual Installation Modes

Tylogger utilizes a native Inno Setup package that natively supports both standard user environments and managed enterprise workstations:

### 1.1 Per-User Installation (Default - No Admin / UAC Required)
- **Target Folder:** `%LOCALAPPDATA%\Programs\Tylogger`
- **Privileges:** `PrivilegesRequired=lowest`
- **Use Case:** Standard developer workstations, restricted enterprise environments without local administrator privileges.
- **Shortcuts:** Created under current user's Start Menu and desktop (optional).
- **Registry:** Written to `HKEY_CURRENT_USER\Software\Classes`.

### 1.2 All-Users Installation (Elevated - Administrator Privileges)
- **Target Folder:** `%ProgramFiles%\Tylogger`
- **Privileges:** Invoked via UAC elevation dialog or `/ALLUSERS` command-line switch.
- **Use Case:** Shared laboratory machines, multi-user terminal servers, centralized system administration.
- **Shortcuts:** Created under `C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Tylogger`.
- **Registry:** Written to `HKEY_LOCAL_MACHINE\Software\Classes`.

---

## 2. Enterprise Silent Installation Parameters

For automated deployment via Microsoft Intune, SCCM, PDQ Deploy, or PowerShell scripts, the installer supports standard non-interactive flags:

```powershell
# Standard fully silent installation (no GUI, suppresses all message boxes, prevents auto-restart)
.\Tylogger-1.0.0-windows-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART

# Silent installation with custom target directory
.\Tylogger-1.0.0-windows-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /DIR="C:\Tools\Tylogger"

# Silent installation with detailed logging for enterprise verification
.\Tylogger-1.0.0-windows-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG="C:\Temp\tylogger_install.log"
```

### Supported Parameters Summary:
- `/SILENT`: Shows only the installation progress window without wizard steps.
- `/VERYSILENT`: Completely headless installation with zero windows or taskbar items.
- `/SUPPRESSMSGBOXES`: Automatically answers default to any interactive prompts.
- `/NORESTART`: Prevents any system reboot requests.
- `/DIR="x:\path"`: Overrides default installation directory.
- `/LOG="x:\path.log"`: Records verbose installation actions and registry entries.
- `/CURRENTUSER` or `/ALLUSERS`: Forces specific installation scope.

---

## 3. Single-Instance Running Application Detection (`AppMutex`)

To prevent file-in-use errors or binary corruption when upgrading an active instance:
- The installer defines `AppMutex=TyloggerSingleInstanceMutex`.
- The `tylogger.exe` runtime creates this named mutex upon starting the Win32 GUI message loop.
- If Tylogger is running when the installer or uninstaller runs:
  - Interactive mode displays a friendly dialog prompting the user to save open sessions and close the application.
  - Silent mode waits or retries cleanly, avoiding ungraceful terminations.

---

## 4. Safe Uninstallation Guarantees (RSK-30)

Uninstallation safety is governed by strict non-destructive policies:
1. **Application Files Only:** The uninstaller only deletes files originally registered by the installation engine inside `{app}` (`tylogger.exe`, documentation, and sample templates).
2. **User Data Preservation:**
   - User configuration in `%APPDATA%\Tylogger\config.json` is **never deleted**.
   - User session history, crash recovery snapshots, and workspace files are **never deleted**.
   - Any log files being monitored, analyzed, or stored in `{app}\logs` or user directories remain **100% untouched**.
3. **Clean Registry Deregistration:** Document associations (`.tysession`, `.tyworkspace`) and context menu handlers (`SystemFileAssociations`) are cleanly deregistered without leaving orphaned keys.

