# Native desktop direction

The HUD is a small desktop accessory. Rust/egui draws it directly in native OS windows; the native executable has no HTML, CSS, JavaScript, WebView or scrolling dashboard.

- Compressed: 160 × 56 logical pixels. Regular: 280 × 56. Rounded dark capsule, tiny lime/pink instrument icons, monospace readings, separate drag grip.
- Double click the grip to compress. Drag it to move. Arrow keys nudge; Shift makes a one-pixel adjustment. Right click opens quick controls. Hide goes to the system tray.
- Settings: one fixed 620 × 460 window with Appearance first, then Metrics, Connections and Startup. Drives use pages. Details use a separate fixed window. No scroll areas.
- Neon Circuit (lime/pink), Cyberpunk Night (pink/cyan), Aurora (mint/violet) share one visual language. Brief hover bounce and question shake stop under pressure or reduced motion. No constant idle animation.
- Gradients communicate stress and remaining allowance; numbers and exclamation marks carry the same meaning. Unknown telemetry stays unknown. Codex and ordinary ChatGPT allowance remain separate; only provider-reported five-hour windows are shown.

The preview copies the old profile into a separate `native-preview` directory once, then persists its own preferences. It never rewrites the installed WebView HUD profile on launch. Preview startup has its own registration and is opt-in. Existing automatic-update preferences are retained as data; the preview does not consume the stable updater manifest.

Acceptance: Windows/Mac native builds and core regression tests; actual hosted Windows render; bounded settings content; profile roundtrip and drain reset/staleness tests; tray/drag/startup runtime proof on the target OS. The native UI removes WebView helpers, but current Codex transport and Claude command hooks still use child processes. Strict single-process connectors and signed native automatic upgrades remain separate unfinished gates.
