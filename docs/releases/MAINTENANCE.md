# Publishing application updates

## Native preview channel

The native egui/glow application and the legacy WebView application have separate versioning, packages and update feeds. Publishing one must not retarget the other's manifest.

1. Update native versions and the current README, native setup guide, feature guide, changelog and versioned release notes. Retain the native updater public key and all bundled license notices.
2. Run the exact-source Windows/macOS gates in [native.yml](../../.github/workflows/native.yml): locked formatting/tests/builds, dependency closure, physical Windows interaction checks and packaging. Trusted publication signs the native metadata and archives using the configured repository secret; never commit private material.
3. Publish the native versioned release with its real Windows x64 ZIP, macOS arm64 app/DMG, signed metadata and checksum/signature assets. Bind downloaded public asset sizes/digests to the accepted source and inspect architecture/notices.
4. Run [native-update-proof.yml](../../.github/workflows/native-update-proof.yml) from an actual older supported release against the public feed. Require physical Check/Download/Restart, rejection of tampered metadata/archive, normal old-app exit, exact replacement hash/acknowledgement, retained profile/layout/startup and working replacement Controls/Settings/Quit.
5. Record evidence in [Validation](../VALIDATION.md). A hosted GUI upgrade, target-machine package installation and target-machine updater execution are separate results. Update the public GitHub release body from the versioned release notes after verification.

Automatic native checks/downloads are distinct from the explicit **Restart and update** action. OS code signing/notarization is also distinct from updater signatures. A portable Windows ZIP has no NSIS installer/uninstaller; preserve profiles and retire old package files only after the verified replacement launches. Native Intel Mac packages are not currently supplied.

## Self-signed Windows development build

Dispatch [native.yml](../../.github/workflows/native.yml) with `development_signing=true`, `publish=false` and `proof=false`. The Windows-only job checks formatting, locked dependencies/tests and a fresh native release build, then uses [sign-native-development.ps1](../../scripts/sign-native-development.ps1). It creates a non-exportable ephemeral RSA development certificate, signs with SHA-256 and an RFC3161 SHA-256 timestamp, and independently verifies the file with SignTool and PowerShell. Temporary verification trust is limited to the disposable GitHub-hosted VM's LocalMachine Root store; an already elevated runner is required and no elevation is requested. The certificate and persisted CNG key must be absent before packaging. The job also runs physical Nook interaction checks against the signed binary.

The `native-windows-self-signed-development` artifact contains the complete license-bearing ZIP, public `.cer`, signature receipt and checksums. Its honest publisher label is **Neon HUD Development**. Do not import its certificate into a user's trust stores or describe it as publicly trusted. Authenticode integrity, updater authenticity, Defender determination and observed target runtime are separate checks. This test artifact is not attached to a production release or written to an update feed. No private key or PFX is exported.

Timestamping uses [DigiCert's documented RFC3161 endpoint](https://knowledge.digicert.com/solution/troubleshooting-timestamping-problems) with `/tr http://timestamp.digicert.com /td SHA256`; SignTool verifies the signed timestamp response. The job verifies the packaged executable/certificate hashes and records public signature metadata and archive checksums in its log.

Signing, trust import and both independent verifiers have 120-second child-process limits and UTC stage diagnostics. The CI-only supervisor can stop only its own child tree. Injected nonzero-exit and timeout scenarios must confirm removal of the exact certificate and persisted key. A public metadata journal records their identity before any trust import, so an `always()` cleanup step can retry even after a partially completed import. Diagnostics upload on failure; no private key is exported. The signing step and cleanup step also have workflow deadlines. A forcibly terminated job cannot guarantee that `finally` executes; disposal of the GitHub-hosted VM is the last containment boundary, and no interrupted job supplies an accepted package.

For a future trusted release, sign the final Windows executable before packaging and generating the updater manifest/hashes, then repeat publication and actual update acceptance. A development signature does not resolve a behavioral quarantine; keep affected target installation/launch blocked while that determination is unresolved.

## Legacy WebView channel

1. Bump package.json/package-lock.json, Cargo.toml/Cargo.lock and tauri.conf.json to the same version; update the changelog and release notes. Keep the existing updater public key.
2. Push the reviewed source. Require the exact commit's frontend verification, audit, Windows/Mac native tests, launch smoke and packaging jobs to pass.
3. Download that run's Windows installer and `.exe.sig`, universal Mac DMG, and `.app.tar.gz` with its `.sig`. Trusted CI uses `TAURI_SIGNING_PRIVATE_KEY` from GitHub Secrets. Never commit the private key. Fork PRs use `tauri.pr.conf.json` and cannot publish updates.
4. Publish the versioned release with those actual artifacts and SHA256SUMS.txt. Verify release asset digests against the downloaded artifacts.
5. Commit `updates/latest.json` only after the assets are public. It must contain version, RFC3339 pub_date, notes and platforms `windows-x86_64`, `darwin-x86_64`, `darwin-aarch64`. Each platform needs an HTTPS release-asset URL and the complete corresponding `.sig` contents. Both Mac keys use the universal archive. The Windows URL is the NSIS executable, not a zip. A signature is data, not a URL.
6. Confirm the manifest endpoint and asset URLs resolve, then inspect **Preferences → Check updates** in the installed app. A current-version check proves endpoint access; future download/install proof requires an actual newer published version. Do not create a dummy release to claim that proof.

The app rejects unverified update packages. Automatic installation and restart require the separate auto-install preference, which defaults off for existing users. Enabling it authorizes future verified upgrades; quiet-window and fresh-reading gates still defer installation. Keep a direct installer available for recovery and upgrades from 0.1.0. OS code-signing certificates and macOS notarization are separate production-distribution prerequisites.
