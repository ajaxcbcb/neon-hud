# Native updater execution plan

1. Repair the smoke test to match a fresh compressed profile. Preserve the icon and native right-click popup changes.
2. Implement the independent signed release worker, safe staging and normal-exit helper. Integrate bounded Settings controls, separate update preferences and startup acknowledgement.
3. Resolve and commit the native lockfile using the hosted runner; format and run locked tests, Windows interactions, Mac packaging and signature rejection checks. Review the integrated updater security path.
4. Publish an updater-capable alpha.3 bootstrap and a newer alpha.4, with signed native manifests, release notes, changelog and verified asset hashes. Exercise the real signed channel between these releases.
5. Obtain fresh resource admission, quit the old HUD normally, bootstrap without changing profiles or startup, then apply the newer release through the updater. Verify the installed executable, visible pill/menu, up-to-date result and retained configuration. Record proof and remaining limits.

Owners: coordinator owns UI, release pipeline and installation; bounded native updater worker owns `src-native/src/updater.rs`. Hosted CI owns compilation and desktop test execution. One local runtime phase at a time; no automatic process termination.
