# Validation

Checked on 7 October 2026.

## 0.1.4 floating placement and compression

- Source adds a native drag grip, physical-pixel keyboard nudging, a 160 × 56 compressed window and persisted custom placement. Geometry tests cover DPI changes, removed displays, duplicate display names and popover growth at all four edges independently of a corner preset.
- Independent source review passed after repairing preset-dependent popover movement, ambiguous identical-monitor restoration and hover resizing during native dragging.
- [Final CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37583511703), source `1a16805dcc802fac32f1d60a3e417418879e3c16`, passed frontend checks/tests/build with zero Svelte errors/warnings and zero audit vulnerabilities, Rust formatting, 30 Windows and 27 Mac tests, both signed platform packages and Windows launch/minimize-to-tray.
- The [public v0.1.4 preview](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.4) contains all five exact-run artifacts and SHA256SUMS.txt. All six public asset digests match locally, and both updater payload signatures and trusted comments verify against the existing public key. Windows setup SHA-256: `d4a56f3dcba5f66cb6fb6c25d0a9bdd2192189f1c1bd648278ddd60d736fe7ae`.
- Windows v0.1.4 was installed after a renewed runtime reservation. The previous GUI and all eight observed descendants exited through the normal message loop; the verified installer returned 0. The installed executable reports 0.1.4, and the original profile bytes were preserved during installation.
- The own installed HUD compressed from 280 × 56 to 160 × 56 and returned to regular size. Non-layout preferences and the Claude configuration hash stayed unchanged. Dragging, keyboard nudging and restart placement were inconclusive because the human was concurrently moving/resizing the HUD; automated input stopped after that was confirmed. No restart-persistence proof is claimed for 0.1.4.
- The installed snapshot had nine owned processes: GUI, six WebView2 helpers, Codex and its console host. Strict single-process operation remains open. The automatic-update manifest still targets 0.1.3 while replacement gates remain unfinished.

## 0.2.0 native desktop source

- The native preview uses Rust/egui/glow and the headless Rust monitoring core. Source contains no scroll areas or HTML/CSS/JavaScript renderer. Settings, details and hover readings have separate bounded native windows.
- Preview settings are copied once into a separate directory; Claude bridge data stays in the shared parent directory. Tests cover original-profile retention and bridge-directory routing. Native startup uses a distinct opt-in registration.
- [Exact-source native CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37596488001), source `df029bc77a625b628def337ec71285e875c03b4d`, passed Rust formatting, five native tests per platform, 31 Windows and 28 Mac core tests in both desktop and headless configurations, locked release builds, native dependency closure and packaging. The native closure contains no Tauri, Wry or WebView renderer.
- Actual Windows captures were inspected for Appearance, Connections, Preferences/Instruments, Preferences/Startup, the instrument window and the tiny pill. Settings measured 740 × 680, instruments 460 × 460 and the pill 160 × 56 pixels on the CI desktop. The rounded custom header, bold hero, cards, vector check/arrow, fixed footer and bounded pages rendered without clipping or an OS title bar. Capture checks require each window to be in the monitor work area and foreground; all four smoke sessions exited with code 0.
- [Settings capture](assets/native-settings.png), [pill capture](assets/native-pill.png) and [instruments capture](assets/native-instruments.png) are actual native Windows output. The Appearance pill is labeled sample data; instrument readings belong to the hosted runner. CI used hash-pinned Mesa software OpenGL for its display, which is excluded from public packages. These renders do not measure hardware performance.
- The [public v0.2.0-alpha.1 prerelease](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.2.0-alpha.1) is tagged at the exact CI source. All four public asset digests match the downloaded packages and checksum file. ZIP inspection verified Windows x64 PE and Mac arm64 Mach-O binaries, Apache 2.0/MIT project licenses, third-party notices and the Ubuntu Bold license/copyright notice. Software OpenGL DLLs are absent from the Windows ZIP. The Mac DMG packages the same app bundle as the verified ZIP.
- On 2026-10-07, the verified public Windows ZIP was installed as a separate native preview, with Desktop and Start menu shortcuts pointing to the extracted executable. Extracted-file hashes matched the ZIP. Executable SHA-256: `e62e277f0e28e5e4676230a927a8da1d233f36e1786374c25a88e3f005c18b91`. No local compilation ran.
- The normal installed native app launched and its 280 × 56 floating pill was captured and inspected on the target machine. The observed process tree contained one process and no WebView descendants. Provider connections were not activated during this check; it does not prove connector-inclusive single-process operation or resource savings.
- Both original HUD profile copies and the Claude configuration retained their pre-installation hashes. Native settings imported into the separate preview profile; all preference values matched the original except normalized window position. Installation did not enable startup or modify the stable updater manifest. The native preview was left running, and the existing HUD received no lifecycle actions.
- Settings access through the pill context menu was inconclusive after bounded own-window input. Further desktop input stopped; no target-machine Settings, tray, drag or startup proof is claimed. Existing Codex/Claude helpers, signed native updates, Mac runtime and full-process performance remain unfinished gates.

## 0.1.3 quiet updates and retained profiles

- Local frontend checks passed with zero Svelte errors/warnings, 63 tests across 12 files and a successful production build. Dependency audit reported zero vulnerabilities.
- Tests cover quiet restart eligibility, fresh pressure checks, signature/download boundaries, failed automatic installation/manual retry, a changed update preference during a pending check, pending profile flush, serialized writes and recovery after write failure.
- [Final native CI](https://github.com/ajaxcbcb/neon-hud/actions/runs/37574695191), source `44b579e2d8d2fa796990b8d20138fc917a76125e`, passed 29 Windows and 26 Mac tests, including legacy-profile migration and malformed-file retention. Windows launch and minimize-to-tray passed: the window hid while the app remained running.
- Independent review passed after two bypassed save paths were routed through the serialized settings writer.
- The [public v0.1.3 preview](https://github.com/ajaxcbcb/neon-hud/releases/tag/v0.1.3) contains all five CI artifacts and SHA256SUMS.txt. All six uploaded asset digests matched locally; both updater payload signatures and trusted comments verified against the configured public key. The Windows setup SHA-256 is `cc500b59f14a2e07a7be1cd5b120b937e33bde1fa1e4a83d56b2a3e459b21e71`.
- The updater manifest targets Windows x64 and universal Intel/Apple Silicon Mac payloads. Windows v0.1.3 was installed from the verified setup with exit code 0; the installed executable reports 0.1.3. Automatic checking/downloading and installation are enabled in the active profile. Execution of a future automatic upgrade and Mac runtime remain unverified.
- Installed Windows checks passed: one HUD GUI, the compact floating interface after relaunch, minimize hiding the window while the app remained alive, no named Neon HUD taskbar entry while hidden, and restoration through the own-window native handler. Actual tray-menu restoration and tray Quit were not exercised because the tray icon could not be identified through accessibility; own-window restoration is separate evidence.
- Normal message-loop shutdown stopped the GUI and all eight observed descendants without force termination. Relaunch retained every original profile value, including theme, compact size, placement, startup and provider choices. Migration only added missing GPU and sampling fields. Both update preferences survived a runtime write after relaunch; the Claude configuration hash remained unchanged. These checks used a stable settings backup and a normal native exit, rather than exercising the frontend tray Quit flush.
- Windows packaged-host launches can use a virtualized `LocalCache/Roaming` profile separate from ordinary `%APPDATA%`. The active profile was identified through its write after launch and matched against the original byte backup. The two profiles were preserved separately; parity across a launch outside the packaged host or an OS login remains unverified.
- A strict single OS process is not implemented: the installed release uses WebView2 and a Codex helper. A native renderer and in-process connector/update design remain separate work; one GUI instance does not satisfy that requirement.

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
