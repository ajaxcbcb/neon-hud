# Implementation checkpoint

project_path: neon-hud

Objective: Publish a lightweight Windows/Mac neon HUD and installer source, with appearance-first configuration, supported AI quota sources, adaptive resources, stress gradients, release documentation and dual licensing.

Status: v0.1.2 public preview. Frontend checks, independent backend review, Windows/Mac native tests, signed packaging, Windows cloud launch and artifact verification passed. Local visual/native installation waits for shared host admission; installed 0.1.0 remains in place. Priority P1.

Next: After shared host admission, reuse the isolated browser profile to inspect the GPU pill and settings, then install the verified Windows update. Check the installed UI, GPU readings and updater. Measure actual frame rate and full-process resource use only when that measurement lane is admitted. A real Claude Code session is still required for provider quota evidence.

| Job | Owner | Dependency | Resource | State / acceptance |
|---|---|---|---|---|
| GPU provider and recovery | Backend worker / coordinator | Independent review | Source write | Complete; review findings resolved |
| Frontend integration | Coordinator | GPU contract | Light local | Complete; 51 tests, zero check warnings/errors, build and audit |
| Native tests and packages | GitHub CI | Source integration | Cloud | Complete; source 1fa44bc, 26 Windows / 23 Mac tests, signed payloads and Windows launch |
| Public preview and manifest | Coordinator | Verified artifacts | Release write | Complete; v0.1.2, five matching CI asset hashes and both cryptographic signature checks |
| GPU visual inspection | Coordinator | Host visual admission | Isolated browser | Queued; prepared fixture reuses retained profile |
| Windows upgrade and measurements | Coordinator | Verified installer and host admission | Local installer / measurement | Queued; existing installed 0.1.0 retained |

Current refinement: GPU telemetry, adapter selection, fast sampling and finite display-synced animation are integrated. Independent review's Windows counter-recovery finding is resolved. All 51 frontend tests, Svelte checks, production build, Rust formatting and dependency audit passed. Windows/Mac tests, signed packaging and Windows launch passed in cloud CI; artifact hashes and update signatures verified before publication. Installation is queued; no local native compilation is reserved. Release writes and installation run sequentially; runtime estimates are unknown.

GPU/high-refresh increment: the coordinator owns sampling and UI integration; a backend worker owns the GPU provider module and dependency declarations. Acceptance covers multiple adapters, honest unavailable/stale fields, cached native discovery, GPU utilization/memory/temperature where supported, isolated hardware/animation/quota cadences, hidden-window backoff, pressure/reduced-motion handling and timer cleanup. Fast sampling is adjustable down to 250 ms for cheap counters; animation follows the display without a fixed 60 Hz cap. Native frame rate and telemetry accuracy require separate installed-platform evidence.

Verified: 51 frontend unit tests, Svelte checks, production build, dependency audit, independent backend review, 26 Windows and 23 Mac native tests, both platform packages, Windows cloud launch, signed updater payloads and public asset hashes. Earlier browser-fixture and installed 0.1.0 appearance evidence is recorded in VALIDATION.md. Claude Code is authenticated and its status line plus seven bridge hooks are configured; live provider quota, installed 0.1.2 UI/GPU/frame rate, native Mac installation/runtime, OS signing/notarization and full-process resource measurements remain unverified.

Must keep: honest missing/stale data; provider-reported five-hour windows only; separate ordinary ChatGPT and Codex quota; shared Claude limits; no credentials or transcripts; reversible Claude hook setup; optional login startup; reduced motion; numerical measurements alongside SVG gauges and bars.

Design direction: a 280 × 56 floating utility window containing a rounded neon capsule, expressive tiny metric icons and readable numerals. Hover/focus opens exact measurements; click opens existing meters/graphs; right click opens quick controls. Punchy lime/pink, springy icon reactions and brief question shakes give it mischievous personality without continuous idle animation. Pressure management quiets motion and spaces out monitoring. Neon Circuit is the default; Cyberpunk Night and Aurora use the same hierarchy. Appearance is the first configuration step. Connection buttons immediately show progress and report connected, sign-in needed, waiting for readings or failed from actual source state. Rust handles monitoring and tray/startup; Tauri's desktop WebView renders the interface.

Acceptance: frontend type checks/tests/build; Rust parsing/settings/hook tests; no known high-severity npm audit findings; screenshot inspection at desktop and 400px; first-run flow and persisted preferences; Windows NSIS and universal Mac DMG packaging checks; verify public GitHub visibility and licenses. Platform install/runtime and performance measurements must be explicitly recorded as unverified when unavailable.
