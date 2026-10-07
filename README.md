# Neon HUD

The **native Nook preview** is a compact Rust desktop HUD for Windows x64 and Apple Silicon Mac. A small black capsule peeks on hover and expands into media, calendar, notes, tasks, timer and file-reference widgets. CPU/GPU/RAM/network/drives/Codex/Claude readings, allowance drain indicators, tray access and the custom neon Settings remain available.

[Native Nook guide and setup](https://github.com/ajaxcbcb/neon-hud/blob/codex/native-desktop/README.md) · [Native source](https://github.com/ajaxcbcb/neon-hud/tree/codex/native-desktop/src-native) · [Alpha.6 release](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.2.0-alpha.6) · [Features](https://github.com/ajaxcbcb/neon-hud/blob/codex/native-desktop/docs/FEATURES.md) · [Changelog](https://github.com/ajaxcbcb/neon-hud/blob/codex/native-desktop/CHANGELOG.md)

![Native Nook preview with sample readings](https://raw.githubusercontent.com/ajaxcbcb/neon-hud/codex/native-desktop/docs/assets/native-nook.png)

Hosted Windows screenshot with sample readings. Existing profiles retain Pill until **Preferences → Instruments → HUD layout → Nook** is selected; explicit choices persist. Nook peeks on hover, expands on click, pins open and collapses with Escape. Right-click or Shift+F10 opens Controls.

**Windows target status:** Defender quarantined the installed alpha.6 executable. Target launch and Nook replacement are blocked. A separate self-signed development package is [being verified in cloud CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37682274154); it does not establish public publisher trust or Defender clearance and does not change public release assets or the updater feed. See [validation evidence](https://github.com/ajaxcbcb/neon-hud/blob/codex/native-desktop/docs/VALIDATION.md).

The native preview is independent of NotchNook; exact reference animation timing is unmeasured. Apache 2.0/MIT and third-party notices are bundled.

## Legacy WebView edition

The following installation and development instructions describe the earlier Tauri/Svelte edition maintained on `main`. Use the native guide above for the Nook preview.

A small desktop cockpit for CPU, GPU, memory, multiple drives, network traffic and AI allowance. Built with Rust, Tauri 2, Svelte and SVG instruments for Windows and macOS.

[Download the preview release](https://github.com/ajaxcbcb/neon-hud/releases) · [Feature introduction](docs/FEATURES.md) · [Changelog](CHANGELOG.md) · [Release notes](docs/releases/v0.1.4.md)

![Tiny pill HUD showing sample readings](docs/assets/pill.png)

Sample readings above. The floating window is 280 × 56 pixels at default scale; the visible capsule is 272 × 48.

## Install and first launch

Get preview installers from [GitHub Releases](https://github.com/ajaxcbcb/neon-hud/releases). Packages are built by [GitHub Actions](https://github.com/ajaxcbcb/neon-hud/actions). Windows: download the x64 NSIS setup executable and follow the installer. Mac: open the universal DMG and drag Neon HUD to Applications. Preview installers are not OS code-signed or notarized. Application update packages have a separate cryptographic signature verified by Neon HUD.

The first page after installation is **Appearance**: choose Neon Circuit, Cyberpunk Night or Aurora, choose Chaotic, Playful or Quiet motion, then size. The default is a 280 × 56 floating pill on the right side, with six placement choices and optional text scaling. Hover reveals measurements; click opens the larger meters and graphs. A clearly marked sample HUD previews your choices. Next connect optional AI sources, then choose metrics and alerts. System monitoring works without an AI connection. Existing users can select **Appearance → Pill** to switch from an expanded layout.

Hover or focus any metric for numerical details and pressure explanations. Right-click the HUD (or press Shift+F10) for expand, pause/resume, pin, theme, motion, refresh, settings and hide controls. Open the gear or tray/menu-bar **Configure** item to change settings later. **Preferences → Launch Neon HUD at login** enables Windows startup or a macOS login agent; switching it off removes that registration. It defaults to off. Closing the HUD hides it to the tray; use **Quit** to exit.

## Measurements

- CPU: total utilization, per-core bars and reported frequency. Temperatures appear only when supported by the machine.
- GPU: select an adapter or automatically show the busiest reported adapter. Windows uses DXGI identity and PDH activity/dedicated-memory counters. Mac uses Metal identity; global activity, memory usage and temperature remain unavailable where the supported source cannot report them. Expand for every adapter and its capability/source details.
- RAM: used/total GiB and utilization percentage.
- Network: download/upload MiB/s, 60-second bars and interface transfer totals. Automatic selection uses one default-route interface, avoiding aggregate VPN double counting. Select another interface if the route cannot be determined.
- Storage: mounted-volume capacity percentage and used/free/total GiB. Select drives in Preferences or include all detected drives automatically. Disconnected selections remain in settings until forgotten.
- Stress: green → amber → hot pink gradients on gauges and bars. AI allowance reverses the scale as remaining allowance falls. Numerical readings and **!** badges accompany colors. CPU/GPU/RAM badges require 10 seconds of sustained pressure; high reported temperature and low free space warn immediately. Thresholds are configurable; thermal throttling is not measured. Network colors show recent relative activity, not link capacity or proof of congestion.
- AI: percentage remaining, provider-reported windows and reset times. A five-hour window appears only if the provider reports one. Weekly and other windows remain separate. Missing data is unavailable; old data is marked stale; expired timers await a refresh.
- Drain: a time-weighted average of allowance consumption over up to 30 minutes, requiring at least two minutes of fresh readings. Shows percentage/hour and estimated time to exhaustion. **Fast drain** means exhaustion is projected before the reported reset. This estimate assumes the same pace; it is not a provider guarantee. Cached snapshots do not count as new measurements; resets restart the observation. Allowance percentage is not a token count.

Exact cumulative token counts are unavailable from the connected quota sources. The fast-drain indicator therefore measures reported allowance consumption against elapsed time. Claude context-window token counts describe the current context, not cumulative consumption, and are not relabeled as tokens spent.

## AI connections

**Codex:** install the official Codex CLI and select Connect Codex. Neon HUD uses its supported local app-server and provider-managed sign-in. It reads Codex account allowance; this does **not** describe ordinary ChatGPT chat quotas. An automatic supported ChatGPT chat allowance source is unavailable in this version. Existing Codex desktop conversations are not observed for questions.

**Claude:** install Claude Code, sign in with `claude auth login`, then select Enable bridge. The button shows progress immediately and confirms the installed statusline and seven hooks. **Waiting for readings** means configuration succeeded but no supported account-limit reading has arrived. Start or resume Claude Code and use it normally; the local statusline receives supported account-limit readings when Claude Code supplies them. Claude and Claude Code share these limits when using the same account; the HUD does not add the two together. The bridge also observes explicit questions and permission requests. The icon shakes briefly, then retains a question badge until resolved or dismissed. Reduced-motion preferences replace the shake with a static badge.

The Claude bridge backs up settings, preserves other hooks and chains an existing statusline. **Remove bridge before uninstalling Neon HUD**, so Claude settings do not reference a removed executable. Removal changes only the HUD's configuration; your unrelated settings stay in place. No automatic approvals are issued.

## Privacy and performance

Settings and minimal readings stay in per-user application data. No telemetry, passwords, API keys or conversation content are collected. Provider authentication stays with the installed provider CLI. The interface uses bundled Latin fonts, SVG gauges and CSS bars. Anime.js adds short interaction bursts capped at 24 particles, removed after completion. Chaotic mode adds staggered transform animations to small icons; Playful uses interaction motion and Quiet disables motion. System reduced-motion preferences override all modes.

Smart resource mode is on by default. Sustained CPU/GPU/RAM pressure or high reported temperature slows checks and quiets motion without changing your saved motion choice. The AUTO chip and expanded resource strategy show the reason and suggested actions. Hidden UI polling pauses; manual refresh remains available while paused. The app adjusts its own budget; it does not close other apps or change OS power settings.

| Resource mode | CPU/RAM/network | GPU | Sensors | AI checks | Drive cache | Route cache | CPU clock cache |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Normal | 250ms default; 250/500/1000/2000ms selectable | 1s | 2s | 5s | 30s | 30s | 15s |
| Pressure | 4s | 4s | 4s | 10s | 90s | 60s | 30s |
| Critical | 8s | 8s | 8s | 15s | 180s | 120s | 60s |

Critical mode starts after sustained ≥98% CPU/GPU or ≥97% memory (subject to higher configured thresholds), or a sensor ≥10°C above its configured threshold. Recovery needs 20 seconds of fresh readings comfortably below thresholds per step. Missing/stale readings cannot prove recovery. Turn adaptation off in Preferences for the normal budget. Provider refresh cadence may be slower than UI checks; cached readings are not relabeled as new.

Gauges and bar widths use finite 180ms display-synced transitions, with no fixed 60 Hz cap. Numerical values remain the latest measured samples. Idle meters schedule no animation frames; hidden and reduced-motion displays settle immediately. Actual native frame rate requires measurement on the installed machine.

Performance targets are under 1% idle CPU and a small full-process memory footprint. These are targets until measured on each platform, including WebView and provider helper processes. See [validation](docs/VALIDATION.md) for evidence and limitations.

## Application updates

Preferences enables automatic update checks at startup and every six hours, including while hidden in the tray. Automatic checks defer during high resource pressure. Available packages download in the background and must pass signature verification before installation.

From 0.1.3, enable **Automatically install updates and restart** to authorize future upgrades. A verified package waits at least one minute, with no HUD interaction during the last minute. Open settings or provider details, pending questions, connection attempts, paused monitoring, unsaved preferences and high resource pressure defer the restart. A fresh system/provider preflight also runs from the tray before installation; unavailable readings defer it. Only one preflight per minute runs while a verified package is waiting. Existing users keep automatic installation off until they choose it. Disable either update switch to prevent automatic installation; **Install & restart** remains available manually. An installation failure waits for a manual retry rather than restarting repeatedly.

Updates use the public [`updates/latest.json`](updates/latest.json) manifest and versioned GitHub Release assets. The first 0.1.0 installation must be upgraded with the installer. See [release maintenance](docs/releases/MAINTENANCE.md) for the signed publishing gate.

## Develop

Install Node.js 24, Rust stable and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npm run verify
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

Browser-only preview: `npm run dev`. It displays sample data only on the appearance page; real system and account sources require the native app.

Unsigned developer Windows installer: `npm run tauri build -- --bundles nsis --config src-tauri/tauri.pr.conf.json`.
Unsigned developer Mac universal installer: add Rust targets `aarch64-apple-darwin` and `x86_64-apple-darwin`, then `npm run tauri build -- --target universal-apple-darwin --bundles dmg --config src-tauri/tauri.pr.conf.json`. Trusted CI builds require the repository's update signing secret; fork pull requests use the unsigned developer configuration.

## License

Licensed under **MIT OR Apache-2.0**, at your option. See [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), [NOTICE](NOTICE) and [third-party notices](THIRD-PARTY-NOTICES.md). Provider names and logos identify integrations; they remain their owners' trademarks. This project is independent and is not endorsed by OpenAI or Anthropic.
