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
