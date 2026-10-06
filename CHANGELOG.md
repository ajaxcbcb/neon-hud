# Changelog

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
