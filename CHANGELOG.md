# Changelog

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
