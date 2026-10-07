use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

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
                Color32::from_rgb(215, 255, 99),
                Color32::from_rgb(255, 110, 166),
            ),
        };
        Self {
            bg: Color32::from_rgb(15, 20, 28),
            panel: Color32::from_rgb(26, 33, 44),
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
pub fn gauge(ui: &mut egui::Ui, value: Option<f64>, label: &str, p: Palette) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(130., 100.), egui::Sense::hover());
    let c = r.center() + Vec2::new(0., 8.);
    for i in 0..40 {
        let a = std::f32::consts::PI * (1.15 + i as f32 / 39. * 1.7);
        let color = if value.is_some_and(|v| i as f64 / 39. * 100. <= v) {
            stress(i as f64 / 39. * 100.)
        } else {
            p.panel
        };
        ui.painter().line_segment(
            [c + Vec2::angled(a) * 38.0_f32, c + Vec2::angled(a) * 45.0_f32],
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
pub fn icon(painter: &egui::Painter, c: Pos2, key: &str, color: Color32, size: f32) {
    let s = size / 16.;
    let pt = |x: f32, y: f32| c + Vec2::new(x * s, y * s);
    let stroke = Stroke::new(1.6_f32, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([pt(a.0, a.1), pt(b.0, b.1)], stroke);
    };
    match key {
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
