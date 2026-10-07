# Floating placement implementation

Owner: coordinator. This is a light source phase; local builds and desktop checks need fresh coordinator admission.

1. `src/lib/model.ts` and `src-tauri/src/lib.rs`: add `compressed` and nullable `windowPosition` with logical x/y offsets and nullable monitor name; validate finite bounded coordinates; keep legacy defaults. Tests prove migration and round-trip retention.
2. `src/lib/layout.ts`: derive capsule dimensions and custom-position mapping/clamping. Keep the capsule stationary while popovers grow; choose a wider popover window when compressed. Tests exercise monitor removal/DPI/edge cases.
3. `src/App.svelte`: distinguish programmatic placement from user movement; capture user movement after settling and before profile flush; preserve custom position across configuration/restore; clear it only for preset/reset actions. Add the smallest required drag capability.
4. `src/components/Pill.svelte` and `src/pill.css`: grip, compression controls and compact metric selection; preserve hover, numeric and attention behavior. `Icon.svelte` gets the compression icon.
5. Frontend checks/tests/build plus Rust formatting/profile tests; independent source review. Use admitted runtime interaction to prove dragging, compression, tray behavior and persisted restart. Synchronize verified source to the existing public repository; replacement install requires fresh admission.

The strict single-process branch keeps these data contracts when replacing the renderer. Connector and updater feasibility remain separate gates; no WebView process-count claim follows from this increment.
