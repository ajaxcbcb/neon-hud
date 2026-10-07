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
assets include an overall demo and timer/task/note/music clips.

OBSERVED in representative frames from five muted official clips, during the
admitted IAB session on 2026-10-07 (14:59:14–15:04:13 UTC):
- [Overall demo](https://getnotchnook.com/assets/notchnook-demo-loop-dissolve-v1.webm),
  about 6.52 s: a wide, shallow black panel with rounded lower corners and flatter
  upper shoulders, simultaneous widgets and fine grey dividers. The video was
  displayed at 974 × 537.7 CSS pixels; the panel occupied about 961 × 131 of those
  displayed pixels. These are displayed-frame measurements, not native dimensions.
- [Timer](https://getnotchnook.com/assets/notchnook-timer-editorial.mp4), about
  0.52 s: three hour/minute/second wheels, blue centre values, dim neighbours,
  short presets and a circular play action. Hover adds a grey wheel backing.
- [Tasks](https://getnotchnook.com/assets/notchnook-tasks-editorial.mp4), about
  0.65 s: a new-task input, circular completion controls and stars at the right.
- [Notes](https://getnotchnook.com/assets/notchnook-notes-editorial.mp4), about
  6.45 s: a grey note tile with text, blue indicator and B/I/U actions below it.
- [Music](https://getnotchnook.com/assets/notchnook-music-editorial.mp4), about
  6.45 s: square artwork, truncated track/artist text, pause/skip, progress and
  elapsed/total times. Adjacent widgets remain visible in the individual clips.

The overall frame also showed a circular mirror, compact week/date calendar,
blue selected date, an empty-today label and vertical blue quick actions. Nook
and Tray sit at the upper left. The owned temporary IAB tab was closed within
the five-minute grant. This establishes representative frame observation, not
continuous playback coverage or a measured native match.

UNKNOWN: exact native collapsed/expanded dimensions, corner curves, typography,
spring/easing parameters, hover delays, transition duration, blur and opacity,
tab-switch choreography, drag trajectories, and how every state is dismissed.
Exact animation timing was not measured during the bounded reference session.

## User constraints that remain invariant

All original HUD capabilities remain reachable: numerical CPU/RAM/GPU/network
and selected-drive readings; gradient stress meters/history; accurate provider
allowances, five-hour reset windows and average allowance drain; question and
thermal/performance attention; adaptive sampling; connectors; retained profiles;
reduced motion; tray access; startup choice; signed updates. Keep the movable,
compressible pill as an alternative presentation. Never replace an unavailable
provider value with invented token counts.

The Nook adds local notes, tasks, deadline-based timers and persistent file
references. Media, calendar, camera and power now have source adapters, with
explicit unavailable/permission states; their runtime acceptance remains open.
Preserve the native renderer and custom Settings frame. The new
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
  Added an independent productivity store and observed five official clips at
  representative frames. Refined the source to simultaneous shallow widgets,
  wheel timers, notes formatting, task stars and a persistent core readings rail.
  Exact-source builds, native interactions, frame comparison and installed
  acceptance remain pending for this revision.
