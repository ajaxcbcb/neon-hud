//! Shallow, native Nook widgets. No web surface and no outer scrolling page.
use crate::{nook::PresentationMode, nook_ui::Tab, paint, productivity, utilities, App};
use chrono::{Datelike, Local, TimeZone};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

const INK: Color32 = Color32::from_rgb(240, 241, 244);
const DIM: Color32 = Color32::from_rgb(138, 141, 150);
const BLUE: Color32 = Color32::from_rgb(66, 151, 255);
const TILE: Color32 = Color32::from_rgb(35, 37, 43);

fn label(ui: &egui::Ui, r: Rect, text: impl Into<String>, size: f32, color: Color32) {
    let mut job =
        egui::text::LayoutJob::simple(text.into(), FontId::proportional(size), color, r.width());
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter_at(r).galley(r.min, galley, color);
}
fn action(ui: &mut egui::Ui, r: Rect, key: &str, tip: &str, enabled: bool, active: bool) -> bool {
    let response = ui.interact(
        r,
        ui.id()
            .with(("nook_action", key, r.min.x.to_bits(), r.min.y.to_bits())),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if enabled && (response.hovered() || active) {
        ui.painter().rect_filled(
            r,
            6,
            if active {
                BLUE.gamma_multiply(0.22)
            } else {
                TILE
            },
        );
    }
    paint::icon(
        ui.painter(),
        r.center(),
        key,
        if !enabled {
            DIM.gamma_multiply(0.45)
        } else if active {
            BLUE
        } else {
            INK
        },
        14.,
    );
    let clicked = enabled && response.clicked();
    response.on_hover_text(tip);
    clicked
}
fn time_label(seconds: f64) -> String {
    let n = if seconds.is_finite() {
        seconds.max(0.) as u64
    } else {
        0
    };
    format!("{}:{:02}", n / 60, n % 60)
}
fn circle_image(
    ui: &egui::Ui,
    texture: egui::TextureId,
    size: [usize; 2],
    r: Rect,
    mirrored: bool,
) {
    let mut mesh = egui::epaint::Mesh::with_texture(texture);
    let uv_scale = Vec2::new(
        (size[1] as f32 / size[0] as f32).min(1.),
        (size[0] as f32 / size[1] as f32).min(1.),
    ) * 0.5;
    mesh.vertices.push(egui::epaint::Vertex {
        pos: r.center(),
        uv: Pos2::new(0.5, 0.5),
        color: Color32::WHITE,
    });
    for i in 0..=48 {
        let v = Vec2::angled(i as f32 * std::f32::consts::TAU / 48.);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: r.center() + v * (r.width().min(r.height()) / 2.),
            uv: Pos2::new(
                0.5 + v.x * uv_scale.x * if mirrored { -1. } else { 1. },
                0.5 + v.y * uv_scale.y,
            ),
            color: Color32::WHITE,
        });
        if i > 0 {
            mesh.indices.extend_from_slice(&[0, i as u32, i as u32 + 1]);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

impl App {
    pub(crate) fn nook_home(&mut self, ui: &mut egui::Ui, r: Rect, ctx: &egui::Context) {
        let row = Rect::from_min_size(r.min, Vec2::new(r.width(), r.height() - 26.));
        let compact = row.width() < 760.;
        let cards: &[(usize, f32)] = if compact {
            &[(0, 0.32), (2, 0.30), (3, 0.38)]
        } else {
            &[
                (0, 0.21),
                (1, 0.15),
                (2, 0.18),
                (3, 0.19),
                (4, 0.23),
                (5, 0.04),
            ]
        };
        let width = row.width() - (cards.len() - 1) as f32 * 10.;
        let mut x = row.left();
        for &(i, weight) in cards {
            let card = Rect::from_min_size(
                Pos2::new(x, row.top()),
                Vec2::new(width * weight, row.height()),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .id_salt(("nook_widget", i))
                    .max_rect(card)
                    .sense(Sense::hover()),
                |ui| {
                    ui.set_clip_rect(card.intersect(ui.clip_rect()));
                    match i {
                        0 => self.nook_media_card(ui, card, ctx),
                        1 if self.nook_ui.mirror_enabled => self.nook_mirror_card(ui, card, ctx),
                        1 => self.nook_calendar_card(ui, card, false),
                        2 => self.nook_notes_card(ui, card),
                        3 => self.nook_timer_wheels(ui, card),
                        4 => self.nook_task_card(ui, card),
                        _ => self.nook_shortcuts(ui, card, ctx),
                    }
                },
            );
            x = card.right() + 10.;
            if card.right() < row.right() - 1. {
                ui.painter().line_segment(
                    [
                        Pos2::new(x - 5., row.top() + 4.),
                        Pos2::new(x - 5., row.bottom() - 4.),
                    ],
                    Stroke::new(1., Color32::from_gray(34)),
                );
            }
        }
        let keys = self.nook_metric_keys();
        let rail = Rect::from_min_max(Pos2::new(r.left(), r.bottom() - 25.), r.max);
        ui.painter().line_segment(
            [
                rail.left_top() - Vec2::new(0., 3.),
                rail.right_top() - Vec2::new(0., 3.),
            ],
            Stroke::new(1., Color32::from_gray(30)),
        );
        let cell_width = rail.width() / keys.len() as f32;
        for (i, key) in keys.into_iter().enumerate() {
            let cell = Rect::from_min_size(
                rail.min + Vec2::new(i as f32 * cell_width, 0.),
                Vec2::new(cell_width - 5., 24.),
            );
            let (value, number, attention) = self.reading(key);
            let response = ui.interact(cell, ui.id().with(("nook_rail", key)), Sense::click());
            if response.hovered() {
                ui.painter().rect_filled(cell, 5, TILE);
            }
            let remaining = ["codex", "claude"].contains(&key);
            let color = value
                .map(|v| paint::stress(crate::stress_percent(v, remaining)))
                .unwrap_or(DIM);
            let (reduced, pressure) = self.nook_motion_flags();
            let shake =
                if attention && Instant::now() < self.attention_until && !reduced && !pressure {
                    ctx.request_repaint_after(Duration::from_millis(32));
                    (ctx.input(|i| i.time) as f32 * 40.).sin() * 1.5
                } else {
                    0.
                };
            paint::icon(
                ui.painter(),
                cell.left_center() + Vec2::new(9. + shake, -1.),
                key,
                if attention { self.palette().pop } else { color },
                13.,
            );
            label(
                ui,
                Rect::from_min_size(
                    cell.min + Vec2::new(21., 2.),
                    Vec2::new(cell.width() - 34., 16.),
                ),
                number,
                11.,
                INK,
            );
            if attention {
                ui.painter().text(
                    cell.right_top() - Vec2::new(5., -3.),
                    egui::Align2::RIGHT_TOP,
                    "!",
                    paint::bold(11.),
                    self.palette().pop,
                );
            }
            if let Some(v) = value {
                let bar = Rect::from_min_size(
                    cell.min + Vec2::new(22., 21.),
                    Vec2::new((cell.width() - 31.).max(1.), 2.),
                );
                ui.painter().rect_filled(bar, 1, Color32::from_gray(40));
                let filled = bar.width() * v.clamp(0., 100.) as f32 / 100.;
                for step in 0..16 {
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            bar.min + Vec2::new(filled * step as f32 / 16., 0.),
                            Vec2::new(filled / 16., 2.),
                        ),
                        0,
                        paint::stress(crate::stress_percent(v * step as f64 / 16., remaining)),
                    );
                }
            }
            if response.clicked() {
                self.selected = key.into();
                self.details = true;
                self.hover = None;
            } else if response.hovered() && !self.details {
                self.hover = ctx
                    .input(|i| i.viewport().outer_rect)
                    .map(|bounds| (key.into(), bounds));
            }
        }
    }

    pub(crate) fn nook_power_badge(&mut self, ui: &mut egui::Ui, r: Rect) {
        let power = &self.utility_snapshot.power;
        let text = if let Some(percent) = power.charge_percent {
            format!(
                "{} {percent}%",
                if power.charging == Some(true) {
                    "↯"
                } else {
                    ""
                }
            )
        } else if power.on_ac == Some(true) {
            "AC power".into()
        } else {
            String::new()
        };
        if !text.is_empty() {
            paint::icon(
                ui.painter(),
                r.left_center() + Vec2::new(8., 0.),
                "battery",
                if power.charge_percent.is_some_and(|n| n <= 15) {
                    paint::stress(95.)
                } else {
                    DIM
                },
                14.,
            );
            label(
                ui,
                Rect::from_min_size(r.min + Vec2::new(21., 5.), Vec2::new(r.width() - 22., 15.)),
                text,
                11.,
                DIM,
            );
            let hint = power
                .remaining_seconds
                .map(|s| format!("{} remaining", time_label(s as f64)))
                .unwrap_or_else(|| power.status.label().into());
            ui.interact(r, ui.id().with("power_status"), Sense::hover())
                .on_hover_text(hint);
        }
    }

    pub(crate) fn nook_media_card(&mut self, ui: &mut egui::Ui, r: Rect, ctx: &egui::Context) {
        let media = self.utility_snapshot.media.clone();
        let enabled = self
            .productivity
            .as_ref()
            .is_some_and(|c| c.state.utilities.media_enabled);
        if !enabled {
            label(ui, r.shrink(4.), "Music", 13., INK);
            if ui
                .put(
                    Rect::from_min_size(
                        r.min + Vec2::new(0., 27.),
                        Vec2::new(r.width().min(150.), 26.),
                    ),
                    egui::Button::new("Connect media").fill(TILE),
                )
                .clicked()
            {
                self.edit_productivity(|s| {
                    s.utilities.media_enabled = true;
                    Ok(())
                });
            }
            label(
                ui,
                Rect::from_min_size(r.min + Vec2::new(0., 61.), Vec2::new(r.width(), 17.)),
                "Uses your media app",
                10.,
                DIM,
            );
            return;
        }
        if media.status != utilities::Availability::Ready {
            paint::icon(ui.painter(), r.min + Vec2::new(16., 21.), "media", DIM, 23.);
            label(
                ui,
                Rect::from_min_size(r.min + Vec2::new(34., 9.), Vec2::new(r.width() - 34., 22.)),
                "Music",
                13.,
                INK,
            );
            label(
                ui,
                Rect::from_min_size(r.min + Vec2::new(0., 44.), Vec2::new(r.width(), 23.)),
                match &media.status {
                    utilities::Availability::Error(_) => "Couldn't read media",
                    utilities::Availability::Denied(_) => "Allow media access",
                    _ => media.status.label(),
                },
                11.,
                DIM,
            );
            ui.interact(r, ui.id().with("media_connection_status"), Sense::hover())
                .on_hover_text(media.status.label());
            if ui
                .put(
                    Rect::from_min_size(
                        r.min + Vec2::new(0., 70.),
                        Vec2::new(r.width().min(155.), 23.),
                    ),
                    egui::Button::new("Retry / change source")
                        .small()
                        .frame(false),
                )
                .clicked()
            {
                self.edit_productivity(|s| {
                    s.utilities.media_enabled = false;
                    Ok(())
                });
            }
            return;
        }
        let art = Rect::from_min_size(r.min + Vec2::new(0., 4.), Vec2::splat(45.));
        if let Some(frame) = media.artwork.filter(|f| f.valid()) {
            let id = Arc::as_ptr(&frame) as usize;
            if self.nook_ui.artwork_frame != id {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width, frame.height],
                    &frame.rgba,
                );
                if let Some(texture) = &mut self.nook_ui.artwork_texture {
                    texture.set(image, egui::TextureOptions::LINEAR);
                } else {
                    self.nook_ui.artwork_texture = Some(ctx.load_texture(
                        "nook_media_art",
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                self.nook_ui.artwork_frame = id;
            }
            if let Some(texture) = &self.nook_ui.artwork_texture {
                ui.put(
                    art,
                    egui::Image::new((texture.id(), art.size())).corner_radius(7),
                );
            }
        } else {
            ui.painter().rect_filled(art, 7, TILE);
            paint::icon(ui.painter(), art.center(), "media", DIM, 24.);
        }
        label(
            ui,
            Rect::from_min_size(r.min + Vec2::new(52., 5.), Vec2::new(r.width() - 53., 19.)),
            &media.title,
            12.,
            INK,
        );
        label(
            ui,
            Rect::from_min_size(r.min + Vec2::new(52., 25.), Vec2::new(r.width() - 53., 15.)),
            &media.artist,
            10.,
            DIM,
        );
        for (i, key, tip, can, command) in [
            (
                0,
                "previous",
                "Previous track",
                media.can_previous,
                utilities::MediaCommand::Previous,
            ),
            (
                1,
                if media.playing { "pause" } else { "play" },
                "Play / pause",
                media.can_toggle,
                utilities::MediaCommand::Toggle,
            ),
            (
                2,
                "skip",
                "Next track",
                media.can_next,
                utilities::MediaCommand::Next,
            ),
        ] {
            if action(
                ui,
                Rect::from_min_size(
                    r.min + Vec2::new(r.width() / 2. - 39. + i as f32 * 26., 51.),
                    Vec2::new(24., 21.),
                ),
                key,
                tip,
                can,
                false,
            ) && !self.utilities.media(command)
            {
                self.productivity_status = "Media controls busy; try again".into();
            }
        }
        let progress =
            Rect::from_min_size(r.min + Vec2::new(2., 77.), Vec2::new(r.width() - 4., 4.));
        ui.painter()
            .rect_filled(progress, 2, Color32::from_gray(46));
        if media.duration_seconds.is_finite() && media.duration_seconds > 0. {
            let fraction = (media.position_seconds / media.duration_seconds).clamp(0., 1.) as f32;
            ui.painter().rect_filled(
                Rect::from_min_size(progress.min, Vec2::new(progress.width() * fraction, 4.)),
                2,
                INK,
            );
            let response = ui.interact(
                progress.expand2(Vec2::new(0., 5.)),
                ui.id().with("media_seek"),
                if media.can_seek {
                    Sense::click_and_drag()
                } else {
                    Sense::hover()
                },
            );
            if (response.clicked() || response.drag_stopped()) && media.can_seek {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.utilities.media(utilities::MediaCommand::Seek(
                        f64::from(((pos.x - progress.left()) / progress.width()).clamp(0., 1.))
                            * media.duration_seconds,
                    ));
                }
            }
        }
        label(
            ui,
            Rect::from_min_size(r.min + Vec2::new(2., 84.), Vec2::new(r.width() / 2., 12.)),
            time_label(media.position_seconds),
            9.,
            DIM,
        );
        ui.painter().text(
            Pos2::new(r.right() - 2., r.top() + 84.),
            egui::Align2::RIGHT_TOP,
            time_label(media.duration_seconds),
            FontId::proportional(9.),
            DIM,
        );
        if !self.utility_snapshot.message.is_empty() {
            ui.interact(r, ui.id().with("media_error"), Sense::hover())
                .on_hover_text(&self.utility_snapshot.message);
        }
    }

    pub(crate) fn nook_notes_card(&mut self, ui: &mut egui::Ui, r: Rect) {
        let Some(controller) = &self.productivity else {
            label(ui, r, "Notes unavailable", 11., DIM);
            return;
        };
        let mut note = controller.state.note.clone();
        let prefs = controller.state.utilities.clone();
        let writable = controller.writable();
        ui.painter().rect_filled(r, 9, TILE);
        let text_rect = Rect::from_min_max(r.min + Vec2::new(9., 7.), r.max - Vec2::new(9., 25.));
        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = wrap_width;
            job.append(
                buffer.as_str(),
                0.,
                egui::TextFormat {
                    font_id: if prefs.note_bold {
                        paint::bold(11.)
                    } else {
                        FontId::proportional(11.)
                    },
                    color: INK,
                    italics: prefs.note_italic,
                    underline: if prefs.note_underline {
                        Stroke::new(0.8, INK)
                    } else {
                        Stroke::NONE
                    },
                    ..Default::default()
                },
            );
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let response = ui
            .add_enabled_ui(writable, |ui| {
                ui.put(
                    text_rect,
                    egui::TextEdit::multiline(&mut note)
                        .frame(false)
                        .hint_text("Write a quick note…")
                        .desired_width(text_rect.width())
                        .desired_rows(3)
                        .char_limit(productivity::MAX_NOTE_BYTES / 4)
                        .layouter(&mut layouter),
                )
            })
            .inner;
        if response.changed() {
            self.edit_productivity(|s| s.set_note(note));
        }
        let y = r.bottom() - 22.;
        ui.painter()
            .circle_filled(Pos2::new(r.left() + 14., y + 10.), 3., BLUE);
        for (i, title, active) in [
            (0, "B", prefs.note_bold),
            (1, "I", prefs.note_italic),
            (2, "U", prefs.note_underline),
        ] {
            let rect = Rect::from_min_size(
                Pos2::new(r.right() - 73. + i as f32 * 23., y),
                Vec2::new(21., 20.),
            );
            let response = ui.interact(
                rect,
                ui.id().with(("note_format", i)),
                if writable {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            if active || response.hovered() {
                ui.painter().rect_filled(rect, 4, BLUE.gamma_multiply(0.14));
            }
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                title,
                if i == 0 {
                    paint::bold(11.)
                } else {
                    FontId::proportional(11.)
                },
                if active { BLUE } else { DIM },
            );
            if response.clicked() {
                self.edit_productivity(|s| {
                    match i {
                        0 => s.utilities.note_bold = !active,
                        1 => s.utilities.note_italic = !active,
                        _ => s.utilities.note_underline = !active,
                    };
                    Ok(())
                });
            }
        }
    }

    pub(crate) fn nook_timer_wheels(&mut self, ui: &mut egui::Ui, r: Rect) {
        let Some(controller) = &self.productivity else {
            label(ui, r, "Timer unavailable", 11., DIM);
            return;
        };
        let timer = controller.state.timer.clone();
        let writable = controller.writable();
        let ms = (crate::now() * 1000.) as i64;
        let running = matches!(timer, productivity::Timer::Running { .. });
        let paused = matches!(timer, productivity::Timer::Paused { .. });
        let seconds = if running || paused {
            (timer.remaining_ms(ms).max(0) as u64)
                .div_ceil(1000)
                .min(359999) as u32
        } else {
            self.nook_ui.timer_seconds
        };
        let values = [seconds / 3600, (seconds / 60) % 60, seconds % 60];
        let editable = writable && !running && !paused;
        let wheels_width = (r.width() - 32.).max(90.);
        for (i, (value, suffix, unit, max)) in values
            .into_iter()
            .zip([("h", 3600_u32, 99_u32), ("m", 60, 59), ("s", 1, 59)])
            .enumerate()
            .map(|(i, (value, (s, u, m)))| (i, (value, s, u, m)))
        {
            let wheel = Rect::from_min_size(
                r.min + Vec2::new(i as f32 * wheels_width / 3., 2.),
                Vec2::new(wheels_width / 3. - 2., 69.),
            );
            let response = ui.interact(
                wheel,
                ui.id().with(("timer_wheel", i)),
                if editable {
                    Sense::click_and_drag()
                } else {
                    Sense::hover()
                },
            );
            if response.hovered() && editable {
                ui.painter().rect_filled(wheel, 8, TILE);
            }
            for (line, n, color) in [
                (0, value.saturating_sub(1), DIM.gamma_multiply(0.30)),
                (1, value, BLUE),
                (2, (value + 1).min(max), DIM.gamma_multiply(0.30)),
            ] {
                ui.painter().text(
                    Pos2::new(wheel.center().x, wheel.top() + 12. + line as f32 * 23.),
                    egui::Align2::CENTER_CENTER,
                    format!("{n:02}{suffix}"),
                    FontId::monospace(if line == 1 { 16. } else { 13. }),
                    color,
                );
            }
            if editable {
                let scroll = if response.hovered() {
                    ui.input(|i| i.raw_scroll_delta.y)
                } else {
                    0.
                };
                let direction = if scroll > 0. {
                    1
                } else if scroll < 0. {
                    -1
                } else if response.clicked() {
                    if response
                        .interact_pointer_pos()
                        .is_some_and(|p| p.y < wheel.center().y)
                    {
                        1
                    } else {
                        -1
                    }
                } else if response.drag_stopped() {
                    if response.drag_delta().y < -4. {
                        1
                    } else if response.drag_delta().y > 4. {
                        -1
                    } else {
                        0
                    }
                } else {
                    0
                };
                if direction != 0 {
                    let new = (value as i32 + direction).clamp(0, max as i32) as u32;
                    self.nook_ui.timer_seconds = self
                        .nook_ui
                        .timer_seconds
                        .saturating_sub(value * unit)
                        .saturating_add(new * unit)
                        .min(359999);
                }
            }
        }
        let play = Rect::from_min_size(r.right_top() - Vec2::new(27., -25.), Vec2::splat(26.));
        ui.painter()
            .circle_filled(play.center(), 13., if writable { BLUE } else { TILE });
        if action(
            ui,
            play,
            if running { "pause" } else { "play" },
            if running {
                "Pause timer"
            } else if paused {
                "Resume timer"
            } else {
                "Start timer"
            },
            writable && (running || paused || self.nook_ui.timer_seconds > 0),
            false,
        ) {
            let duration = i64::from(self.nook_ui.timer_seconds) * 1000;
            self.edit_productivity(|s| {
                if running {
                    s.timer.pause(ms);
                } else if paused {
                    s.timer.resume(ms);
                } else {
                    s.timer.start(ms, duration)?;
                }
                Ok(())
            });
        }
        let preset_y = r.top() + 77.;
        for (i, seconds) in [60, 300, 1500].into_iter().enumerate() {
            let preset = Rect::from_min_size(
                Pos2::new(r.left() + i as f32 * 34., preset_y),
                Vec2::new(31., 20.),
            );
            if ui
                .add_enabled_ui(editable, |ui| {
                    ui.put(
                        preset,
                        egui::Button::new(format!("{}m", seconds / 60))
                            .small()
                            .frame(false),
                    )
                })
                .inner
                .clicked()
            {
                self.nook_ui.timer_seconds = seconds;
            }
        }
        if action(
            ui,
            Rect::from_min_size(Pos2::new(r.right() - 24., preset_y), Vec2::new(23., 20.)),
            "reset",
            "Reset timer",
            writable,
            false,
        ) {
            self.edit_productivity(|s| {
                s.timer.cancel();
                Ok(())
            });
        }
        if matches!(timer, productivity::Timer::Completed { .. }) {
            label(
                ui,
                Rect::from_min_size(r.min + Vec2::new(0., 102.), Vec2::new(r.width(), 16.)),
                "Timer complete",
                11.,
                BLUE,
            );
        }
    }

    fn nook_task_card(&mut self, ui: &mut egui::Ui, r: Rect) {
        let writable = self.productivity.as_ref().is_some_and(|c| c.writable());
        let tasks = self
            .productivity
            .as_ref()
            .map(|c| c.state.tasks.clone())
            .unwrap_or_default();
        let input = ui
            .add_enabled_ui(writable, |ui| {
                ui.put(
                    Rect::from_min_size(r.min, Vec2::new(r.width(), 22.)),
                    egui::TextEdit::singleline(&mut self.nook_ui.task_draft)
                        .frame(false)
                        .hint_text("New task…  ↵")
                        .font(FontId::proportional(11.))
                        .char_limit(128),
                )
            })
            .inner;
        if input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let draft = std::mem::take(&mut self.nook_ui.task_draft);
            if !draft.trim().is_empty() {
                self.edit_productivity(|s| s.add_task(draft).map(|_| ()));
            }
        }
        for (i, task) in tasks.iter().filter(|t| !t.complete).take(2).enumerate() {
            let row = Rect::from_min_size(
                r.min + Vec2::new(0., 29. + i as f32 * 27.),
                Vec2::new(r.width(), 23.),
            );
            let complete = Rect::from_min_size(row.min, Vec2::splat(23.));
            let response = ui.interact(
                complete,
                ui.id().with(("task_complete", task.id)),
                if writable {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            ui.painter().circle_stroke(
                complete.center(),
                6.,
                Stroke::new(1., if response.hovered() { BLUE } else { DIM }),
            );
            if response.clicked() {
                self.edit_productivity(|s| {
                    s.set_task_complete(task.id, true);
                    Ok(())
                });
            }
            label(
                ui,
                Rect::from_min_size(
                    row.min + Vec2::new(26., 4.),
                    Vec2::new(row.width() - 50., 17.),
                ),
                &task.text,
                11.,
                INK,
            );
            if action(
                ui,
                Rect::from_min_size(row.right_top() - Vec2::new(23., 0.), Vec2::splat(23.)),
                "star",
                "Favorite task",
                writable,
                task.favorite,
            ) {
                self.edit_productivity(|s| {
                    s.set_task_favorite(task.id, !task.favorite);
                    Ok(())
                });
            }
        }
        if tasks.iter().all(|t| t.complete) {
            label(
                ui,
                Rect::from_min_size(r.min + Vec2::new(4., 38.), Vec2::new(r.width() - 8., 16.)),
                "A clear little corner",
                11.,
                DIM,
            );
        }
        let view = Rect::from_min_size(
            Pos2::new(r.left(), r.bottom() - 15.),
            Vec2::new(r.width(), 15.),
        );
        if ui
            .put(
                view,
                egui::Button::new(format!("All tasks · {}", tasks.len()))
                    .small()
                    .frame(false),
            )
            .clicked()
        {
            self.nook_ui.tab = Tab::Tasks;
        }
    }

    pub(crate) fn nook_calendar_card(&mut self, ui: &mut egui::Ui, r: Rect, configure: bool) {
        let calendar = self.utility_snapshot.calendar.clone();
        let body = if configure {
            Rect::from_min_size(r.min, Vec2::new((r.width() * 0.52).min(440.), r.height()))
        } else {
            r
        };
        let today = Local::now().date_naive();
        self.nook_ui.calendar_day = self.nook_ui.calendar_day.clamp(0, 6);
        label(
            ui,
            Rect::from_min_size(body.min, Vec2::new(body.width(), 16.)),
            today.format("%B %Y").to_string(),
            11.,
            INK,
        );
        let cell_width = body.width() / 7.;
        for i in 0..7 {
            let date = today + chrono::Duration::days(i);
            let rect = Rect::from_min_size(
                body.min + Vec2::new(i as f32 * cell_width, 19.),
                Vec2::new(cell_width - 1., 36.),
            );
            let active = self.nook_ui.calendar_day == i;
            if active {
                ui.painter().rect_filled(rect, 6, BLUE.gamma_multiply(0.20));
            }
            ui.painter().text(
                rect.center_top() + Vec2::new(0., 7.),
                egui::Align2::CENTER_CENTER,
                date.format("%a").to_string().chars().next().unwrap_or(' '),
                FontId::proportional(8.),
                DIM,
            );
            ui.painter().text(
                rect.center_top() + Vec2::new(0., 25.),
                egui::Align2::CENTER_CENTER,
                format!("{:02}", date.day()),
                FontId::proportional(12.),
                if active { BLUE } else { INK },
            );
            if ui
                .interact(rect, ui.id().with(("calendar_day", i)), Sense::click())
                .clicked()
            {
                self.nook_ui.calendar_day = i;
            }
        }
        let date = today + chrono::Duration::days(self.nook_ui.calendar_day);
        let start = date
            .and_hms_opt(0, 0, 0)
            .and_then(|d| Local.from_local_datetime(&d).earliest())
            .map(|d| d.timestamp_millis())
            .unwrap_or(0);
        let end = (date + chrono::Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .and_then(|d| Local.from_local_datetime(&d).earliest())
            .map(|d| d.timestamp_millis())
            .unwrap_or(start + 86_400_000);
        let events = calendar
            .events
            .iter()
            .filter(|e| e.starts_at_ms < end && e.ends_at_ms > start)
            .collect::<Vec<_>>();
        if calendar.status == utilities::Availability::Ready {
            if events.is_empty() {
                label(
                    ui,
                    Rect::from_min_size(
                        body.min + Vec2::new(2., 67.),
                        Vec2::new(body.width() - 4., 18.),
                    ),
                    "Nothing for this day",
                    10.,
                    DIM,
                );
            } else {
                for (i, event) in events
                    .iter()
                    .take(if configure { 3 } else { 1 })
                    .enumerate()
                {
                    let at = if event.all_day {
                        "All day".into()
                    } else {
                        Local
                            .timestamp_millis_opt(event.starts_at_ms)
                            .single()
                            .map(|d| d.format("%H:%M").to_string())
                            .unwrap_or_default()
                    };
                    let text = format!("{at}  {}", event.title);
                    label(
                        ui,
                        Rect::from_min_size(
                            body.min + Vec2::new(2., 62. + i as f32 * 18.),
                            Vec2::new(body.width() - 4., 17.),
                        ),
                        text,
                        10.,
                        INK,
                    );
                }
                if !configure && events.len() > 1 {
                    label(
                        ui,
                        Rect::from_min_size(
                            body.min + Vec2::new(2., 81.),
                            Vec2::new(body.width() - 4., 12.),
                        ),
                        format!("+{} more", events.len() - 1),
                        9.,
                        DIM,
                    );
                }
            }
        } else {
            let connect =
                Rect::from_min_size(body.min + Vec2::new(0., 65.), Vec2::new(body.width(), 24.));
            if ui
                .put(
                    connect,
                    egui::Button::new("Connect calendar").small().frame(false),
                )
                .on_hover_text(calendar.status.label())
                .clicked()
            {
                self.nook_ui.tab = Tab::Calendar;
            }
        }
        if configure {
            let right = Rect::from_min_max(Pos2::new(body.right() + 24., r.top()), r.max);
            let writable = self.productivity.as_ref().is_some_and(|c| c.writable());
            label(
                ui,
                Rect::from_min_size(right.min, Vec2::new(right.width(), 17.)),
                "Calendar source",
                12.,
                INK,
            );
            if self.nook_ui.calendar_draft.is_empty() {
                self.nook_ui.calendar_draft = self
                    .productivity
                    .as_ref()
                    .and_then(|c| c.state.utilities.calendar_path.as_ref())
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
            }
            ui.add_enabled_ui(writable, |ui| {
                ui.put(
                    Rect::from_min_size(
                        right.min + Vec2::new(0., 23.),
                        Vec2::new(right.width() - 65., 25.),
                    ),
                    egui::TextEdit::singleline(&mut self.nook_ui.calendar_draft)
                        .hint_text("Path to a .ics calendar")
                        .char_limit(4096),
                );
                if ui
                    .put(
                        Rect::from_min_size(
                            right.right_top() + Vec2::new(-58., 23.),
                            Vec2::new(58., 25.),
                        ),
                        egui::Button::new("Import").small(),
                    )
                    .clicked()
                {
                    let path = self
                        .nook_ui
                        .calendar_draft
                        .trim()
                        .trim_matches('"')
                        .to_owned();
                    if !path.is_empty() {
                        self.edit_productivity(|s| {
                            s.utilities.calendar_path = Some(PathBuf::from(path));
                            s.utilities.native_calendar = false;
                            Ok(())
                        });
                    }
                }
                if cfg!(target_os = "macos")
                    && ui
                        .put(
                            Rect::from_min_size(
                                right.min + Vec2::new(0., 55.),
                                Vec2::new(155., 24.),
                            ),
                            egui::Button::new("Connect Mac Calendar").small(),
                        )
                        .clicked()
                {
                    self.edit_productivity(|s| {
                        s.utilities.calendar_path = None;
                        s.utilities.native_calendar = true;
                        Ok(())
                    });
                    if !self.utilities.connect_calendar() {
                        self.productivity_status = "Calendar busy; try again".into();
                    }
                }
                if ui
                    .put(
                        Rect::from_min_size(
                            right.right_top() + Vec2::new(-85., 55.),
                            Vec2::new(85., 24.),
                        ),
                        egui::Button::new("Disconnect").small().frame(false),
                    )
                    .clicked()
                {
                    self.edit_productivity(|s| {
                        s.utilities.calendar_path = None;
                        s.utilities.native_calendar = false;
                        Ok(())
                    });
                    self.nook_ui.calendar_draft.clear();
                }
            });
            label(
                ui,
                Rect::from_min_size(
                    right.min + Vec2::new(0., 88.),
                    Vec2::new(right.width(), 16.),
                ),
                calendar.status.label(),
                10.,
                DIM,
            );
            label(
                ui,
                Rect::from_min_size(
                    right.min + Vec2::new(0., 107.),
                    Vec2::new(right.width(), 14.),
                ),
                if calendar.source.is_empty() {
                    "Local calendar · seven-day view"
                } else {
                    &calendar.source
                },
                9.,
                DIM,
            );
        }
    }

    pub(crate) fn nook_mirror_card(&mut self, ui: &mut egui::Ui, r: Rect, ctx: &egui::Context) {
        let mirror = self.utility_snapshot.mirror.clone();
        let size = (r.height() - 21.).min(r.width()).min(84.).max(20.);
        let image = Rect::from_center_size(
            Pos2::new(r.center().x, r.top() + size / 2.),
            Vec2::splat(size),
        );
        ui.painter().circle_filled(image.center(), size / 2., TILE);
        if let Some(frame) = mirror.frame.filter(|f| f.valid()) {
            let id = Arc::as_ptr(&frame) as usize;
            if self.nook_ui.mirror_frame != id {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width, frame.height],
                    &frame.rgba,
                );
                if let Some(texture) = &mut self.nook_ui.mirror_texture {
                    texture.set(image, egui::TextureOptions::LINEAR);
                } else {
                    self.nook_ui.mirror_texture =
                        Some(ctx.load_texture("nook_mirror", image, egui::TextureOptions::LINEAR));
                }
                self.nook_ui.mirror_frame = id;
            }
            if let Some(texture) = &self.nook_ui.mirror_texture {
                circle_image(ui, texture.id(), [frame.width, frame.height], image, true);
            }
        } else {
            paint::icon(ui.painter(), image.center(), "mirror", DIM, 25.);
        }
        let pressure = self.mode != crate::model::Mode::Normal;
        let tip = if pressure && self.nook_ui.mirror_enabled {
            "Camera paused under pressure"
        } else {
            mirror.status.label()
        };
        ui.interact(image, ui.id().with("mirror_status"), Sense::hover())
            .on_hover_text(tip);
        let button = Rect::from_center_size(
            Pos2::new(r.center().x, r.bottom() - 10.),
            Vec2::new(r.width().min(180.), 20.),
        );
        if ui
            .put(
                button,
                egui::Button::new(if self.nook_ui.mirror_enabled {
                    "Camera off"
                } else {
                    "Enable camera"
                })
                .small()
                .frame(false),
            )
            .on_hover_text(tip)
            .clicked()
        {
            self.nook_ui.mirror_enabled = !self.nook_ui.mirror_enabled;
        }
        if r.width() > 220. {
            label(
                ui,
                Rect::from_min_size(
                    r.min + Vec2::new(0., 7.),
                    Vec2::new((r.width() - size) / 2. - 10., 25.),
                ),
                tip,
                11.,
                DIM,
            );
        }
    }

    pub(crate) fn nook_shortcuts(&mut self, ui: &mut egui::Ui, r: Rect, ctx: &egui::Context) {
        let vertical = r.width() < 80.;
        let actions = [
            ("mirror", "Camera mirror"),
            ("codex", "AI allowance"),
            ("settings", "Settings"),
            ("nook", "Small floating pill"),
        ];
        for (i, (key, tip)) in actions.into_iter().enumerate() {
            let rect = if vertical {
                Rect::from_min_size(
                    r.min + Vec2::new(0., i as f32 * 24.),
                    Vec2::new(r.width(), 22.),
                )
            } else {
                Rect::from_min_size(
                    r.min + Vec2::new(i as f32 * 152., 17.),
                    Vec2::new(140., 70.),
                )
            };
            if action(
                ui,
                rect,
                key,
                tip,
                true,
                key == "mirror" && self.nook_ui.mirror_enabled,
            ) {
                match key {
                    "mirror" => {
                        self.nook_ui.mirror_enabled = !self.nook_ui.mirror_enabled;
                        self.nook_ui.tab = Tab::Home;
                    }
                    "codex" => self.nook_ui.tab = Tab::Ai,
                    "settings" => self.settings = true,
                    _ => self.set_presentation(PresentationMode::Pill, ctx),
                }
            }
            if !vertical {
                label(
                    ui,
                    Rect::from_min_size(
                        rect.left_bottom() - Vec2::new(0., 2.),
                        Vec2::new(rect.width(), 18.),
                    ),
                    tip,
                    11.,
                    DIM,
                );
            }
        }
    }
}
