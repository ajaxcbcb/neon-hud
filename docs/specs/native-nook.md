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
starting values informed by representative official frames; exact motion timing
is still unknown. Inspect native frames before declaring a reference match.

An optional black compact Nook with native inward expansion, a pinned state,
keyboard dismissal, interactive tabs and bounded expansion motion. Keep the
movable/compressible floating pill and custom three-step Settings window. Store
Nook preferences separately from the existing metric/provider profile and retain
the pill's saved position. The default for a new Nook preference is Nook.

The expanded Home panel shows media, calendar, notes, timer, tasks and quick
actions together, separated by fine grey dividers. A compact monitor layout
keeps media, notes and timer visible; the other widgets remain reachable from
the icon navigation. Nook and Tray are the primary choices. Focused utility,
system and AI views provide deeper controls without an outer scrolling page.
The bottom readings rail remains visible on Home. Core readings remain numerical,
with gradient gauges/bars and attention
icons. CPU/RAM/GPU/network/multiple selected drives, provider-reported five-hour
windows, reset times, average drain and fast depletion, thermal/performance
attention, adaptive sampling, connectors, tray, startup and signed update remain
available. Keep missing/stale values honest.

Local notes/tasks/timers/file references persist with a versioned, bounded store,
backup recovery and revision acknowledgements. Timers use epoch deadlines rather
than frame counts; completion is persisted before alerting. Files remain in their
original locations and missing references are identifiable.

Windows media uses its current media session; macOS media opts into controlling
Music through public Apple Events. macOS Calendar uses EventKit permission;
Windows accepts a local ICS file because this unpackaged application cannot
assume native-calendar access. Both platforms support bounded ICS recurrence
and timezone expansion. Power uses native battery status. Mirror is explicitly
enabled and pauses when hidden or under resource pressure. Camera startup is
reported until a validated frame arrives. Quick actions open real HUD features.

The utility worker uses bounded queues, cached snapshots and slower polling
under pressure. A macOS camera helper owns AVFoundation calls and callback
cleanup; cancellation does not wait on a stalled platform call. A helper that
never returns can remain busy, so macOS camera quit/start/stop runtime acceptance
is required. No synthetic live values substitute for an unavailable adapter.

## Execution and checks

1. Retain the passed public alpha.3-to-alpha.4 GUI signed-update proof and native
   icon/menu/hidden-shutdown gates; test the next published update separately.
2. Use the observed representative official frames; record exact timings as
   unknown until measured, and compare integrated native frames before fidelity.
3. Integrate pure presentation state plus separate productivity worker. Retain
   save/normal-Quit/updater handoff and profile boundaries.
4. Add utility adapters with truthful capabilities. Keep idle work bounded,
   adaptive polling and immediate reduced-motion/pressure settlement.
5. Run exact-source locked Windows/macOS tests/builds; inspect native frames and
   playback; test eight edges, interrupted motion, Escape/pin/drop/text focus,
   restarts/corruption, timer completion, tray and signed updates.
6. Publish release notes and signed artifacts, then update the target machine
   only under the central coordinator's fresh exact-phase admission.

Acceptance remains open until native frame comparison, integrated runtime checks
and target-machine installation pass. Source-only utility code is not runtime
proof. Strict connector-inclusive single-process operation remains a separate
open gate in the existing native implementation.
