# Implementation checkpoint

project_path: C:\Users\fuhui\Documents\ChatGPT\neon-hud

Objective: Deliver a small native Windows/Mac HUD, with fixed settings pages, numerical system/AI readings, neon instruments, tray access, retained preferences, opt-in startup and public Apache 2.0/MIT source.

Status: The separate unsigned native v0.2.0-alpha.1 is installed and running on the target Windows machine, with verified Desktop and Start menu shortcuts and a rendered 280 × 56 pill. The original v0.1.4 profile and Claude configuration remain unchanged. The native Rust preview on `codex/native-desktop` passed Windows/Mac tests, locked builds and Windows rendered-page inspection; its public artifact digests are verified. Target-machine Settings access from the pill was inconclusive, and desktop input has stopped.

Next: Publish the verified alpha.2 packages and apply an admitted target-machine update after the current native preview exits normally. Pill Settings/context-menu access remains open. Tray/drag/startup checks need their own runtime reservation. Complete in-process connectors and signed native updates before migration. Keep the stable updater manifest on 0.1.3 until replacement gates pass.

| Job | Owner | Dependency | Resource | State / acceptance |
|---|---|---|---|---|
| v0.1.4 movement/compression | Coordinator | Public exact-run artifacts | Cloud + admitted own-app check | Installed version, installer exit 0, original profile bytes and compression passed; concurrent human testing prevented drag/restart proof |
| Headless Rust core | Native core worker | Optional desktop features | Source | Desktop and headless cloud regression tests passed on both platforms |
| Native pill/settings/instruments | Coordinator | Headless core | Light source + cloud | Fixed pages without scrolling; preview profile and opt-in startup separate from installed app |
| Native packages/render inspection | CI / coordinator | Integrated source | Cloud | Passed; Windows portable ZIP and Apple Silicon Mac app/DMG, unsigned preview; all fixed pages and pill inspected |
| Edge-aware hover | Coordinator | Native pill + cached monitor bounds | Light source + cloud | Passed exact-source Windows/Mac CI; eight Windows cursor positions opened inward and stayed in the work area; native captures inspected |
| Native target-machine check | Coordinator | Verified artifacts + admission | Own-app runtime | Installed and pill inspected; shortcuts and profile preservation passed; Settings from pill inconclusive, input stopped; tray/drag/startup remain open |
| Strict single OS process | Coordinator | In-process provider design | Source + runtime | Open; native renderer removes WebView, existing Codex and Claude helpers remain |
| Native automatic updates | Coordinator | Signed release/restart path | Source + package/runtime | Open; preview does not consume installed updater manifest |

Verified: [v0.1.4 exact-source CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37583511703), 63 frontend tests, checks/build/audit, 30 Windows and 27 Mac core tests, signed updater payloads and all six public asset digests. Installed Windows version 0.1.4, normal exit of the old nine-process tree, installer exit 0, profile byte retention, 280 × 56 to 160 × 56 compression, non-layout preference retention and unchanged Claude configuration hash passed. Earlier minimize/taskbar and own-window restore evidence belongs to 0.1.3. Actual tray-menu actions, 0.1.4 restart/drag, cross-host profile parity, GPU accuracy/frame rate, Mac runtime, OS signing/notarization and full-process resource measurements remain unverified.

Verified native: [exact-source CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37596488001), source `df029bc77a625b628def337ec71285e875c03b4d`; five native tests per platform, 31 Windows and 28 Mac core tests in both feature configurations, formatting, dependency closure, locked builds and packages. All native settings pages, instruments and 160 × 56 pill inspected in CI. Public alpha tag and four asset digests match; archive architecture, project licenses and font notices verified. On 2026-10-07, the public Windows ZIP was installed, its extracted files and executable verified, and the normal 280 × 56 pill inspected. The initial inactive receipt observed one process and no WebView descendants. A later runtime reconciliation observed the native process plus Codex and console children; strict single-process operation remains open. Both original profile copies and Claude configuration remained unchanged; imported native preferences differ only in normalized window position. See [validation](VALIDATION.md) for proof boundaries.

Must keep: honest missing/stale data; provider-reported five-hour windows only; ordinary ChatGPT allowance separate from Codex; no invented token counts; time-weighted allowance drain with reset/staleness handling; reversible explicit Claude hook setup; no credentials/transcripts; reduced motion and pressure backoff; numeric readings alongside gradient gauges/bars; public release notes and licenses.

Native design: [specification](specs/native-desktop.md). Rust/egui/glow draws 160 × 56 and 280 × 56 floating capsules directly, plus a custom neon frame with fixed Appearance/Connections/Preferences steps and separate instruments. Preferences has bounded Instruments and Startup subpages; storage choices use pages. Short hover/question reactions stop under pressure or reduced motion. Preview preferences import once into `native-preview`; shared Claude readings remain in the parent directory.

Acceptance: exact-source locked Windows/Mac tests and builds; native dependency closure without WebView; actual rendered Windows inspection; original-profile retention and save/error handling; target-platform tray/drag/startup evidence; public artifact hashes. A native GUI instance alone does not prove strict single-process operation or measured performance savings.
