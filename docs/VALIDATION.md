# Validation

Checked on 7 October 2026.

- `npm run verify`: Svelte check with zero errors/warnings; 21 tests in five files; production frontend build passed.
- `npm audit --audit-level=high`: zero reported vulnerabilities.
- Browser fixtures: first-run preferences, two detected drives, selection persistence, startup preference persistence, threshold migration, 360 × 240 compact fit, 400px settings width, hover details, keyboard/right-click menu, pause/manual refresh, theme changes, reduced motion and finite bursts capped at 24 particles passed.
- Adaptive browser fixture: a simulated 95 °C sensor moved the governor into critical mode, cleared bursts, quieted motion and sent the critical budget to the native IPC fixture. Exactly one automatic system poll occurred in 8.8 seconds; provider checks slowed to 15 seconds. Disabling adaptive mode restored the selected motion and persisted.
- The compact 360 × 240 layout also passed with two drives, critical mode and pressure badges after 16.5 seconds of high-load fixture readings.
- Unit coverage includes sustained pressure, stale/unknown readings, slower sampling, recovery hysteresis, storage-only warnings and gradient direction/clamping.
- Appearance screenshot was inspected at desktop size. Its preview is labeled **SAMPLE DATA**. Browser fixtures exercise frontend behavior; they do not prove native sensors or account integration.
- Rust formatting passed. Native builds ran in GitHub CI because local MSVC `link.exe` was unavailable.
- [Final native CI run](https://github.com/ajaxcbcb/neon-hud/actions/runs/37535203987), source `f329d87822bd6398164077a1300ed6cfd44ea905`: all 18 Rust tests passed on Windows and Mac, including real background sensor collection across resource modes. Windows x64 NSIS and universal Mac DMG packaging passed; frontend verification and audit also passed in that run.
- Windows CI launch smoke passed: the executable created a visible Neon HUD window and remained running for another 10 seconds.

Windows x64 setup was installed successfully (installer exit 0), then the installed 0.1.0 executable launched and stayed running. UI Automation confirmed Step 1, Appearance, the three themes and the labeled sample preview. A capture of the installed app window was inspected. Login startup remained off. This replaced an earlier build that exposed a Windows `RPC_E_CHANGED_MODE` startup panic; sensor construction/refresh/destruction now stay on one background thread rather than the UI thread.

Release installer SHA-256:

| Asset | SHA-256 |
|---|---|
| `Neon HUD_0.1.0_x64-setup.exe` | `4667f56d4a3099ffd0ec6519057b0578dff73090262c0fcf93bc618a05cb1124` |
| `Neon HUD_0.1.0_universal.dmg` | `b6cab438d22d546b74c7ca0230daf12503b1a33d2443aaa7f15ea65c16a334c1` |

Live provider accounts, native Mac installation, signing/notarization, and full-process idle CPU/memory targets require separate platform evidence. A successful compile or screenshot is not proof of those checks.
