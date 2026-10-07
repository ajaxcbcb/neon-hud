# Native Nook and core HUD retention

The user requested the NotchNook desktop layout, transitions, animations and
features, with all Neon HUD functions retained. Reference study and proof status
are in [the study](../../reference-learning/notchnook/study.md).

## Deliverable

Design direction: a near-black, top-centred capsule which expands within the
same native window. White typography, restrained grey dividers, icon-led tabs
and a quiet background keep it small; selected instruments retain the user's
neon colours. Hover peeks, click opens, pin holds and Escape closes. Its saved
anchor is separate from the existing pill position. Geometry and timing are
starting values until representative official playback is observed; tune them
against the study before declaring a reference match.

An optional black compact Nook with native inward expansion, a pinned state,
keyboard dismissal, interactive tabs and measured reference motion. Keep the
movable/compressible floating pill and custom three-step Settings window. Store
Nook preferences separately from the existing metric/provider profile and retain
the pill's saved position. The default for a new Nook preference is Nook.

The expanded panel provides system instruments, AI allowances and local utility
tabs. Core readings remain numerical, with gradient gauges/bars and attention
icons. CPU/RAM/GPU/network/multiple selected drives, provider-reported five-hour
windows, reset times, average drain and fast depletion, thermal/performance
attention, adaptive sampling, connectors, tray, startup and signed update remain
available. Keep missing/stale values honest.

Local notes/tasks/timers/file references persist with a versioned, bounded store,
backup recovery and revision acknowledgements. Timers use epoch deadlines rather
than frame counts; completion is persisted before alerting. Files remain in their
original locations and missing references are identifiable.

Media, calendar, mirror, shortcuts and battery/device activity require real
platform adapters and explicit capability/error states. Do not substitute a fake
live card for an unavailable integration. OS-specific differences must be stated
in the feature notes.

## Execution and checks

1. Finish the public alpha.3-to-alpha.4 real GUI signed-update proof, preserving
   the known native icon/menu/hidden-shutdown gates.
2. Observe official demo, timer, task, note and music clips under admitted browser
   scope; document actual states, geometry and motion before claiming fidelity.
3. Integrate pure presentation state plus separate productivity worker. Retain
   save/normal-Quit/updater handoff and profile boundaries.
4. Add utility adapters with truthful capabilities. Keep idle work bounded,
   adaptive polling and immediate reduced-motion/pressure settlement.
5. Run exact-source locked Windows/macOS tests/builds; inspect native frames and
   playback; test eight edges, interrupted motion, Escape/pin/drop/text focus,
   restarts/corruption, timer completion, tray and signed updates.
6. Publish release notes and signed artifacts, then update the target machine
   only under the central coordinator's fresh exact-phase admission.

Acceptance remains open until reference observation, integrated runtime checks
and target-machine installation pass. Source-only utility code is not runtime
proof. Strict connector-inclusive single-process operation remains a separate
open gate in the existing native implementation.
