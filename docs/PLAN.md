# Implementation checkpoint

project_path: C:\Users\fuhui\Documents\ChatGPT\neon-hud

Objective: Deliver a small native Windows/Mac HUD, with fixed settings pages, numerical system/AI readings, neon instruments, tray access, retained preferences, opt-in startup and public Apache 2.0/MIT source.

Status: Windows v0.1.4 is installed and the human is actively testing it. The native Rust preview is implemented on `codex/native-desktop`; Windows/Mac cloud compilation, tests, packaging and screenshot checks are in progress. Native local installation has not run. Keep the installed app and its active profile untouched during human testing.

Next: Resolve cloud checks, inspect actual native screenshots, publish verified preview artifacts and record their limits. Target-machine tray/drag/startup checks need their own runtime reservation. Keep the stable updater manifest on 0.1.3 until replacement gates pass.

| Job | Owner | Dependency | Resource | State / acceptance |
|---|---|---|---|---|
| v0.1.4 movement/compression | Coordinator | Public exact-run artifacts | Cloud + admitted own-app check | Installed version, installer exit 0, original profile bytes and compression passed; concurrent human testing prevented drag/restart proof |
| Headless Rust core | Native core worker | Optional desktop features | Source | Implemented; cloud regression tests in progress |
| Native pill/settings/instruments | Coordinator | Headless core | Light source + cloud | Fixed pages without scrolling; preview profile and opt-in startup separate from installed app |
| Native packages/render inspection | CI / coordinator | Integrated source | Cloud | In progress; Windows portable ZIP and Mac app/DMG, unsigned preview |
| Native target-machine check | Coordinator | Verified artifacts + admission | Own-app runtime | Pending; preserve human testing and profile |
| Strict single OS process | Coordinator | In-process provider design | Source + runtime | Open; native renderer removes WebView, existing Codex and Claude helpers remain |
| Native automatic updates | Coordinator | Signed release/restart path | Source + package/runtime | Open; preview does not consume installed updater manifest |

Verified: [v0.1.4 exact-source CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37583511703), 63 frontend tests, checks/build/audit, 30 Windows and 27 Mac core tests, signed updater payloads and all six public asset digests. Installed Windows version 0.1.4, normal exit of the old nine-process tree, installer exit 0, profile byte retention, 280 × 56 to 160 × 56 compression, non-layout preference retention and unchanged Claude configuration hash passed. Earlier minimize/taskbar and own-window restore evidence belongs to 0.1.3. Actual tray-menu actions, 0.1.4 restart/drag, cross-host profile parity, GPU accuracy/frame rate, Mac runtime, OS signing/notarization and full-process resource measurements remain unverified.

Must keep: honest missing/stale data; provider-reported five-hour windows only; ordinary ChatGPT allowance separate from Codex; no invented token counts; time-weighted allowance drain with reset/staleness handling; reversible explicit Claude hook setup; no credentials/transcripts; reduced motion and pressure backoff; numeric readings alongside gradient gauges/bars; public release notes and licenses.

Native design: [specification](specs/native-desktop.md). Rust/egui/glow draws 160 × 56 and 280 × 56 floating capsules directly, plus a custom neon frame with fixed Appearance/Connections/Preferences steps and separate instruments. Preferences has bounded Instruments and Startup subpages; storage choices use pages. Short hover/question reactions stop under pressure or reduced motion. Preview preferences import once into `native-preview`; shared Claude readings remain in the parent directory.

Acceptance: exact-source locked Windows/Mac tests and builds; native dependency closure without WebView; actual rendered Windows inspection; original-profile retention and save/error handling; target-platform tray/drag/startup evidence; public artifact hashes. A native GUI instance alone does not prove strict single-process operation or measured performance savings.
