# Changelog

## Unreleased — native Nook

- Native black capsule with hover peek, click expansion, pinning, Escape dismissal and inward screen placement. The floating pill remains selectable and retains its saved position.
- Instruments and AI tabs retain numerical system readings, gradient stress bars, history, five-hour reset windows and measured allowance drain. Local notes, tasks, deadline timers and file references use separate bounded, recoverable storage.
- Saved Codex connection intent and last-known allowance survive retry failures. Stale readings show their original timestamp and connection state; explicit disconnect clears them.
- Bridge migration recognises owned Windows version folders and macOS versioned app bundles while preserving foreign Claude configuration.
- Added source regressions for timer state retention, monitor scale/stacking, retry and bridge ownership, plus hosted physical Nook controls and native frame captures. Reference playback, integrated validation and target installation remain pending; this section is not a published-release claim.

## [0.2.0-alpha.4] - 2026-10-07

- The Download update action stays visible and disabled while an update operation runs. Hovering identifies the signed release version, and the status and activity indicator show progress.
- Receives updates from the signed native-preview channel introduced in alpha.3, retaining the app icon, screen-aware right-click controls and saved native preferences.

## [0.2.0-alpha.3] - 2026-10-07

- Right-click any metric, the drag grip or pill background to open a separate native controls popup. The complete menu opens inward at screen edges, supports Settings, compression, pause/resume, hide, reset and Quit, and dismisses with Escape or a focus change. Shift+F10 also opens it.
- Embedded the Neon N icon in the Windows executable and native windows; the tray now uses the same mark. macOS retains its bundled app icon.
- Controls use two columns when a small monitor limits the popup height, keeping every action reachable without scrolling.
- Added real Windows mouse-input checks at all eight edges/corners, all six menu actions and Escape dismissal, plus executable/window icon checks.
- Fixed Windows pointer re-entry at identical client coordinates, so moving the pill between screen edges does not leave right-click input without a pointer position.
- Hidden Windows HUDs receive coalesced background callbacks, preserving tray actions, profile saves and updater completion at the existing slower sampling rate.
- Added a separate signed native update channel with bounded downloads, archive size/hash verification, normal-exit installation, retained application backup and startup acknowledgement. Settings includes automatic checks/downloads, Check now, progress and Restart and update; failed automatic downloads wait for a new check. The stable webview updater feed is unchanged.

## [0.2.0-alpha.2] - 2026-10-07

- Native hover readings open inward at all four screen edges and corners and stay within the HUD monitor. Placement follows dragging and compression, including negative monitor origins and different display scales.
- Placement uses cached display bounds and does not resize the pill or rewrite saved preferences.
- Added geometric edge/monitor tests and a Windows CI check that moves the native pill and hovers with the real cursor at eight positions.

## [0.2.0-alpha.1] - 2026-10-07

- Rust/egui/glow renderer with a 160 × 56 compressed pill and 280 × 56 regular pill, native tray controls and separate instrument windows.
- Custom neon settings frame with Appearance, Connections and Preferences steps, theme cards and a sample pill preview. Instruments and Startup choices fit inside Preferences; multiple drives use pages. No scroll areas.
- Custom neon metric icons, gradient gauges/bars, brief question reactions and motion that quiets under pressure.
- Separate preview preferences, monitor-relative placement by display identity, debounced saves and opt-in preview startup.
- Shared Claude bridge readings remain available while preview preferences stay separate. Codex and Claude connectors still use helpers; strict single-process operation and signed native automatic updates remain unfinished.
- Windows/macOS tests and locked builds passed. All three setup steps, both Preferences pages, instruments and the 160 × 56 pill were rendered and inspected on Windows CI. An unsigned Windows x64 ZIP and Apple Silicon Mac app/DMG are available as a separate prerelease.
- Embedded Ubuntu Bold headings, vector checkmarks/arrows and a fixed footer retain the reference layout without unsupported glyphs or taskbar overlap in the inspected render. Font license and copyright notices are bundled.
- This preview has not replaced the installed v0.1.4 or the automatic-update manifest. Native installation, Mac runtime and hardware performance checks remain open.

## [0.1.4] - 2026-10-07

### Added

- Drag grip and keyboard nudging with saved monitor-relative placement; DPI and disconnected-monitor recovery keep the HUD onscreen.
- A 160 × 56 compressed pill with numerical AI allowances, the most stressed system metric, hover details and attention badges.
- Right-click compression and Reset position controls; compressed size is selectable on the first configuration page.

### Improved

- Saved custom placement survives size changes, hover popovers, settings and tray restoration.
- Pending movement is captured before Quit, restart and update preference flushes. Existing profiles keep their docked placement until moved.

## [0.1.3] - 2026-10-07

### Added

- Optional automatic installation and restart after verified downloads and a quiet minute, including from the system tray or Mac menu bar.
- Restart deferrals for settings/details, questions, connection attempts, paused monitoring, unsaved settings and high resource pressure; fresh system/provider preflight before installation.

### Improved

- Preserve older and partial profiles across launches; unreadable settings remain intact instead of being replaced with defaults.
- Serialize preference writes and flush pending edits before tray Quit or update installation.
- Minimize hides the Windows window to the tray and removes its taskbar presence. Tray actions restore the current HUD or settings view.
- Automatic-install failures stop automatic retries and retain the manual recovery control.
- Disabling automatic checks during an in-flight check prevents the following automatic download.
- Serialized preference saves prevent an older autosave from clearing a newer pending-save gate. Existing settings migrate with automatic installation off.

### Validation

- Frontend policy, signature, restart, retry, switch-change and migration tests; native preference round-trip and migration tests.
- Installed upgrade and future-release delivery require runtime evidence; build and fixture results are recorded separately in validation.

## [0.1.2] - 2026-10-07

### Added

- GPU icon, pill measurement and speedometer, adapter selector, expanded multi-adapter measurements and source/capability details.
- Windows DXGI adapter discovery and cached PDH global GPU activity/dedicated-memory measurements; Mac Metal identity with honest unavailable fields for unsupported global readings.
- Adjustable 250/500/1000/2000ms CPU, memory and network sampling, independent of slower GPU, sensor and AI polling.
- Finite display-synced meter transitions without a fixed 60 Hz cap; numerical values retain the measured sample.
- GPU pressure thresholds and badges; adaptive backoff and recovery require distinct, fresh GPU evidence.

### Improved

- Separate non-overlapping system and provider polling, with hidden-window pause and immediate resume.
- Cached native snapshots prevent repeated calls from forcing expensive measurements; GPU discovery stays cached for 30 seconds.
- Idle, hidden and reduced-motion meters stop scheduling animation frames.
- Existing settings migrate with GPU and fast-sampling defaults while preserving saved choices.

### Validation boundaries

- Unsupported/stale GPU readings stay unavailable. Thermal throttling is not measured.
- Native GPU accuracy, actual display frame rate, Mac installation and full-process resource usage require separate installed-platform checks.

## [0.1.1] - 2026-10-07

### Highlights

- A 280 × 56 side pill with a 272 × 48 neon capsule, readable numbers and metric/provider icons.
- Hover measurements, click-through meters, keyboard/right-click quick controls, and six display positions.
- Immediate connection progress and honest connected, waiting, sign-in-needed and failed states.
- Automatic GitHub version checks and signed downloads, with explicit Install & restart control.

### Reliability

- Asynchronous provider and bridge commands keep the interface responsive.
- Bridge status checks the actual statusline and all seven hooks; cached readings cannot imply configuration.
- Serialized refreshes isolate a previous failed request from the next connection action.
- Window resizing is queued, with DPI-aware placement and narrow settings support.
- Idle pill animations stop; hover and question reactions remain brief, with reduced motion and adaptive budgets.
- Fork pull requests build without release secrets; trusted CI produces signed update packages.

### Limits

- Rust handles native monitoring, tray and startup; the interface still uses Tauri's desktop WebView.
- OS code signing/notarization, native Mac runtime and full-process performance measurements remain pending.
- Configuring Claude does not prove a live quota reading; it waits for supported Claude Code statusline data.

## [0.1.0] - 2026-10-07

First public preview.

### Highlights

- Tiny floating neon HUD for Windows and macOS, with tray/menu-bar access.
- Three arcade themes, interactive icons, chaotic finite bursts and quiet/reduced-motion modes.
- Green → amber → hot pink stress gradients, numerical values and explainable pressure badges.
- Adaptive monitoring budgets that quiet animation and slow checks under system pressure.
- Configurable multiple-drive capacity monitoring alongside CPU, RAM and network readings.
- Supported Codex and Claude Code allowance sources, reported reset windows and fast-drain estimates.

### Added

- Appearance-first setup, optional AI connections, persistent preferences and optional auto startup.
- Compact/expanded views, display/corner placement, pinning, text scaling and opacity controls.
- CPU total/core utilization, reported clock and available sensor temperatures.
- Memory used/available/total and network rates, time-positioned 60-second history and lifetime counters.
- Drive selection, free/used/total measurements and configurable capacity thresholds.
- Ten-second sustained CPU/RAM warnings, immediate sensor/capacity warnings and hover explanations.
- AUTO strategy chip, stepped recovery and suggested CPU/memory/temperature actions.
- Provider-reported five-hour and other windows kept separate; stale and expired states labeled.
- Time-weighted allowance drain estimates; token-rate support only when a source supplies cumulative counts.
- Explicit question/permission badges with brief shakes and reduced-motion alternatives.
- Right-click/keyboard controls, pause/resume and manual refresh.
- Windows x64 NSIS setup and universal Apple Silicon/Intel macOS DMG preview builds.
- MIT OR Apache-2.0 licensing, bundled dependency licenses and third-party notices.

### Reliability and efficiency

- Non-overlapping polling, hidden-window pause, expensive-sensor caches and capped particles.
- Dedicated background sensor thread keeps Windows WMI/COM initialization away from the UI; CI checks window creation and startup survival.
- Compact Claude index avoids parsing all historical sessions on every poll.
- Locked concurrent bridge writes preserve quota and unresolved attention updates.
- Precise bridge-command ownership and fresh backups preserve unrelated Claude configuration.
- Final selected-drive removal is prevented; CPU hover temperatures use CPU-labeled sensors.

### Preview limitations

- Installers are unsigned; macOS notarization and native Mac installation/runtime checks are pending.
- Codex allowance does not represent ordinary ChatGPT chat quotas; that source is unavailable.
- Connected quota sources do not supply cumulative spent tokens. Drain uses allowance percentage/time.
- Live provider integration and full-process performance targets have not been measured.

[0.1.0]: https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.0
[0.1.2]: https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.2
[0.1.3]: https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.3
[0.1.1]: https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.1
