# Implementation checkpoint

project_path: neon-hud

Objective: Publish a lightweight Windows/Mac neon HUD and installer source, with appearance-first configuration, supported AI quota sources and dual licensing.

Status: Implementation and verification in progress.

Next: Native backend tests, visual review, Windows and Mac packaging, public repository verification.

Must keep: honest missing/stale data; provider-reported five-hour windows only; separate ordinary ChatGPT and Codex quota; shared Claude limits; no credentials or transcripts; reversible Claude hook setup; optional login startup; reduced motion; numerical measurements alongside SVG gauges and bars.

Design direction: a compact dark cockpit with restrained neon instrument accents, sharp numerals and calm low-frequency updates. Neon Circuit is the default; Cyberpunk Night and Aurora use the same hierarchy. Appearance is the first configuration step, before Connections and Preferences.

Acceptance: frontend type checks/tests/build; Rust parsing/settings/hook tests; no known high-severity npm audit findings; screenshot inspection at desktop and 400px; first-run flow and persisted preferences; Windows NSIS and universal Mac DMG packaging checks; verify public GitHub visibility and licenses. Platform install/runtime and performance measurements must be explicitly recorded as unverified when unavailable.
