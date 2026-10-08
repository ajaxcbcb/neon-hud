//! Pure presentation preferences and input-driven Nook transition helpers.
use crate::model::{HoverPlacement, Screen};
use serde::{Deserialize, Serialize};

// STARTING_VALUE: reference timing has not yet been admitted or measured.
pub const STARTING_VALUE_HOVER_DWELL_MS: i64 = 120;
pub const STARTING_VALUE_COLLAPSE_DELAY_MS: i64 = 500;
pub const STARTING_VALUE_MOTION_MS: i64 = 180;
pub const STARTING_VALUE_FRAME_MS: i64 = 16;
pub const STARTING_VALUE_PEEK_PROGRESS: f32 = 0.32;
pub const STARTING_VALUE_MIN_PANEL_HEIGHT_PX: f64 = 32.;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    #[default]
    Nook,
    Pill,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NookSize {
    Mini,
    #[default]
    Standard,
}

impl NookSize {
    pub fn collapsed_size(self) -> [f64; 2] {
        match self {
            Self::Mini => [200., 36.],
            Self::Standard => [240., 40.],
        }
    }
    pub fn peek_size(self) -> [f64; 2] {
        match self {
            Self::Mini => [480., 104.],
            Self::Standard => [520., 112.],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionStyle {
    #[default]
    Playful,
    Chaotic,
}

fn motion_easing(fraction: f32, style: MotionStyle) -> f32 {
    let t = fraction.clamp(0., 1.);
    match style {
        MotionStyle::Playful => 1. - (1. - t).powi(3),
        MotionStyle::Chaotic => {
            // A brief, bounded settle; no idle animation or geometry overshoot.
            (1. - (1. - t).powi(4) + (t * std::f32::consts::TAU * 2.).sin() * t * (1. - t) * 0.12)
                .clamp(0., 1.)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedNookPosition {
    pub monitor_id: String,
    /// Monitor-relative logical coordinates, independent of the Pill position.
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresentationPreference {
    pub mode: PresentationMode,
    pub pinned: bool,
    pub nook_position: Option<SavedNookPosition>,
    pub size: NookSize,
    pub hover_to_peek: bool,
    pub auto_collapse: bool,
}

impl Default for PresentationPreference {
    fn default() -> Self {
        Self {
            mode: PresentationMode::Nook,
            pinned: false,
            nook_position: None,
            size: NookSize::Standard,
            hover_to_peek: true,
            auto_collapse: true,
        }
    }
}

impl PresentationPreference {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(position) = &self.nook_position {
            if position.monitor_id.is_empty()
                || position.monitor_id.len() > 256
                || i64::from(position.x).abs() > 100_000
                || i64::from(position.y).abs() > 100_000
            {
                return Err("Invalid saved Nook position".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Collapsed,
    Peek,
    Expanded,
    Pinned,
}

#[derive(Clone, Copy, Debug)]
struct Motion {
    from: f32,
    to: f32,
    started_ms: i64,
    style: MotionStyle,
}

#[derive(Clone, Debug)]
pub struct PresentationState {
    pub phase: Phase,
    hover_since_ms: Option<i64>,
    collapse_at_ms: Option<i64>,
    motion: Option<Motion>,
    dragging: bool,
    hover_to_peek: bool,
    auto_collapse: bool,
    style: MotionStyle,
}

impl PresentationState {
    pub fn new(preference: &PresentationPreference) -> Self {
        Self {
            phase: if preference.pinned {
                Phase::Pinned
            } else {
                Phase::Collapsed
            },
            hover_since_ms: None,
            collapse_at_ms: None,
            motion: None,
            dragging: false,
            hover_to_peek: preference.hover_to_peek,
            auto_collapse: preference.auto_collapse,
            style: MotionStyle::Playful,
        }
    }
    pub fn configure(&mut self, preference: &PresentationPreference, style: MotionStyle) {
        self.hover_to_peek = preference.hover_to_peek;
        self.auto_collapse = preference.auto_collapse;
        self.style = style;
        if !self.hover_to_peek {
            self.hover_since_ms = None;
        }
        if !self.auto_collapse {
            self.collapse_at_ms = None;
        }
    }
    pub fn is_pinned(&self) -> bool {
        self.phase == Phase::Pinned
    }
    pub fn progress(&self, now_ms: i64, reduced_motion: bool, pressure: bool) -> f32 {
        let target = phase_progress(self.phase);
        if reduced_motion || pressure {
            return target;
        }
        match self.motion {
            Some(motion) => {
                let elapsed = now_ms.saturating_sub(motion.started_ms).max(0);
                let fraction = (elapsed as f32 / STARTING_VALUE_MOTION_MS as f32).clamp(0., 1.);
                let eased = motion_easing(fraction, motion.style);
                (motion.from + (motion.to - motion.from) * eased).clamp(0., 1.)
            }
            None => target,
        }
    }
    fn transition(
        &mut self,
        next: Phase,
        now_ms: i64,
        reduced_motion: bool,
        pressure: bool,
    ) -> bool {
        if self.phase == next {
            return false;
        }
        let from = self.progress(now_ms, reduced_motion, pressure);
        self.phase = next;
        let to = phase_progress(next);
        self.motion = if reduced_motion || pressure || (from - to).abs() < f32::EPSILON {
            None
        } else {
            Some(Motion {
                from,
                to,
                started_ms: now_ms,
                style: self.style,
            })
        };
        true
    }
    /// Call on input or at next_wake_ms; returns whether the visible phase changed.
    pub fn advance(&mut self, now_ms: i64, reduced_motion: bool, pressure: bool) -> bool {
        if reduced_motion
            || pressure
            || self.motion.is_some_and(|motion| {
                now_ms.saturating_sub(motion.started_ms) >= STARTING_VALUE_MOTION_MS
            })
        {
            self.motion = None;
        }
        if self
            .hover_since_ms
            .is_some_and(|since| now_ms.saturating_sub(since) >= STARTING_VALUE_HOVER_DWELL_MS)
        {
            self.hover_since_ms = None;
            return self.transition(Phase::Peek, now_ms, reduced_motion, pressure);
        }
        if self.collapse_at_ms.is_some_and(|at| now_ms >= at) && !self.dragging {
            self.collapse_at_ms = None;
            return self.transition(Phase::Collapsed, now_ms, reduced_motion, pressure);
        }
        false
    }
    pub fn next_wake_ms(&self, now_ms: i64) -> Option<i64> {
        let mut next = self
            .hover_since_ms
            .map(|since| since.saturating_add(STARTING_VALUE_HOVER_DWELL_MS));
        if let Some(collapse) = self.collapse_at_ms {
            next = Some(next.map_or(collapse, |wake| wake.min(collapse)));
        }
        if let Some(motion) = self.motion {
            let end = motion.started_ms.saturating_add(STARTING_VALUE_MOTION_MS);
            if now_ms < end {
                let frame = now_ms.saturating_add(STARTING_VALUE_FRAME_MS).min(end);
                next = Some(next.map_or(frame, |wake| wake.min(frame)));
            }
        }
        next.map(|wake| wake.max(now_ms))
    }
    pub fn hover_enter(&mut self, now_ms: i64) {
        self.collapse_at_ms = None;
        if self.phase == Phase::Collapsed && self.hover_to_peek {
            self.hover_since_ms = Some(now_ms);
        }
    }
    pub fn hover_leave(&mut self, now_ms: i64) {
        self.hover_since_ms = None;
        if self.auto_collapse
            && matches!(self.phase, Phase::Peek | Phase::Expanded)
            && !self.dragging
        {
            self.collapse_at_ms = Some(now_ms.saturating_add(STARTING_VALUE_COLLAPSE_DELAY_MS));
        }
    }
    pub fn click(&mut self, now_ms: i64, reduced_motion: bool, pressure: bool) -> bool {
        self.hover_since_ms = None;
        self.collapse_at_ms = None;
        match self.phase {
            Phase::Collapsed | Phase::Peek => {
                self.transition(Phase::Expanded, now_ms, reduced_motion, pressure)
            }
            Phase::Expanded => self.transition(Phase::Collapsed, now_ms, reduced_motion, pressure),
            Phase::Pinned => false,
        }
    }
    pub fn set_pinned(
        &mut self,
        pinned: bool,
        now_ms: i64,
        reduced_motion: bool,
        pressure: bool,
    ) -> bool {
        if !pinned && !self.is_pinned() {
            return false;
        }
        self.hover_since_ms = None;
        self.collapse_at_ms = None;
        self.transition(
            if pinned {
                Phase::Pinned
            } else {
                Phase::Expanded
            },
            now_ms,
            reduced_motion,
            pressure,
        )
    }
    pub fn escape(&mut self, now_ms: i64, reduced_motion: bool, pressure: bool) -> bool {
        self.hover_since_ms = None;
        self.collapse_at_ms = None;
        self.dragging = false;
        self.transition(Phase::Collapsed, now_ms, reduced_motion, pressure)
    }
    pub fn outside(&mut self, now_ms: i64, reduced_motion: bool, pressure: bool) -> bool {
        self.hover_since_ms = None;
        if !self.auto_collapse || self.is_pinned() || self.dragging {
            return false;
        }
        self.collapse_at_ms = None;
        self.transition(Phase::Collapsed, now_ms, reduced_motion, pressure)
    }
    pub fn drag_start(&mut self, now_ms: i64, reduced_motion: bool, pressure: bool) -> bool {
        self.dragging = true;
        self.hover_since_ms = None;
        self.collapse_at_ms = None;
        if self.is_pinned() {
            false
        } else {
            self.transition(Phase::Expanded, now_ms, reduced_motion, pressure)
        }
    }
    pub fn drag_end(&mut self, inside: bool, now_ms: i64) {
        self.dragging = false;
        if self.auto_collapse && !inside && !self.is_pinned() {
            self.collapse_at_ms = Some(now_ms.saturating_add(STARTING_VALUE_COLLAPSE_DELAY_MS));
        }
    }
}

fn phase_progress(phase: Phase) -> f32 {
    match phase {
        Phase::Collapsed => 0.,
        Phase::Peek => STARTING_VALUE_PEEK_PROGRESS,
        Phase::Expanded | Phase::Pinned => 1.,
    }
}

/// Saves a root position in logical monitor units, independent from Pill placement.
pub fn remember_nook_position(point: [f64; 2], screens: &[Screen]) -> Option<SavedNookPosition> {
    if point.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let screen = screens
        .iter()
        .filter(|screen| valid_screen(screen))
        .min_by(|a, b| distance_to_screen(point, a).total_cmp(&distance_to_screen(point, b)))?;
    let x = ((point[0] - screen.origin[0]) / screen.scale).round();
    let y = ((point[1] - screen.origin[1]) / screen.scale).round();
    if x.abs() > 100_000. || y.abs() > 100_000. {
        return None;
    }
    Some(SavedNookPosition {
        monitor_id: screen.id.clone(),
        x: x as i32,
        y: y as i32,
    })
}

pub fn restore_nook_position(
    saved: &SavedNookPosition,
    logical_size: [f64; 2],
    screens: &[Screen],
) -> Option<[f64; 2]> {
    if logical_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.)
    {
        return None;
    }
    let screen = screens
        .iter()
        .find(|screen| valid_screen(screen) && screen.id == saved.monitor_id)
        .or_else(|| {
            screens
                .iter()
                .find(|screen| valid_screen(screen) && screen.primary)
        })
        .or_else(|| screens.iter().find(|screen| valid_screen(screen)))?;
    let root_size = [
        logical_size[0] * screen.scale,
        logical_size[1] * screen.scale,
    ];
    Some([
        (screen.origin[0] + saved.x as f64 * screen.scale).clamp(
            screen.origin[0],
            screen.origin[0] + (screen.size[0] - root_size[0]).max(0.),
        ),
        (screen.origin[1] + saved.y as f64 * screen.scale).clamp(
            screen.origin[1],
            screen.origin[1] + (screen.size[1] - root_size[1]).max(0.),
        ),
    ])
}

/// Recover a collapsed anchor from a dragged physical root on the actual monitor.
/// Selecting by both axes also handles vertically stacked and mixed-DPI monitors.
pub fn nook_anchor_from_root(
    root: [f64; 4],
    logical_size: [f64; 2],
    screens: &[Screen],
) -> Option<[f64; 4]> {
    if root
        .iter()
        .chain(logical_size.iter())
        .any(|v| !v.is_finite())
        || root[2] <= 0.
        || root[3] <= 0.
        || logical_size.iter().any(|v| *v <= 0.)
    {
        return None;
    }
    let center = [root[0] + root[2] / 2., root[1] + root[3] / 2.];
    let screen = screens.iter().filter(|s| valid_screen(s)).max_by(|a, b| {
        overlap(root, a)
            .total_cmp(&overlap(root, b))
            .then_with(|| distance_to_screen(center, b).total_cmp(&distance_to_screen(center, a)))
    })?;
    let size = [
        (logical_size[0] * screen.scale).min(screen.size[0]),
        (logical_size[1] * screen.scale).min(screen.size[1]),
    ];
    let bottom = center[1] > screen.origin[1] + screen.size[1] / 2.;
    let x = (center[0] - size[0] / 2.).clamp(
        screen.origin[0],
        screen.origin[0] + screen.size[0] - size[0],
    );
    let y = (if bottom {
        root[1] + root[3] - size[1]
    } else {
        root[1]
    })
    .clamp(
        screen.origin[1],
        screen.origin[1] + screen.size[1] - size[1],
    );
    Some([x, y, size[0], size[1]])
}

/// Root and screen bounds are physical desktop pixels; requested size/gap are logical units.
/// The panel grows toward available screen space and stays inside the selected monitor.
pub fn nook_panel_placement(
    root: [f64; 4],
    requested_logical: [f64; 2],
    gap_logical: f64,
    screens: &[Screen],
) -> Option<HoverPlacement> {
    if root
        .iter()
        .chain(requested_logical.iter())
        .any(|value| !value.is_finite())
        || !gap_logical.is_finite()
        || root[2] <= 0.
        || root[3] <= 0.
        || requested_logical.iter().any(|value| *value <= 0.)
        || gap_logical < 0.
    {
        return None;
    }
    let center = [root[0] + root[2] / 2., root[1] + root[3] / 2.];
    let screen = screens
        .iter()
        .filter(|screen| valid_screen(screen))
        .max_by(|a, b| {
            overlap(root, a).total_cmp(&overlap(root, b)).then_with(|| {
                distance_to_screen(center, b).total_cmp(&distance_to_screen(center, a))
            })
        })?;
    let gap = (gap_logical * screen.scale)
        .min(screen.size[0] / 8.)
        .min(screen.size[1] / 8.);
    let min_x = screen.origin[0] + gap;
    let max_x = screen.origin[0] + screen.size[0] - gap;
    let min_y = screen.origin[1] + gap;
    let max_y = screen.origin[1] + screen.size[1] - gap;
    let below = (max_y - root[1] - root[3] - gap).max(0.);
    let above = (root[1] - gap - min_y).max(0.);
    let go_below = below >= above;
    let available_height = below.max(above);
    // Very small monitors may force overlap; keep a bounded reachable panel.
    let height = (requested_logical[1] * screen.scale).min(
        available_height
            .max(STARTING_VALUE_MIN_PANEL_HEIGHT_PX)
            .min(max_y - min_y),
    );
    let width = (requested_logical[0] * screen.scale).min(max_x - min_x);
    if width <= 0. || height <= 0. {
        return None;
    }
    let x = (center[0] - width / 2.).clamp(min_x, max_x - width);
    let desired_y = if go_below {
        root[1] + root[3] + gap
    } else {
        root[1] - height - gap
    };
    let y = desired_y.clamp(min_y, max_y - height);
    Some(HoverPlacement {
        position: [x, y],
        size: [width, height],
    })
}

/// Morph the root around a *collapsed* anchor. Never save this transient result
/// as the anchor: otherwise every expansion walks the HUD across the desktop.
pub fn nook_root_placement(
    anchor: [f64; 4],
    requested_logical: [f64; 2],
    screens: &[Screen],
) -> Option<HoverPlacement> {
    if anchor
        .iter()
        .chain(requested_logical.iter())
        .any(|v| !v.is_finite())
        || anchor[2] <= 0.
        || anchor[3] <= 0.
        || requested_logical.iter().any(|v| *v <= 0.)
    {
        return None;
    }
    let center = [anchor[0] + anchor[2] / 2., anchor[1] + anchor[3] / 2.];
    let screen = screens.iter().filter(|s| valid_screen(s)).max_by(|a, b| {
        overlap(anchor, a)
            .total_cmp(&overlap(anchor, b))
            .then_with(|| distance_to_screen(center, b).total_cmp(&distance_to_screen(center, a)))
    })?;
    let width = (requested_logical[0] * screen.scale).min(screen.size[0]);
    let height = (requested_logical[1] * screen.scale).min(screen.size[1]);
    let grows_down = center[1] <= screen.origin[1] + screen.size[1] / 2.;
    let x =
        (center[0] - width / 2.).clamp(screen.origin[0], screen.origin[0] + screen.size[0] - width);
    let y = (if grows_down {
        anchor[1]
    } else {
        anchor[1] + anchor[3] - height
    })
    .clamp(screen.origin[1], screen.origin[1] + screen.size[1] - height);
    Some(HoverPlacement {
        position: [x, y],
        size: [width, height],
    })
}

fn valid_screen(screen: &Screen) -> bool {
    screen
        .origin
        .iter()
        .chain(screen.size.iter())
        .all(|value| value.is_finite())
        && screen.size.iter().all(|value| *value > 0.)
        && screen.scale.is_finite()
        && screen.scale > 0.
}
fn distance_to_screen(point: [f64; 2], screen: &Screen) -> f64 {
    let dx = point[0] - point[0].clamp(screen.origin[0], screen.origin[0] + screen.size[0]);
    let dy = point[1] - point[1].clamp(screen.origin[1], screen.origin[1] + screen.size[1]);
    dx * dx + dy * dy
}
fn overlap(root: [f64; 4], screen: &Screen) -> f64 {
    ((root[0] + root[2]).min(screen.origin[0] + screen.size[0]) - root[0].max(screen.origin[0]))
        .max(0.)
        * ((root[1] + root[3]).min(screen.origin[1] + screen.size[1])
            - root[1].max(screen.origin[1]))
        .max(0.)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn screen(origin: [f64; 2], size: [f64; 2], scale: f64, id: &str) -> Screen {
        Screen {
            id: id.into(),
            name: id.into(),
            origin,
            size,
            scale,
            primary: id == "first",
        }
    }
    #[test]
    fn drag_and_restore_use_the_target_stacked_monitor_scale() {
        let screens = [
            screen([0., -1080.], [1920., 1080.], 1., "first"),
            screen([0., 0.], [2560., 1440.], 2., "second"),
        ];
        let root = [1700., 900., 800., 500.];
        let anchor = nook_anchor_from_root(root, [240., 40.], &screens).unwrap();
        assert_eq!(anchor, [1860., 1320., 480., 80.]);
        let saved = remember_nook_position([anchor[0], anchor[1]], &screens).unwrap();
        assert_eq!(saved.monitor_id, "second");
        assert_eq!(
            restore_nook_position(&saved, [240., 40.], &screens),
            Some([1860., 1320.])
        );
        let edge = SavedNookPosition {
            monitor_id: "second".into(),
            x: 1280,
            y: 720,
        };
        assert_eq!(
            restore_nook_position(&edge, [240., 40.], &screens),
            Some([2080., 1360.])
        );
    }
    #[test]
    fn root_morph_keeps_anchor_and_inward_edge_on_mixed_scale_monitors() {
        let screens = vec![
            screen([-1920., 0.], [1920., 1080.], 1., "first"),
            screen([0., -100.], [2560., 1440.], 2., "second"),
        ];
        for anchor in [
            [-1800., 0., 240., 40.],
            [-400., 1040., 240., 40.],
            [0., -100., 480., 80.],
            [2080., 1260., 480., 80.],
        ] {
            let expanded = nook_root_placement(anchor, [620., 288.], &screens).unwrap();
            let collapsed = nook_root_placement(anchor, [240., 40.], &screens).unwrap();
            assert_eq!(collapsed.position, [anchor[0], anchor[1]]);
            let screen = screens
                .iter()
                .find(|s| {
                    expanded.position[0] >= s.origin[0]
                        && expanded.position[0] + expanded.size[0] <= s.origin[0] + s.size[0]
                })
                .unwrap();
            assert!(expanded.position[1] >= screen.origin[1]);
            assert!(expanded.position[1] + expanded.size[1] <= screen.origin[1] + screen.size[1]);
            let at_top = anchor[1] < screen.origin[1] + screen.size[1] / 2.;
            if at_top {
                assert_eq!(expanded.position[1], anchor[1]);
            } else {
                assert_eq!(
                    expanded.position[1] + expanded.size[1],
                    anchor[1] + anchor[3]
                );
            }
        }
    }
    #[test]
    fn hover_dwell_interruption_and_delayed_collapse() {
        let mut state = PresentationState::new(&PresentationPreference::default());
        state.hover_enter(0);
        assert!(!state.advance(STARTING_VALUE_HOVER_DWELL_MS - 1, false, false));
        state.hover_leave(100);
        assert!(!state.advance(500, false, false));
        state.hover_enter(1_000);
        assert!(state.advance(1_000 + STARTING_VALUE_HOVER_DWELL_MS, false, false));
        assert_eq!(state.phase, Phase::Peek);
        state.hover_leave(1_300);
        state.hover_enter(1_400);
        assert!(!state.advance(2_000, false, false));
        state.hover_leave(2_000);
        assert!(state.advance(2_000 + STARTING_VALUE_COLLAPSE_DELAY_MS, false, false));
        assert_eq!(state.phase, Phase::Collapsed);
    }
    #[test]
    fn escape_outside_pin_and_drag() {
        let mut state = PresentationState::new(&PresentationPreference::default());
        state.drag_start(0, false, false);
        assert_eq!(state.phase, Phase::Expanded);
        assert!(!state.outside(20, false, false));
        state.drag_end(false, 20);
        assert!(state.set_pinned(true, 30, false, false));
        assert_eq!(state.phase, Phase::Pinned);
        assert!(!state.outside(40, false, false));
        assert!(state.escape(50, false, false));
        assert_eq!(state.phase, Phase::Collapsed);
    }
    #[test]
    fn reduced_motion_pressure_and_progress_clamps() {
        let mut state = PresentationState::new(&PresentationPreference::default());
        state.click(1_000, false, false);
        assert_eq!(state.progress(900, false, false), 0.);
        assert!((0.0..=1.0).contains(&state.progress(i64::MAX, false, false)));
        assert_eq!(state.progress(1_001, true, false), 1.);
        state.advance(1_001, true, false);
        assert_eq!(state.next_wake_ms(1_001), None);
        state.escape(1_010, false, true);
        assert_eq!(state.progress(1_010, false, true), 0.);
    }
    #[test]
    fn existing_preferences_upgrade_without_changing_layout_or_pin() {
        let preference: PresentationPreference = serde_json::from_value(serde_json::json!({
            "mode": "pill", "pinned": true,
            "nook_position": { "monitor_id": "first", "x": 12, "y": 34 }
        }))
        .unwrap();
        assert_eq!(preference.mode, PresentationMode::Pill);
        assert!(preference.pinned);
        assert_eq!(preference.size, NookSize::Standard);
        assert!(preference.hover_to_peek && preference.auto_collapse);
        assert_eq!(preference.nook_position.unwrap().x, 12);
    }
    #[test]
    fn hover_and_auto_collapse_policies_apply_to_pending_input_and_drag() {
        let mut preference = PresentationPreference::default();
        let mut state = PresentationState::new(&preference);
        state.hover_enter(0);
        preference.hover_to_peek = false;
        state.configure(&preference, MotionStyle::Playful);
        assert!(!state.advance(500, false, false));
        state.hover_enter(600);
        assert_eq!(state.next_wake_ms(600), None);
        assert!(state.click(700, true, false));
        state.hover_leave(800);
        preference.auto_collapse = false;
        state.configure(&preference, MotionStyle::Playful);
        assert!(!state.advance(1500, true, false));
        assert!(!state.outside(1500, true, false));
        state.drag_start(1600, true, false);
        state.drag_end(false, 1700);
        assert_eq!(state.next_wake_ms(1700), None);
        assert_eq!(state.phase, Phase::Expanded);
        assert!(state.escape(1800, true, false));
        preference.hover_to_peek = true;
        preference.auto_collapse = true;
        state.configure(&preference, MotionStyle::Chaotic);
        state.hover_enter(2000);
        assert!(state.advance(2000 + STARTING_VALUE_HOVER_DWELL_MS, true, false));
        assert_eq!(state.phase, Phase::Peek);
        state.hover_leave(2200);
        assert!(state.advance(2200 + STARTING_VALUE_COLLAPSE_DELAY_MS, true, false));
        assert_eq!(state.phase, Phase::Collapsed);
    }
    #[test]
    fn motion_responds_early_stays_bounded_and_retargets_without_a_jump() {
        let preference = PresentationPreference::default();
        let mut playful = PresentationState::new(&preference);
        playful.click(1000, false, false);
        let mut chaotic = PresentationState::new(&preference);
        chaotic.configure(&preference, MotionStyle::Chaotic);
        chaotic.click(1000, false, false);
        assert!(playful.progress(1045, false, false) > 0.5);
        assert!(
            (playful.progress(1045, false, false) - chaotic.progress(1045, false, false)).abs()
                > 0.05
        );
        for elapsed in 0..=STARTING_VALUE_MOTION_MS {
            assert!((0.0..=1.0).contains(&chaotic.progress(1000 + elapsed, false, false)));
        }
        let before = chaotic.progress(1080, false, false);
        chaotic.escape(1080, false, false);
        assert!((chaotic.progress(1080, false, false) - before).abs() < f32::EPSILON);
        chaotic.advance(1080 + STARTING_VALUE_MOTION_MS, false, false);
        assert_eq!(chaotic.progress(2000, false, false), 0.);
        assert_eq!(chaotic.next_wake_ms(2000), None);
    }
    #[test]
    fn geometry_clamps_across_edges_and_scaled_monitors() {
        let screens = [
            screen([0., 0.], [800., 600.], 1., "first"),
            screen([-500., 0.], [500., 300.], 1.5, "second"),
        ];
        for root in [
            [0., 0., 80., 20.],
            [720., 580., 80., 20.],
            [-500., 0., 80., 20.],
            [-80., 280., 80., 20.],
        ] {
            let placement = nook_panel_placement(root, [400., 300.], 8., &screens).unwrap();
            let screen = if root[0] < 0. {
                &screens[1]
            } else {
                &screens[0]
            };
            assert!(placement.position[0] >= screen.origin[0]);
            assert!(placement.position[1] >= screen.origin[1]);
            assert!(placement.position[0] + placement.size[0] <= screen.origin[0] + screen.size[0]);
            assert!(placement.position[1] + placement.size[1] <= screen.origin[1] + screen.size[1]);
        }
        let saved = remember_nook_position([-300., 10.], &screens).unwrap();
        assert_eq!(saved.monitor_id, "second");
        assert!(restore_nook_position(&saved, [80., 20.], &screens).unwrap()[0] < 0.);
        let tiny = [screen([0., 0.], [120., 90.], 1.25, "tiny")];
        let placement = nook_panel_placement([30., 0., 60., 18.], [400., 300.], 8., &tiny).unwrap();
        assert!(placement.size[0] <= 120. && placement.size[1] <= 90.);
        assert!(placement.position[1] + placement.size[1] <= 90.);
    }
}
