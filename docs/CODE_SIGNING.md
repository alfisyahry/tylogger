# Tylogger Production Code Signing Specification
## Authenticode Digital Signatures & RFC 3161 Timestamping Guide (RSK-32)

**Target Milestone:** 1.0.0 GA  
**Security Standard:** Authenticode (SHA-256 Digest + RFC 3161 Compliant Timestamping)  
**Applicability:** `dist/*/tylogger.exe`, `dist/*-setup.exe`  

---

## 1. Objectives & Compliance

To ensure seamless execution across all supported Windows versions (Windows 7 SP1 through Windows 11) and maintain enterprise SmartScreen reputation:
1. **Binary Authenticity:** Guarantee that the executable has not been altered or tampered with since compilation.
2. **Publisher Verification:** Provide verifiable identity attribution to authorized Tylogger release managers.
3. **Long-Term Validation:** Use RFC 3161 timestamping so that signatures remain valid even after certificate expiration.
4. **Policy RSK-32 Compliance:** Strict prohibition of fake self-signed certificates falsely claiming trusted publisher status.

---

## 2. Prerequisites & Tooling

- **Tool:** Microsoft SignTool (`signtool.exe`), included with Windows 10/11 SDK.
- **Certificate:** Valid Sectigo, DigiCert, or SSL.com Code Signing Certificate (Hardware Token EV or Cloud HSM recommended).
- **Digest Algorithm:** `SHA256` (SHA-1 is deprecated and rejected by modern Windows).
- **Timestamp Server:** RFC 3161 compliant server (e.g., `http://timestamp.digicert.com` or `http://timestamp.sectigo.com`).

---

## 3. Signing Procedure (Standard Release Workflow)

### 3.1 Step 1: Sign Binaries Before Packaging
Both 32-bit and 64-bit universal executables must be signed prior to building installer and portable archives:

```powershell
# Sign 64-bit release binary
signtool.exe sign /fd SHA256 `
    /sha1 <CERT_THUMBPRINT> `
    /tr http://timestamp.digicert.com /td SHA256 `
    /d "Tylogger Log Analyzer (x64)" `
    /du "https://github.com/LogExperts/LogExpert" `
    dist\universal-x64\tylogger.exe

# Sign 32-bit release binary
signtool.exe sign /fd SHA256 `
    /sha1 <CERT_THUMBPRINT> `
    /tr http://timestamp.digicert.com /td SHA256 `
    /d "Tylogger Log Analyzer (x86)" `
    /du "https://github.com/LogExperts/LogExpert" `
    dist\universal-x86\tylogger.exe
```

### 3.2 Step 2: Sign Inno Setup Uninstaller & Installer Package
Inno Setup supports signing the built-in uninstaller during compilation (`SignTool=...`) as well as the final setup executable:

```powershell
# Sign the installer executable
signtool.exe sign /fd SHA256 `
    /sha1 <CERT_THUMBPRINT> `
    /tr http://timestamp.digicert.com /td SHA256 `
    /d "Tylogger Setup" `
    /du "https://github.com/LogExperts/LogExpert" `
    dist\Tylogger-1.0.0-windows-setup.exe
```

### 3.3 Step 3: Verification
Verify signature validity, chain of trust, and timestamp:

```powershell
signtool.exe verify /pa /all /v dist\universal-x64\tylogger.exe
signtool.exe verify /pa /all /v dist\Tylogger-1.0.0-windows-setup.exe
```

---

## 4. Development Environment Policy (RSK-32)

In accordance with Security Risk RSK-32:
- Local development, continuous integration, and staging builds remain **unsigned** or rely on cryptographic SHA-256 build manifests (`build-manifest.json`).
- Under no circumstances shall developers inject unauthorized or self-signed test certificates claiming public authority.
- Official code signing is reserved for final staging under authenticated release credentials.
