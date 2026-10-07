# Implementation checkpoint

project_path: neon-hud

Objective: Publish a lightweight Windows/Mac neon HUD and installer source, with appearance-first configuration, supported AI quota sources, adaptive resources, stress gradients, release documentation and dual licensing.

Status: v0.1.0 delivered. v0.1.1 refinement in progress: tiny floating pill, transparent desktop utility window, and responsive source connection feedback. Priority P1.

Next: Package the verified v0.1.1 source in cloud CI, publish matching signed update assets and manifest, then install the exact Windows artifact after shared host admission. No local native build or installer is running.

| Job | Owner | Dependency | Resource | State / acceptance |
|---|---|---|---|---|
| Bridge recovery repair | Backend worker | Focused review | Light local | Complete; regression passed on Windows and Mac |
| Source integration | Coordinator | Startup repair | Repository write | Complete; dedicated monitor thread and launch regression |
| Native tests and installers | GitHub CI | Source integration | Cloud | Complete; 18 tests each platform, both installers, Windows launch |
| Windows installation | Coordinator | Verified Windows installer | Local installer | Complete; installer exit 0, visible Appearance window and inspected capture |
| Public preview | Coordinator | Verified installers | Release write | Complete; public v0.1.0 release, both assets, matching hashes and validation evidence |

Current refinement: frontend/layout/config/docs integrated; async provider commands, exact bridge detection and native updater integrated. Independent review findings are addressed: prior refresh failure isolation, unsigned PR packaging, and the required signed manifest publication gate. 36 frontend tests, zero Svelte errors/warnings, production build, Rust formatting and pill browser fixture passed. Native packaging and installation are queued; no local native compilation is reserved. Release writes and installation run sequentially; runtime estimates are unknown.

Verified: 21 frontend unit tests, Svelte checks, production build, dependency audit, browser-fixture interaction/adaptive checks, 18 native tests each on Windows/Mac, both platform installers, Windows launch and installed Appearance capture. Live provider integration, native Mac installation/runtime, signing/notarization and full-process resource measurements remain unverified; see VALIDATION.md.

Must keep: honest missing/stale data; provider-reported five-hour windows only; separate ordinary ChatGPT and Codex quota; shared Claude limits; no credentials or transcripts; reversible Claude hook setup; optional login startup; reduced motion; numerical measurements alongside SVG gauges and bars.

Design direction: a 280 × 56 floating utility window containing a rounded neon capsule, expressive tiny metric icons and readable numerals. Hover/focus opens exact measurements; click opens existing meters/graphs; right click opens quick controls. Punchy lime/pink, springy icon reactions and brief question shakes give it mischievous personality without continuous idle animation. Pressure management quiets motion and spaces out monitoring. Neon Circuit is the default; Cyberpunk Night and Aurora use the same hierarchy. Appearance is the first configuration step. Connection buttons immediately show progress and report connected, sign-in needed, waiting for readings or failed from actual source state. Rust handles monitoring and tray/startup; Tauri's desktop WebView renders the interface.

Acceptance: frontend type checks/tests/build; Rust parsing/settings/hook tests; no known high-severity npm audit findings; screenshot inspection at desktop and 400px; first-run flow and persisted preferences; Windows NSIS and universal Mac DMG packaging checks; verify public GitHub visibility and licenses. Platform install/runtime and performance measurements must be explicitly recorded as unverified when unavailable.
