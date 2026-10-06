# Neon HUD

A small desktop cockpit for CPU, memory, network traffic and AI allowance. Built with Rust, Tauri 2, Svelte and SVG instruments for Windows and macOS.

## Install and first launch

Preview installers are built by [GitHub Actions](https://github.com/ajaxcbcb/neon-hud/actions). Windows: download the x64 NSIS setup executable and follow the installer. Mac: open the universal DMG and drag Neon HUD to Applications. Preview builds are unsigned; signed production distribution requires platform certificates and Mac notarization.

The first page after installation is **Appearance**: choose Neon Circuit, Cyberpunk Night or Aurora, choose Chaotic, Playful or Quiet motion, then size. The default floating HUD is 360 by 210 pixels; placement and readability controls are tucked into an expandable section. A clearly marked sample HUD previews your choices. Next connect optional AI sources, then choose metrics and alerts. System monitoring works without an AI connection.

Open the gear or tray/menu-bar **Configure** item to change settings later. **Preferences → Launch Neon HUD at login** enables Windows startup or a macOS login agent; switching it off removes that registration. It defaults to off. Closing the HUD hides it to the tray; use **Quit** to exit.

## Measurements

- CPU: total utilization, per-core bars and reported frequency. Temperatures appear only when supported by the machine.
- RAM: used/total GiB and utilization percentage.
- Network: download/upload MiB/s, 60-second bars and interface transfer totals. Automatic selection uses one default-route interface, avoiding aggregate VPN double counting. Select another interface if the route cannot be determined.
- AI: percentage remaining, provider-reported windows and reset times. A five-hour window appears only if the provider reports one. Weekly and other windows remain separate. Missing data is unavailable; old data is marked stale; expired timers await a refresh.
- Drain: a time-weighted average of allowance consumption over up to 30 minutes, requiring at least two minutes of fresh readings. Shows percentage/hour and estimated time to exhaustion. **Fast drain** means exhaustion is projected before the reported reset. This estimate assumes the same pace; it is not a provider guarantee. Cached snapshots do not count as new measurements; resets restart the observation. Allowance percentage is not a token count.

Exact cumulative token counts are unavailable from the connected quota sources. The fast-drain indicator therefore measures reported allowance consumption against elapsed time. Claude context-window token counts describe the current context, not cumulative consumption, and are not relabeled as tokens spent.

## AI connections

**Codex:** install the official Codex CLI and select Connect Codex. Neon HUD uses its supported local app-server and provider-managed sign-in. It reads Codex account allowance; this does **not** describe ordinary ChatGPT chat quotas. An automatic supported ChatGPT chat allowance source is unavailable in this version. Existing Codex desktop conversations are not observed for questions.

**Claude:** install Claude Code, then select Enable bridge. A local statusline receives supported account-limit readings after an assistant response. Claude and Claude Code share these limits when using the same account; the HUD does not add the two together. The bridge also observes explicit questions and permission requests. The icon shakes briefly, then retains a question badge until resolved or dismissed. Reduced-motion preferences replace the shake with a static badge.

The Claude bridge backs up settings, preserves other hooks and chains an existing statusline. **Remove bridge before uninstalling Neon HUD**, so Claude settings do not reference a removed executable. Removal changes only the HUD's configuration; your unrelated settings stay in place. No automatic approvals are issued.

## Privacy and performance

Settings and minimal readings stay in per-user application data. No telemetry, passwords, API keys or conversation content are collected. Provider authentication stays with the installed provider CLI. The interface uses bundled assets and system fonts, no charting framework. Chaotic mode adds staggered transform animations to small icons; Playful uses interaction motion and Quiet disables motion. System reduced-motion preferences override all modes. System polling is every two seconds; Codex quota polling is slower. Hidden UI polling pauses.

Performance targets are under 1% idle CPU and a small full-process memory footprint. These are targets until measured on each platform, including WebView and provider helper processes. See [validation](docs/VALIDATION.md) for evidence and limitations.

## Develop

Install Node.js 24, Rust stable and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npm run verify
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

Browser-only preview: `npm run dev`. It displays sample data only on the appearance page; real system and account sources require the native app.

Windows installer: `npm run tauri build -- --bundles nsis`.
Mac universal installer: add Rust targets `aarch64-apple-darwin` and `x86_64-apple-darwin`, then `npm run tauri build -- --target universal-apple-darwin --bundles dmg`.

## License

Licensed under **MIT OR Apache-2.0**, at your option. See [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), [NOTICE](NOTICE) and [third-party notices](THIRD-PARTY-NOTICES.md). Provider names and logos identify integrations; they remain their owners' trademarks. This project is independent and is not endorsed by OpenAI or Anthropic.
