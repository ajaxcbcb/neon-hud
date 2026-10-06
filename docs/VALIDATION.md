# Validation

Checked on 7 October 2026.

- `npm run verify`: Svelte check with zero errors/warnings; 21 tests in five files; production frontend build passed.
- `npm audit --audit-level=high`: zero reported vulnerabilities.
- Browser fixtures: first-run preferences, two detected drives, selection persistence, startup preference persistence, threshold migration, 360 × 240 compact fit, 400px settings width, hover details, keyboard/right-click menu, pause/manual refresh, theme changes, reduced motion and finite bursts capped at 24 particles passed.
- Adaptive browser fixture: a simulated 95 °C sensor moved the governor into critical mode, cleared bursts, quieted motion and sent the critical budget to the native IPC fixture. Exactly one automatic system poll occurred in 8.8 seconds; provider checks slowed to 15 seconds. Disabling adaptive mode restored the selected motion and persisted.
- The compact 360 × 240 layout also passed with two drives, critical mode and pressure badges after 16.5 seconds of high-load fixture readings.
- Unit coverage includes sustained pressure, stale/unknown readings, slower sampling, recovery hysteresis, storage-only warnings and gradient direction/clamping.
- Appearance screenshot was inspected at desktop size. Its preview is labeled **SAMPLE DATA**. Browser fixtures exercise frontend behavior; they do not prove native sensors or account integration.
- Rust formatting passed. Local Rust execution is blocked by missing MSVC `link.exe`; native test and installer evidence will be recorded from the final GitHub CI run.

Windows installation and launch evidence will be added after verified CI packaging.

Live provider accounts, native Mac installation, signing/notarization, and full-process idle CPU/memory targets require separate platform evidence. A successful compile or screenshot is not proof of those checks.
