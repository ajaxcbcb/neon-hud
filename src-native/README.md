# Neon HUD native preview

A Rust desktop utility drawn directly with egui/glow. There is no WebView or scrolling dashboard. The tiny floating pill is 160 × 56 logical pixels; regular mode is 280 × 56. Settings and instruments open in separate fixed native windows.

Build with `cargo run --manifest-path src-native/Cargo.toml --locked --release` on Windows or macOS. New profiles start compressed; imported profiles keep their saved size. Windows previews are portable ZIPs; macOS previews are `.app` bundles and DMGs. Preview packages are unsigned. Hosted builds and runtime checks are recorded in the repository validation log.

Drag the dotted grip to move, double click it to compress, click an icon for readings, hover for a quick peek, and right click for controls. Arrow keys move the focused pill; Shift moves one logical pixel per press. Escape hides it in the tray. Settings uses a custom neon frame with a drag header, theme cards, motion tiles and a sample pill preview. Appearance, Connections and Preferences are fixed steps; drive choices use pages. There is no scrolling or conventional OS title bar.

The preview imports your existing profile once into `io.github.ajaxcbcb.neonhud/native-preview`, then saves that separate profile. Startup is opt-in under **Neon HUD Native Preview**, separate from the installed HUD. A profile that cannot be read is retained and changes are disabled rather than overwriting it.

Claude's bridge continues to read and write in the shared parent app directory so its existing command hooks can supply both renderers. Enabling or disabling those hooks is an explicit global Claude configuration action; ordinary preview launch does not change them.

System readings use the existing Rust monitor. Pressure spaces out polling and disables animation. Codex and Claude use the existing connectors when explicitly connected; these connectors still use helpers. Ordinary ChatGPT allowance and unsupported temperature/token readings are displayed as unavailable. Five-hour limits and drain averages come from reported allowance windows, never an invented token budget.

The installed v0.1.4 remains supported. This preview does not replace it or consume its updater manifest. Signed native automatic updates, strict single-process connectors and target-machine native tray/drag/startup verification remain open before migration.

Licensed under Apache 2.0 or MIT, at your choice. See the repository licenses and notices.
