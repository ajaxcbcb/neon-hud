# NotchNook reconstruction study

Status: **partial-observation**. Scope: the desktop utility's surface, state changes,
transitions and documented utilities, adapted to native Windows and macOS. The
Toolfolio listing is the user's reference pointer; the developer's site is the
primary reference. No visual-match or measured-motion claim is established yet.

Sources, accessed 2026-10-07:
- https://toolfolio.com/tools/notch-nook
- https://getnotchnook.com/
- https://getnotchnook.com/whats-new
- https://getnotchnook.com/press

## Evidence and coverage

OBSERVED in official documents: a top-screen Nook also works without a physical
notch; it has music controls, notes, tasks, timers, calendar, a mirror, shortcuts,
a persistent file tray and transient battery/device activities. Official media
assets include an overall demo and timer/task/note/music clips. This is document
evidence, not a pixel or playback observation.

UNKNOWN: exact collapsed/expanded dimensions, corner curves, typography,
spring/easing parameters, hover delays, transition duration, blur and opacity,
tab-switch choreography, drag trajectories, and how every state is dismissed.
The text reader could not display the press images. Bounded in-app-browser
observation is queued under the shared host's browser reservation. No clips
have been played and no reference frames have been inspected.

## User constraints that remain invariant

All original HUD capabilities remain reachable: numerical CPU/RAM/GPU/network
and selected-drive readings; gradient stress meters/history; accurate provider
allowances, five-hour reset windows and average allowance drain; question and
thermal/performance attention; adaptive sampling; connectors; retained profiles;
reduced motion; tray access; startup choice; signed updates. Keep the movable,
compressible pill as an alternative presentation. Never replace an unavailable
provider value with invented token counts.

The Nook adds local notes, tasks, deadline-based timers and persistent file
references. Other utilities require real platform adapters before being presented
as connected. Preserve the native renderer and custom Settings frame. The new
presentation must not overwrite the existing pill's saved position.

## Implementation and acceptance

Use the existing Rust/egui renderer and worker lifecycle. New presentation state
and productivity storage are separate from provider authentication. Animate only
bounded state changes; settle immediately under reduced motion or high pressure.
Test inward placement at eight edges, interrupted expansion, Escape/pin/focus,
text input, drops, save/recovery, timer restart, hidden Quit and signed update.

STARTING_VALUE geometry/timing may support initial implementation, but remains
adjustable until actual reference playback and frame comparison. A final copy
claim requires inspected native renders and representative reference states.

## Revision log

- 2026-10-07: user requested ultra-copycat NotchNook design, transitions,
  animations and features, then explicitly retained the core HUD capabilities.
  Added an independent source-only productivity branch. Reference playback and
  native reconstruction validation remain pending.
