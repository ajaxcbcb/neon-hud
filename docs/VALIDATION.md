# Validation

Checked on 7 October 2026.

## 0.1.2 GPU and fast-sampling increment

- Local Svelte/TypeScript check: zero errors and warnings. Frontend tests: 51 passed, including adapter selection, stale/null capability boundaries, cached GPU pressure timing, recovery evidence, independent polling and finite display transitions with an injected 144 Hz clock.
- Production frontend build passed; dependency audit reported zero vulnerabilities. Rust formatting passed.
- The 144 Hz test is a deterministic frame-clock fixture, not a measurement of the installed app's frame rate.
- [Final native CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37567052327), source `1fa44bc8a1503485d51b2b656f81a1f4589006a1`, passed all 26 Windows and 23 Mac native tests, Windows x64 setup and universal Mac packaging, and the Windows visible-window launch check. This follows a repaired test-only missing thread import in the earlier failed run.
- The [public v0.1.2 preview](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.2) contains five CI artifacts and SHA256SUMS.txt. All five artifact digests matched GitHub's uploaded asset digests; both updater payload signatures and trusted comments verified against the configured public key. The Windows setup SHA-256 is `ff4281fa25a6aeea56b066c86ee9613b5447e7c4378a5a579df5265941a0569f`.
- `updates/latest.json` covers Windows x64 and both Intel/Apple Silicon Mac updater targets using the verified payloads. Installed updater execution, native GPU values and full-process performance remain unmeasured.
- No new local visual/native installation batch ran while the shared machine's interactive lane was reserved. The existing 0.1.1 pill visual evidence remains below; the added GPU layout needs installed inspection.

## 0.1.1 refinement

- `npm run verify`: 36 tests across nine files, zero Svelte errors/warnings and successful production build.
- Rust formatting passed; dependency audit reported zero vulnerabilities.
- Browser fixture passed connection pending/disabled state, Claude bridge waiting/removal, Codex connected feedback, updater current-version feedback and persisted update preference.
- The pill measured 272 × 48 inside a 280 × 56 window with no horizontal overflow. Hover GiB details, bounded quick controls, Escape/Shift+F10 and 430px settings passed without JavaScript errors. Pill, hover, menu and narrow settings captures were inspected.
- Update tests cover concurrent checks, progress, signature failures/retries, no install before verification or without explicit action, and cleanup after an in-flight download. Provider overlap tests cover prior rejected requests and serialized actions.
- Independent review identified and resolved the prior-refresh rejection race and fork-PR signing-secret gap. Publishing a real signed platform manifest is required before updater delivery can be claimed.
- The installed Claude bridge was configured with all seven hooks; the official CLI confirmed authenticated status. No live account-limit reading has been observed yet. Fixtures use sample data and do not prove provider quota.

- [Final signed native CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37564389145), source `908c9e4625a4552d69a4fee10c7eb102764bc8d7`, passed frontend verification, Windows/Mac tests and packaging, and Windows launch. The universal Mac build explicitly includes the application updater archive alongside the DMG; CI checks both platforms have a matching signed update payload.
- The [public v0.1.1 preview](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.1) contains Windows setup, universal Mac DMG, both signed update payloads and SHA256SUMS.txt. All five downloaded artifact digests matched GitHub's published asset digests. Both updater signatures and their trusted comments verified against the configured public key before publication.
- `updates/latest.json` points to the verified Windows x64 and universal Mac archives, covering Intel and Apple Silicon updater targets.

Installed 0.1.1 window and updater execution remain pending. Updater signing is distinct from OS code signing/notarization, which is not configured. The existing 0.1.0 evidence follows.

## 0.1.0 baseline

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
| `Neon.HUD_0.1.0_x64-setup.exe` | `4667f56d4a3099ffd0ec6519057b0578dff73090262c0fcf93bc618a05cb1124` |
| `Neon.HUD_0.1.0_universal.dmg` | `b6cab438d22d546b74c7ca0230daf12503b1a33d2443aaa7f15ea65c16a334c1` |

The [public v0.1.0 prerelease](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.0) includes both assets and SHA256SUMS.txt. GitHub-reported installer digests match the downloaded CI assets. GitHub converts spaces in uploaded asset names to periods; the checksum file uses those published names.

Live provider accounts, native Mac installation, signing/notarization, and full-process idle CPU/memory targets require separate platform evidence. A successful compile or screenshot is not proof of those checks.
