use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "hud-bold".into(),
        egui::FontData::from_static(include_bytes!("../assets/Ubuntu-Bold.ttf")).into(),
    );
    let mut fallback = fonts.families[&egui::FontFamily::Proportional].clone();
    fallback.insert(0, "hud-bold".into());
    fonts
        .families
        .insert(egui::FontFamily::Name("hud-bold".into()), fallback);
    ctx.set_fonts(fonts);
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("hud-bold".into()))
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub ink: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub pop: Color32,
}
impl Palette {
    pub fn new(theme: &str) -> Self {
        let (accent, pop) = match theme {
            "cyberpunk" => (
                Color32::from_rgb(255, 91, 179),
                Color32::from_rgb(91, 226, 255),
            ),
            "aurora" => (
                Color32::from_rgb(113, 255, 210),
                Color32::from_rgb(183, 151, 255),
            ),
            _ => (
                Color32::from_rgb(201, 255, 53),
                Color32::from_rgb(247, 139, 255),
            ),
        };
        Self {
            bg: Color32::from_rgb(18, 18, 34),
            panel: Color32::from_rgb(35, 35, 57),
            ink: Color32::from_rgb(239, 245, 249),
            dim: Color32::from_rgb(146, 160, 180),
            accent,
            pop,
        }
    }
    pub fn apply(self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = self.bg;
        visuals.window_fill = self.bg;
        visuals.override_text_color = Some(self.ink);
        visuals.selection.bg_fill = self.accent.gamma_multiply(0.25);
        visuals.selection.stroke = Stroke::new(1.0, self.accent);
        visuals.widgets.inactive.bg_fill = self.panel;
        visuals.widgets.hovered.bg_fill = self.accent.gamma_multiply(0.18);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.accent);
        ctx.set_visuals(visuals);
    }
}
pub fn stress(v: f64) -> Color32 {
    let (a, b, t) = if v < 60.0 {
        (
            Color32::from_rgb(140, 255, 163),
            Color32::from_rgb(255, 207, 99),
            (v / 60.0).clamp(0., 1.),
        )
    } else {
        (
            Color32::from_rgb(255, 207, 99),
            Color32::from_rgb(255, 94, 145),
            ((v - 60.0) / 40.0).clamp(0., 1.),
        )
    };
    Color32::from_rgb(
        (a.r() as f64 + (b.r() as f64 - a.r() as f64) * t) as u8,
        (a.g() as f64 + (b.g() as f64 - a.g() as f64) * t) as u8,
        (a.b() as f64 + (b.b() as f64 - a.b() as f64) * t) as u8,
    )
}
pub fn bar(ui: &mut egui::Ui, value: Option<f64>, width: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(width, 8.), egui::Sense::hover());
    ui.painter()
        .rect_filled(r, 4, Color32::from_rgb(38, 46, 57));
    if let Some(v) = value {
        let n = (width * v.clamp(0., 100.) as f32 / 100.).round() as usize;
        for i in 0..n {
            let cell = Rect::from_min_size(r.min + Vec2::new(i as f32, 0.), Vec2::new(1., 8.));
            ui.painter()
                .rect_filled(cell, 0, stress(i as f64 / width as f64 * 100.));
        }
    }
}
pub fn gauge(ui: &mut egui::Ui, value: Option<f64>, label: &str, remaining: bool, p: Palette) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(130., 100.), egui::Sense::hover());
    let c = r.center() + Vec2::new(0., 8.);
    for i in 0..40 {
        let a = std::f32::consts::PI * (1.15 + i as f32 / 39. * 1.7);
        let color = if value.is_some_and(|v| i as f64 / 39. * 100. <= v) {
            stress(super::model::stress_percent(
                i as f64 / 39. * 100.,
                remaining,
            ))
        } else {
            p.panel
        };
        ui.painter().line_segment(
            [
                c + Vec2::angled(a) * 38.0_f32,
                c + Vec2::angled(a) * 45.0_f32,
            ],
            Stroke::new(3.0_f32, color),
        );
    }
    ui.painter().text(
        c,
        egui::Align2::CENTER_CENTER,
        super::model::percent(value),
        FontId::monospace(24.),
        p.ink,
    );
    ui.painter().text(
        r.center_top() + Vec2::new(0., 4.),
        egui::Align2::CENTER_TOP,
        label,
        FontId::proportional(12.),
        p.dim,
    );
}

pub fn choice_card(
    ui: &mut egui::Ui,
    rect: Rect,
    id: &str,
    title: &str,
    subtitle: &str,
    icon_key: &str,
    selected: bool,
    p: Palette,
) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(id), egui::Sense::click());
    let painter = ui.painter_at(rect.expand(6.));
    let dark_ink = Color32::from_rgb(18, 18, 34);
    let foreground = if selected { dark_ink } else { p.ink };
    let secondary = if selected {
        dark_ink.gamma_multiply(0.75)
    } else {
        p.dim
    };
    let shadow = Color32::from_rgb(255, 91, 179).gamma_multiply(if selected { 0.8 } else { 0.2 });
    painter.rect_filled(rect.translate(Vec2::new(4., 5.)), 15., shadow);
    let fill = if selected {
        p.accent
    } else if response.hovered() {
        Color32::from_rgb(46, 42, 72)
    } else {
        p.panel
    };
    painter.rect_filled(rect, 15., fill);
    painter.rect_stroke(
        rect,
        15.,
        Stroke::new(
            1.,
            if selected {
                p.accent
            } else {
                Color32::from_rgb(78, 71, 112)
            },
        ),
        egui::StrokeKind::Inside,
    );
    let icon_center = rect.left_top() + Vec2::new(26., 25.);
    icon(&painter, icon_center, icon_key, foreground, 21.);
    painter.text(
        rect.left_top() + Vec2::new(54., 18.),
        egui::Align2::LEFT_CENTER,
        title,
        bold(15.),
        foreground,
    );
    painter.text(
        rect.left_top() + Vec2::new(54., 36.),
        egui::Align2::LEFT_CENTER,
        subtitle,
        FontId::proportional(11.),
        secondary,
    );
    if selected {
        icon(
            &painter,
            rect.right_center() - Vec2::new(18., 0.),
            "check",
            foreground,
            14.,
        );
    }
    response
}

pub fn motion_card(
    ui: &mut egui::Ui,
    rect: Rect,
    id: &str,
    title: &str,
    icon_key: &str,
    selected: bool,
    p: Palette,
) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(id), egui::Sense::click());
    let painter = ui.painter_at(rect.expand(5.));
    let foreground = if selected {
        Color32::from_rgb(18, 18, 34)
    } else {
        p.ink
    };
    painter.rect_filled(
        rect.translate(Vec2::new(3., 4.)),
        13.,
        Color32::from_rgb(255, 91, 179).gamma_multiply(if selected { 0.75 } else { 0.15 }),
    );
    painter.rect_filled(
        rect,
        13.,
        if selected {
            p.accent
        } else if response.hovered() {
            Color32::from_rgb(46, 42, 72)
        } else {
            p.panel
        },
    );
    painter.rect_stroke(
        rect,
        13.,
        Stroke::new(
            1.,
            if selected {
                p.accent
            } else {
                Color32::from_rgb(78, 71, 112)
            },
        ),
        egui::StrokeKind::Inside,
    );
    icon(
        &painter,
        Pos2::new(rect.center().x, rect.top() + rect.height() * 0.36),
        icon_key,
        foreground,
        23.,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.top() + rect.height() * 0.74),
        egui::Align2::CENTER_CENTER,
        title,
        FontId::proportional(13.),
        foreground,
    );
    response
}

pub fn nook_preview(ui: &mut egui::Ui, rect: Rect, p: Palette, mini: bool, reduced: bool) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 18., p.panel);
    painter.rect_stroke(
        rect,
        18.,
        Stroke::new(1., p.dim.gamma_multiply(0.4)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.left_top() + Vec2::new(16., 20.),
        egui::Align2::LEFT_CENTER,
        "NOOK PREVIEW",
        FontId::monospace(11.),
        p.ink,
    );
    painter.text(
        rect.right_top() + Vec2::new(-14., 20.),
        egui::Align2::RIGHT_CENTER,
        "SAMPLE DATA",
        FontId::monospace(9.),
        p.pop,
    );
    let response = ui.interact(
        rect.shrink2(Vec2::new(12., 40.)),
        ui.id().with("nook-preview"),
        Sense::hover(),
    );
    let t = ui.ctx().animate_bool_with_time(
        ui.id().with("nook-preview-motion"),
        response.hovered(),
        if reduced { 0. } else { 0.18 },
    );
    let closed = if mini {
        Vec2::new(200., 36.)
    } else {
        Vec2::new(240., 40.)
    };
    let size = closed + (Vec2::new(338., 110.) - closed) * t;
    let capsule = Rect::from_center_size(rect.center(), size);
    painter.rect_filled(capsule, 18, Color32::from_rgb(8, 9, 12));
    painter.rect_stroke(
        capsule,
        18,
        Stroke::new(1., p.accent.gamma_multiply(0.55)),
        egui::StrokeKind::Inside,
    );
    icon(
        &painter,
        capsule.left_top() + Vec2::new(22., 20.),
        "cpu",
        p.accent,
        16.,
    );
    painter.text(
        capsule.left_top() + Vec2::new(37., 20.),
        egui::Align2::LEFT_CENTER,
        "42%",
        FontId::monospace(12.),
        p.ink,
    );
    icon(
        &painter,
        capsule.right_top() - Vec2::new(73., -20.),
        "codex",
        p.pop,
        16.,
    );
    painter.text(
        capsule.right_top() - Vec2::new(58., -20.),
        egui::Align2::LEFT_CENTER,
        "65%",
        FontId::monospace(12.),
        p.ink,
    );
    icon(
        &painter,
        capsule.right_top() - Vec2::new(19., -20.),
        "settings",
        p.dim,
        15.,
    );
    if t > 0.2 {
        let painter = painter.with_clip_rect(capsule.shrink(4.));
        for (i, (key, label)) in [
            ("cpu", "42%"),
            ("ram", "33%"),
            ("network", "2 MB/s"),
            ("claude", "38%"),
        ]
        .into_iter()
        .enumerate()
        {
            let x = capsule.left() + 42. + i as f32 * (capsule.width() - 84.) / 3.;
            icon(
                &painter,
                Pos2::new(x, capsule.top() + 58.),
                key,
                p.accent.gamma_multiply(t),
                16.,
            );
            painter.text(
                Pos2::new(x, capsule.top() + 81.),
                egui::Align2::CENTER_CENTER,
                label,
                FontId::monospace(10.),
                p.ink.gamma_multiply(t),
            );
        }
    }
    painter.text(
        rect.center_bottom() - Vec2::new(0., 22.),
        egui::Align2::CENTER_CENTER,
        "Hover here to peek · your compact native notch",
        FontId::proportional(11.),
        p.dim,
    );
}

pub fn appearance_preview(ui: &mut egui::Ui, rect: Rect, p: Palette, compact: bool) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 18., p.panel);
    painter.rect_stroke(
        rect,
        18.,
        Stroke::new(1., Color32::from_rgb(78, 71, 112)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.left_top() + Vec2::new(20., 21.),
        egui::Align2::LEFT_CENTER,
        "HUD PREVIEW",
        FontId::monospace(12.),
        p.ink,
    );
    let badge = Rect::from_min_size(
        rect.right_top() + Vec2::new(-119., 12.),
        Vec2::new(101., 22.),
    );
    painter.rect_filled(badge, 4, p.pop);
    painter.text(
        badge.center(),
        egui::Align2::CENTER_CENTER,
        "SAMPLE DATA",
        FontId::monospace(10.),
        p.bg,
    );
    let center = rect.center() + Vec2::new(0., 3.);
    let radius_x = (rect.width() * 0.31).min(116.);
    let radius_y = (rect.height() * 0.37).min(108.);
    for i in 0..36 {
        let a = i as f32 * std::f32::consts::TAU / 36.;
        let b = a + std::f32::consts::TAU / 72.;
        let at = |angle: f32| center + Vec2::new(angle.cos() * radius_x, angle.sin() * radius_y);
        painter.line_segment(
            [at(a), at(b)],
            Stroke::new(1.4, Color32::from_rgb(255, 91, 179)),
        );
    }
    let pill = Rect::from_center_size(center, Vec2::new(if compact { 160. } else { 280. }, 56.));
    painter.rect_filled(
        pill.translate(Vec2::new(3., 4.)),
        15.,
        p.pop.gamma_multiply(0.5),
    );
    painter.rect_filled(pill, 15., p.bg);
    painter.rect_stroke(
        pill,
        15.,
        Stroke::new(1.4, p.accent),
        egui::StrokeKind::Inside,
    );
    for y in [-4., 0., 4.] {
        painter.circle_filled(pill.left_center() + Vec2::new(10., y), 0.8, p.dim);
    }
    let keys: &[(&str, &str)] = if compact {
        &[
            ("cpu", "42%"),
            ("ram", "33%"),
            ("codex", "65%"),
            ("claude", "38%"),
        ]
    } else {
        &[
            ("cpu", "42%"),
            ("gpu", "57%"),
            ("ram", "33%"),
            ("network", "—"),
            ("storage", "79%"),
            ("codex", "65%"),
            ("claude", "38%"),
        ]
    };
    let width = (pill.width() - 25.) / keys.len() as f32;
    for (i, (key, label)) in keys.iter().enumerate() {
        let c = pill.left_top() + Vec2::new(22. + (i as f32 + 0.5) * width, 18.);
        icon(&painter, c, key, p.accent, 15.);
        painter.text(
            c + Vec2::new(0., 18.),
            egui::Align2::CENTER_CENTER,
            *label,
            FontId::monospace(9.),
            p.ink,
        );
        painter.line_segment(
            [c + Vec2::new(-7., 27.), c + Vec2::new(7., 27.)],
            Stroke::new(1., p.accent),
        );
    }
    painter.text(
        rect.center_bottom() + Vec2::new(0., -18.),
        egui::Align2::CENTER_CENTER,
        "Drag to move · right-click for controls",
        FontId::proportional(11.),
        p.dim,
    );
}
pub fn icon(painter: &egui::Painter, c: Pos2, key: &str, color: Color32, size: f32) {
    let s = size / 16.;
    let pt = |x: f32, y: f32| c + Vec2::new(x * s, y * s);
    let stroke = Stroke::new(1.6_f32, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([pt(a.0, a.1), pt(b.0, b.1)], stroke);
    };
    match key {
        "nook" => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(15. * s, 9. * s)),
                4,
                stroke,
                egui::StrokeKind::Middle,
            );
            line((-4., 0.), (4., 0.));
        }
        "play" => {
            painter.add(egui::Shape::convex_polygon(
                vec![pt(-3., -6.), pt(6., 0.), pt(-3., 6.)],
                color,
                Stroke::NONE,
            ));
        }
        "pause" => {
            painter.rect_filled(Rect::from_min_max(pt(-5., -6.), pt(-1., 6.)), 1, color);
            painter.rect_filled(Rect::from_min_max(pt(2., -6.), pt(6., 6.)), 1, color);
        }
        "skip" | "previous" => {
            let direction = if key == "skip" { 1. } else { -1. };
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pt(-5. * direction, -5.),
                    pt(3. * direction, 0.),
                    pt(-5. * direction, 5.),
                ],
                color,
                Stroke::NONE,
            ));
            line((6. * direction, -5.), (6. * direction, 5.));
        }
        "media" => {
            line((-2., 5.), (-2., -5.));
            line((-2., -5.), (6., -7.));
            line((6., -7.), (6., 3.));
            painter.circle_filled(pt(-5., 5.), 3. * s, color);
            painter.circle_filled(pt(3., 3.), 3. * s, color);
        }
        "calendar" => {
            painter.rect_stroke(
                Rect::from_min_max(pt(-7., -5.), pt(7., 7.)),
                2,
                stroke,
                egui::StrokeKind::Middle,
            );
            line((-7., -1.), (7., -1.));
            line((-3., -8.), (-3., -3.));
            line((3., -8.), (3., -3.));
            for (x, y) in [(-3., 2.), (2., 2.), (-3., 5.), (2., 5.)] {
                painter.circle_filled(pt(x, y), 0.7 * s, color);
            }
        }
        "mirror" => {
            painter.circle_stroke(pt(0., -2.), 6. * s, stroke);
            line((0., 4.), (0., 8.));
            line((-5., 8.), (5., 8.));
            line((-3., -3.), (0., -6.));
            line((0., 0.), (3., -3.));
        }
        "shortcuts" => {
            painter.add(egui::Shape::line(
                vec![
                    pt(1., -8.),
                    pt(-5., 1.),
                    pt(0., 1.),
                    pt(-1., 8.),
                    pt(5., -1.),
                    pt(0., -1.),
                ],
                stroke,
            ));
        }
        "battery" => {
            painter.rect_stroke(
                Rect::from_min_max(pt(-7., -4.), pt(5., 4.)),
                2,
                stroke,
                egui::StrokeKind::Middle,
            );
            line((7., -1.), (7., 1.));
            line((-4., 0.), (2., 0.));
        }
        "reset" => {
            let points = (0..=20)
                .map(|i| c + Vec2::angled(-1. + i as f32 * 5. / 20.) * 6. * s)
                .collect();
            painter.add(egui::Shape::line(points, stroke));
            line((-6., -5.), (-6., 0.));
            line((-6., -5.), (-1., -5.));
        }
        "settings" => {
            let points = (0..=24)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / 24.;
                    let radius = if i % 4 == 0 || i % 4 == 3 { 8. } else { 6. };
                    c + Vec2::angled(angle) * radius * s
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
            painter.circle_stroke(c, 2.6 * s, stroke);
        }
        "pin" => {
            line((-4., -7.), (4., -7.));
            line((-3., -7.), (-3., -1.));
            line((3., -7.), (3., -1.));
            line((-3., -1.), (-6., 2.));
            line((3., -1.), (6., 2.));
            line((-6., 2.), (6., 2.));
            line((0., 2.), (0., 8.));
        }
        "notes" => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(12. * s, 15. * s)),
                2,
                stroke,
                egui::StrokeKind::Middle,
            );
            for y in [-4., 0., 4.] {
                line((-3., y), (3., y));
            }
        }
        "tasks" => {
            painter.rect_stroke(
                Rect::from_center_size(pt(-4., -4.), Vec2::splat(5. * s)),
                1,
                stroke,
                egui::StrokeKind::Middle,
            );
            line((0., -4.), (7., -4.));
            line((-6., 4.), (-4., 6.));
            line((-4., 6.), (-1., 2.));
            line((1., 4.), (7., 4.));
        }
        "timer" => {
            painter.circle_stroke(pt(0., 1.), 6.5 * s, stroke);
            line((-3., -8.), (3., -8.));
            line((0., -8.), (0., -6.));
            line((0., 1.), (0., -3.));
            line((0., 1.), (3., 3.));
        }
        "files" => {
            painter.add(egui::Shape::line(
                vec![
                    pt(-8., 6.),
                    pt(-8., -6.),
                    pt(-2., -6.),
                    pt(0., -3.),
                    pt(8., -3.),
                    pt(8., 6.),
                    pt(-8., 6.),
                ],
                stroke,
            ));
            line((-8., -1.), (8., -1.));
        }
        "check" => {
            line((-5., 0.), (-1., 4.));
            line((-1., 4.), (6., -5.));
        }
        "next" => {
            line((-7., 0.), (7., 0.));
            line((3., -4.), (7., 0.));
            line((7., 0.), (3., 4.));
        }
        "star" => {
            let mut points = Vec::with_capacity(11);
            for i in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.;
                let radius = if i % 2 == 0 { 8. } else { 3.5 };
                points.push(c + Vec2::angled(a) * radius * s);
            }
            points.push(points[0]);
            painter.add(egui::Shape::line(points, stroke));
        }
        "orbit" => {
            let points = (0..=24)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 24.;
                    c + Vec2::new(a.cos() * 8. * s, a.sin() * 5. * s)
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
            painter.circle_filled(pt(6., -4.), 1.8 * s, color);
        }
        "sparkle" => {
            let points = [
                pt(0., -8.),
                pt(2., -2.),
                pt(8., 0.),
                pt(2., 2.),
                pt(0., 8.),
                pt(-2., 2.),
                pt(-8., 0.),
                pt(-2., -2.),
            ];
            for i in 0..points.len() {
                painter.add(egui::Shape::convex_polygon(
                    vec![c, points[i], points[(i + 1) % points.len()]],
                    color,
                    Stroke::NONE,
                ));
            }
        }
        "cpu" | "gpu" => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(10. * s)),
                2,
                stroke,
                egui::StrokeKind::Middle,
            );
            painter.rect_filled(Rect::from_center_size(c, Vec2::splat(4. * s)), 1, color);
            for v in [-3., 0., 3.] {
                line((v, -8.), (v, -5.));
                line((v, 5.), (v, 8.));
                line((-8., v), (-5., v));
                line((5., v), (8., v));
            }
        }
        "ram" => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(16. * s, 9. * s)),
                2,
                stroke,
                egui::StrokeKind::Middle,
            );
            for x in [-5., 0., 5.] {
                line((x, -2.), (x, 2.));
                line((x, 5.), (x, 7.));
            }
        }
        "network" => {
            line((-4., -7.), (-4., 7.));
            line((-7., -3.), (-4., -7.));
            line((-1., -3.), (-4., -7.));
            line((4., -7.), (4., 7.));
            line((1., 3.), (4., 7.));
            line((7., 3.), (4., 7.));
        }
        "storage" => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(15. * s, 12. * s)),
                3,
                stroke,
                egui::StrokeKind::Middle,
            );
            line((-5., 2.), (5., 2.));
            painter.circle_filled(pt(4., 4.), s, color);
        }
        "claude" => {
            for i in 0..12 {
                let a = i as f32 * std::f32::consts::TAU / 12.;
                painter.line_segment(
                    [c + Vec2::angled(a) * 2. * s, c + Vec2::angled(a) * 8. * s],
                    stroke,
                );
            }
        }
        "codex" | "chatgpt" => {
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::TAU / 6.;
                let center = c + Vec2::angled(a) * 3.5 * s;
                let mut pts = Vec::new();
                for j in 0..5 {
                    let b = a + j as f32 * std::f32::consts::TAU / 6.;
                    pts.push(center + Vec2::angled(b) * 4.7 * s);
                }
                painter.add(egui::Shape::line(pts, Stroke::new(1.1_f32, color)));
            }
        }
        _ => {
            painter.circle_stroke(c, 7. * s, stroke);
        }
    }
}
