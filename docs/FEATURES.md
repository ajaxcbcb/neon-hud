# Meet Neon HUD

A tiny, interactive desktop instrument panel that stays out of your way. Start with **Appearance**, choose your neon atmosphere, connect optional AI sources, then set the measurements you want. The default 360 × 240 HUD lives in a display corner and can hide to the system tray or Mac menu bar.

## Read your machine at a glance

CPU speedometer, memory bar, upload/download rates and selected-drive capacity share a compact layout. Expand for per-core activity, 60-second traffic history, full drive measurements and provider windows. Percentages, GiB, MiB/s, GHz and reported °C accompany the visuals.

Green → amber → hot pink means increasing stress. AI meters become hotter as allowance runs out. Hover or keyboard-focus a metric for exact measurements. A **!** identifies sustained CPU/RAM pressure, a high reported temperature or a nearly full drive and explains the cause. Network colors describe relative transfer activity; they do not claim congestion.

## Your small neon playground

**Neon Circuit:** acid lime and candy pink. **Cyberpunk Night:** hot pink and yellow. **Aurora:** mint and lavender. Chunky theme tickets, responsive icons and short Anime.js particle bursts add energy. Choose Chaos, Playful or Quiet; reduced-motion preferences suppress animation and keep static question badges. Particle bursts are finite and capped.

## Keep an eye on AI allowance

Connect the official local Codex app-server for Codex account allowance, or enable the reversible Claude Code statusline/hook bridge for shared Claude account allowance. Windows such as five hours appear only when reported. Reset countdowns, remaining percentages and fresh/stale labels make the reading explicit.

After two minutes of fresh readings, the HUD averages allowance consumption against elapsed time. A fast-drain lightning indicator means the current pace predicts depletion before the reported reset. Current quota sources do not report cumulative spent tokens, so allowance drain is labeled as allowance rather than an invented token count. Claude questions and permission requests briefly shake the icon and keep a badge until answered or dismissed.

## Smart enough to quiet itself

Smart resource mode detects sustained CPU/RAM pressure and high reported temperatures. It slows its own checks, caches expensive measurements longer, and quiets animation. Hover the **AUTO** chip for the reason, polling intervals and suggested actions; expand for the full resource strategy. Fresh healthy readings restore normal monitoring in 20-second steps. Your selected motion style returns automatically. Low drive space stays a capacity warning rather than triggering a compute slowdown.

## Quick controls, simple preferences

Right-click the HUD or press **Shift+F10** to expand, pause/resume, pin, switch theme, quiet motion, refresh, open settings or hide to tray. Arrow keys, Home/End and Esc work in the menu. Settings select drives, network interface, metrics, thresholds, monitor/corner, text scale, motion and notifications. **Launch Neon HUD at login** defaults off. Closing hides the window; tray/menu-bar **Quit** exits.

## Local by design

Settings and minimal readings stay in per-user application data. Provider sign-in stays with the installed provider CLI. No passwords, API keys or transcripts are collected. See the [README](../README.md), [validation evidence](VALIDATION.md), [changelog](../CHANGELOG.md) and [preview release notes](releases/v0.1.0.md).
