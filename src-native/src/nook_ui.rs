//! Native Nook surface. Dimensions are starting values until reference playback
//! is compared. The original Pill and Settings remain independent surfaces.
use crate::{desktop, model::*, nook::*, paint, productivity, App};
use eframe::egui::{
    self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2, ViewportCommand, ViewportId,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

pub const COLLAPSED_SIZE: [f64; 2] = [240., 40.];
const PEEK_SIZE: [f64; 2] = [520., 112.];
const OPEN_SIZE: [f64; 2] = [620., 288.];
const INK: Color32 = Color32::from_rgb(240, 241, 244);
const DIM: Color32 = Color32::from_rgb(145, 148, 156);
const BLACK: Color32 = Color32::from_rgb(8, 9, 12);
const SURFACE: Color32 = Color32::from_rgb(24, 26, 31);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Instruments,
    Ai,
    Notes,
    Tasks,
    Timer,
    Files,
}
impl Tab {
    fn title(self) -> &'static str {
        match self {
            Self::Instruments => "Instruments",
            Self::Ai => "AI",
            Self::Notes => "Notes",
            Self::Tasks => "Tasks",
            Self::Timer => "Timer",
            Self::Files => "Files",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Instruments => "cpu",
            Self::Ai => "codex",
            Self::Notes => "notes",
            Self::Tasks => "tasks",
            Self::Timer => "timer",
            Self::Files => "files",
        }
    }
}

enum FileJob {
    Resolve(PathBuf),
    Inspect(Vec<PathBuf>),
    Open(PathBuf),
}
enum FileReply {
    Resolved(Result<PathBuf, String>),
    Inspected(Vec<(PathBuf, bool)>),
    Opened(Result<(), String>),
}

/// Filesystem inspection and OS opening run off the paint thread, only on a
/// user action or while the file shelf is visible. One bounded worker owns I/O.
struct FileWorker {
    tx: mpsc::SyncSender<FileJob>,
    rx: mpsc::Receiver<FileReply>,
}
impl FileWorker {
    fn new(ctx: egui::Context) -> Self {
        let (tx, commands) = mpsc::sync_channel(8);
        let (events, rx) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(job) = commands.recv() {
                let reply = match job {
                    FileJob::Resolve(path) => {
                        let mut state = productivity::ProductivityState::default();
                        FileReply::Resolved(
                            state.add_file(path, 0).map(|_| state.files.remove(0).path),
                        )
                    }
                    FileJob::Inspect(paths) => FileReply::Inspected(
                        paths
                            .into_iter()
                            .map(|path| {
                                let exists = path.exists();
                                (path, exists)
                            })
                            .collect(),
                    ),
                    FileJob::Open(path) => FileReply::Opened(if path.exists() {
                        open::that(path).map_err(|e| e.to_string())
                    } else {
                        Err("The original file is no longer at this location".into())
                    }),
                };
                if events.send(reply).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self { tx, rx }
    }
}

pub struct NookUi {
    pub presentation: PresentationState,
    pub tab: Tab,
    pub initialized: bool,
    pub force_pill: bool,
    pub smoke_tab: Option<usize>,
    pill_initialized: bool,
    anchor: Option<[f64; 4]>,
    commanded: Option<HoverPlacement>,
    coordinate_scale: f64,
    inside: bool,
    focused: bool,
    dragging: bool,
    drag_seen_down: bool,
    drag_started: Instant,
    tab_changed_ms: i64,
    task_draft: String,
    task_page: usize,
    timer_minutes: u32,
    file_draft: String,
    file_page: usize,
    files: FileWorker,
    file_presence: HashMap<PathBuf, bool>,
    last_file_probe: Instant,
    file_probe_pending: bool,
    file_pending: usize,
    last_smoke_state: Option<(Phase, Tab)>,
}
impl NookUi {
    pub fn new(ctx: egui::Context, force_pill: bool, smoke_tab: Option<usize>) -> Self {
        Self {
            presentation: PresentationState::new(&PresentationPreference::default()),
            tab: Tab::Instruments,
            initialized: false,
            force_pill,
            smoke_tab,
            pill_initialized: false,
            anchor: None,
            commanded: None,
            coordinate_scale: 1.,
            inside: false,
            focused: false,
            dragging: false,
            drag_seen_down: false,
            drag_started: Instant::now(),
            tab_changed_ms: 0,
            task_draft: String::new(),
            task_page: 0,
            timer_minutes: 25,
            file_draft: String::new(),
            file_page: 0,
            files: FileWorker::new(ctx),
            file_presence: HashMap::new(),
            last_file_probe: Instant::now() - Duration::from_secs(60),
            file_probe_pending: false,
            file_pending: 0,
            last_smoke_state: None,
        }
    }
}

fn epoch_ms() -> i64 {
    (crate::now() * 1000.) as i64
}
fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}
fn capsule_size(progress: f32) -> [f64; 2] {
    let (a, b, t) = if progress <= STARTING_VALUE_PEEK_PROGRESS {
        (
            COLLAPSED_SIZE,
            PEEK_SIZE,
            progress / STARTING_VALUE_PEEK_PROGRESS,
        )
    } else {
        (
            PEEK_SIZE,
            OPEN_SIZE,
            (progress - STARTING_VALUE_PEEK_PROGRESS) / (1. - STARTING_VALUE_PEEK_PROGRESS),
        )
    };
    [
        a[0] + (b[0] - a[0]) * f64::from(t),
        a[1] + (b[1] - a[1]) * f64::from(t),
    ]
}

fn icon_button(
    ui: &mut egui::Ui,
    rect: Rect,
    key: &str,
    label: &str,
    active: bool,
    accent: Color32,
) -> egui::Response {
    let response = ui.interact(
        rect,
        ui.id()
            .with((key, label, rect.min.x.to_bits(), rect.min.y.to_bits())),
        Sense::click(),
    );
    if response.hovered() || active {
        ui.painter().rect_filled(
            rect,
            8,
            if active {
                accent.gamma_multiply(0.15)
            } else {
                SURFACE
            },
        );
    }
    paint::icon(
        ui.painter(),
        rect.center(),
        key,
        if active { accent } else { INK },
        16.,
    );
    // This tooltip stays inside the panel's content. Tiny root hover readings use
    // the existing inward-facing native hover viewport instead.
    response.on_hover_text(label)
}

impl App {
    pub(crate) fn is_nook(&self) -> bool {
        !self.nook_ui.force_pill
            && self
                .productivity
                .as_ref()
                .is_none_or(|c| c.state.presentation.mode == PresentationMode::Nook)
    }
    fn nook_motion_flags(&self) -> (bool, bool) {
        (
            flag(&self.profile, "reducedMotion") || text(&self.profile, "motion") == "quiet",
            self.mode != Mode::Normal,
        )
    }
    fn edit_productivity(
        &mut self,
        edit: impl FnOnce(&mut productivity::ProductivityState) -> Result<(), String>,
    ) {
        let result = self
            .productivity
            .as_mut()
            .ok_or("Nook storage is unavailable".to_owned())
            .and_then(|controller| {
                if !controller.writable() {
                    return Err("Nook storage is not writable".into());
                }
                edit(&mut controller.state)?;
                controller.mark_changed(epoch_ms()).map(|_| ())
            });
        if let Err(error) = result {
            self.productivity_status = error;
        }
    }
    pub(crate) fn set_presentation(&mut self, mode: PresentationMode, ctx: &egui::Context) {
        if self.productivity.as_ref().is_none_or(|c| !c.writable()) {
            return;
        }
        self.edit_productivity(|state| {
            state.presentation.mode = mode;
            Ok(())
        });
        self.nook_ui.initialized = false;
        self.nook_ui.pill_initialized = false;
        self.nook_ui.commanded = None;
        self.nook_ui.force_pill = false;
        self.hover = None;
        if mode == PresentationMode::Pill {
            self.resize(ctx);
            let width = if text(&self.profile, "size") == "compressed" {
                160.
            } else {
                280.
            };
            let scale = desktop::coordinate_scale(ctx.pixels_per_point());
            if let Some([x, y]) =
                restore_position(&self.profile["windowPosition"], &self.screens, [width, 56.])
            {
                ctx.send_viewport_cmd_to(
                    ViewportId::ROOT,
                    ViewportCommand::OuterPosition(Pos2::new(
                        (x / scale) as f32,
                        (y / scale) as f32,
                    )),
                );
            }
            self.position_hold = Instant::now() + Duration::from_millis(350);
        }
        ctx.request_repaint();
    }
    pub(crate) fn toggle_nook(&mut self, ctx: &egui::Context) {
        let (reduced, pressure) = self.nook_motion_flags();
        let was_pinned = self.nook_ui.presentation.is_pinned();
        self.nook_ui
            .presentation
            .escape(epoch_ms(), reduced, pressure);
        if was_pinned {
            self.edit_productivity(|s| {
                s.presentation.pinned = false;
                Ok(())
            });
        }
        ctx.request_repaint();
    }
    pub(crate) fn reset_nook(&mut self, ctx: &egui::Context) {
        self.edit_productivity(|s| {
            s.presentation.nook_position = None;
            Ok(())
        });
        self.nook_ui.anchor = None;
        self.nook_ui.initialized = false;
        self.nook_ui.commanded = None;
        ctx.request_repaint();
    }
    pub(crate) fn initialize_pill(&mut self, ctx: &egui::Context) {
        if self.nook_ui.pill_initialized
            || !self.loaded
            || self.productivity.as_ref().is_some_and(|c| c.loading())
        {
            return;
        }
        self.nook_ui.pill_initialized = true;
        self.resize(ctx);
        if self.smoke {
            return;
        }
        let width = if text(&self.profile, "size") == "compressed" {
            160.
        } else {
            280.
        };
        let scale = desktop::coordinate_scale(ctx.pixels_per_point());
        if let Some([x, y]) =
            restore_position(&self.profile["windowPosition"], &self.screens, [width, 56.])
        {
            ctx.send_viewport_cmd_to(
                ViewportId::ROOT,
                ViewportCommand::OuterPosition(Pos2::new((x / scale) as f32, (y / scale) as f32)),
            );
            self.position_hold = Instant::now() + Duration::from_millis(350);
        }
    }
    fn initialize_nook(&mut self, ctx: &egui::Context) {
        if self.nook_ui.initialized
            || !self.loaded
            || self.productivity.as_ref().is_some_and(|c| c.loading())
        {
            return;
        }
        let preference = self
            .productivity
            .as_ref()
            .map(|c| c.state.presentation.clone())
            .unwrap_or_default();
        self.nook_ui.presentation = PresentationState::new(&preference);
        let scale = desktop::coordinate_scale(ctx.pixels_per_point());
        let target_screen = preference
            .nook_position
            .as_ref()
            .and_then(|p| self.screens.iter().find(|s| s.id == p.monitor_id))
            .or_else(|| self.screens.iter().find(|s| s.primary))
            .or(self.screens.first());
        let target_scale = target_screen.map_or(scale, |s| s.scale);
        let size = [
            COLLAPSED_SIZE[0] * target_scale,
            COLLAPSED_SIZE[1] * target_scale,
        ];
        let position = preference
            .nook_position
            .as_ref()
            .and_then(|p| restore_nook_position(p, COLLAPSED_SIZE, &self.screens))
            .or_else(|| {
                target_screen.map(|s| {
                    [
                        s.origin[0] + (s.size[0] - size[0]).max(0.) / 2.,
                        s.origin[1],
                    ]
                })
            })
            .unwrap_or([60. * scale, 60. * scale]);
        self.nook_ui.anchor = Some([position[0], position[1], size[0], size[1]]);
        self.nook_ui.coordinate_scale = scale;
        self.nook_ui.commanded = None;
        self.nook_ui.initialized = true;
        if let Some(index) = self.nook_ui.smoke_tab {
            self.nook_ui.tab = [
                Tab::Instruments,
                Tab::Ai,
                Tab::Notes,
                Tab::Tasks,
                Tab::Timer,
                Tab::Files,
            ][index.min(5)];
            self.nook_ui.presentation.click(epoch_ms(), true, false);
        }
    }
    fn save_nook_anchor(&mut self) {
        if self.smoke {
            return;
        }
        if let Some(anchor) = self.nook_ui.anchor {
            if let Some(position) = remember_nook_position([anchor[0], anchor[1]], &self.screens) {
                if self.productivity.as_ref().is_some_and(|c| {
                    c.writable() && c.state.presentation.nook_position.as_ref() != Some(&position)
                }) {
                    self.edit_productivity(|s| {
                        s.presentation.nook_position = Some(position);
                        Ok(())
                    });
                }
            }
        }
    }
    fn place_nook(&mut self, ctx: &egui::Context, progress: f32) {
        if self.nook_ui.dragging {
            return;
        }
        let Some(anchor) = self.nook_ui.anchor else {
            return;
        };
        let Some(placement) = nook_root_placement(anchor, capsule_size(progress), &self.screens)
        else {
            return;
        };
        let scale = desktop::coordinate_scale(ctx.pixels_per_point());
        let changed = self.nook_ui.commanded.as_ref().is_none_or(|old| {
            old.position
                .iter()
                .zip(placement.position)
                .any(|(a, b)| (a - b).abs() >= 0.5)
                || old
                    .size
                    .iter()
                    .zip(placement.size)
                    .any(|(a, b)| (a - b).abs() >= 0.5)
        }) || (scale - self.nook_ui.coordinate_scale).abs() > 0.01;
        if changed {
            ctx.send_viewport_cmd_to(
                ViewportId::ROOT,
                ViewportCommand::InnerSize(Vec2::new(
                    (placement.size[0] / scale) as f32,
                    (placement.size[1] / scale) as f32,
                )),
            );
            ctx.send_viewport_cmd_to(
                ViewportId::ROOT,
                ViewportCommand::OuterPosition(Pos2::new(
                    (placement.position[0] / scale) as f32,
                    (placement.position[1] / scale) as f32,
                )),
            );
            self.nook_ui.commanded = Some(placement);
            self.nook_ui.coordinate_scale = scale;
        }
    }
    pub(crate) fn nook_hud(&mut self, ctx: &egui::Context) {
        if self.hidden {
            return;
        }
        self.initialize_nook(ctx);
        self.nook_file_events(ctx);
        let ms = epoch_ms();
        let (reduced, pressure) = self.nook_motion_flags();
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        let inside = ctx.input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|pos| i.content_rect().contains(pos))
        }) || (focused && ctx.wants_keyboard_input());
        if inside != self.nook_ui.inside {
            if inside {
                self.nook_ui.presentation.hover_enter(ms);
            } else {
                self.nook_ui.presentation.hover_leave(ms);
            }
            self.nook_ui.inside = inside;
        }
        if self.nook_ui.focused
            && !focused
            && !self.settings
            && !self.details
            && self.controls.is_none()
        {
            self.nook_ui.presentation.outside(ms, reduced, pressure);
        }
        self.nook_ui.focused = focused;
        if focused && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.toggle_nook(ctx);
        }
        self.nook_ui.presentation.advance(ms, reduced, pressure);
        let progress = self.nook_ui.presentation.progress(ms, reduced, pressure);
        self.place_nook(ctx, progress);
        let phase = self.nook_ui.presentation.phase;
        let settled = match phase {
            Phase::Collapsed => progress <= 0.001,
            Phase::Peek => (progress - STARTING_VALUE_PEEK_PROGRESS).abs() <= 0.001,
            Phase::Expanded | Phase::Pinned => progress >= 0.999,
        };
        if self.smoke && settled && self.nook_ui.last_smoke_state != Some((phase, self.nook_ui.tab))
        {
            self.nook_ui.last_smoke_state = Some((phase, self.nook_ui.tab));
            eprintln!(
                "NEON_NOOK phase={} tab={}",
                match phase {
                    Phase::Collapsed => "collapsed",
                    Phase::Peek => "peek",
                    Phase::Expanded => "expanded",
                    Phase::Pinned => "pinned",
                },
                self.nook_ui.tab.title()
            );
        }
        self.hover = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect().shrink(1.);
                let rounding = (18. + progress * 10.).round() as u8;
                ui.painter().rect_filled(rect, rounding, BLACK);
                ui.painter().rect_stroke(
                    rect,
                    rounding,
                    Stroke::new(1., Color32::from_gray(32)),
                    egui::StrokeKind::Inside,
                );
                let header = Rect::from_min_size(
                    rect.min + Vec2::new(10., 2.),
                    Vec2::new(rect.width() - 20., 36.),
                );
                self.nook_header(ui, header, progress, ctx);
                if rect.height() >= 76. && progress > 0.15 {
                    if progress < 0.65 {
                        self.nook_peek(ui, rect, ctx);
                    } else {
                        self.nook_content(ui, rect, ctx, reduced || pressure);
                    }
                }
                let right_click = ctx.input(|i| {
                    (i.pointer.button_clicked(egui::PointerButton::Secondary)
                        && i.pointer.interact_pos().is_some_and(|p| rect.contains(p)))
                        || (focused && i.modifiers.shift && i.key_pressed(egui::Key::F10))
                });
                if right_click {
                    self.controls = ctx.input(|i| i.viewport().outer_rect);
                    self.controls_generation = self.controls_generation.wrapping_add(1);
                    self.controls_focused = false;
                    self.hover = None;
                }
            });
        self.nook_drag_and_keys(ctx);
        if let Some(wake) = self.nook_ui.presentation.next_wake_ms(ms) {
            ctx.request_repaint_after(Duration::from_millis(wake.saturating_sub(ms).max(1) as u64));
        }
        if !self.nook_ui.dragging && self.nook_ui.presentation.phase == Phase::Collapsed {
            self.save_nook_anchor();
        }
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .take(8)
                .collect::<Vec<_>>()
        });
        for path in dropped {
            self.queue_nook_file(FileJob::Resolve(path));
        }
    }
    fn nook_header(&mut self, ui: &mut egui::Ui, r: Rect, progress: f32, ctx: &egui::Context) {
        let accent = self.palette().accent;
        let grip = Rect::from_min_size(r.min, Vec2::new(20., r.height()));
        for y in [-4., 0., 4.] {
            ui.painter()
                .circle_filled(grip.center() + Vec2::new(0., y), 1., DIM);
        }
        let drag = ui.interact(grip, ui.id().with("nook_drag"), Sense::click_and_drag());
        if drag.drag_started() {
            let (reduced, pressure) = self.nook_motion_flags();
            // Keep native geometry fixed during OS dragging; morph after release.
            self.nook_ui
                .presentation
                .drag_start(epoch_ms(), reduced, pressure);
            self.nook_ui.dragging = true;
            self.nook_ui.drag_seen_down = true;
            self.nook_ui.drag_started = Instant::now();
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let expanded = progress > 0.65;
        let right = if expanded { 78. } else { 34. };
        let toggle = Rect::from_min_max(r.min + Vec2::new(26., 0.), r.max - Vec2::new(right, 0.));
        let response = ui.interact(toggle, ui.id().with("nook_toggle"), Sense::click());
        if response.clicked() {
            let (reduced, pressure) = self.nook_motion_flags();
            self.nook_ui
                .presentation
                .click(epoch_ms(), reduced, pressure);
        }
        if expanded {
            paint::icon(
                ui.painter(),
                toggle.left_center() + Vec2::new(9., 0.),
                self.nook_ui.tab.key(),
                INK,
                18.,
            );
            ui.painter().text(
                toggle.left_center() + Vec2::new(25., 0.),
                egui::Align2::LEFT_CENTER,
                self.nook_ui.tab.title(),
                FontId::proportional(14.),
                INK,
            );
        } else {
            let (cpu, cpu_label, cpu_attention) = self.reading("cpu");
            let (_, ai_label, ai_attention) = self.reading("codex");
            paint::icon(
                ui.painter(),
                toggle.left_center() + Vec2::new(9., 0.),
                "cpu",
                if cpu_attention {
                    paint::stress(100.)
                } else {
                    DIM
                },
                16.,
            );
            ui.painter().text(
                toggle.left_center() + Vec2::new(24., 0.),
                egui::Align2::LEFT_CENTER,
                cpu_label,
                FontId::monospace(12.),
                if cpu.is_some() { INK } else { DIM },
            );
            let timer_label = self
                .productivity
                .as_ref()
                .and_then(|c| match c.state.timer {
                    productivity::Timer::Running { .. } | productivity::Timer::Paused { .. } => {
                        Some(format_timer(c.state.timer.remaining_ms(epoch_ms())))
                    }
                    _ => None,
                });
            let label = if Instant::now() < self.timer_notice_until {
                "Timer done".into()
            } else {
                timer_label.unwrap_or(ai_label)
            };
            let x = toggle.right() - 43.;
            paint::icon(
                ui.painter(),
                Pos2::new(x - 12., toggle.center().y),
                if self.productivity.as_ref().is_some_and(|c| {
                    matches!(
                        c.state.timer,
                        productivity::Timer::Running { .. } | productivity::Timer::Paused { .. }
                    )
                }) {
                    "timer"
                } else {
                    "codex"
                },
                if ai_attention {
                    self.palette().pop
                } else {
                    accent
                },
                16.,
            );
            ui.painter().text(
                Pos2::new(x, toggle.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                FontId::monospace(12.),
                INK,
            );
        }
        let controls =
            Rect::from_min_size(Pos2::new(r.right() - 30., r.top() + 3.), Vec2::splat(30.));
        if icon_button(ui, controls, "settings", "Settings", false, accent).clicked() {
            self.settings = true;
            self.hover = None;
        }
        if expanded {
            let pin =
                Rect::from_min_size(Pos2::new(r.right() - 68., r.top() + 3.), Vec2::splat(30.));
            if icon_button(
                ui,
                pin,
                "pin",
                "Keep open",
                self.nook_ui.presentation.is_pinned(),
                accent,
            )
            .clicked()
            {
                let pinned = !self.nook_ui.presentation.is_pinned();
                let (reduced, pressure) = self.nook_motion_flags();
                self.nook_ui
                    .presentation
                    .set_pinned(pinned, epoch_ms(), reduced, pressure);
                self.edit_productivity(|s| {
                    s.presentation.pinned = pinned;
                    Ok(())
                });
            }
        }
    }
    fn nook_peek(&mut self, ui: &mut egui::Ui, r: Rect, ctx: &egui::Context) {
        let keys = self.nook_metric_keys();
        let width = (r.width() - 28.) / keys.len().max(1) as f32;
        for (index, key) in keys.iter().enumerate() {
            let cell = Rect::from_min_size(
                r.min + Vec2::new(14. + index as f32 * width, 44.),
                Vec2::new(width, (r.height() - 51.).clamp(28., 55.)),
            );
            self.nook_metric(ui, cell, key, false, ctx);
        }
    }
    fn nook_metric_keys(&self) -> Vec<&'static str> {
        let mut keys = ["cpu", "gpu", "ram", "network", "storage", "codex", "claude"]
            .into_iter()
            .filter(|key| {
                flag(
                    &self.profile["metrics"],
                    if ["codex", "claude"].contains(key) {
                        "ai"
                    } else {
                        key
                    },
                )
            })
            .collect::<Vec<_>>();
        if keys.is_empty() {
            keys.push("cpu");
        }
        keys
    }
    fn nook_metric(
        &mut self,
        ui: &mut egui::Ui,
        cell: Rect,
        key: &str,
        large: bool,
        ctx: &egui::Context,
    ) {
        let response = ui.interact(cell, ui.id().with(("nook_metric", key)), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(cell, 12, SURFACE);
        }
        let (value, label, attention) = self.reading(key);
        let remaining = ["codex", "claude"].contains(&key);
        let color = value
            .map(|v| paint::stress(stress_percent(v, remaining)))
            .unwrap_or(DIM);
        let (reduced, pressure) = self.nook_motion_flags();
        let shake = if attention && Instant::now() < self.attention_until && !reduced && !pressure {
            ctx.request_repaint_after(Duration::from_millis(32));
            (ctx.input(|i| i.time) as f32 * 40.).sin() * 2.
        } else {
            0.
        };
        let c = cell.center_top() + Vec2::new(shake, if large { 19. } else { 11. });
        paint::icon(
            ui.painter(),
            c,
            key,
            if attention { self.palette().pop } else { color },
            16.,
        );
        if attention {
            ui.painter().text(
                c + Vec2::new(13., -7.),
                egui::Align2::CENTER_CENTER,
                "!",
                FontId::monospace(11.),
                self.palette().pop,
            );
        }
        let font_size = if large { 20. } else { 12. };
        let measured = ui
            .painter()
            .layout_no_wrap(label.clone(), FontId::monospace(font_size), INK)
            .size()
            .x;
        let font = FontId::monospace(
            (font_size * (cell.width() - 10.).max(1.) / measured.max(1.)).clamp(8., font_size),
        );
        ui.painter().text(
            cell.center_top() + Vec2::new(0., if large { 43. } else { 31. }),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            INK,
        );
        if large {
            ui.painter().text(
                cell.center_bottom() + Vec2::new(0., -28.),
                egui::Align2::CENTER_BOTTOM,
                metric_label(key),
                FontId::proportional(11.),
                DIM,
            );
            let bar = Rect::from_min_size(
                cell.left_bottom() + Vec2::new(10., -13.),
                Vec2::new(cell.width() - 20., 3.),
            );
            ui.painter().rect_filled(bar, 2, Color32::from_gray(48));
            if let Some(v) = value {
                let filled = bar.width() * v.clamp(0., 100.) as f32 / 100.;
                for i in 0..24 {
                    let x = i as f32 / 24. * filled;
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            bar.min + Vec2::new(x, 0.),
                            Vec2::new((filled / 24.).max(0.1), 3.),
                        ),
                        0,
                        paint::stress(stress_percent(i as f64 / 24. * v, remaining)),
                    );
                }
            }
        }
        if response.clicked() {
            self.selected = key.into();
            self.details = true;
            self.hover = None;
        } else if response.hovered() && !self.details {
            self.hover = ctx
                .input(|i| i.viewport().outer_rect)
                .map(|r| (key.into(), r));
        }
    }
    fn nook_content(&mut self, ui: &mut egui::Ui, r: Rect, ctx: &egui::Context, settle: bool) {
        let accent = self.palette().accent;
        let tabs = [
            Tab::Instruments,
            Tab::Ai,
            Tab::Notes,
            Tab::Tasks,
            Tab::Timer,
            Tab::Files,
        ];
        let tab_width = ((r.width() - 32.) / tabs.len() as f32).min(98.);
        for (i, tab) in tabs.into_iter().enumerate() {
            let tab_rect = Rect::from_min_size(
                r.min + Vec2::new(16. + i as f32 * tab_width, 43.),
                Vec2::new(tab_width - 4., 31.),
            );
            let response = ui.interact(
                tab_rect,
                ui.id().with(("nook_tab", tab.key())),
                Sense::click(),
            );
            let active = self.nook_ui.tab == tab;
            if active || response.hovered() {
                ui.painter().rect_filled(
                    tab_rect,
                    9,
                    if active {
                        Color32::from_gray(38)
                    } else {
                        SURFACE
                    },
                );
            }
            paint::icon(
                ui.painter(),
                tab_rect.left_center() + Vec2::new(15., 0.),
                tab.key(),
                if active { INK } else { DIM },
                14.,
            );
            ui.painter().text(
                tab_rect.left_center() + Vec2::new(28., 0.),
                egui::Align2::LEFT_CENTER,
                tab.title(),
                FontId::proportional(11.),
                if active { INK } else { DIM },
            );
            if response.clicked() && !active {
                self.nook_ui.tab = tab;
                self.nook_ui.tab_changed_ms = epoch_ms();
                self.hover = None;
            }
        }
        ui.painter().line_segment(
            [
                r.min + Vec2::new(16., 84.),
                Pos2::new(r.right() - 16., r.top() + 84.),
            ],
            Stroke::new(1., Color32::from_gray(30)),
        );
        let elapsed = epoch_ms()
            .saturating_sub(self.nook_ui.tab_changed_ms)
            .max(0);
        let fade = if settle {
            1.
        } else {
            (elapsed as f32 / 140.).clamp(0., 1.)
        };
        if fade < 1. {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        let content = Rect::from_min_max(
            r.min + Vec2::new(16., 94. + (1. - fade) * 5.),
            r.max - Vec2::new(16., 30.),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
            ui.set_clip_rect(content.intersect(ui.clip_rect()));
            ui.set_opacity(fade);
            match self.nook_ui.tab {
                Tab::Instruments => self.nook_instruments(ui, content, ctx),
                Tab::Ai => self.nook_ai(ui, content, ctx),
                Tab::Notes => self.nook_notes(ui, content),
                Tab::Tasks => self.nook_tasks(ui, content),
                Tab::Timer => self.nook_timer(ui, content),
                Tab::Files => self.nook_files(ui, content),
            }
        });
        let status = if self.productivity.as_ref().is_some_and(|c| !c.writable()) {
            self.productivity_status.as_str()
        } else if Instant::now() < self.timer_notice_until {
            "Timer complete"
        } else if self.nook_ui.tab == Tab::Files {
            self.productivity_status.as_str()
        } else if self.paused {
            "Monitoring paused"
        } else if self.mode != Mode::Normal {
            self.mode.name()
        } else {
            "Hover to peek · click to open · Esc to close"
        };
        ui.painter().text(
            r.left_bottom() + Vec2::new(20., -13.),
            egui::Align2::LEFT_CENTER,
            status,
            FontId::proportional(10.),
            DIM,
        );
        ui.painter().circle_filled(
            r.right_bottom() - Vec2::new(20., 13.),
            2.5,
            if self.paused {
                DIM
            } else if self.mode != Mode::Normal {
                paint::stress(95.)
            } else {
                accent
            },
        );
    }
    fn nook_instruments(&mut self, ui: &mut egui::Ui, r: Rect, ctx: &egui::Context) {
        let keys = self.nook_metric_keys();
        let width = r.width() / keys.len() as f32;
        for (i, key) in keys.iter().enumerate() {
            let cell = Rect::from_min_size(
                r.min + Vec2::new(i as f32 * width, 0.),
                Vec2::new(width - 4., 107.),
            );
            self.nook_metric(ui, cell, key, true, ctx);
        }
        let history = Rect::from_min_size(
            r.min + Vec2::new(7., 118.),
            Vec2::new((r.width() - 14.).max(1.), 32.),
        );
        let stride = history.width() / 120.;
        for (i, (_, value)) in self.history.iter().rev().take(120).rev().enumerate() {
            if let Some(value) = value {
                let height = (*value as f32 / 100.).clamp(0., 1.) * history.height();
                let x = history.right() - (self.history.len().min(120) - i) as f32 * stride;
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        Pos2::new(x, history.bottom() - height),
                        Vec2::new((stride - 1.).max(0.5), height),
                    ),
                    1,
                    paint::stress(*value).gamma_multiply(0.7),
                );
            }
        }
        if self.history.is_empty() {
            ui.painter().text(
                history.center(),
                egui::Align2::CENTER_CENTER,
                "Waiting for a live sample",
                FontId::proportional(12.),
                DIM,
            );
        }
    }
    fn nook_ai(&mut self, ui: &mut egui::Ui, r: Rect, ctx: &egui::Context) {
        let width = (r.width() - 12.) / 2.;
        for (i, key) in ["codex", "claude"].into_iter().enumerate() {
            let card = Rect::from_min_size(
                r.min + Vec2::new(i as f32 * (width + 12.), 0.),
                Vec2::new(width, r.height()),
            );
            ui.painter().rect_filled(card, 14, SURFACE);
            self.nook_metric(
                ui,
                Rect::from_min_size(card.min + Vec2::new(3., 0.), Vec2::new(105., 108.)),
                key,
                true,
                ctx,
            );
            let usage = self.usage(key);
            let window = array(&usage, "windows")
                .iter()
                .find(|w| number(w, "minutes") == Some(300.))
                .or_else(|| array(&usage, "windows").first());
            let line = if let Some(window) = window {
                let minutes = number(window, "minutes").unwrap_or(0.) as i64;
                let reset = number(window, "resetsAt")
                    .filter(|t| *t > crate::now())
                    .map(|t| {
                        format!(
                            "Resets in {}",
                            format_timer(((t - crate::now()) * 1000.) as i64)
                        )
                    })
                    .unwrap_or_else(|| "Waiting for reset data".into());
                let allowance = if minutes % 60 == 0 {
                    format!("{}h", minutes / 60)
                } else {
                    format!("{minutes}m")
                };
                format!(
                    "{allowance} allowance{}\n{reset}",
                    if text(&usage, "state") == "connected" {
                        ""
                    } else {
                        " · last known"
                    }
                )
            } else {
                format!(
                    "{}\nConnect in Settings",
                    if text(&usage, "state").is_empty() {
                        "Unavailable"
                    } else {
                        text(&usage, "state")
                    }
                )
            };
            ui.painter().text(
                card.min + Vec2::new(112., 18.),
                egui::Align2::LEFT_TOP,
                line,
                FontId::proportional(12.),
                DIM,
            );
            if let Some(window) = window {
                let rate = self.drain.rate(&usage, window, crate::now());
                let drain = rate
                    .per_hour
                    .map(|v| format!("{:.2}% / min avg", v / 60.))
                    .unwrap_or_else(|| "Measuring average drain".into());
                ui.painter().text(
                    card.min + Vec2::new(112., 71.),
                    egui::Align2::LEFT_TOP,
                    drain,
                    FontId::monospace(10.),
                    INK,
                );
            }
            let action_rect = Rect::from_min_size(
                card.left_bottom() + Vec2::new(12., -38.),
                Vec2::new(width - 24., 27.),
            );
            if ui
                .put(
                    action_rect,
                    egui::Button::new("Allowance, tokens & attention ↗")
                        .fill(Color32::from_gray(35)),
                )
                .clicked()
            {
                self.selected = key.into();
                self.details = true;
            }
        }
    }
    fn nook_notes(&mut self, ui: &mut egui::Ui, r: Rect) {
        let Some(controller) = self.productivity.as_mut() else {
            ui.label(&self.productivity_status);
            return;
        };
        let writable = controller.writable();
        ui.add_enabled_ui(writable, |ui| {
            let response = ui.put(
                r,
                egui::TextEdit::multiline(&mut controller.state.note)
                    .hint_text("A thought, before it disappears…")
                    .frame(false)
                    .font(FontId::proportional(16.))
                    .char_limit(productivity::MAX_NOTE_BYTES),
            );
            if response.changed() {
                while controller.state.note.len() > productivity::MAX_NOTE_BYTES {
                    controller.state.note.pop();
                }
                if let Err(error) = controller.mark_changed(epoch_ms()) {
                    self.productivity_status = error;
                }
            }
        });
    }
    fn nook_tasks(&mut self, ui: &mut egui::Ui, r: Rect) {
        let tasks = self
            .productivity
            .as_ref()
            .map(|c| c.state.tasks.clone())
            .unwrap_or_default();
        let writable = self.productivity.as_ref().is_some_and(|c| c.writable());
        self.nook_ui.task_page = self
            .nook_ui
            .task_page
            .min(tasks.len().saturating_sub(1) / 4);
        ui.add_enabled_ui(writable, |ui| {
            let input = ui.put(
                Rect::from_min_size(r.min, Vec2::new(r.width() - 66., 27.)),
                egui::TextEdit::singleline(&mut self.nook_ui.task_draft)
                    .hint_text("Add a task…")
                    .char_limit(128),
            );
            let add = ui.put(
                Rect::from_min_size(r.right_top() - Vec2::new(58., 0.), Vec2::new(58., 27.)),
                egui::Button::new("Add"),
            );
            if add.clicked()
                || (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                let draft = std::mem::take(&mut self.nook_ui.task_draft);
                if !draft.trim().is_empty() {
                    self.edit_productivity(|s| s.add_task(draft).map(|_| ()));
                }
            }
            for (i, task) in tasks
                .iter()
                .skip(self.nook_ui.task_page * 4)
                .take(4)
                .enumerate()
            {
                let row = Rect::from_min_size(
                    r.min + Vec2::new(0., 35. + i as f32 * 26.),
                    Vec2::new(r.width(), 24.),
                );
                let mut complete = task.complete;
                if ui
                    .put(
                        Rect::from_min_size(row.min, Vec2::new(24., 24.)),
                        egui::Checkbox::without_text(&mut complete),
                    )
                    .changed()
                {
                    self.edit_productivity(|s| {
                        s.set_task_complete(task.id, complete);
                        Ok(())
                    });
                }
                let mut task_text = task.text.clone();
                if ui
                    .put(
                        Rect::from_min_size(
                            row.min + Vec2::new(28., 0.),
                            Vec2::new(row.width() - 98., 24.),
                        ),
                        egui::TextEdit::singleline(&mut task_text)
                            .frame(false)
                            .char_limit(128)
                            .text_color(if complete { DIM } else { INK }),
                    )
                    .changed()
                    && !task_text.trim().is_empty()
                {
                    self.edit_productivity(|s| s.edit_task(task.id, task_text).map(|_| ()));
                }
                if icon_button(
                    ui,
                    Rect::from_min_size(row.right_top() - Vec2::new(60., 0.), Vec2::splat(24.)),
                    "star",
                    "Favorite task",
                    task.favorite,
                    self.palette().accent,
                )
                .clicked()
                {
                    self.edit_productivity(|s| {
                        s.set_task_favorite(task.id, !task.favorite);
                        Ok(())
                    });
                }
                if ui
                    .put(
                        Rect::from_min_size(row.right_top() - Vec2::new(27., 0.), Vec2::splat(24.)),
                        egui::Button::new("×").frame(false),
                    )
                    .clicked()
                {
                    self.edit_productivity(|s| {
                        s.remove_task(task.id);
                        Ok(())
                    });
                }
            }
        });
        let bottom = r.left_bottom() - Vec2::new(0., 21.);
        if ui
            .put(
                Rect::from_min_size(bottom, Vec2::new(110., 23.)),
                egui::Button::new("Clear completed").small(),
            )
            .clicked()
        {
            self.edit_productivity(|s| {
                s.clear_completed();
                Ok(())
            });
        }
        self.nook_task_paging(ui, r, tasks.len());
    }
    fn nook_task_paging(&mut self, ui: &mut egui::Ui, r: Rect, count: usize) {
        let y = r.bottom() - 21.;
        if ui
            .put(
                Rect::from_min_size(Pos2::new(r.right() - 100., y), Vec2::new(24., 23.)),
                egui::Button::new("‹").small(),
            )
            .clicked()
        {
            self.nook_ui.task_page = self.nook_ui.task_page.saturating_sub(1);
        }
        ui.painter().text(
            Pos2::new(r.right() - 50., y + 12.),
            egui::Align2::CENTER_CENTER,
            format!(
                "{}/{}",
                self.nook_ui.task_page + 1,
                count.max(1).div_ceil(4)
            ),
            FontId::monospace(10.),
            DIM,
        );
        if ui
            .put(
                Rect::from_min_size(Pos2::new(r.right() - 24., y), Vec2::new(24., 23.)),
                egui::Button::new("›").small(),
            )
            .clicked()
            && (self.nook_ui.task_page + 1) * 4 < count
        {
            self.nook_ui.task_page += 1;
        }
    }
    fn nook_timer(&mut self, ui: &mut egui::Ui, r: Rect) {
        let timer = self
            .productivity
            .as_ref()
            .map(|c| c.state.timer.clone())
            .unwrap_or(productivity::Timer::Idle);
        let remaining = timer.remaining_ms(epoch_ms());
        let text = if matches!(timer, productivity::Timer::Idle) {
            format_timer(i64::from(self.nook_ui.timer_minutes) * 60_000)
        } else if matches!(timer, productivity::Timer::Completed { .. }) {
            "Done".into()
        } else {
            format_timer(remaining)
        };
        let c = r.left_center() + Vec2::new(r.width() * 0.29, -8.);
        ui.painter().text(
            c,
            egui::Align2::CENTER_CENTER,
            text,
            FontId::monospace(48.),
            INK,
        );
        ui.painter().text(
            c + Vec2::new(0., 39.),
            egui::Align2::CENTER_CENTER,
            if matches!(timer, productivity::Timer::Paused { .. }) {
                "Paused"
            } else {
                "Focus timer"
            },
            FontId::proportional(12.),
            DIM,
        );
        let panel = Rect::from_min_max(r.min + Vec2::new(r.width() * 0.58, 2.), r.max);
        let writable = self.productivity.as_ref().is_some_and(|c| c.writable());
        ui.scope_builder(egui::UiBuilder::new().max_rect(panel), |ui| {
            ui.add_enabled_ui(writable, |ui| {
                ui.horizontal(|ui| {
                    for minutes in [5, 10, 25] {
                        if ui
                            .selectable_label(
                                self.nook_ui.timer_minutes == minutes,
                                format!("{minutes}m"),
                            )
                            .clicked()
                        {
                            self.nook_ui.timer_minutes = minutes;
                        }
                    }
                });
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    ui.label("Minutes");
                    ui.add(egui::DragValue::new(&mut self.nook_ui.timer_minutes).range(1..=180));
                });
                ui.add_space(16.);
                ui.horizontal(|ui| {
                    let label = match timer {
                        productivity::Timer::Running { .. } => "Pause",
                        productivity::Timer::Paused { .. } => "Resume",
                        _ => "Start timer",
                    };
                    if ui.button(label).clicked() {
                        let minutes = self.nook_ui.timer_minutes;
                        self.edit_productivity(|s| match s.timer {
                            productivity::Timer::Running { .. } => {
                                s.timer.pause(epoch_ms());
                                Ok(())
                            }
                            productivity::Timer::Paused { .. } => {
                                s.timer.resume(epoch_ms());
                                Ok(())
                            }
                            _ => s.timer.start(epoch_ms(), i64::from(minutes) * 60_000),
                        });
                    }
                    if !matches!(timer, productivity::Timer::Idle) && ui.button("Reset").clicked() {
                        self.edit_productivity(|s| {
                            s.timer.cancel();
                            Ok(())
                        });
                    }
                });
            });
        });
    }
    fn queue_nook_file(&mut self, job: FileJob) {
        if self.productivity.as_ref().is_none_or(|c| !c.writable()) {
            return;
        }
        if self.nook_ui.files.tx.try_send(job).is_ok() {
            self.nook_ui.file_pending += 1;
        } else {
            self.productivity_status = "File shelf busy; try again".into();
        }
    }
    fn nook_file_events(&mut self, ctx: &egui::Context) {
        while let Ok(reply) = self.nook_ui.files.rx.try_recv() {
            self.nook_ui.file_pending = self.nook_ui.file_pending.saturating_sub(1);
            match reply {
                FileReply::Resolved(Ok(path)) => {
                    self.edit_productivity(|s| {
                        if s.files.iter().any(|f| same_file(&f.path, &path)) {
                            return Ok(());
                        }
                        if s.files.len() >= productivity::MAX_FILES {
                            return Err("File shelf is full".into());
                        }
                        s.files.push(productivity::FileReference {
                            path,
                            added_at_ms: epoch_ms(),
                        });
                        Ok(())
                    });
                    self.nook_ui.tab = Tab::Files;
                    if self.nook_ui.presentation.phase == Phase::Collapsed
                        || self.nook_ui.presentation.phase == Phase::Peek
                    {
                        let (reduced, pressure) = self.nook_motion_flags();
                        self.nook_ui
                            .presentation
                            .click(epoch_ms(), reduced, pressure);
                    }
                    self.nook_ui.last_file_probe = Instant::now() - Duration::from_secs(60);
                }
                FileReply::Resolved(Err(e)) | FileReply::Opened(Err(e)) => {
                    self.productivity_status = e
                }
                FileReply::Opened(Ok(())) => {
                    self.productivity_status = "Opened original file".into()
                }
                FileReply::Inspected(paths) => {
                    self.nook_ui.file_probe_pending = false;
                    self.nook_ui.file_presence = paths.into_iter().collect();
                    self.nook_ui.last_file_probe = Instant::now();
                }
            }
        }
        if self.nook_ui.file_pending > 0 {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
    fn nook_files(&mut self, ui: &mut egui::Ui, r: Rect) {
        let files = self
            .productivity
            .as_ref()
            .map(|c| c.state.files.clone())
            .unwrap_or_default();
        let writable = self.productivity.as_ref().is_some_and(|c| c.writable());
        self.nook_ui.file_page = self
            .nook_ui
            .file_page
            .min(files.len().saturating_sub(1) / 3);
        if !self.nook_ui.file_probe_pending
            && !files.is_empty()
            && self.nook_ui.last_file_probe.elapsed() > Duration::from_secs(30)
        {
            if self
                .nook_ui
                .files
                .tx
                .try_send(FileJob::Inspect(
                    files.iter().map(|f| f.path.clone()).collect(),
                ))
                .is_ok()
            {
                self.nook_ui.file_probe_pending = true;
                self.nook_ui.file_pending += 1;
            }
        }
        ui.add_enabled_ui(writable, |ui| {
            let input = ui.put(
                Rect::from_min_size(r.min, Vec2::new(r.width() - 72., 27.)),
                egui::TextEdit::singleline(&mut self.nook_ui.file_draft)
                    .hint_text("Drop files here, or paste a file path…")
                    .char_limit(4096),
            );
            if ui
                .put(
                    Rect::from_min_size(r.right_top() - Vec2::new(64., 0.), Vec2::new(64., 27.)),
                    egui::Button::new("Add file"),
                )
                .clicked()
                || (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                let draft = std::mem::take(&mut self.nook_ui.file_draft);
                if !draft.trim().is_empty() {
                    self.queue_nook_file(FileJob::Resolve(PathBuf::from(
                        draft.trim().trim_matches('"'),
                    )));
                }
            }
            for (i, file) in files
                .iter()
                .skip(self.nook_ui.file_page * 3)
                .take(3)
                .enumerate()
            {
                let row = Rect::from_min_size(
                    r.min + Vec2::new(0., 36. + i as f32 * 32.),
                    Vec2::new(r.width(), 29.),
                );
                let missing = self.nook_ui.file_presence.get(&file.path) == Some(&false);
                let name = file
                    .path
                    .file_name()
                    .unwrap_or(file.path.as_os_str())
                    .to_string_lossy();
                if ui
                    .put(
                        Rect::from_min_size(row.min, Vec2::new(row.width() - 120., 29.)),
                        egui::Button::new(format!("{}  {name}", if missing { "!" } else { "↗" }))
                            .frame(false),
                    )
                    .on_hover_text(file.path.display().to_string())
                    .clicked()
                {
                    self.queue_nook_file(FileJob::Open(file.path.clone()));
                }
                ui.painter().text(
                    row.right_center() - Vec2::new(40., 0.),
                    egui::Align2::RIGHT_CENTER,
                    if missing { "Missing" } else { "Reference" },
                    FontId::proportional(11.),
                    if missing { paint::stress(100.) } else { DIM },
                );
                if ui
                    .put(
                        Rect::from_min_size(row.right_top() - Vec2::new(27., 0.), Vec2::splat(27.)),
                        egui::Button::new("×").frame(false),
                    )
                    .clicked()
                {
                    self.edit_productivity(|s| {
                        s.files.retain(|f| !same_file(&f.path, &file.path));
                        Ok(())
                    });
                }
            }
        });
        ui.painter().text(
            r.left_bottom() - Vec2::new(0., 8.),
            egui::Align2::LEFT_BOTTOM,
            if files.is_empty() {
                "A small shelf. Your files stay where they are."
            } else {
                "Removing a reference keeps your original file."
            },
            FontId::proportional(11.),
            DIM,
        );
        let y = r.bottom() - 25.;
        if ui
            .put(
                Rect::from_min_size(Pos2::new(r.right() - 82., y), Vec2::new(25., 23.)),
                egui::Button::new("‹").small(),
            )
            .clicked()
        {
            self.nook_ui.file_page = self.nook_ui.file_page.saturating_sub(1);
        }
        ui.painter().text(
            Pos2::new(r.right() - 40., y + 12.),
            egui::Align2::CENTER_CENTER,
            format!("{}", self.nook_ui.file_page + 1),
            FontId::monospace(10.),
            DIM,
        );
        if ui
            .put(
                Rect::from_min_size(Pos2::new(r.right() - 24., y), Vec2::new(24., 23.)),
                egui::Button::new("›").small(),
            )
            .clicked()
            && (self.nook_ui.file_page + 1) * 3 < files.len()
        {
            self.nook_ui.file_page += 1;
        }
    }
    fn nook_drag_and_keys(&mut self, ctx: &egui::Context) {
        let scale = desktop::coordinate_scale(ctx.pixels_per_point());
        if self.nook_ui.dragging {
            let down = ctx.input(|i| i.pointer.primary_down());
            if let Some(root) = ctx.input(|i| i.viewport().outer_rect) {
                let physical = [
                    root.min.x as f64 * scale,
                    root.min.y as f64 * scale,
                    root.width() as f64 * scale,
                    root.height() as f64 * scale,
                ];
                if let Some(anchor) = nook_anchor_from_root(physical, COLLAPSED_SIZE, &self.screens)
                {
                    self.nook_ui.anchor = Some(anchor);
                }
            }
            if !down
                && self.nook_ui.drag_seen_down
                && self.nook_ui.drag_started.elapsed() > Duration::from_millis(100)
            {
                self.nook_ui.dragging = false;
                self.nook_ui.commanded = None;
                self.nook_ui
                    .presentation
                    .drag_end(self.nook_ui.inside, epoch_ms());
                self.save_nook_anchor();
            }
        }
        if ctx.input(|i| i.viewport().focused.unwrap_or(false)) && !ctx.wants_keyboard_input() {
            let delta = ctx.input(|i| {
                let step = if i.modifiers.shift { 1. } else { 10. };
                [
                    (i.key_pressed(egui::Key::ArrowRight) as i8
                        - i.key_pressed(egui::Key::ArrowLeft) as i8) as f64
                        * step
                        * scale,
                    (i.key_pressed(egui::Key::ArrowDown) as i8
                        - i.key_pressed(egui::Key::ArrowUp) as i8) as f64
                        * step
                        * scale,
                ]
            });
            if delta != [0., 0.] {
                if let Some(mut anchor) = self.nook_ui.anchor {
                    anchor[0] += delta[0];
                    anchor[1] += delta[1];
                    if let Some(placement) =
                        nook_root_placement(anchor, COLLAPSED_SIZE, &self.screens)
                    {
                        anchor = [
                            placement.position[0],
                            placement.position[1],
                            placement.size[0],
                            placement.size[1],
                        ];
                        self.nook_ui.anchor = Some(anchor);
                        self.nook_ui.commanded = None;
                        self.save_nook_anchor();
                    }
                }
            }
        }
    }
    pub(crate) fn timer_notice_window(&mut self, ctx: &egui::Context) {
        if Instant::now() >= self.timer_notice_until {
            return;
        }
        let scale = desktop::coordinate_scale(ctx.pixels_per_point());
        let root = ctx
            .input(|i| i.viewport().outer_rect)
            .map(|r| {
                [
                    r.min.x as f64 * scale,
                    r.min.y as f64 * scale,
                    r.width() as f64 * scale,
                    r.height() as f64 * scale,
                ]
            })
            .or(self.nook_ui.anchor);
        let Some(root) = root else {
            return;
        };
        let Some(placement) =
            hover_placement(root, [240. * scale, 64. * scale], 8. * scale, &self.screens)
        else {
            return;
        };
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("nook_timer_notice"),
            egui::ViewportBuilder::default()
                .with_title("Neon HUD · Timer complete")
                .with_icon(desktop::app_icon())
                .with_inner_size([
                    (placement.size[0] / scale) as f32,
                    (placement.size[1] / scale) as f32,
                ])
                .with_position([
                    (placement.position[0] / scale) as f32,
                    (placement.position[1] / scale) as f32,
                ])
                .with_decorations(false)
                .with_resizable(false)
                .with_transparent(true)
                .with_taskbar(false)
                .with_active(false)
                .with_mouse_passthrough(true)
                .with_always_on_top(),
            |ctx, _| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        let r = ui.max_rect().shrink(1.);
                        ui.painter().rect_filled(r, 18, BLACK);
                        paint::icon(
                            ui.painter(),
                            r.left_center() + Vec2::new(27., 0.),
                            "timer",
                            self.palette().accent,
                            22.,
                        );
                        ui.painter().text(
                            r.left_center() + Vec2::new(52., 0.),
                            egui::Align2::LEFT_CENTER,
                            "Timer complete",
                            FontId::proportional(16.),
                            INK,
                        );
                    });
                ctx.request_repaint_after(Duration::from_millis(500));
            },
        );
        ctx.request_repaint_after(
            self.timer_notice_until
                .saturating_duration_since(Instant::now()),
        );
    }
}

fn metric_label(key: &str) -> &str {
    match key {
        "cpu" => "CPU",
        "gpu" => "GPU",
        "ram" => "Memory",
        "network" => "Network",
        "storage" => "Drives",
        "codex" => "Codex left",
        "claude" => "Claude left",
        _ => key,
    }
}
fn format_timer(ms: i64) -> String {
    let ms = ms.max(0);
    let seconds = ms / 1000 + i64::from(ms % 1000 != 0);
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capsule_geometry_is_bounded_and_exact_at_each_state() {
        assert_eq!(capsule_size(0.), COLLAPSED_SIZE);
        assert_eq!(capsule_size(STARTING_VALUE_PEEK_PROGRESS), PEEK_SIZE);
        assert_eq!(capsule_size(1.), OPEN_SIZE);
        for i in 0..=100 {
            let s = capsule_size(i as f32 / 100.);
            assert!(s[0] >= COLLAPSED_SIZE[0] && s[0] <= OPEN_SIZE[0]);
            assert!(s[1] >= COLLAPSED_SIZE[1] && s[1] <= OPEN_SIZE[1]);
        }
    }
    #[test]
    fn timer_display_uses_deadline_and_rounds_remaining_seconds_up() {
        assert_eq!(format_timer(1), "00:01");
        assert_eq!(format_timer(60_001), "01:01");
        assert_eq!(format_timer(-500), "00:00");
        assert_eq!(format_timer(25 * 60_000), "25:00");
    }
}
