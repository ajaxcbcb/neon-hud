# Implementation checkpoint

project_path: neon-hud

Objective: Publish a lightweight Windows/Mac neon HUD and installer source, with appearance-first configuration, supported AI quota sources, adaptive resources, stress gradients, release documentation and dual licensing.

Status: v0.1.3 public preview and signed updater packages verified and installed on Windows. Active profile choices and both enabled update preferences survived relaunch. Minimize hid the window without ending monitoring or leaving a named taskbar entry; own-window restoration passed. 63 frontend tests, 29 Windows and 26 Mac tests, frontend checks/build/audit/formatting and independent review passed. Actual tray-menu restoration and strict single-process operation remain open. Priority P1.

Next: Verify actual tray-menu restoration/Quit and profile parity when launched outside the Windows packaged host. Resolve strict single-process connector and updater constraints before replacing the WebView renderer. Observe the next automatic upgrade when a newer verified release exists. GPU accuracy/frame rate and full-process resource measurements remain separate checks.

| Job | Owner | Dependency | Resource | State / acceptance |
|---|---|---|---|---|
| GPU provider and recovery | Backend worker / coordinator | Independent review | Source write | Complete; review findings resolved |
| Frontend integration | Coordinator | GPU contract | Light local | Complete; 63 tests, zero check warnings/errors, build and audit |
| Native tests and packages | GitHub CI | Source integration | Cloud | Complete; source 44b579e2, 29 Windows / 26 Mac tests, signed payloads and Windows launch/minimize |
| Public preview and manifest | Coordinator | Verified artifacts | Release write | Complete; v0.1.3, six matching uploaded asset hashes and both cryptographic signature checks |
| GPU visual inspection | Coordinator | Host visual admission | Isolated browser | Queued; prepared fixture reuses retained profile |
| Windows upgrade and persistence | Coordinator | Verified installer | Local installer / own-app checks | Complete; installed 0.1.3, original profile values retained, both update preferences enabled, minimize and own-window restore passed |
| Tray and profile-path parity | Coordinator | Installed runtime | Own-app checks | Open; actual tray-menu actions and launch outside packaged host |
| Strict single OS process | Coordinator | Native architecture constraints | Source design / native checks | Open; current WebView2, Codex and installer child processes exceed the requirement |

Current refinement: v0.1.3 retains partial legacy profiles, preserves unreadable files, serializes and flushes preferences before Quit/restart/update, minimizes to tray and optionally installs verified updates after fresh quiet/pressure checks. Independent review passed. Source 44b579e2, cloud native tests/packages, release digests and cryptographic signatures verified. Installed Windows checks confirmed version, active-profile retention, enabled update preferences and minimize behavior; no local native build or hardware resource measurement was needed.

GPU/high-refresh increment: the coordinator owns sampling and UI integration; a backend worker owns the GPU provider module and dependency declarations. Acceptance covers multiple adapters, honest unavailable/stale fields, cached native discovery, GPU utilization/memory/temperature where supported, isolated hardware/animation/quota cadences, hidden-window backoff, pressure/reduced-motion handling and timer cleanup. Fast sampling is adjustable down to 250 ms for cheap counters; animation follows the display without a fixed 60 Hz cap. Native frame rate and telemetry accuracy require separate installed-platform evidence.

Verified: 63 frontend tests, Svelte checks, production build, dependency audit, Rust formatting, independent review, 29 Windows and 26 Mac native tests, both platform packages, signed updater payloads and six uploaded public asset hashes. Installed Windows 0.1.3 runtime, compact interface, active-profile relaunch retention, both update preferences, minimize/taskbar absence and own-window restoration passed. Earlier evidence remains in VALIDATION.md. Claude Code authentication and bridge setup are confirmed; live quota, actual tray-menu actions, cross-host-launch profile parity, future automatic-upgrade execution, GPU accuracy/frame rate, native Mac runtime, OS signing/notarization and full-process resource measurements remain unverified. Strict single-process operation is not implemented.

Must keep: honest missing/stale data; provider-reported five-hour windows only; separate ordinary ChatGPT and Codex quota; shared Claude limits; no credentials or transcripts; reversible Claude hook setup; optional login startup; reduced motion; numerical measurements alongside SVG gauges and bars.

Design direction: a 280 × 56 floating utility window containing a rounded neon capsule, expressive tiny metric icons and readable numerals. Hover/focus opens exact measurements; click opens existing meters/graphs; right click opens quick controls. Punchy lime/pink, springy icon reactions and brief question shakes give it mischievous personality without continuous idle animation. Pressure management quiets motion and spaces out monitoring. Neon Circuit is the default; Cyberpunk Night and Aurora use the same hierarchy. Appearance is the first configuration step. Connection buttons immediately show progress and report connected, sign-in needed, waiting for readings or failed from actual source state. Rust handles monitoring and tray/startup; Tauri's desktop WebView renders the interface.

Acceptance: frontend type checks/tests/build; Rust parsing/settings/hook tests; no known high-severity npm audit findings; screenshot inspection at desktop and 400px; first-run flow and persisted preferences; Windows NSIS and universal Mac DMG packaging checks; verify public GitHub visibility and licenses. Platform install/runtime and performance measurements must be explicitly recorded as unverified when unavailable.
