# Publishing application updates

1. Bump package.json/package-lock.json, Cargo.toml/Cargo.lock and tauri.conf.json to the same version; update the changelog and release notes. Keep the existing updater public key.
2. Push the reviewed source. Require the exact commit's frontend verification, audit, Windows/Mac native tests, launch smoke and packaging jobs to pass.
3. Download that run's Windows installer and `.exe.sig`, universal Mac DMG, and `.app.tar.gz` with its `.sig`. Trusted CI uses `TAURI_SIGNING_PRIVATE_KEY` from GitHub Secrets. Never commit the private key. Fork PRs use `tauri.pr.conf.json` and cannot publish updates.
4. Publish the versioned release with those actual artifacts and SHA256SUMS.txt. Verify release asset digests against the downloaded artifacts.
5. Commit `updates/latest.json` only after the assets are public. It must contain version, RFC3339 pub_date, notes and platforms `windows-x86_64`, `darwin-x86_64`, `darwin-aarch64`. Each platform needs an HTTPS release-asset URL and the complete corresponding `.sig` contents. Both Mac keys use the universal archive. The Windows URL is the NSIS executable, not a zip. A signature is data, not a URL.
6. Confirm the manifest endpoint and asset URLs resolve, then inspect **Preferences → Check updates** in the installed app. A current-version check proves endpoint access; future download/install proof requires an actual newer published version. Do not create a dummy release to claim that proof.

The app rejects unverified update packages. Downloading does not authorize installation/restart. Keep a direct installer available for recovery and upgrades from 0.1.0. OS code-signing certificates and macOS notarization are separate production-distribution prerequisites.
