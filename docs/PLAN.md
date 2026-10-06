# Implementation checkpoint

project_path: neon-hud

Objective: Publish a lightweight Windows/Mac neon HUD and installer source, with appearance-first configuration, supported AI quota sources, adaptive resources, stress gradients, release documentation and dual licensing.

Status: Frontend and native builds verified. Windows installation exposed a COM startup conflict; repair and launch regression are ready for CI. Delivery priority is P2 (no deadline specified).

Next: Verify the background monitor and Windows launch in CI, install the corrected Windows build, verify first-run Appearance, then publish both prerelease installers and hashes.

| Job | Owner | Dependency | Resource | State / acceptance |
|---|---|---|---|---|
| Bridge recovery repair | Backend worker | Focused review | Light local | Complete; regression passed on Windows and Mac |
| Source integration | Coordinator | Startup repair | Repository write | Ready; dedicated monitor thread and launch regression |
| Native tests and installers | GitHub CI | Source integration | Cloud | Next; Windows and Mac jobs plus Windows launch pass |
| Windows installation | Coordinator | Verified Windows installer | Local installer | Repair required; first installer completed but startup exited |
| Public preview | Coordinator | Verified installers | Release write | Waiting; release assets, hashes and validation evidence |

Local native compilation is not reserved. Installation and release writes run sequentially; runtime estimates are unknown.

Verified: 21 frontend unit tests, Svelte checks, production build, dependency audit, browser-fixture interaction/adaptive checks and inspected appearance screenshot. Native account and platform runtime checks require separate evidence.

Must keep: honest missing/stale data; provider-reported five-hour windows only; separate ordinary ChatGPT and Codex quota; shared Claude limits; no credentials or transcripts; reversible Claude hook setup; optional login startup; reduced motion; numerical measurements alongside SVG gauges and bars.

Design direction: a compact neon arcade panel with chunky theme tickets, sharp numerals, interactive icons and capped finite animation bursts. Pressure management quiets motion and spaces out monitoring. Neon Circuit is the default; Cyberpunk Night and Aurora use the same hierarchy. Appearance is the first configuration step, before Connections and Preferences.

Acceptance: frontend type checks/tests/build; Rust parsing/settings/hook tests; no known high-severity npm audit findings; screenshot inspection at desktop and 400px; first-run flow and persisted preferences; Windows NSIS and universal Mac DMG packaging checks; verify public GitHub visibility and licenses. Platform install/runtime and performance measurements must be explicitly recorded as unverified when unavailable.
