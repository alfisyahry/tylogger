# Tylogger — Design System & Desktop UX Specification

| Property | Value |
|---|---|
| Product | Tylogger |
| Technical identifier | `tylogger` |
| Application | `tylogger.exe` |
| Document version | 1.0 |
| Status | Normative design baseline; implementation and OS compatibility require verification |
| Platforms | Windows 7 SP1, 8, 8.1, 10, 11; x86 and x64 |
| UI architecture | Native Rust + Win32; no browser runtime |
| Reference | LogExpert behavior at the pinned upstream commit in `docs/UPSTREAM_AUDIT.md` |

## 1. Purpose and authority

Tylogger is a native desktop log viewer and analysis tool. Its primary purpose is to let users inspect, search, filter, correlate and monitor large or continuously changing log files with minimal friction.

Preserve verified LogExpert behavior, workflows, shortcuts, formats and semantics. Improve presentation without silently removing capabilities or changing their meaning. Feature parity is defined by `docs/FEATURE_PARITY.md`, not by visual similarity alone.

This document is the single source of truth for visual design, interaction, UI component contracts and acceptance criteria. It applies to every phase, every window, every dialog and every new feature. Read it before modifying any user-facing code. The PRD governs product scope, the parity matrix governs inherited behavior, and `docs/WINDOWS_COMPATIBILITY.md` governs platform verification. Record and resolve conflicts explicitly instead of choosing silently.

**MUST** = required; **SHOULD** = default unless a documented reason justifies deviation; **MAY** = optional. All visual measurements below are design targets in device-independent pixels (DIP), not benchmark results or verified screenshots. No design requirement may silently weaken correctness, accessibility, or Windows 7 support.

## 2. Product identity

- Display name: `Tylogger`. Technical name: `tylogger`. Never use `TyLogger`, `TYLOGGER` or `LogExpert` as the current product name.
- Use Tylogger in window titles, About, taskbar, dialogs, help, CLI, executable resources and installer. Retain LogExpert where referring to upstream, licensing or compatibility formats.
- Main window title: `Tylogger` when empty; `<file name> - Tylogger` when a document is active. Full paths belong in tooltips or document properties.
- Logo direction: a compact geometric log/terminal mark; no mascot, gradient, text embedded in the icon, or excessive detail. This is a direction, not an approved final logo.
- Final Windows `.ico` must contain appropriately drawn 16, 20, 24, 32, 40, 48, 64, 128 and 256 px variants, with alpha transparency. Check small sizes against light/dark taskbars. Use the same master identity in the executable and installer.
- Do not claim Tylogger is an official LogExpert release. Preserve all applicable upstream and third-party notices.
- Rebranding resources are integrated in Phase 1 when required; final visual assets and their verification belong to Phase 2. Installer visual integration is completed at release.

## 3. Design principles and prohibited patterns

1. Content first: maximize usable log viewport; keep secondary information contextual and collapsible.
2. Native desktop behavior first: respect Windows window management, keyboard conventions, mouse interactions and system accessibility.
3. Quiet interface: neutral surfaces, one accent, readable text, restrained severity colors. Visual inspiration may come from professional editors and Apple HIG restraint, but do not imitate macOS window controls or sacrifice Windows conventions.
4. High information density with adjustable typography, not excessive whitespace.
5. Consistency over novelty: reuse controls, tokens, command identifiers, error wording and layouts.
6. Immediate feedback: every action must show its state without waiting for slow disk or background work.
7. Preserve user context: opening a dialog, filtering, switching tabs or changing theme must not unexpectedly reset selection, scroll or session state.

Prohibited: web-style dashboards as the default workspace; hero headers; decorative subtitles or eyebrows; cards wrapping ordinary tables; pill-shaped controls everywhere; gradient backgrounds; glassmorphism; neon colors; fake depth; ornamental illustrations; excessive corner rounding; unnecessary animations; redundant labels; unnecessary confirmation dialogs; floating controls that cover log content; fake controls, mock data presented as real, and placeholders for advertised features.

## 4. Platform and rendering contract

- Use Rust and Win32 APIs through a dedicated platform adapter. Keep UI and core engine independent. Do not add Electron, Tauri, WebView2, Chromium or a mandatory .NET runtime for the UI.
- Native top-level frame, menu, dialog and accessibility behavior are the default. Use owner-drawn/virtualized controls where required for theme consistency or high-volume log rendering.
- Use GDI as the dependable baseline renderer; optionally use DirectWrite/Direct2D after checking supported APIs and OS capabilities. Handle renderer initialization failure, device loss, Remote Desktop and software rendering without crashing. Do not assume newer Direct2D interfaces exist on Windows 7.
- Avoid undocumented dark-mode APIs and hard imports of newer Windows APIs in legacy builds. Resolve optional functions at runtime and provide fallbacks. Do not simulate a native dark title bar by breaking caption buttons, system menu, window dragging or accessibility. Windows 7 system chrome, menus and common dialogs may remain light; document this exception.
- On Windows 7, use supported system-DPI behavior and a safe baseline; on newer Windows, use per-monitor DPI awareness where available. Do not claim per-monitor DPI v2 support on Windows 7.
- Use physical-pixel snapping for 1 px separators after DPI conversion. Recompute layout, font metrics, hit targets and icons after supported DPI changes.
- No new renderer, GUI dependency or OS API can be introduced without a compatibility review and a recorded decision. Visual quality is not evidence of OS compatibility.
- Respect high-contrast mode and system text scaling. Preserve text selection, copy, focus, IME and keyboard behavior in custom controls.
- Expose accessible names, roles, selection and focus for custom controls through supported Windows accessibility APIs (MSAA/UI Automation where available). Virtualized rows must expose visible/realized items without allocating an accessibility object for the entire file.

## 5. Theme system

All colors below are sRGB hex tokens. Implement semantic tokens in one theme module. Do not scatter literal RGB/hex values across view code. Native controls that cannot be themed safely may use system colors as documented exceptions.

### 5.1 Base semantic tokens

| Token | Light | Dark | Soft Dark |
|---|---|---|---|
| `window.bg` | `#F4F5F7` | `#171A1F` | `#20242A` |
| `surface.primary` | `#FFFFFF` | `#1E2228` | `#282D34` |
| `surface.secondary` | `#F7F8FA` | `#252A32` | `#2D333B` |
| `surface.hover` | `#EDF0F4` | `#2D343E` | `#363D46` |
| `surface.pressed` | `#E1E6ED` | `#363F4B` | `#414A55` |
| `border.subtle` | `#DFE3E9` | `#353C46` | `#3E4651` |
| `border.control` | `#929DAA` | `#778392` | `#84909E` |
| `text.primary` | `#232932` | `#E5E8EC` | `#DEE3E9` |
| `text.secondary` | `#505D6D` | `#B0BBC8` | `#B4BEC9` |
| `text.muted` | `#657181` | `#99A5B3` | `#A3AFBD` |
| `text.disabled` | `#929BA6` | `#737D89` | `#7F8995` |
| `accent.primary` | `#315F9A` | `#91B8ED` | `#A0BDE5` |
| `accent.on_solid` | `#FFFFFF` | `#182333` | `#182333` |
| `focus.ring` | `#315F9A` | `#91B8ED` | `#A0BDE5` |
| `selection.bg` | `#DCE9FA` | `#304763` | `#3A4C62` |
| `selection.fg` | `#172C45` | `#EAF2FF` | `#EFF5FF` |
| `selection.inactive` | `#E5E9EE` | `#30363F` | `#3C434D` |
| `row.bg` | `#FFFFFF` | `#1E2228` | `#282D34` |
| `row.alternate` | `#F8F9FB` | `#22272E` | `#2B3038` |
| `row.gutter` | `#F3F5F8` | `#191D23` | `#242930` |
| `search.match.bg` | `#F9E5A8` | `#5B4925` | `#5D4C2C` |
| `search.match.fg` | `#422E08` | `#FFF0C2` | `#FFF0C2` |
| `search.current.bg` | `#F1CB69` | `#806126` | `#82612E` |
| `search.current.fg` | `#302200` | `#FFFFFF` | `#FFFFFF` |

### 5.2 Semantic severity tokens

| Severity | Light fg / bg | Dark fg / bg | Soft Dark fg / bg |
|---|---|---|---|
| Trace / Debug | `#526780` / `#EDF2F8` | `#A7BBD4` / `#273344` | `#AFBFD2` / `#303B49` |
| Info / Success | `#286344` / `#E9F3ED` | `#9DCCA9` / `#26382E` | `#A7CDB2` / `#303F35` |
| Notice | `#315F9A` / `#E8F0FC` | `#A7C8F3` / `#28384E` | `#B2C9EA` / `#344257` |
| Warning | `#81530A` / `#FCF2D8` | `#E5C183` / `#453923` | `#E1C28B` / `#493E2D` |
| Error | `#A12D35` / `#FBECEE` | `#F0A5A9` / `#462D33` | `#E9ABAE` / `#49363A` |
| Critical | `#8E1830` / `#F9DFE5` | `#FFB1C1` / `#522A39` | `#F4B1BD` / `#543442` |

- Use neutral text for unclassified raw logs. Classify severity only from the active parser or explicit user rules; do not infer all HTTP 4xx as application errors or 5xx as critical incidents.
- Prefer a colored text value, subtle background or narrow gutter marker, not saturated whole-row fills by default. Full-row highlighting is allowed for configured rules.
- Color must never be the only signal. Keep severity text, icons or labels available.
- Test actual rendered foreground/background pairs: target 4.5:1 for normal text and 3:1 for meaningful non-text controls, focus and indicators. Adjust tokens centrally if measured contrast fails. Disabled controls are exempt from text contrast targets but must remain identifiable.
- Selection, highlights and search: selected rows use selection background; preserve severity/bookmark markers in the gutter; search match spans may overlay selection using contrast-safe foreground/background; keyboard focus is an independent outline. Current match has an additional non-color cue. Multiple rules follow explicit user rule priority. Never obscure selection or make text unreadable.
- Theme options: System, Light, Dark, Soft Dark, plus system high-contrast override. Clean install defaults to System; fall back to Light if the system preference cannot be determined reliably. Theme changes apply to all open Tylogger windows without losing state. Native OS surfaces may differ.

## 6. Typography, spacing and geometry

| Role | Font | Default |
|---|---|---|
| UI body, menus, buttons | Segoe UI (system font fallback) | 12 DIP |
| Panel title | Segoe UI Semibold | 12 DIP |
| Dialog title | System caption / Segoe UI Semibold | 14 DIP if in client area |
| Status bar / metadata | Segoe UI | 11 DIP |
| Log content | Consolas | 13 DIP |
| Log fallback | Courier New / compatible monospace | 13 DIP |
| Line numbers | Same monospace as log | 12 DIP |

Use system font substitution for missing glyphs and Unicode scripts. Do not bundle fonts without compatible licensing. Keep proportional UI text and monospace log content separate. Preserve whitespace, tabs, grapheme boundaries and actual copied text. Font-size settings: log 10–24 DIP, UI 11–18 DIP; allow keyboard zoom and reset. Defaults are compact, not fixed maximums.

| Token | Value / rule |
|---|---|
| Spacing scale | 2, 4, 8, 12, 16, 24 DIP |
| Standard outer padding | 8 DIP |
| Compact control height | 26 DIP |
| Standard control height | 30 DIP |
| Icon visual size | 16 DIP; 20 DIP where needed |
| Icon button hit target | at least 28 × 28 DIP, 32 × 32 preferred |
| Control corner radius | 2–4 DIP for custom controls; native geometry otherwise |
| Log row height | 20 DIP default; compact 18, comfortable 24, expand for font metrics |
| Column header height | 26 DIP default; expand for font metrics |
| Divider | 1 physical pixel where possible |
| Splitter | 4 DIP visual/hit area minimum, enlarge for easier dragging |
| Focus | visible 1–2 DIP outline or native equivalent |

Do not truncate controls vertically at high DPI or large font sizes. Density presets change spacing and row height, not accessibility requirements. No arbitrary per-screen spacing values unless a measured layout issue is recorded.

## 7. Main window layout

```text
+-----------------------------------------------------------------------+
| <active file> - Tylogger                        [min] [max] [close]    |
+-----------------------------------------------------------------------+
| File   Edit   View   Search   Filter   Tools   Window   Help            |
+-----------------------------------------------------------------------+
| Open  Reload | Find  Filter  Bookmark | Follow tail  Pause | Settings  |
+-----------------------------------------------------------------------+
| access.log       error.log       syslog                       [+] [v] |
+-----------------------------------------------------------------------+
| Contextual find / filter strip (shown only when relevant)               |
+------+-------+--------+------+-----------------------------------+----+
| Line | Time  | Level  | ...  | Message                           |Map |
+------+-------+--------+------+-----------------------------------+----+
|      Virtualized log viewport                                   |    |
|      Actual log content occupies the main area                   |    |
|                                                                 |    |
+-----------------------------------------------------------------+----+
| Filter results / Bookmarks / Details / Analytics       [^] [x]         |
| Optional resizable lower or side pane                                  |
+-----------------------------------------------------------------------+
| Ready | UTF-8 | 1,024 lines | 120 MB | Follow tail | Background work    |
+-----------------------------------------------------------------------+
```

Menu names and positions are a proposed grouping. Reconcile with the verified upstream command inventory before implementation; every inherited command must remain discoverable. The diagram is a layout contract, not a pixel-exact screenshot or proof of feature implementation.

- Main window initial size: 1100 × 720 DIP, clamped to the available work area. Target usable layout: 800 × 600 at 100% DPI. At smaller effective sizes, collapse optional panels, use toolbar overflow and scrollable tabs. Do not require a 1080p screen.
- Preserve native maximize, minimize, restore, system menu, taskbar behavior, snap where supported and multi-monitor restoration. Clamp restored windows to a visible work area.
- Menu bar: command hierarchy, access keys, shortcut labels and actual checked/disabled states. Group related actions with separators. Avoid redundant submenus.
- Toolbar: common operations only; consistent icon family, no duplicate neighboring actions. Use dropdown for related variants and overflow for lesser-used tools. Tooltips include command name and shortcut, no marketing copy.
- Tab strip: active/inactive/hover/close states, file name, optional activity marker, tooltip with full path, drag reorder, middle-click close where supported, overflow menu. Do not rely on tab color alone. Do not treat live tail as a dirty file.
- Main viewport must retain priority over optional panels. Bottom panel starts at about 28% of available client height when first opened; resizable, collapsible and persistent. Side panels are optional and resizable. Avoid mandatory navigation sidebars.
- Marker bar: compact vertical overview of search results, bookmarks and configured highlights; bin marks by visible pixel height and allow navigation with tooltips. Do not allocate one widget per log line.
- Status bar: useful facts only, including active file state, encoding, size, line count, position/selection, tail state and progress. Unknown totals must be marked as indexing/unknown rather than displayed as final values.

## 8. Reusable control contracts

| Control | Required behavior |
|---|---|
| Button | Clear label; default, hover, pressed, focus, disabled states; no silent click |
| Toggle | Visible checked/unchecked state plus accessible label; optional mixed state only when meaningful |
| Text input | Label, focus, selection, clear where useful, validation and keyboard editing; placeholder is not a label |
| Search input | Clear, history where appropriate, Enter/Shift+Enter navigation, Esc cancel/close when applicable, explicit mode/options |
| Combo box | Native keyboard navigation, type-ahead; searchable when options exceed 10 or are difficult to scan; search must not destroy selected value |
| Context menu | Context-specific commands, keyboard access, correct availability, no inaccessible hover-only actions |
| Dialog | Clear task title, concise labels, standard OK/Cancel or descriptive action, initial focus, Enter/Esc according to task, resize if content requires |
| Tooltip | Plain actionable information, delayed display, no critical information accessible only on hover |
| Progress | Determinate when total known, otherwise indeterminate; show actual operation, allow cancel where supported |
| Empty state | One concise explanation and a relevant action only; no decorative graphics or fake content |
| Error state | What failed, affected file/operation, actionable recovery, optional technical details and Copy details |
| Split panel | Drag, keyboard resize where feasible, collapse/restore and persist sizes; avoid stealing focus |
| Icon | Consistent 16 DIP outline or native equivalents; distinguish actions at 100% DPI; label unfamiliar actions |

Every interactive control must have observable hover (where applicable), focus, pressed, selected/checked and disabled states. Loading must not masquerade as disabled without explanation. Use native controls where they satisfy the contract; custom paint must preserve equivalent keyboard/accessibility behavior.

Use a central command registry with stable command IDs, labels, shortcuts, icon IDs, enabled/checked predicates and handlers. Menu, toolbar and context menu call the same command, not separate implementations. Do not hardcode parallel command logic in each view.

## 9. Log grid and ordinary tables

### 9.1 Primary log viewport

- Use a virtualized grid with 64-bit logical positions. UI controls with 32-bit item or scrollbar limits must use a bounded viewport and mapping layer; never cast the total line count to `i32` without checking.
- No pagination for the primary log grid or live filter results; provide continuous scrolling, jump-to-line, marker navigation and total/visible range. Pagination would break live-tail and original LogExpert workflows.
- Show line numbers by default, then columns provided by the active columnizer. Raw mode shows line number and message. Do not invent columns when parsing is unavailable.
- Support horizontal scroll, adjustable column widths, reorder and visibility, optional frozen line-number column, double-click autosize using bounded sampling, and full-value inspection for clipped content.
- Default to no wrapping for logs. Optional wrap must not corrupt logical line selection, bookmarks or source offset mapping. If not implemented, document the gap rather than expose a dead toggle.
- Use a stable logical row identity (file identity + generation + line identifier). UI selection must never be keyed only by visible row index across filters/rotation.
- Keep line-number gutter quiet, alignment consistent and tabular numbers readable. Do not add a heavy border around every cell. Long messages must remain accessible without allocating unbounded display strings.
- Support click, Shift range, Ctrl multi-select where parity requires, keyboard navigation, context menu, copy and select all with safe behavior on huge selections. Confirm only expensive/destructive operations when justified; stream large exports with cancellation.
- Preserve display-to-source mapping. Copy/export semantics, raw versus formatted values, separators and line endings must be defined by parity tests; add explicit commands if both modes are available.
- Do not silently sort the live chronological log. Sorting parsed fields belongs in an explicitly supported derived view unless verified upstream behavior specifies otherwise.

### 9.2 Ordinary management and analytics tables

For bookmarks, highlight rules, plugin lists, statistics and configuration tables, apply appropriate controls based on the dataset:

- Search/filter when more than 10 items are common; keep clear action and result count.
- Sort only supported columns. Show sort direction and stable tie handling.
- Pagination when needed for large non-streaming datasets: page sizes 25, 50, 100, 250; default 50; first/previous/next/last where supported; show range and total; reset page on filter change. For smaller local lists or virtualized tables, continuous scrolling is preferred.
- Numeric columns right-aligned, text left-aligned, timestamps consistent. Do not place arbitrary colored badges on every ordinary value.
- Multi-select and bulk actions only where functional. Show selected count and disable incompatible actions. Do not hide essential actions in hover-only menus.
- Preserve column sizes, sort, search and other relevant preferences when appropriate. Avoid restoring stale selection across different files.

## 10. Feature-specific UX contracts

### 10.1 Open files and workspaces

Open local files, UNC paths and verified upstream source types using native dialogs or relevant source-specific controls. Support drag-and-drop and recent files. Report partial success on multiple-file open. Loading/indexing should not prevent opening another document or canceling work. Do not modify source files. Session restoration reports missing files individually rather than failing the entire workspace.

### 10.2 Live tail, rotation and activity

Follow tail and monitoring are distinct states. Following means the viewport tracks new content; monitoring may continue while the user inspects older lines. Expose an obvious toggle and a way to jump to the newest line. Respect verified upstream auto-stop/pause behavior, and never silently resume after user pause. Show pending new lines if useful. Batch incoming updates, avoid flicker and preserve scroll anchors. Indicate truncation, replacement, reconnect and rotation unobtrusively; do not silently reassign bookmarks to new content. Errors must differentiate missing, inaccessible, disconnected and reconnecting sources.

### 10.3 Find and search

Compact contextual strip; focus query on open. Show query, case sensitivity, regex, direction/next/previous, match count when known and close. Add column selection and other modes where verified. Distinguish search navigation from filtering. Invalid regex must show a concise inline message; never crash or replace valid results with misleading results. Running searches show progress and cancel. Ignore results from stale queries. Preserve query history according to user privacy settings. The matching semantics, including .NET regex compatibility limitations, must follow parity evidence; do not silently substitute unsupported expressions.

### 10.4 Filters and derived tabs

Expose include/exclude, case/regex, column, range, fuzzy matching, invert, back/fore spread, and live filtering according to the audited upstream semantics. Advanced options are collapsible but discoverable. Show active filter summary, result count/progress, clear/reset, and whether results update live. Keep original file visible unless the user explicitly chooses a derived tab. Filter-to-tab preserves source mapping, ordering, deduplication and update behavior. Debounce expensive input changes; Apply/Enter must not feel delayed. Do not label partial counts as final.

### 10.5 Highlights, bookmarks and triggers

Rule manager: name, enabled, match mode, expression, target, foreground/background preview, priority, and actual configured actions. Provide add/edit/duplicate/delete/reorder where supported. Show inline validation. Bookmarks use a gutter marker and keyboard navigation; comments are editable and discoverable. Triggers and external tools are explicit user configurations, not commands inferred from log contents. Warn on dangerous external actions and do not execute raw log text as code. Never silently trigger commands during preview. Follow verified upstream action semantics.

### 10.6 Columnizers, encoding and timestamps

Expose active parser and encoding in document controls/status bar. Provide auto-detection only when available; manual selection overrides it. Preview parser configuration with real sample data, no fake preview. Handle malformed lines and missing fields without hiding the original text. Time synchronization and shifts must display their state, time zone/offset where relevant, and clear reset behavior. Do not alter original log timestamps. Preserve the upstream semantics of time spread, synchronized groups and navigation.

### 10.7 Analytics and remote sources (later scope)

Analytics live in optional panels or separate derived tabs, never replace the default log viewport. Prioritize tables with exact counts, then charts where time distribution or comparison benefits. Label time windows, units, filters, aggregation and incomplete results. Clicking a chart or table row may navigate to matching logs only when the operation is implemented. Do not invent IP geolocation, counts or security classifications. Remote-source dialogs distinguish authentication, connection state and errors; do not persist plaintext secrets, and disable unverified legacy-network functionality by default if required by the security design.

## 11. Keyboard, mouse and focus

Maintain `docs/COMMANDS.md` as a command/shortcut registry derived from the pinned LogExpert version. Verify the upstream mappings before assigning keys. The following are proposed fallback conventions only when upstream behavior does not conflict:

| Action | Proposed convention |
|---|---|
| Open file | Ctrl+O |
| Find | Ctrl+F |
| Find next / previous | F3 / Shift+F3 |
| Copy | Ctrl+C |
| Close tab | Ctrl+W |
| Switch tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Go to line | Ctrl+G |
| Context menu | Shift+F10 |
| Close transient panel / cancel | Esc, contextual |
| Zoom log font | Ctrl+Plus / Ctrl+Minus / Ctrl+0 |

Keep menu mnemonics, shortcuts and tooltips synchronized with the registry. Do not override standard text-editing shortcuts when an input has focus. Support Tab/Shift+Tab, arrow keys, Page Up/Down, Home/End and keyboard-only operation. Focus returns to the originating control after dialogs, or the log grid after navigation when appropriate. Double-click behavior must match upstream where required. Avoid keyboard traps and click-only actions.

## 12. Feedback, text and error handling

- Use short, concrete UI strings in sentence case: `Open file`, `Follow tail`, `Find next`, `Reconnect`, `Cancel indexing`. Product name retains exact casing. Avoid explanatory subtitles and redundant status messages.
- Labels describe actions, not implementation: use `Open file` rather than `Execute file-source initialization`.
- Store user-facing strings centrally to support future localization. Initial language is English; do not hardcode date/number separators where locale preferences matter. Preserve raw log strings exactly for copy/export.
- Distinguish ready, loading/indexing, running, canceled, partial, empty, disconnected, failed and stale states. A zero-result search is different from an invalid query or a file not indexed yet.
- Feedback hierarchy: inline validation first, status/panel notifications for recoverable background events, modal only when user action is required. Avoid repeated modal notifications during continuous tailing.
- Example error: `Cannot open error.log. Access was denied.` Actions: `Retry`, `Copy details` when applicable. Technical details may include error codes and paths without exposing secrets unnecessarily.
- Do not show success until operation results confirm it. Cancel is not failure. Progress, counts and timestamps must reflect engine state, not fabricated or simulated values.

## 13. Responsiveness and performance

The GUI thread handles rendering, input and lightweight state updates only. All file I/O, indexing, parsing, search, filtering and large exports execute in workers. Communicate using bounded queues, coalesced events and generation-aware updates. Post results to the UI thread; never update native windows from worker threads without an approved thread-safe mechanism.

| Metric | Initial engineering target (not measured) |
|---|---|
| Visible feedback after input | p95 <= 50 ms on stated benchmark hardware |
| Cached viewport retrieval | p95 <= 50 ms |
| Append detection | p95 <= 500 ms, default monitoring profile |
| Idle working set | <= 50 MB, baseline empty workspace |
| Idle CPU | <= 1%, record sampling and normalization method |
| Startup | <= 1 second on SSD, define first-interactive window |
| Release EXE | <= 15 MB per architecture, report actual size |

Measure cold/warm states, OS, architecture, hardware, display DPI, renderer and workload. Do not claim targets are met without evidence. Test legacy hardware or a constrained VM separately. Prioritize correctness over artificial performance gains.

- Render only visible rows plus bounded prefetch. Never create millions of native UI objects. Cache layout and text measurements within explicit limits; invalidate on font, DPI, encoding or file generation changes.
- No animated scrolling or heavy transitions during monitoring. Use simple native feedback and redraw only changed areas when possible. Optional animation must respect system settings.
- Show pending work without blocking interaction. Long operations expose cancellation when safe. Prevent a flood of tail events from starving input or the paint queue.
- Do not block startup with a network check, full log indexing or optional analytics. Expensive automatic workspace restore must be asynchronous.

## 14. Settings and persistence

Settings categories: General, Appearance, Log viewer, Search & filters, Monitoring, Advanced, Plugins (when implemented). Use a native settings dialog with clear sections; search only if the number of settings warrants it. Apply changes safely, with Cancel restoring unapplied changes. Clearly label settings requiring reload/reindex/restart.

Persist theme, font, density, column visibility/widths, window bounds, splitter positions, recent files, shortcuts and relevant workspace state through the approved persistence layer. Use versioned schemas and atomic writes; never overwrite existing user data during rebranding migration. Support `%APPDATA%\Tylogger` and the approved portable `config\` directory. If portable storage is not writable, explain the issue and offer an explicit alternative; do not silently scatter files into another location. Provide reset for individual categories and an explicit full reset with confirmation.

Do not persist log contents, sensitive searches, credentials or exported data unnecessarily. Follow the project's privacy and credential storage policy. No mandatory internet or telemetry. Keep configuration data separate from source logs.

## 15. Implementation organization and reuse

Adapt to the existing workspace instead of renaming/restructuring stable modules blindly. The following are logical responsibilities, not a mandatory new crate layout:

```text
ui/
  theme/          semantic tokens, system colors, contrast
  metrics/        spacing, fonts, DPI, geometry
  commands/       command registry and shortcuts
  controls/       reusable native/custom widgets
  layout/         window, toolbar, tabs, split panes
  log_grid/       virtual viewport and selection
  panels/         find, filter, bookmarks, analytics
  dialogs/        settings, open, about, error details
  accessibility/  focus, roles, names and automation
platform-win32/   API capability detection and native adapters
assets/           icons and Windows resource metadata
```

Reuse one implementation of each repeated component. Theme data is a typed semantic palette, not CSS or HTML. Use state-driven painting and command availability. Resource handles must be RAII-managed. Keep unsafe platform calls isolated and documented. Do not duplicate settings, selection, search or tail state between windows and core engine without a synchronization contract.

## 16. Phase integration

| Phase | Design obligations |
|---|---|
| 0 / PoC | Adopt this baseline; record differences; no need to rewrite verified prototype solely for cosmetics |
| 1 | Technical Tylogger branding, native compatibility, core/UI contracts, shared tokens and minimal working PoC surface; no full visual rebuild required |
| 2 | Implement native shell, theme system, icon assets, virtual grid, focus/DPI, reusable controls and layout; wire to real core |
| 3 | Multi-tab, file management, live tail and rotation UI follow existing contracts |
| 4 | Search/filter panels, validation, progress, keyboard and derived views |
| 5 | Highlights, bookmarks and triggers; precedence and accessible indicators |
| 6 | Columnizers, encoding controls, time sync and time analysis |
| 7 | Settings, persistence and plugin interfaces; migration and error states |
| 8 | Optional analytics and additional tools with functional source mapping |
| 9 | Full feature parity, UX regression, accessibility and visual consistency audit |
| 10 | Installer and release assets, compatibility checks, final polish and verified performance |

Do not advance phase status just because mockups, screenshots or UI controls exist. A feature is ready only when real behavior is wired and tested. Unverified OS compatibility remains a release blocker even if implementation proceeds.

## 17. Definition of done and design QA

Before merging any user-facing change, check:

- [ ] The command is functional, or intentionally not exposed until it is functional.
- [ ] Behavior matches audited upstream semantics or an approved, documented extension.
- [ ] New UI reuses shared tokens, components and command registry.
- [ ] Light, Dark, Soft Dark and applicable system/high-contrast modes are checked.
- [ ] Focus, hover, pressed, selected, disabled, loading, empty and error states are covered where applicable.
- [ ] Keyboard-only flow, tab order, shortcuts and accessibility labels are verified.
- [ ] Text and markers remain legible at actual rendered contrast and DPI.
- [ ] Long paths, long lines, Unicode, missing values and large counts are handled.
- [ ] Resizing, overflow, small screens, high DPI and optional panels do not hide essential controls.
- [ ] Live updates do not jump scroll/selection unexpectedly; stale events cannot corrupt the view.
- [ ] UI thread stays responsive during I/O and heavy operations.
- [ ] No unsupported Windows API or mandatory runtime was introduced without compatibility evidence.
- [ ] No fake data, placeholder actions, decorative clutter or inconsistent Tylogger identity.
- [ ] Relevant tests, parity matrix and screenshots/observations are updated.

### Visual verification matrix

| Dimension | Minimum coverage |
|---|---|
| Operating systems | Windows 7 SP1 x86/x64; Windows 8/8.1, 10, 11 according to release matrix |
| DPI | 100%, 125%, 150%, 200% where supported; legacy system-DPI and modern monitor transitions separately |
| Display | 1024×768 legacy screen, 1366×768 laptop, 1920×1080 desktop, high-DPI and multi-monitor where available |
| Themes | Light, Dark, Soft Dark, System, high contrast |
| Rendering | GDI baseline, optional accelerated path, fallback, Remote Desktop where supported |
| Data | Empty, one line, long Unicode line, millions of lines, partial indexing, live append, rotation, invalid query, mixed severity |
| Interaction | Mouse, keyboard, menu, context menu, multi-select, drag/drop, settings and cancel |

Use actual running application captures, not conceptual images, for visual regression evidence. Record OS/build, architecture, theme, DPI, window size, renderer, commit and scenario. Keep reference screenshots by theme and representative resolution; use tolerance for native OS chrome/font rendering differences and inspect meaningful changes manually. Never mark Windows 7 verified without executing the corresponding binary on a suitable Windows 7 environment.

## 18. Design governance for coding agents

1. Read this file and the current feature parity/architecture/compatibility documents at the start of every phase.
2. Audit existing implementation and preserve working behavior. Do not regenerate the app from scratch to apply this file.
3. Use a shared token and component implementation. If a new control is needed, define its states, keyboard behavior, compatibility and reuse contract first.
4. Do not introduce independent palettes, icon styles, layouts or per-page design systems.
5. Do not redesign unrelated screens opportunistically. Document any necessary deviation in `docs/DESIGN_DECISIONS.md` with date, commit, reason, alternatives, affected components and verification. Material product/UX changes require explicit approval; compatibility fixes may proceed when documented and behavior is preserved.
6. Keep `docs/DESIGN.md` version-controlled. Update this baseline intentionally, never silently to justify an inconsistent implementation.
7. Each phase report must include design compliance, actual UI tests, evidence paths, unresolved deviations, compatibility gaps and next-phase readiness. Never claim verification that was not performed.

The final application must feel like one coherent native desktop tool across every screen, phase, theme and supported Windows version. Correct log handling and reliable interaction take precedence over cosmetic uniformity that would compromise compatibility.
