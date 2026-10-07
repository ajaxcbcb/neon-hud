# Meet Neon HUD

## Native Nook preview

A tiny black capsule opens into a shallow row of tools: music, a week calendar, notes, timer wheels, tasks and quick actions. Nook and Tray switch between the widget overview and a persistent file shelf. Hover peeks, click opens, pin keeps it open and Escape collapses it. The original movable/compressible neon pill remains available in Appearance; both presentations use custom native frames.

The numerical bottom rail keeps CPU, GPU, memory, network, selected drives, Codex and Claude visible. Focused instruments and AI views retain stress gradients, histories, source details, reported reset windows and allowance drain. Missing readings stay unavailable and stale readings keep their original timestamps. Saved connection intent and independent utility storage survive restarts.

Notes include B/I/U; tasks include completion and favourites; deadline timers include hour/minute/second wheels and presets. Windows media controls use system sessions, Mac Music control is opt-in, calendar accepts local recurring ICS events, and Mac can request native Calendar permission. The explicit camera mirror stops when hidden or under pressure. Permission and unsupported-source states are visible. The custom three-step Settings flow retains themes, motion, configured drives, tray access, opt-in startup and signed native updates.

[Nook release notes](releases/v0.2.0-alpha.5.md) · [Native controls](../src-native/README.md) · [Reference evidence](../reference-learning/notchnook/study.md) · [Validation](VALIDATION.md)

## Existing WebView release

The remaining feature introduction describes the established WebView release. Its Anime.js effects and installer packages belong to that renderer.

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

Settings and minimal readings stay in per-user application data. Provider sign-in stays with the installed provider CLI. No passwords, API keys or transcripts are collected. See the [README](../README.md), [validation evidence](VALIDATION.md), [changelog](../CHANGELOG.md) and [preview release notes](releases/v0.1.3.md).
