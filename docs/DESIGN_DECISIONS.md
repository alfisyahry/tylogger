# Tylogger — Design Decisions & Deviations Log

This document records architectural, UX, and design system decisions and any documented deviations from `docs/DESIGN.md`.

## Decision Log

### DEC-001: Adoption of Normative Design System Baseline (Phase 1)
- **Date**: 2026-11-25
- **Context**: User provided `DESIGN -1-.md` as the authoritative design system and UX specification across all phases.
- **Decision**: Adopted `docs/DESIGN.md` as the permanent normative baseline.
- **Scope**:
  - Technical rebranding to `Tylogger` (`tylogger.exe`).
  - Semantic theme token architecture established in `gui::theme` (Light, Dark, Soft Dark).
  - Preservation of working native Win32 PoC interface during Phase 1 while implementing core engine contracts.
  - Windows 7 SP1 compatibility maintained via GDI baseline renderer and runtime feature detection.
- **Deviations**: None at baseline adoption.

### DEC-008: Feature Parity, Functional Audit, and Integration Hardening Baseline (Phase 8)
- **Date**: 2026-11-25
- **Context**: Phase 8 stabilization, audit, and hardening to establish verified release readiness baseline.
- **Decision**:
  - Pinned upstream LogExpert reference commit `a147ed316cca14f4d90a41186079a116f91ef9ee`.
  - Audited all 18 feature categories with exact numerator/denominator metrics.
  - Eliminated mock placeholders and connected 100% of Win32 GUI commands to active backend handlers.
  - Hardened security: dynamic loading of `comdlg32.dll` to prevent Win7 delay-load symbol breakage, strict SHA-256 host-key abort (MitM defense), Windows user-scoped DPAPI credential encryption, and bounded streaming decompression (100:1 / 2GB anti-zip-bomb limits).
  - Documented intentional behavioral differences: linear DFA regex execution rejecting ReDoS, strict rejection of external DTD/XXE entities in legacy XML, and isolated Windows Job Objects for external tool processes.
- **Deviations**: Intentional security hardening as mandated by `docs/DESIGN.md` and `docs/SECURITY_RISK_REGISTER.md`. Zero cosmetic or UX deviations.
