# Neon HUD native preview · 0.2.0-alpha.7 candidate

A Rust desktop utility drawn directly with egui/glow. **Nook** offers Mini (200 × 36, hover 480 × 104) and Regular (240 × 40, hover 520 × 112) logical-pixel capsules, with a 900 × 192 shallow expanded panel, clamped to the monitor work area. Its Home view shows simultaneous media, calendar, notes, timer, tasks and quick actions above a numerical system/AI rail. Smaller screens retain additional tools through icon views. The 160 × 56 compressed and 280 × 56 regular floating pills remain selectable. Settings and detailed instruments use separate fixed native windows without scrolling.

Build with `cargo run --manifest-path src-native/Cargo.toml --locked --release` on Windows or macOS. Imported profiles keep their existing presentation and saved pill position; choose **Preferences → Notch → HUD layout → Nook** to switch. Fresh profiles start in Nook. Windows previews are portable ZIPs; macOS previews are `.app` bundles and DMGs. Hosted builds and runtime checks are recorded in the repository validation log; development signatures are separate from public publisher trust.

[Native downloads](https://github.com/ajaxcbcb/neon-hud/releases) · [Nook release notes](../docs/releases/v0.2.0-alpha.6.md) · [Validation](../docs/VALIDATION.md). The Mac package supports Apple Silicon (arm64). The native renderer requires OpenGL 2.0 or later. Mac utility permissions and hardware behavior still require target-platform runtime verification.

In the floating pill, drag the dotted grip to move, double click it to compress, click an icon for readings, hover for a quick peek, and right click for controls. Arrow keys move the focused pill; Shift moves one logical pixel per press. Escape hides the pill to the tray. Settings uses a custom neon frame with a drag header, theme cards, motion tiles and a sample pill preview. Appearance, Connections and Preferences are fixed steps; drive choices use pages. There is no scrolling or conventional OS title bar.

## Install and configure

1. Download the [alpha.6 release](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.2.0-alpha.6) for Windows x64 or Apple Silicon Mac. Extract the complete Windows ZIP into a permanent folder; on Mac copy the app from the DMG to Applications or extract the app ZIP.
2. Run the app and start at **Appearance**. Choose Neon Circuit, Cyberpunk Night or Aurora, then Quiet, Playful or Chaotic motion. The appearance preview is labeled sample data.
3. Open **Connections** for optional Codex and Claude sources. System monitoring works while these are disconnected or waiting for readings.
4. Open **Preferences → Notch** for Nook/Pill, closed size, hover, auto-close, pin and placement. **Instruments** controls metrics, drives and monitoring; **Startup & updates** controls login startup and signed native updates.

For an upgrade, leave the profile and shared bridge data intact, use the existing updater when supported, and retire the previous package only after the replacement launches. A portable ZIP has no NSIS setup/uninstaller. Keep bundled licenses and notices; update signatures are separate from OS code signing.

Existing profiles retain Pill unless Nook is selected in Preferences. Layout lives in `nook-productivity.json` under `presentation.mode`; other settings, notes, tasks and timers must be retained when changing it. The target alpha.6 executable was quarantined by Defender, so its launch/layout acceptance is pending. The separate [development-signed build](https://github.com/ajaxcbcb/neon-hud/actions/runs/37689111604) passed signature/timestamp verification, exact temporary certificate/key cleanup and real Nook input. [Development signing](../docs/releases/MAINTENANCE.md#self-signed-windows-development-build) does not establish public publisher trust or Defender clearance, and this test package is outside the automatic-update feed.

## Interaction

Right-click or **Shift+F10** opens a separate native Controls popup with Settings, compression, pause/resume, hide, reset position and Quit. Controls opens on button press, so hover expansion cannot consume the click. Popovers and controls stay inside the current monitor and open inward at its edges. Hiding preserves tray/menu-bar access; use Quit to exit.

In Nook, hover opens a peek and click expands. Pin keeps the panel open when the pointer leaves; Escape collapses it. Nook is the widget overview and Tray is a persistent file-reference shelf. Icon views provide instruments, AI, notes, tasks, timer, media, calendar, mirror and quick actions. Notes support B/I/U, tasks have completion and favourites, and timer wheels accept scroll/click/drag with 1/5/25-minute presets. Notes, tasks, deadline timers, file references and utility choices save separately using bounded atomic storage and recoverable errors.

Windows media controls use the active system media session; macOS Music control is opt-in. Calendar accepts a local ICS file with timezone-aware recurrence; macOS also offers opt-in native Calendar access. The mirror requires explicit camera permission and an actual valid frame before reporting connected. Hiding or collapsing Nook, leaving the mirror view or entering resource pressure stops capture and releases the texture. Unsupported hardware or denied permissions remain visible as unavailable states.

## Profiles and connections

The preview imports your existing profile once into `io.github.ajaxcbcb.neonhud/native-preview`, then saves that separate profile with debounced writes. Existing/imported presentations and saved pill placement survive the Nook upgrade; fresh profiles start in Nook. Startup is opt-in under **Neon HUD Native Preview**; a replacement installation should retain a previously enabled startup choice. A profile that cannot be read is retained and changes are disabled rather than overwriting it.

Claude's bridge continues to read and write in the shared parent app directory so its existing command hooks can supply both renderers. Enabling or disabling those hooks is an explicit global Claude configuration action; ordinary preview launch does not change them.

Codex connection intent and last-known allowance persist through retry errors and restart. Alpha.7 fixes the installed profile's erroneous test-mode guard so Connect/Disconnect and restoration can run. The original reading timestamp and pending/error state remain visible until a supported fresh reading arrives; explicit disconnect clears the retained connection. Connections displays full action feedback. Claude bridge migration recognises owned Windows version directories and macOS versioned bundles while preserving other hooks/statuslines. After enabling the bridge, start or restart Claude Code to send readings and questions. An enabled bridge can be waiting for data; provider sign-in remains managed by the installed provider CLI. Explicit smoke mode uses temporary profiles and blocks real connection actions.

System readings use the existing Rust monitor. Pressure spaces out polling and disables animation. Codex uses its official app-server helper; some platform utilities also require supported helpers. Ordinary ChatGPT allowance and unsupported temperature/token readings are displayed as unavailable. Five-hour limits and drain averages come from reported allowance windows, never an invented token budget. Source failures, stale values and permissions are labeled explicitly.

## Updates and validation

Preferences → Startup & updates controls opt-in login startup, automatic checks and automatic low-pressure downloads. **Check now → Download update → Restart and update** verifies signed metadata and archives, saves preferences, waits for normal shutdown and requires a startup acknowledgement from the replacement. Native restart is an explicit action, separate from automatic download. The [public alpha.4 → alpha.6 GUI update](https://github.com/ajaxcbcb/neon-hud/actions/runs/37663561512) passed real input, tamper rejection, retained Pill/settings/startup and replacement Controls/Settings/Quit. Target-machine installation evidence is recorded separately in [Validation](../docs/VALIDATION.md).

The WebView release's stable manifest is unchanged. Strict connector-inclusive single-process operation, Mac permission/hardware runtime and full-process performance measurements remain open. Reference states were observed; exact NotchNook motion timing is unmeasured.

Licensed under Apache 2.0 or MIT, at your choice. See the repository licenses and notices.
