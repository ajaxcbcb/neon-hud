# Neon HUD feature guide

## Native Nook · 0.2.0-alpha.6

A tiny black capsule opens into a shallow row of tools: music, a week calendar, notes, timer wheels, tasks and quick actions. Nook and Tray switch between the widget overview and a persistent file shelf. Hover peeks, click opens, pin keeps it open and Escape collapses it. Choose either layout in **Preferences → Instruments → HUD layout**. The original movable/compressible neon pill keeps its saved position; both presentations use custom native frames. Existing profiles keep their presentation, and fresh profiles start in Nook.

The numerical bottom rail keeps CPU, GPU, memory, network, selected drives, Codex and Claude visible. Focused instruments and AI views retain stress gradients, histories, source details, reported reset windows and allowance drain. Missing readings stay unavailable and stale readings keep their original timestamps. Saved connection intent and independent utility storage survive restarts.

Notes include B/I/U; tasks include completion and favourites; deadline timers include hour/minute/second wheels and presets. Windows media controls use system sessions, Mac Music control is opt-in, calendar accepts local recurring ICS events, and Mac can request native Calendar permission. The explicit camera mirror stops when hidden or under pressure. Permission and unsupported-source states are visible. The custom three-step Settings flow retains themes, motion, configured drives, tray access, opt-in startup and signed native updates.

[Download alpha.6](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.2.0-alpha.6) · [Release notes](releases/v0.2.0-alpha.6.md) · [Native setup and controls](../src-native/README.md) · [Reference evidence](../reference-learning/notchnook/study.md) · [Validation](VALIDATION.md)

The separate Windows [self-signed development build](https://github.com/ajaxcbcb/neon-hud/actions/runs/37689111604) passed timestamped Authenticode verification, temporary certificate/key cleanup and physical Nook interaction checks. It is a test artifact, outside public release assets and the updater feed. The target alpha.6 Defender quarantine remains unresolved; a development signature does not establish publicly trusted publisher status or target runtime acceptance.

### Measurements and attention

| Metric | Numerical reading | Visual and attention behavior |
| --- | --- | --- |
| CPU | Total/per-core utilization and reported frequency | Speedometer, per-core bars and sustained-pressure badge. Temperature appears only from a supported sensor. |
| GPU | Adapter identity and supported utilization/memory | Gradient instruments and pressure badge; unavailable fields stay labeled. Windows supports native counters; Mac global activity remains source-dependent. |
| RAM | Used/total GiB and percentage | Bar and sustained-pressure badge. |
| Network | Upload/download MiB/s and history | Activity graphs; colors describe relative traffic, not measured link congestion. |
| Storage | Selected mounted volumes, used/free/total GiB | Capacity bars and low-free-space attention. Disconnected selections remain saved. |
| Codex / Claude | Reported remaining allowance, window and reset | Five-hour and weekly windows remain separate. Fresh/stale/waiting states distinguish readings from configuration. |
| Allowance drain | Time-weighted percentage/hour and estimated depletion | Fast-drain attention when measured pace projects depletion before reset. At least two minutes of fresh readings are required. |

Green → amber → hot pink indicates increasing system stress or decreasing AI allowance. Numbers and explanatory **!** badges accompany the color. Supported high temperature can explain pressure; thermal throttling is not measured. Smart resource mode spaces out its own polling and quiets motion under sustained pressure without closing other applications or changing power settings. Hardware savings are not yet measured.

Claude's bridge observes explicit questions and permission requests, briefly reacts at the icon and retains a question badge. Reduced-motion preferences use a static badge. Codex desktop conversations are not observed for questions.

### Everyday tools

| Tool | Available behavior |
| --- | --- |
| Music | Windows system media session playback/seek; opt-in macOS Music control. |
| Calendar | Local ICS file, timezone-aware recurring events; opt-in EventKit Calendar on Mac. |
| Notes | Local notes with B/I/U formatting. |
| Tasks | Completion, favourites and pages. |
| Timer | Deadline-based hour/minute/second wheels and 1/5/25-minute presets. |
| Tray shelf | Persistent local file references; distinct from the OS system tray. |
| Battery and quick actions | Supported battery status and focused utility actions. |
| Camera mirror | Opt-in preview; requires a validated frame and stops when hidden, collapsed or under pressure. |

Denied permissions and absent hardware/services remain visible. Notes, tasks, timer state and file references use bounded recoverable local storage; a timer does not need the HUD to stay expanded.

### Small, interactive and persistent

The alpha.7 candidate offers Mini Nook (200 × 36 collapsed, 480 × 104 hover) and Regular Nook (240 × 40 collapsed, 520 × 112 hover), with a 900 × 192 expanded panel, clamped to the monitor work area. **Preferences → Notch** selects layout, size, hover, auto-close, pin and placement. These settings save independently of utility content. Nook follows selected metrics and theme; Playful and Chaos have distinct finite motion, with stable click targets during expansion and corrected high-DPI placement. The alternate floating pill measures 160 × 56 compressed or 280 × 56 regular. Right-click/Shift+F10 opens Settings, compression, pause/resume, hide, reset position and Quit. Hover panels follow the current monitor and open away from its edges. Tray/menu-bar access remains available while hidden.

Neon Circuit, Cyberpunk Night and Aurora apply to the custom native UI. Quiet, Playful and Chaotic motion react to input and respect reduced-motion preferences. The Rust renderer uses egui/glow, with no HTML/CSS/JavaScript or WebView in its rendering path. The three setup steps are fixed pages without scrolling or an OS title bar.

Existing profiles retain presentation, placement and monitoring choices. Saved Codex connection intent survives retries and restart; stale allowance keeps its original timestamp. Explicit disconnect clears it. Claude migration retargets owned bridge commands without replacing unrelated configuration. Startup is opt-in and a replacement should preserve an existing choice.

Automatic native update checks and low-pressure downloads are configurable. **Check now → Download update → Restart and update** uses signed metadata/archive verification, saved preferences, normal shutdown and replacement acknowledgement. The public alpha.4-to-alpha.6 GUI upgrade passed. Native restart requires the restart action; the legacy WebView automatic-install policy below is separate.

Codex allowance is not ordinary ChatGPT chat quota. Cumulative spent-token counts are not supplied by the connected quota sources; drain measures allowance against elapsed time. The official Codex helper and some platform utility helpers mean a strict single OS process is not guaranteed. Windows/Mac packaging passed; Mac permissions/hardware and resource savings require runtime measurements.

## Legacy WebView release · 0.1.4

The remaining introduction describes the older WebView renderer. Its Anime.js effects, NSIS/universal-DMG packages, extended context menu and automatic-install policy belong to that release.

A tiny, interactive desktop instrument panel that stays out of your way. Start with **Appearance**, choose your neon atmosphere, connect optional AI sources, then set the measurements you want. The default 280 × 56 HUD is a floating side pill and can hide to the system tray or Mac menu bar. Hover reveals measurements and right-click opens quick controls; click opens the full meters and graphs.

Drag the six-dot grip to move the pill, or focus it and use arrow keys to nudge (Shift for one pixel). Its position saves with your profile. Right-click **Compress pill** for a 160 × 56 window showing the most stressed system metric and enabled AI allowance icons. **Uncompress pill** restores the full pill; **Reset position** returns to the selected screen corner. The first configuration page also offers the compressed size.

## Read your machine at a glance

CPU and GPU speedometers, memory bar, upload/download rates and selected-drive capacity share a compact layout. Expand for per-core activity, 60-second traffic history, full drive measurements and provider windows. Percentages, GiB, MiB/s, GHz and reported °C accompany the visuals.

Green → amber → hot pink means increasing stress. AI meters become hotter as allowance runs out. Hover or keyboard-focus a metric for exact measurements. A **!** identifies sustained CPU/GPU/RAM pressure, a high reported temperature or a nearly full drive and explains the cause. Network colors describe relative transfer activity; they do not claim congestion.

GPU selection can follow the busiest reported adapter or one saved adapter. Windows reports system activity and dedicated memory through native counters. Mac reports GPU identity; unsupported global measurements remain unavailable. Meter transitions follow the display refresh rate between hardware samples, then stop when settled.

## Your small neon playground

**Neon Circuit:** acid lime and candy pink. **Cyberpunk Night:** hot pink and yellow. **Aurora:** mint and lavender. Chunky theme tickets, responsive icons and short Anime.js particle bursts add energy. Choose Chaos, Playful or Quiet; reduced-motion preferences suppress animation and keep static question badges. Particle bursts are finite and capped.

## Keep an eye on AI allowance

Connect the official local Codex app-server for Codex account allowance, or enable the reversible Claude Code statusline/hook bridge for shared Claude account allowance. Windows such as five hours appear only when reported. Reset countdowns, remaining percentages and fresh/stale labels make the reading explicit.

After two minutes of fresh readings, the HUD averages allowance consumption against elapsed time. A fast-drain lightning indicator means the current pace predicts depletion before the reported reset. Current quota sources do not report cumulative spent tokens, so allowance drain is labeled as allowance rather than an invented token count. Claude questions and permission requests briefly shake the icon and keep a badge until answered or dismissed.

## Smart enough to quiet itself

Smart resource mode detects sustained CPU/GPU/RAM pressure and high reported temperatures. It slows its own checks, caches expensive measurements longer, and quiets animation. Hover the **AUTO** chip for the reason, polling intervals and suggested actions; expand for the full resource strategy. Fresh healthy readings restore normal monitoring in 20-second steps. Your selected motion style returns automatically. Low drive space stays a capacity warning rather than triggering a compute slowdown.

## Quick controls, simple preferences

Right-click the HUD or press **Shift+F10** to expand, pause/resume, pin, switch theme, quiet motion, refresh, open settings or hide to tray. Arrow keys, Home/End and Esc work in the menu. Settings select GPU, 250–2000ms system sampling, drives, network interface, metrics, thresholds, monitor/corner, text scale, motion and notifications. **Launch Neon HUD at login** defaults off. Closing or minimizing hides the window to the tray; tray/menu-bar **Quit** saves pending preferences and exits. Profiles migrate across versions, and unreadable files are retained for recovery.

## Updates that wait for a quiet moment

Automatic checks run on startup and every six hours. Enable **Automatically install updates and restart** for signed upgrades after a quiet minute, including from the tray. Settings, pending questions, paused monitoring and high pressure defer installation. Fresh readings are checked before the restart, your preferences survive the upgrade, and manual installation remains available. Automatic installation defaults off for existing users.

## Local by design

Settings and minimal readings stay in per-user application data. Provider sign-in stays with the installed provider CLI. No passwords, API keys or transcripts are collected. See the [README](../README.md), [validation evidence](VALIDATION.md), [changelog](../CHANGELOG.md) and [WebView release notes](releases/v0.1.4.md).
