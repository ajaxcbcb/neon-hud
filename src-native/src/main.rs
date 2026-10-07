#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod backend;
mod desktop;
mod model;
mod paint;
mod updater;

use backend::{Command, Event, Worker};
use desktop::{Tray, TrayAction};
use eframe::egui::{
    self, Color32, FontId, Pos2, Rect, Sense, Vec2, ViewportBuilder, ViewportCommand, ViewportId,
};
use model::*;
use paint::Palette;
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
fn main() -> eframe::Result<()> {
    if let Some(result) = updater::run_helper_cli() {
        if let Err(error) = result {
            eprintln!("Native update failed: {error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if neon_hud_lib::native_api::run_bridge_cli() {
        return Ok(());
    }
    let args: Vec<String> = std::env::args().collect();
    let smoke = args.iter().any(|a| a == "--smoke");
    let smoke_interaction = smoke
        && args
            .iter()
            .any(|a| a == "--smoke-hover" || a == "--smoke-menu");
    let smoke_page = args
        .iter()
        .find_map(|a| {
            a.strip_prefix("--smoke-page=")
                .and_then(|v| v.parse::<usize>().ok())
        })
        .unwrap_or(0)
        .min(3);
    let minimized = args.iter().any(|a| a == "--minimized") && !smoke;
    let dir = match desktop::profile_dir(smoke) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return Ok(());
        }
    };
    if let Some(result) = update_cli(&args, &dir) {
        match result {
            Ok(receipt) => println!("{receipt}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    let _instance = match updater::acquire_instance(&dir) {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("{error}");
            return Ok(());
        }
    };
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Neon HUD Native")
            .with_icon(desktop::app_icon())
            .with_inner_size([280., 56.])
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_always_on_top()
            .with_position([60., 60.]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Neon HUD Native",
        options,
        Box::new(move |cc| {
            Ok(Box::new(App::new(
                cc,
                dir,
                smoke,
                minimized,
                smoke_page,
                smoke_interaction,
            )))
        }),
    )
}

fn update_cli(args: &[String], profile: &std::path::Path) -> Option<Result<Value, String>> {
    let mode = args.get(1)?.as_str();
    if !matches!(mode, "--update-check" | "--update-download" | "--update-install"
        | "--verify-native-manifest" | "--verify-native-archive") { return None; }
    Some((|| {
        if mode.starts_with("--verify-native-") {
            let manifest = std::fs::read(args.get(2).ok_or("Manifest path required")?)
                .map_err(|e| e.to_string())?;
            let signature = std::fs::read(args.get(3).ok_or("Signature path required")?)
                .map_err(|e| e.to_string())?;
            let version = updater::verify_manifest_bytes(&manifest, &signature)?;
            if mode == "--verify-native-archive" {
                let platform = if cfg!(windows) { "windows-x86_64" }
                    else if cfg!(target_arch="aarch64") { "macos-aarch64" }
                    else { "macos-x86_64" };
                updater::verify_archive_file(&manifest, &signature, platform,
                    std::path::Path::new(args.get(4).ok_or("Archive path required")?))?;
            }
            return Ok(json!({"verified":true,"version":version}));
        }
        let lease = if mode == "--update-install" {
            Some(updater::acquire_instance(profile)?)
        } else { None };
        let current = env!("CARGO_PKG_VERSION");
        let Some(offer) = updater::check_now()? else {
            return Ok(json!({"status":"current","version":current}));
        };
        let version = offer.version.clone();
        if mode == "--update-check" {
            return Ok(json!({"status":"available","installedVersion":current,
                "version":version,"signed":true}));
        }
        let stage = updater::download_now(profile, offer)?;
        if mode == "--update-install" {
            updater::launch_helper(&stage)?;
            drop(lease);
        }
        Ok(json!({"status":if mode == "--update-install" {"restart_requested"} else {"verified"},
            "installedVersion":current,"version":version,"signed":true}))
    })())
}

struct App {
    worker: Worker,
    profile_dir: PathBuf,
    update_worker: updater::Worker,
    update_preferences: UpdatePreferences,
    update_status: String,
    update_offer: Option<updater::Offer>,
    update_stage: Option<updater::VerifiedStage>,
    update_busy: bool,
    update_progress: Option<(u64, u64)>,
    last_update: Instant,
    applying_update: bool,
    update_acknowledged: bool,
    profile: Value,
    system: Value,
    providers: Value,
    loaded: bool,
    writable: bool,
    revision: u64,
    saved: u64,
    save_pending: bool,
    changed: Instant,
    system_pending: bool,
    providers_pending: bool,
    last_system: Instant,
    last_providers: Instant,
    status: String,
    busy: bool,
    mode: Mode,
    mode_since: Instant,
    tray: Option<Tray>,
    hidden: bool,
    paused: bool,
    settings: bool,
    details: bool,
    page: usize,
    preferences_tab: usize,
    drive_page: usize,
    selected: String,
    hover: Option<(String, Rect)>,
    controls: Option<Rect>,
    controls_focused: bool,
    drain: Drain,
    history: VecDeque<(f64, Option<f64>)>,
    smoke: bool,
    smoke_interaction: bool,
    started: Instant,
    quitting: bool,
    stopping: bool,
    startup_enabled: bool,
    attention_until: Instant,
    last_attention: String,
    screens: Vec<Screen>,
    last_screens: Instant,
    position_hold: Instant,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct UpdatePreferences {
    automatic_checks: bool,
    automatic_downloads: bool,
}
impl Default for UpdatePreferences {
    fn default() -> Self {
        Self { automatic_checks: true, automatic_downloads: true }
    }
}
impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        dir: PathBuf,
        smoke: bool,
        minimized: bool,
        smoke_page: usize,
        smoke_interaction: bool,
    ) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.set_embed_viewports(false);
        paint::configure_fonts(ctx);
        Palette::new("circuit").apply(ctx);
        let tray = Tray::new(ctx.clone());
        let status = tray
            .as_ref()
            .err()
            .map(|e| format!("Tray unavailable: {e}"))
            .unwrap_or_default();
        let tray = tray.ok();
        let hidden = minimized && tray.is_some();
        if hidden {
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }
        let startup_enabled = desktop::startup()
            .and_then(|a| a.is_enabled().map_err(|e| e.to_string()))
            .unwrap_or(false);
        Self {
            worker: Worker::start(dir.clone(), ctx.clone()),
            update_worker: updater::Worker::start(dir.clone(), ctx.clone()),
            update_preferences: std::fs::read(dir.join("updater-settings.json"))
                .ok().and_then(|v| serde_json::from_slice(&v).ok()).unwrap_or_default(),
            profile_dir: dir,
            update_status: "Ready to check for updates".into(),
            update_offer: None,
            update_stage: None,
            update_busy: false,
            update_progress: None,
            last_update: Instant::now() - Duration::from_secs(6 * 3600),
            applying_update: false,
            update_acknowledged: false,
            profile: json!({"theme":"circuit","size":"compact","motion":"playful","alwaysOnTop":true,"reducedMotion":false,"metrics":{"cpu":true,"gpu":true,"ram":true,"network":true,"storage":true,"ai":true},"resources":{"adaptive":true,"samplingMs":1000},"storageDriveIds":[]}),
            system: Value::Null,
            providers: Value::Null,
            loaded: false,
            writable: false,
            revision: 0,
            saved: 0,
            save_pending: false,
            changed: Instant::now(),
            system_pending: false,
            providers_pending: false,
            last_system: Instant::now() - Duration::from_secs(20),
            last_providers: Instant::now() - Duration::from_secs(60),
            status,
            busy: false,
            mode: Mode::Normal,
            mode_since: Instant::now(),
            tray,
            hidden,
            paused: false,
            settings: smoke && !smoke_interaction,
            details: smoke && smoke_page == 0 && !smoke_interaction,
            page: if smoke { smoke_page.min(2) } else { 0 },
            preferences_tab: usize::from(smoke && smoke_page == 3),
            drive_page: 0,
            selected: "cpu".into(),
            hover: None,
            controls: None,
            controls_focused: false,
            drain: Drain::default(),
            history: VecDeque::new(),
            smoke,
            smoke_interaction,
            started: Instant::now(),
            quitting: false,
            stopping: false,
            startup_enabled,
            attention_until: Instant::now(),
            last_attention: String::new(),
            screens: desktop::screens(),
            last_screens: Instant::now(),
            position_hold: Instant::now(),
        }
    }
    fn palette(&self) -> Palette {
        Palette::new(text(&self.profile, "theme"))
    }
    fn dirty(&mut self) {
        self.revision += 1;
        self.changed = Instant::now();
    }
    fn command(&mut self, c: Command) -> bool {
        if self.worker.tx.try_send(c).is_ok() {
            true
        } else {
            self.status = "Monitor busy; please try again".into();
            false
        }
    }
    fn action(&mut self, c: Command) {
        if self.busy {
            return;
        }
        if self.command(c) {
            self.busy = true;
            self.status = "Connecting…".into();
        }
    }
    fn compress(&mut self, ctx: &egui::Context) {
        self.profile["size"] = json!(if text(&self.profile, "size") == "compressed" {
            "compact"
        } else {
            "compressed"
        });
        self.dirty();
        self.resize(ctx);
    }
    fn resize(&self, ctx: &egui::Context) {
        let width = if text(&self.profile, "size") == "compressed" {
            160.
        } else {
            280.
        };
        ctx.send_viewport_cmd_to(
            ViewportId::ROOT,
            ViewportCommand::InnerSize(Vec2::new(width, 56.)),
        );
    }
    fn show(&mut self, ctx: &egui::Context) {
        self.hidden = false;
        ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Focus);
    }
    fn hide(&mut self, ctx: &egui::Context) {
        if self.tray.is_some() {
            self.hidden = true;
            self.hover = None;
            self.controls = None;
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Visible(false));
        } else {
            self.status = "Tray unavailable; HUD remains visible".into();
            self.settings = true;
        }
    }
    fn events(&mut self, ctx: &egui::Context) {
        while let Ok(e) = self.worker.rx.try_recv() {
            match e {
                Event::Loaded(Ok(v)) => {
                    self.profile = v;
                    self.loaded = true;
                    self.writable = true;
                    if text(&self.profile, "size") == "expanded" {
                        self.profile["size"] = json!("compact");
                        self.dirty();
                    }
                    self.resize(ctx);
                    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(
                        if flag(&self.profile, "alwaysOnTop") {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        },
                    ));
                    if !self.smoke {
                        let width = if text(&self.profile, "size") == "compressed" {
                            160.
                        } else {
                            280.
                        };
                        let scale = desktop::coordinate_scale(ctx.pixels_per_point());
                        if let Some([x, y]) = restore_position(
                            &self.profile["windowPosition"],
                            &self.screens,
                            [width, 56.],
                        ) {
                            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(
                                (x / scale) as f32,
                                (y / scale) as f32,
                            )));
                            self.position_hold = Instant::now() + Duration::from_millis(350);
                        }
                        self.settings = !flag(&self.profile, "completed");
                    }
                    // Preview launch never enables connectors or startup automatically.
                }
                Event::Loaded(Err(e)) => {
                    self.loaded = true;
                    self.writable = false;
                    self.status = e;
                    self.settings = true;
                }
                Event::System(r) => {
                    self.system_pending = false;
                    match r {
                        Ok(v) => {
                            if self
                                .history
                                .back()
                                .is_none_or(|(t, _)| Some(*t) != number(&v, "sampledAt"))
                            {
                                self.history.push_back((
                                    number(&v, "sampledAt").unwrap_or(now()),
                                    number(&v, "cpu"),
                                ));
                                while self.history.len() > 120 {
                                    self.history.pop_front();
                                }
                            }
                            self.system = v;
                            let mode = resource_mode(&self.system, &self.profile, now());
                            if mode == Mode::Critical
                                || self.mode_since.elapsed() > Duration::from_secs(10)
                            {
                                if mode != self.mode {
                                    self.mode = mode;
                                    self.mode_since = Instant::now();
                                }
                            }
                        }
                        Err(e) => self.status = e,
                    }
                }
                Event::Providers(r) => {
                    self.providers_pending = false;
                    match r {
                        Ok(v) => {
                            for u in array(&v, "usages") {
                                self.drain.observe(u, now());
                            }
                            if let Some(a) = array(&v, "attention").first() {
                                let id = text(a, "id");
                                if self.last_attention != id {
                                    self.last_attention = id.into();
                                    self.attention_until = Instant::now() + Duration::from_secs(2);
                                }
                            }
                            self.providers = v;
                        }
                        Err(e) => self.status = e,
                    }
                }
                Event::Saved(rev, r) => {
                    self.save_pending = false;
                    match r {
                        Ok(()) => self.saved = self.saved.max(rev),
                        Err(e) => {
                            self.status = e;
                            self.quitting = false;
                            self.applying_update = false;
                            self.settings = true;
                            self.show(ctx);
                        }
                    }
                }
                Event::Action(r) => {
                    self.busy = false;
                    self.status = r.unwrap_or_else(|e| format!("Connection failed: {e}"));
                    self.last_providers = Instant::now() - Duration::from_secs(120);
                }
                Event::Stopped => {
                    if self.applying_update {
                        let result = self.update_stage.as_ref()
                            .ok_or_else(|| "Verified update is missing".to_string())
                            .and_then(updater::launch_helper);
                        if let Err(error) = result {
                            self.update_status = format!("Could not apply update: {error}");
                            self.applying_update = false;
                            self.quitting = false;
                            self.stopping = false;
                            self.worker = Worker::start(self.profile_dir.clone(), ctx.clone());
                            self.settings = true;
                            self.show(ctx);
                            continue;
                        }
                    }
                    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
                }
            }
        }
    }
    fn poll(&mut self) {
        if !self.loaded || self.quitting {
            return;
        }
        let interval = self.mode.interval(
            number(&self.profile["resources"], "samplingMs").unwrap_or(1000.) as u64,
            self.hidden,
        );
        if !self.paused
            && !self.system_pending
            && self.last_system.elapsed() >= Duration::from_millis(interval)
            && self.command(Command::System(self.mode.name().into(), interval))
        {
            self.system_pending = true;
            self.last_system = Instant::now();
        }
        let provider_interval = if self.mode == Mode::Normal && !self.hidden {
            20
        } else {
            60
        };
        if !self.providers_pending
            && self.last_providers.elapsed() >= Duration::from_secs(provider_interval)
            && self.command(Command::Providers(self.mode.name().into()))
        {
            self.providers_pending = true;
            self.last_providers = Instant::now();
        }
    }
    fn persist(&mut self) {
        if self.writable
            && self.revision > self.saved
            && !self.save_pending
            && (self.quitting || self.changed.elapsed() > Duration::from_millis(500))
            && self.command(Command::Save(self.profile.clone(), self.revision))
        {
            self.save_pending = true;
        }
        if self.quitting
            && !self.save_pending
            && (!self.writable || self.saved == self.revision)
            && !self.stopping
            && self.command(Command::Stop)
        {
            self.stopping = true;
        }
    }
    fn usage(&self, surface: &str) -> Value {
        array(&self.providers, "usages")
            .iter()
            .find(|u| text(u, "surface") == surface)
            .cloned()
            .unwrap_or(Value::Null)
    }
    fn selected_drives(&self) -> Vec<Value> {
        let ids = array(&self.profile, "storageDriveIds");
        array(&self.system, "drives")
            .iter()
            .filter(|d| ids.is_empty() || ids.iter().any(|id| id.as_str() == Some(text(d, "id"))))
            .cloned()
            .collect()
    }
    fn reading(&self, key: &str) -> (Option<f64>, String, bool) {
        match key {
            "cpu" => {
                let n = number(&self.system, "cpu");
                (
                    n,
                    percent(n),
                    n.is_some_and(|v| {
                        v >= number(&self.profile["performance"], "cpuPercent").unwrap_or(90.)
                    }) || component_heat(&self.system, &self.profile),
                )
            }
            "ram" => {
                let n = ratio(
                    number(&self.system["memory"], "used"),
                    number(&self.system["memory"], "total"),
                );
                (
                    n,
                    percent(n),
                    n.is_some_and(|v| {
                        v >= number(&self.profile["performance"], "memoryPercent").unwrap_or(90.)
                    }),
                )
            }
            "gpu" => {
                let g = array(&self.system, "gpus").iter().find(|g| {
                    text(g, "status") == "live"
                        && number(g, "sampledAt")
                            .is_some_and(|t| (0.0..=12.).contains(&(now() - t)))
                });
                let n = g.and_then(|g| number(g, "utilizationPercent"));
                let hot = gpu_heat(&self.system, &self.profile, now());
                (n, percent(n), n.is_some_and(|v| v >= 90.) || hot)
            }
            "storage" => {
                let ds = self.selected_drives();
                let n = ds
                    .iter()
                    .filter_map(|d| ratio(number(d, "usedBytes"), number(d, "totalBytes")))
                    .max_by(f64::total_cmp);
                (
                    n,
                    percent(n),
                    n.is_some_and(|v| {
                        v >= number(&self.profile["performance"], "storagePercent").unwrap_or(90.)
                    }),
                )
            }
            "network" => {
                let interfaces = array(&self.system, "networks");
                let n = interfaces
                    .iter()
                    .filter(|v| {
                        text(&self.profile, "interface") == "auto"
                            || text(v, "name") == text(&self.profile, "interface")
                    })
                    .filter_map(|v| number(v, "down").zip(number(v, "up")).map(|(d, u)| d + u))
                    .reduce(|a, b| a + b);
                (
                    None,
                    n.map(|n| format!("{}/s", bytes(n)))
                        .unwrap_or_else(|| "—".into()),
                    false,
                )
            }
            _ => {
                let u = self.usage(key);
                let current = text(&u, "state") == "connected"
                    && number(&u, "fetchedAt").is_some_and(|t| (0.0..=600.).contains(&(now() - t)));
                let w = array(&u, "windows")
                    .iter()
                    .find(|w| number(w, "minutes") == Some(300.))
                    .or_else(|| array(&u, "windows").first());
                let n = w
                    .filter(|w| current && !number(w, "resetsAt").is_some_and(|t| t <= now()))
                    .and_then(|w| number(w, "usedPercent"))
                    .map(|n| 100. - n);
                let attention = array(&self.providers, "attention").iter().any(|a| {
                    text(a, "surface") == key
                        || (key == "claude" && text(a, "surface") == "claude-code")
                });
                (
                    n,
                    percent(n),
                    attention
                        || n.is_some_and(|n| n <= number(&self.profile, "warning").unwrap_or(20.)),
                )
            }
        }
    }
    fn hud(&mut self, ctx: &egui::Context) {
        if self.hidden {
            return;
        }
        let p = self.palette();
        let compact = text(&self.profile, "size") == "compressed";
        self.hover = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let r = ui.max_rect().shrink(2.);
                ui.painter().rect_filled(r, 26, p.bg);
                ui.painter().rect_stroke(
                    r,
                    26,
                    egui::Stroke::new(1.0_f32, p.accent.gamma_multiply(0.55)),
                    egui::StrokeKind::Inside,
                );
                let grip = Rect::from_min_size(r.min + Vec2::new(3., 7.), Vec2::new(16., 38.));
                for y in [-5., 0., 5.] {
                    ui.painter()
                        .circle_filled(grip.center() + Vec2::new(0., y), 1.2, p.dim);
                }
                let drag = ui.interact(grip, ui.id().with("drag"), Sense::click_and_drag());
                if drag.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    self.compress(ctx);
                }
                let mut keys = Vec::new();
                for key in ["cpu", "ram", "network", "storage", "codex", "claude"] {
                    let enabled = flag(
                        &self.profile["metrics"],
                        if key == "codex" || key == "claude" {
                            "ai"
                        } else {
                            key
                        },
                    );
                    if enabled && (!compact || !["network", "storage"].contains(&key)) {
                        keys.push(key);
                    }
                }
                if keys.is_empty() {
                    keys.push("cpu");
                }
                let width = (r.width() - 25.) / keys.len() as f32;
                for (i, key) in keys.iter().enumerate() {
                    let cell = Rect::from_min_size(
                        r.min + Vec2::new(22. + i as f32 * width, 4.),
                        Vec2::new(width, 44.),
                    );
                    let response = ui.interact(cell, ui.id().with(key), Sense::click());
                    let (value, label, attention) = self.reading(key);
                    let moving = self.mode == Mode::Normal
                        && !flag(&self.profile, "reducedMotion")
                        && text(&self.profile, "motion") != "quiet";
                    let t = ctx.input(|i| i.time) as f32;
                    let shake = if moving && attention && Instant::now() < self.attention_until {
                        (t * 40.).sin() * 2.
                    } else {
                        0.
                    };
                    let bounce = if moving && response.hovered() {
                        (t * 11.).sin() * 1.4
                    } else {
                        0.
                    };
                    let c = cell.center_top() + Vec2::new(shake, 13. + bounce);
                    let color = if attention {
                        p.pop
                    } else if value.is_some() || *key == "network" {
                        p.accent
                    } else {
                        p.dim
                    };
                    if response.hovered() {
                        ui.painter().rect_filled(cell.shrink(1.), 10, p.panel);
                        self.hover = ctx
                            .input(|i| i.viewport().outer_rect)
                            .map(|rect| (key.to_string(), rect));
                    }
                    paint::icon(ui.painter(), c, key, color, 15.);
                    if attention {
                        ui.painter().text(
                            c + Vec2::new(10., -6.),
                            egui::Align2::CENTER_CENTER,
                            "!",
                            FontId::monospace(11.),
                            p.pop,
                        );
                    }
                    let label = if compact && *key == "network" {
                        "NET".into()
                    } else {
                        label
                    };
                    ui.painter().text(
                        cell.center_bottom() + Vec2::new(0., -4.),
                        egui::Align2::CENTER_BOTTOM,
                        label,
                        FontId::monospace(if compact { 10. } else { 11. }),
                        p.ink,
                    );
                    if response.clicked() {
                        self.controls = None;
                        self.selected = key.to_string();
                        self.details = true;
                        self.hover = None;
                    }
                    if moving
                        && (response.hovered()
                            || (attention && Instant::now() < self.attention_until))
                    {
                        ctx.request_repaint_after(Duration::from_millis(32));
                    }
                }
                // Read secondary input for the whole pill, including metric and grip widgets.
                // An egui context_menu would be clipped by this 56-point native viewport.
                let open = ctx.input(|i| {
                    (i.pointer.button_clicked(egui::PointerButton::Secondary)
                        && i.pointer.interact_pos().is_some_and(|pos| r.contains(pos)))
                        || (i.viewport().focused.unwrap_or(false)
                            && i.modifiers.shift
                            && i.key_pressed(egui::Key::F10))
                });
                if open {
                    self.controls = ctx.input(|i| i.viewport().outer_rect);
                    self.controls_focused = false;
                    self.hover = None;
                }
            });
        if ctx.input(|i| i.viewport().focused.unwrap_or(false)) {
            let delta = ctx.input(|i| {
                let step = if i.modifiers.shift { 1. } else { 10. };
                Vec2::new(
                    (i.key_pressed(egui::Key::ArrowRight) as i8
                        - i.key_pressed(egui::Key::ArrowLeft) as i8) as f32
                        * step,
                    (i.key_pressed(egui::Key::ArrowDown) as i8
                        - i.key_pressed(egui::Key::ArrowUp) as i8) as f32
                        * step,
                )
            });
            if delta != Vec2::ZERO {
                if let Some(r) = ctx.input(|i| i.viewport().outer_rect) {
                    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(r.min + delta));
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.hide(ctx);
            }
        }
        if self.loaded && self.writable && !self.smoke && Instant::now() >= self.position_hold {
            if let Some(r) = ctx.input(|i| i.viewport().outer_rect) {
                let scale = desktop::coordinate_scale(ctx.pixels_per_point());
                if let Some(position) = remember_position(
                    [r.min.x as f64 * scale, r.min.y as f64 * scale],
                    [r.width() as f64 * scale, r.height() as f64 * scale],
                    &self.screens,
                ) {
                    if self.profile["windowPosition"] != position {
                        self.profile["windowPosition"] = position;
                        self.dirty();
                    }
                }
            }
        }
    }
    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings {
            return;
        }
        let p = self.palette();
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("settings"),
            ViewportBuilder::default()
                .with_title("Neon HUD · Settings")
                .with_icon(desktop::app_icon())
                .with_inner_size([740., 680.])
                .with_resizable(false)
                .with_decorations(false)
                .with_transparent(true)
                .with_position(if self.smoke { [250., 20.] } else { [120., 20.] }),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    self.settings = false;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        let r = ui.max_rect();
                        let at = |x, y| r.min + Vec2::new(x, y);
                        let border = egui::Stroke::new(1., p.dim.gamma_multiply(0.4));
                        ui.painter().rect_filled(r.shrink(1.), 18, p.bg);
                        ui.painter().rect_stroke(
                            r.shrink(1.),
                            18,
                            border,
                            egui::StrokeKind::Inside,
                        );
                        let header = Rect::from_min_size(at(1., 1.), Vec2::new(738., 58.));
                        ui.painter().rect_filled(header, 17, p.panel);
                        ui.painter()
                            .line_segment([at(1., 59.), at(739., 59.)], border);
                        let drag = Rect::from_min_size(at(0., 0.), Vec2::new(688., 58.));
                        if ui
                            .interact(drag, ui.id().with("settings-drag"), Sense::drag())
                            .drag_started()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                        }
                        ui.painter().rect_filled(
                            Rect::from_min_size(at(28., 19.), Vec2::splat(23.)),
                            5,
                            p.pop,
                        );
                        ui.painter().rect_filled(
                            Rect::from_min_size(at(26., 17.), Vec2::splat(23.)),
                            5,
                            p.accent,
                        );
                        ui.painter().text(
                            at(37., 29.),
                            egui::Align2::CENTER_CENTER,
                            "N",
                            FontId::proportional(19.),
                            p.bg,
                        );
                        ui.painter().text(
                            at(58., 29.),
                            egui::Align2::LEFT_CENTER,
                            "NEON HUD",
                            FontId::monospace(13.),
                            p.ink,
                        );
                        ui.painter().text(
                            at(681., 29.),
                            egui::Align2::RIGHT_CENTER,
                            "PERSONAL COMMAND CENTER",
                            FontId::monospace(9.),
                            p.dim,
                        );
                        let close = Rect::from_min_size(at(698., 15.), Vec2::splat(28.));
                        if ui
                            .interact(close, ui.id().with("settings-close"), Sense::click())
                            .on_hover_text("Return to HUD")
                            .clicked()
                        {
                            self.settings = false;
                        }
                        ui.painter().text(
                            close.center(),
                            egui::Align2::CENTER_CENTER,
                            "×",
                            FontId::proportional(22.),
                            p.dim,
                        );
                        for (i, name) in ["Appearance", "Connections", "Preferences"]
                            .iter()
                            .enumerate()
                        {
                            let tab = Rect::from_min_size(
                                at(28. + i as f32 * 164., 77.),
                                Vec2::new(154., 40.),
                            );
                            let selected = self.page == i;
                            let response =
                                ui.interact(tab, ui.id().with(("setup-step", i)), Sense::click());
                            if selected {
                                ui.painter().rect_filled(
                                    tab.translate(Vec2::new(3., 4.)),
                                    9,
                                    p.pop,
                                );
                                ui.painter().rect_filled(tab, 9, p.accent);
                            } else if response.hovered() {
                                ui.painter().rect_filled(tab, 9, p.panel);
                            }
                            let ink = if selected { p.bg } else { p.dim };
                            let c = tab.left_center() + Vec2::new(23., 0.);
                            ui.painter()
                                .circle_stroke(c, 11., egui::Stroke::new(1., ink));
                            ui.painter().text(
                                c,
                                egui::Align2::CENTER_CENTER,
                                format!("{}", i + 1),
                                FontId::monospace(11.),
                                ink,
                            );
                            ui.painter().text(
                                tab.left_center() + Vec2::new(42., 0.),
                                egui::Align2::LEFT_CENTER,
                                *name,
                                FontId::proportional(12.),
                                ink,
                            );
                            if response.clicked() {
                                self.page = i;
                            }
                        }
                        let content = Rect::from_min_size(at(28., 140.), Vec2::new(684., 474.));
                        ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
                            ui.set_clip_rect(content);
                            ui.add_enabled_ui(self.writable, |ui| match self.page {
                                0 => self.appearance(ui, ctx),
                                1 => self.connections(ui),
                                _ => self.preferences(ui, ctx),
                            });
                        });
                        let footer = Rect::from_min_size(at(1., 623.), Vec2::new(738., 56.));
                        ui.painter().rect_filled(footer, 17, p.panel);
                        ui.painter()
                            .line_segment([at(1., 623.), at(739., 623.)], border);
                        let status = if !self.status.is_empty() {
                            self.status.clone()
                        } else if !self.writable {
                            "Loading profile…".into()
                        } else if self.save_pending || self.saved < self.revision {
                            "Saving preferences…".into()
                        } else {
                            "LOCAL FIRST · SMALL BY DESIGN · SAVED".into()
                        };
                        let short: String = status.chars().take(47).collect();
                        let status_rect = Rect::from_min_size(at(28., 640.), Vec2::new(340., 25.));
                        ui.painter().text(
                            status_rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            short,
                            FontId::monospace(9.),
                            p.dim,
                        );
                        ui.interact(status_rect, ui.id().with("save-status"), Sense::hover())
                            .on_hover_text(status);
                        let back = Rect::from_min_size(at(407., 638.), Vec2::new(114., 32.));
                        ui.painter().text(
                            back.center(),
                            egui::Align2::CENTER_CENTER,
                            "Return to HUD",
                            FontId::proportional(12.),
                            p.accent,
                        );
                        if ui
                            .interact(back, ui.id().with("return-hud"), Sense::click())
                            .clicked()
                        {
                            self.settings = false;
                        }
                        let next = Rect::from_min_size(at(540., 635.), Vec2::new(170., 36.));
                        ui.painter()
                            .rect_filled(next.translate(Vec2::new(3., 4.)), 7, p.pop);
                        ui.painter().rect_filled(next, 7, p.accent);
                        let label = match self.page {
                            0 => "Next: Connect sources",
                            1 => "Next: Preferences",
                            _ => "Ready: Show HUD",
                        };
                        ui.painter().text(
                            next.center() - Vec2::new(8., 0.),
                            egui::Align2::CENTER_CENTER,
                            label,
                            FontId::proportional(12.),
                            p.bg,
                        );
                        paint::icon(
                            ui.painter(),
                            next.right_center() - Vec2::new(14., 0.),
                            "next",
                            p.bg,
                            12.,
                        );
                        if ui
                            .interact(next, ui.id().with("setup-next"), Sense::click())
                            .clicked()
                        {
                            if self.page < 2 {
                                self.page += 1;
                            } else {
                                if self.writable {
                                    self.profile["completed"] = json!(true);
                                    self.dirty();
                                }
                                self.settings = false;
                            }
                        }
                    });
            },
        );
    }
    fn bool_setting(&mut self, ui: &mut egui::Ui, key: &str, label: &str) {
        let mut v = flag(&self.profile, key);
        if ui.checkbox(&mut v, label).changed() {
            self.profile[key] = json!(v);
            self.dirty();
        }
    }
    fn appearance(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let p = self.palette();
        let origin = ui.max_rect().min;
        let at = |x, y| origin + Vec2::new(x, y);
        for x in (0..684).step_by(18) {
            for y in (0..474).step_by(18) {
                ui.painter()
                    .circle_filled(at(x as f32, y as f32), 0.6, p.dim.gamma_multiply(0.18));
            }
        }
        ui.painter().text(
            at(0., 0.),
            egui::Align2::LEFT_TOP,
            "STEP 01 / TUNE YOUR ORBIT",
            FontId::monospace(10.),
            p.accent,
        );
        ui.painter().text(
            at(0., 24.),
            egui::Align2::LEFT_TOP,
            "Tiny HUD.",
            paint::bold(48.),
            p.ink,
        );
        ui.painter().text(
            at(0., 70.),
            egui::Align2::LEFT_TOP,
            "Big energy",
            paint::bold(48.),
            p.accent,
        );
        paint::icon(ui.painter(), at(271., 91.), "sparkle", p.pop, 27.);
        ui.painter().text(
            at(0., 133.),
            egui::Align2::LEFT_TOP,
            "Your machine. Your AI. Your tiny neon playground.",
            FontId::proportional(12.),
            p.dim,
        );
        let sticker = Rect::from_min_size(at(574., 32.), Vec2::new(102., 52.));
        ui.painter()
            .rect_filled(sticker.translate(Vec2::new(3., 4.)), 26, p.accent);
        ui.painter().rect_filled(sticker, 26, p.pop);
        ui.painter().text(
            sticker.center(),
            egui::Align2::CENTER_CENTER,
            "SMALL\nBUT LOUD ↗",
            FontId::monospace(11.),
            p.bg,
        );
        ui.painter().text(
            at(0., 171.),
            egui::Align2::LEFT_TOP,
            "CHOOSE YOUR ATMOSPHERE",
            FontId::monospace(10.),
            p.dim,
        );
        for (i, (key, label, subtitle, icon)) in [
            (
                "circuit",
                "Neon Circuit",
                "Acid lime + candy pink. Electric.",
                "cpu",
            ),
            (
                "cyberpunk",
                "Cyberpunk Night",
                "Hot pink + cyan. After hours.",
                "star",
            ),
            (
                "aurora",
                "Aurora",
                "Mint + lavender. A softer glow.",
                "orbit",
            ),
        ]
        .iter()
        .enumerate()
        {
            let rect = Rect::from_min_size(at(0., 192. + i as f32 * 63.), Vec2::new(294., 54.));
            if paint::choice_card(
                ui,
                rect,
                key,
                label,
                subtitle,
                icon,
                text(&self.profile, "theme") == *key,
                Palette::new(key),
            )
            .clicked()
            {
                self.profile["theme"] = json!(key);
                self.dirty();
            }
        }
        ui.painter().text(
            at(0., 390.),
            egui::Align2::LEFT_TOP,
            "HOW MUCH MISCHIEF?",
            FontId::monospace(10.),
            p.dim,
        );
        for (i, (key, label, icon)) in [
            ("quiet", "Quiet", "orbit"),
            ("playful", "Playful", "star"),
            ("chaotic", "Chaos!", "sparkle"),
        ]
        .iter()
        .enumerate()
        {
            let rect = Rect::from_min_size(at(i as f32 * 101., 408.), Vec2::new(92., 56.));
            if paint::motion_card(
                ui,
                rect,
                key,
                label,
                icon,
                text(&self.profile, "motion") == *key,
                p,
            )
            .clicked()
            {
                self.profile["motion"] = json!(key);
                self.dirty();
            }
        }
        paint::appearance_preview(
            ui,
            Rect::from_min_size(at(314., 172.), Vec2::new(370., 292.)),
            p,
            text(&self.profile, "size") == "compressed",
        );
    }
    fn preferences(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.colored_label(self.palette().accent, "STEP 03 / MAKE IT FIT YOUR DAY");
        ui.add_space(12.);
        ui.horizontal(|ui| {
            for (i, name) in ["Instruments", "Startup & updates"].iter().enumerate() {
                if ui
                    .selectable_label(self.preferences_tab == i, *name)
                    .clicked()
                {
                    self.preferences_tab = i;
                }
            }
        });
        ui.separator();
        if self.preferences_tab == 0 {
            self.metrics(ui);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Pill size");
                for (key, label) in [("compressed", "Tiny · 160"), ("compact", "Regular · 280")] {
                    if ui
                        .selectable_label(text(&self.profile, "size") == key, label)
                        .clicked()
                    {
                        self.profile["size"] = json!(key);
                        self.dirty();
                        self.resize(ctx);
                    }
                }
            });
            self.bool_setting(ui, "reducedMotion", "Reduce motion");
        } else {
            self.startup_page(ui);
            let old = flag(&self.profile, "alwaysOnTop");
            self.bool_setting(ui, "alwaysOnTop", "Keep HUD above other windows");
            if old != flag(&self.profile, "alwaysOnTop") {
                ctx.send_viewport_cmd_to(
                    ViewportId::ROOT,
                    ViewportCommand::WindowLevel(if flag(&self.profile, "alwaysOnTop") {
                        egui::WindowLevel::AlwaysOnTop
                    } else {
                        egui::WindowLevel::Normal
                    }),
                );
            }
        }
    }
    fn metrics(&mut self, ui: &mut egui::Ui) {
        ui.heading("Your instruments");
        ui.horizontal_wrapped(|ui| {
            for (key, label) in [
                ("cpu", "CPU"),
                ("gpu", "GPU"),
                ("ram", "RAM"),
                ("network", "Network"),
                ("storage", "Drives"),
                ("ai", "AI"),
            ] {
                let mut v = flag(&self.profile["metrics"], key);
                if ui.checkbox(&mut v, label).changed() {
                    self.profile["metrics"][key] = json!(v);
                    self.dirty();
                }
            }
        });
        ui.add_space(8.);
        let mut adaptive = flag(&self.profile["resources"], "adaptive");
        if ui
            .checkbox(&mut adaptive, "Ease monitoring and motion under pressure")
            .changed()
        {
            self.profile["resources"]["adaptive"] = json!(adaptive);
            self.dirty();
        }
        ui.horizontal(|ui| {
            ui.label("Sampling");
            for ms in [250, 500, 1000, 2000] {
                if ui
                    .selectable_label(
                        number(&self.profile["resources"], "samplingMs") == Some(ms as f64),
                        format!("{ms} ms"),
                    )
                    .clicked()
                {
                    self.profile["resources"]["samplingMs"] = json!(ms);
                    self.dirty();
                }
            }
        });
        ui.separator();
        ui.label("Drives · unchecked list means all drives");
        let drives = array(&self.system, "drives").to_vec();
        let pages = drives.len().div_ceil(4).max(1);
        self.drive_page = self.drive_page.min(pages - 1);
        for d in drives.iter().skip(self.drive_page * 4).take(4) {
            let id = text(d, "id");
            let mut selected = array(&self.profile, "storageDriveIds")
                .iter()
                .any(|v| v.as_str() == Some(id));
            let label = format!(
                "{}  ·  {} / {}",
                text(d, "mount"),
                bytes(number(d, "usedBytes").unwrap_or(0.)),
                bytes(number(d, "totalBytes").unwrap_or(0.))
            );
            if ui.checkbox(&mut selected, label).changed() {
                let mut ids = array(&self.profile, "storageDriveIds").to_vec();
                ids.retain(|v| v.as_str() != Some(id));
                if selected {
                    ids.push(json!(id));
                }
                self.profile["storageDriveIds"] = json!(ids);
                self.dirty();
            }
        }
        if drives.is_empty() {
            ui.label("Waiting for drive discovery…");
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.drive_page > 0, egui::Button::new("‹"))
                .clicked()
            {
                self.drive_page -= 1;
            }
            ui.label(format!("{} / {}", self.drive_page + 1, pages));
            if ui
                .add_enabled(self.drive_page + 1 < pages, egui::Button::new("›"))
                .clicked()
            {
                self.drive_page += 1;
            }
        });
    }
    fn connections(&mut self, ui: &mut egui::Ui) {
        ui.colored_label(self.palette().accent, "STEP 02 / CONNECT YOUR SOURCES");
        ui.add_space(12.);
        ui.heading("AI connections");
        let codex = self.usage("codex");
        let claude = self.usage("claude");
        ui.label(
            egui::RichText::new(format!(
                "Codex · {}",
                if self.busy {
                    "connecting…"
                } else if text(&codex, "state").is_empty() {
                    "disconnected"
                } else {
                    text(&codex, "state")
                }
            ))
            .color(self.palette().accent),
        );
        ui.add_enabled_ui(!self.busy, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Connect").clicked() {
                    self.action(Command::Connect(false));
                }
                if ui.button("Sign in").clicked() {
                    self.action(Command::Connect(true));
                }
                if ui.button("Disconnect").clicked() {
                    self.action(Command::Disconnect);
                }
            });
        });
        ui.add_space(12.);
        ui.label(format!(
            "Claude · {}",
            if text(&claude, "state").is_empty() {
                "waiting for bridge"
            } else {
                text(&claude, "state")
            }
        ));
        ui.label(egui::RichText::new(text(&claude, "message")).small());
        ui.add_enabled_ui(!self.busy, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Enable shared bridge").clicked() {
                    self.action(Command::Claude(true));
                }
                if ui.button("Disable shared bridge").clicked() {
                    self.action(Command::Claude(false));
                }
            });
        });
        ui.add_space(14.);
        ui.label(egui::RichText::new("ChatGPT chat allowance has no supported local source.\nCodex allowance is shown separately. Claude Code shares\nthe Claude allowance. Five-hour limits appear when reported.").small().color(self.palette().dim));
        ui.add_space(10.);
        ui.label(
            egui::RichText::new("This preview retains the existing connector helpers.").small(),
        );
    }
    fn startup_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Ready when you are");
        let mut enabled = self.startup_enabled;
        if ui
            .checkbox(
                &mut enabled,
                "Start native preview at login · hidden in tray",
            )
            .changed()
        {
            match desktop::startup().and_then(|a| {
                if enabled { a.enable() } else { a.disable() }.map_err(|e| e.to_string())
            }) {
                Ok(()) => {
                    self.startup_enabled = enabled;
                    self.profile["launchAtLogin"] = json!(enabled);
                    self.dirty();
                    self.status = "Preview startup preference saved".into();
                }
                Err(e) => self.status = format!("Startup change failed: {e}"),
            }
        }
        ui.add_space(12.);
        ui.label("Closing the HUD hides it in the tray. Quit exits it.");
        ui.separator();
        ui.heading("Updates");
        let mut changed = ui.checkbox(&mut self.update_preferences.automatic_checks,
            "Check automatically").changed();
        changed |= ui.checkbox(&mut self.update_preferences.automatic_downloads,
            "Download automatically · when pressure is low").changed();
        if changed {
            let result = serde_json::to_vec_pretty(&self.update_preferences)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    let target = self.profile_dir.join("updater-settings.json");
                    let temporary = self.profile_dir.join("updater-settings.json.tmp");
                    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
                    std::fs::rename(temporary, target).map_err(|e| e.to_string())
                });
            if let Err(error) = result {
                self.update_status = format!("Update preference could not be saved: {error}");
            }
        }
        ui.add(egui::Label::new(egui::RichText::new(&self.update_status).small()).truncate())
            .on_hover_text(&self.update_status);
        if let Some((done, total)) = self.update_progress {
            ui.add(egui::ProgressBar::new(done as f32 / total.max(1) as f32)
                .text(format!("{} / {}", bytes(done as f64), bytes(total as f64))));
        }
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.update_busy && !self.quitting,
                egui::Button::new("Check now")).clicked() {
                self.check_updates();
            }
            if self.update_stage.is_some() {
                if ui.add_enabled(!self.update_busy && self.writable && !self.quitting,
                    egui::Button::new("Restart and update")).clicked() {
                    self.applying_update = true;
                    self.quitting = true;
                    self.update_status = "Saving your settings before restarting…".into();
                }
            } else if self.update_offer.is_some() && !self.update_busy {
                if ui.button("Download update").clicked() {
                    self.download_update();
                }
            }
        });
        ui.add_space(6.);
        ui.label(
            egui::RichText::new(format!("{} · native-preview · signed updates\nApache 2.0 + MIT", env!("CARGO_PKG_VERSION")))
                .small()
                .color(self.palette().dim),
        );
    }
    fn check_updates(&mut self) {
        if self.update_busy { return; }
        if self.update_worker.tx.send(updater::Command::Check).is_ok() {
            self.update_busy = true;
            self.update_progress = None;
            self.update_status = "Checking signed native releases…".into();
            self.last_update = Instant::now();
        } else {
            self.update_status = "Update service is unavailable; restart the HUD to retry".into();
        }
    }
    fn download_update(&mut self) {
        if self.update_busy { return; }
        if let Some(offer) = self.update_offer.clone() {
            if self.update_worker.tx.send(updater::Command::Download(offer)).is_ok() {
                self.update_busy = true;
                self.update_status = "Downloading signed update…".into();
            }
        }
    }
    fn update_events(&mut self) {
        while let Ok(event) = self.update_worker.rx.try_recv() {
            match event {
                updater::Event::State(updater::Status::Checking) => {}
                updater::Event::State(updater::Status::Downloading { done, total }) => {
                    self.update_progress = Some((done, total));
                    self.update_status = "Downloading and verifying…".into();
                }
                updater::Event::State(updater::Status::Available(offer)) => {
                    self.update_busy = false;
                    self.update_status = format!("{} available", offer.version);
                    self.update_offer = Some(offer);
                }
                updater::Event::State(updater::Status::Current) => {
                    self.update_busy = false;
                    self.update_offer = None;
                    self.update_stage = None;
                    self.update_status = format!("You're up to date · {}", env!("CARGO_PKG_VERSION"));
                }
                updater::Event::Ready(stage) => {
                    self.update_busy = false;
                    self.update_progress = None;
                    self.update_status = "Update verified · ready to restart".into();
                    self.update_stage = Some(stage);
                }
                updater::Event::Error(error) => {
                    self.update_busy = false;
                    self.update_progress = None;
                    // A failed automatic download waits for a new explicit check.
                    self.update_offer = None;
                    self.update_status = format!("Update failed: {error}");
                }
            }
        }
        if self.smoke || self.quitting || !self.loaded { return; }
        if self.update_preferences.automatic_checks && self.mode == Mode::Normal
            && self.started.elapsed() > Duration::from_secs(5)
            && self.last_update.elapsed() >= Duration::from_secs(6 * 3600) {
            self.check_updates();
        }
        if self.update_preferences.automatic_checks && self.update_preferences.automatic_downloads && self.mode == Mode::Normal
            && self.update_offer.is_some() && self.update_stage.is_none() && !self.update_busy {
            self.download_update();
        }
    }
    fn detail_window(&mut self, ctx: &egui::Context) {
        if !self.details {
            return;
        }
        let p = self.palette();
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("details"),
            ViewportBuilder::default()
                .with_title("Neon HUD · Instruments")
                .with_icon(desktop::app_icon())
                .with_inner_size([460., 460.])
                .with_resizable(false)
                .with_decorations(false)
                .with_transparent(true)
                .with_position(if self.smoke {
                    [430., 250.]
                } else {
                    [500., 180.]
                }),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    self.details = false;
                }
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(p.bg)
                            .inner_margin(20)
                            .corner_radius(16)
                            .stroke(egui::Stroke::new(1., p.dim.gamma_multiply(0.4))),
                    )
                    .show(ctx, |ui| {
                        let (header, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 28.),
                            Sense::hover(),
                        );
                        let drag_rect =
                            Rect::from_min_max(header.min, header.max - Vec2::new(32., 0.));
                        if ui
                            .interact(drag_rect, ui.id().with("details-drag"), Sense::drag())
                            .drag_started()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                        }
                        ui.painter().text(
                            header.left_center(),
                            egui::Align2::LEFT_CENTER,
                            "NEON / INSTRUMENTS",
                            FontId::monospace(12.),
                            p.accent,
                        );
                        let close = Rect::from_center_size(
                            header.right_center() - Vec2::new(12., 0.),
                            Vec2::splat(24.),
                        );
                        ui.painter().text(
                            close.center(),
                            egui::Align2::CENTER_CENTER,
                            "×",
                            FontId::proportional(20.),
                            p.dim,
                        );
                        if ui
                            .interact(close, ui.id().with("details-close"), Sense::click())
                            .clicked()
                        {
                            self.details = false;
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            for (key, label) in [
                                ("cpu", "CPU"),
                                ("gpu", "GPU"),
                                ("ram", "RAM"),
                                ("network", "NET"),
                                ("storage", "DRIVES"),
                                ("codex", "CODEX"),
                                ("claude", "CLAUDE"),
                            ] {
                                if ui
                                    .selectable_label(self.selected == key, label)
                                    .on_hover_text(key.to_uppercase())
                                    .clicked()
                                {
                                    self.selected = key.into();
                                }
                            }
                        });
                        ui.separator();
                        let (value, label, attention) = self.reading(&self.selected);
                        ui.horizontal(|ui| {
                            let remaining =
                                matches!(self.selected.as_str(), "codex" | "claude" | "chatgpt");
                            paint::gauge(ui, value, &self.selected.to_uppercase(), remaining, p);
                            ui.vertical(|ui| {
                                ui.heading(label);
                                if remaining {
                                    ui.label("remaining allowance");
                                }
                                if attention {
                                    ui.colored_label(p.pop, "! Needs attention");
                                }
                                ui.label(format!("{} monitoring", self.mode.name()));
                            });
                        });
                        match self.selected.as_str() {
                            "codex" | "claude" | "chatgpt" => {
                                let u = self.usage(&self.selected);
                                ui.label(egui::RichText::new(text(&u, "message")).small());
                                for w in array(&u, "windows").iter().take(2) {
                                    let fresh = number(&u, "fetchedAt")
                                        .is_some_and(|t| (0.0..=600.).contains(&(now() - t)))
                                        && !number(w, "resetsAt").is_some_and(|t| t <= now());
                                    let remaining = if fresh {
                                        number(w, "usedPercent").map(|v| 100. - v)
                                    } else {
                                        None
                                    };
                                    ui.label(format!(
                                        "{} · {} remaining",
                                        text(w, "label"),
                                        percent(remaining)
                                    ));
                                    paint::bar(ui, remaining.map(|n| 100. - n), 360.);
                                    let r = self.drain.rate(&u, w, now());
                                    let reset = number(w, "resetsAt")
                                        .map(|t| {
                                            format!(
                                                " · resets in {:.0} min",
                                                ((t - now()) / 60.).max(0.)
                                            )
                                        })
                                        .unwrap_or_default();
                                    ui.label(
                                        egui::RichText::new(
                                            r.per_hour
                                                .map(|n| format!("Drain {n:.1}% / hour{reset}"))
                                                .unwrap_or_else(|| {
                                                    format!(
                                                        "Measuring average for 2 minutes{reset}"
                                                    )
                                                }),
                                        )
                                        .small(),
                                    );
                                    if r.fast {
                                        ui.colored_label(
                                            p.pop,
                                            format!(
                                                "! Draining before reset · {:.0} min at this pace",
                                                r.eta_minutes.unwrap_or(0.)
                                            ),
                                        );
                                    }
                                }
                                ui.label(
                                    egui::RichText::new(
                                        "Token rate: unavailable from this allowance source",
                                    )
                                    .small()
                                    .color(p.dim),
                                );
                                let att = array(&self.providers, "attention").first().cloned();
                                if let Some(a) = att {
                                    if ui.button("Acknowledge question").clicked() {
                                        self.action(Command::Dismiss(text(&a, "id").into()));
                                    }
                                }
                            }
                            "ram" => {
                                ui.label(format!(
                                    "Used {} · Available {} · Total {}",
                                    bytes(number(&self.system["memory"], "used").unwrap_or(0.)),
                                    bytes(
                                        number(&self.system["memory"], "available").unwrap_or(0.)
                                    ),
                                    bytes(number(&self.system["memory"], "total").unwrap_or(0.))
                                ));
                                paint::bar(ui, value, 380.);
                            }
                            "storage" => {
                                for d in self.selected_drives().iter().take(4) {
                                    ui.label(format!(
                                        "{} · {} free",
                                        text(d, "mount"),
                                        bytes(number(d, "availableBytes").unwrap_or(0.))
                                    ));
                                    paint::bar(
                                        ui,
                                        ratio(number(d, "usedBytes"), number(d, "totalBytes")),
                                        380.,
                                    );
                                }
                            }
                            "network" => {
                                for n in array(&self.system, "networks").iter().take(5) {
                                    ui.label(format!(
                                        "{} · ↓ {}/s  ↑ {}/s",
                                        text(n, "name"),
                                        bytes(number(n, "down").unwrap_or(0.)),
                                        bytes(number(n, "up").unwrap_or(0.))
                                    ));
                                }
                            }
                            "gpu" => {
                                for g in array(&self.system, "gpus").iter().take(3) {
                                    ui.label(format!(
                                        "{} · {}",
                                        text(g, "name"),
                                        text(g, "status")
                                    ));
                                    ui.label(
                                        egui::RichText::new(
                                            number(g, "temperatureCelsius")
                                                .map(|n| format!("{n:.0} °C"))
                                                .unwrap_or_else(|| {
                                                    "Temperature unavailable".into()
                                                }),
                                        )
                                        .small(),
                                    );
                                }
                            }
                            _ => {
                                ui.label("CPU history · latest 120 samples");
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::new(380., 82.), Sense::hover());
                                for (i, (_, n)) in self.history.iter().enumerate() {
                                    if let Some(n) = n {
                                        let w = 380. / 120.;
                                        let r = Rect::from_min_max(
                                            rect.left_bottom()
                                                + Vec2::new(i as f32 * w, -*n as f32 / 100. * 82.),
                                            rect.left_bottom()
                                                + Vec2::new((i + 1) as f32 * w - 1., 0.),
                                        );
                                        ui.painter().rect_filled(r, 1, paint::stress(*n));
                                    }
                                }
                                let temps = array(&self.system, "temperatures");
                                ui.label(
                                    egui::RichText::new(
                                        temps
                                            .first()
                                            .and_then(|t| number(t, "celsius"))
                                            .map(|t| format!("Temperature {t:.0} °C"))
                                            .unwrap_or_else(|| {
                                                "CPU temperature unavailable on this device".into()
                                            }),
                                    )
                                    .small(),
                                );
                            }
                        }
                        if self.smoke {
                            ui.colored_label(p.dim, "CI render · actual hosted system readings");
                        }
                    });
            },
        );
    }
    fn controls_window(&mut self, ctx: &egui::Context) {
        let Some(hud) = self.controls else {
            return;
        };
        let scale = desktop::coordinate_scale(ctx.pixels_per_point());
        let Some(placement) = hover_placement(
            [
                hud.min.x as f64 * scale,
                hud.min.y as f64 * scale,
                hud.width() as f64 * scale,
                hud.height() as f64 * scale,
            ],
            [240. * scale, 270. * scale],
            8. * scale,
            &self.screens,
        ) else {
            self.controls = None;
            return;
        };
        let p = self.palette();
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("controls"),
            ViewportBuilder::default()
                .with_title("Neon HUD · Controls")
                .with_icon(desktop::app_icon())
                .with_inner_size([
                    (placement.size[0] / scale) as f32,
                    (placement.size[1] / scale) as f32,
                ])
                .with_position([
                    (placement.position[0] / scale) as f32,
                    (placement.position[1] / scale) as f32,
                ])
                .with_resizable(false)
                .with_decorations(false)
                .with_transparent(true)
                .with_taskbar(false)
                .with_active(true)
                .with_always_on_top(),
            |ctx, _| {
                let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
                if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape))
                    || (self.controls_focused && !focused)
                {
                    self.controls = None;
                }
                self.controls_focused |= focused;
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        let Some(layout) = controls_layout(ui.max_rect().size().into()) else {
                            return;
                        };
                        let r = ui.max_rect().shrink(1.);
                        ui.painter().rect_filled(r, 14, p.bg);
                        ui.painter().rect_stroke(
                            r,
                            14,
                            egui::Stroke::new(1., p.accent.gamma_multiply(0.55)),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            Pos2::new(
                                if layout.compact { 8. } else { 14. },
                                layout.header_height / 2.,
                            ),
                            egui::Align2::LEFT_CENTER,
                            if layout.compact {
                                "CONTROLS"
                            } else {
                                "QUICK CONTROLS"
                            },
                            FontId::monospace(11.),
                            p.accent,
                        );
                        if ui
                            .put(
                                Rect::from_min_size(
                                    Pos2::new(
                                        ui.max_rect().width() - 36.,
                                        if layout.compact { 0. } else { 9. },
                                    ),
                                    Vec2::new(26., layout.header_height.min(26.)),
                                ),
                                egui::Button::new("×").frame(false),
                            )
                            .clicked()
                        {
                            self.controls = None;
                        }
                        let compact = text(&self.profile, "size") == "compressed";
                        let full_labels = [
                            "Settings…",
                            if compact {
                                "Expand pill"
                            } else {
                                "Compress pill"
                            },
                            if self.paused {
                                "Resume monitoring"
                            } else {
                                "Pause monitoring"
                            },
                            "Hide to tray",
                            "Reset position",
                            "Quit Neon HUD",
                        ];
                        let short_labels = [
                            "Settings",
                            if compact { "Expand" } else { "Compress" },
                            if self.paused { "Resume" } else { "Pause" },
                            "Hide",
                            "Reset",
                            "Quit",
                        ];
                        let labels = if layout.compact {
                            short_labels
                        } else {
                            full_labels
                        };
                        if layout.compact {
                            ui.spacing_mut().button_padding = Vec2::splat(1.);
                        }
                        for (index, label) in labels.iter().enumerate() {
                            let [x, y, width, height] = layout.buttons[index];
                            let rect =
                                Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
                            let font_size = if layout.compact { height.min(11.) } else { 14. };
                            let clicked = ui
                                .put(
                                    rect,
                                    egui::Button::new(egui::RichText::new(*label).size(font_size)),
                                )
                                .clicked();
                            if clicked {
                                self.controls = None;
                                match index {
                                    0 => self.settings = true,
                                    1 => self.compress(ctx),
                                    2 => self.paused = !self.paused,
                                    3 => self.hide(ctx),
                                    4 => ctx.send_viewport_cmd_to(
                                        ViewportId::ROOT,
                                        ViewportCommand::OuterPosition(Pos2::new(60., 60.)),
                                    ),
                                    5 => self.quitting = true,
                                    _ => unreachable!(),
                                }
                                if self.smoke {
                                    eprintln!(
                                        "NEON_CONTROL index={index} paused={} hidden={}",
                                        self.paused, self.hidden
                                    );
                                }
                            }
                        }
                    });
            },
        );
    }
    fn hover_window(&mut self, ctx: &egui::Context) {
        let Some((key, hud)) = self.hover.clone() else {
            return;
        };
        if self.details || self.settings || self.controls.is_some() {
            return;
        }
        let scale = desktop::coordinate_scale(ctx.pixels_per_point());
        let Some(placement) = hover_placement(
            [
                hud.min.x as f64 * scale,
                hud.min.y as f64 * scale,
                hud.width() as f64 * scale,
                hud.height() as f64 * scale,
            ],
            [248. * scale, 110. * scale],
            8. * scale,
            &self.screens,
        ) else {
            return;
        };
        let (value, label, attention) = self.reading(&key);
        let p = self.palette();
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("hover"),
            ViewportBuilder::default()
                .with_title("Neon HUD reading")
                .with_icon(desktop::app_icon())
                .with_inner_size([
                    (placement.size[0] / scale) as f32,
                    (placement.size[1] / scale) as f32,
                ])
                .with_position([
                    (placement.position[0] / scale) as f32,
                    (placement.position[1] / scale) as f32,
                ])
                .with_resizable(false)
                .with_decorations(false)
                .with_taskbar(false)
                .with_active(false)
                .with_mouse_passthrough(true)
                .with_always_on_top(),
            |ctx, _| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(p.bg).inner_margin(14))
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(key.to_uppercase());
                            ui.label(egui::RichText::new(&label).monospace().color(if attention {
                                p.pop
                            } else {
                                p.accent
                            }));
                        });
                        let remaining = matches!(key.as_str(), "codex" | "claude" | "chatgpt");
                        paint::bar(ui, value.map(|v| stress_percent(v, remaining)), 218.);
                        ui.label(
                            egui::RichText::new(
                                "Click for measurements · right click for controls",
                            )
                            .small(),
                        );
                    });
            },
        );
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_screens.elapsed() >= Duration::from_secs(30) {
            self.screens = desktop::screens();
            self.last_screens = Instant::now();
        }
        self.events(ctx);
        self.update_events();
        self.palette().apply(ctx);
        if let Some(tray) = &self.tray {
            for action in tray.actions() {
                match action {
                    TrayAction::Show => self.show(ctx),
                    TrayAction::Settings => {
                        self.show(ctx);
                        self.controls = None;
                        self.settings = true;
                    }
                    TrayAction::Compress => self.compress(ctx),
                    TrayAction::Quit => self.quitting = true,
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.stopping {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if self.tray.is_some() {
                self.hide(ctx);
            } else {
                self.quitting = true;
            }
        }
        self.poll();
        self.hud(ctx);
        self.controls_window(ctx);
        self.settings_window(ctx);
        self.detail_window(ctx);
        self.hover_window(ctx);
        if self.smoke
            && self.started.elapsed()
                > Duration::from_secs(if self.smoke_interaction { 30 } else { 12 })
        {
            self.quitting = true;
        }
        self.persist();
        if self.loaded && self.writable && !self.update_acknowledged {
            if let Err(error) = updater::acknowledge_from_args(&self.profile_dir) {
                self.update_status = format!("Update startup acknowledgement failed: {error}");
            }
            self.update_acknowledged = true;
        }
        let interval = self.mode.interval(
            number(&self.profile["resources"], "samplingMs").unwrap_or(1000.) as u64,
            self.hidden,
        );
        ctx.request_repaint_after(Duration::from_millis(interval));
        if self.busy || self.save_pending || self.quitting || self.update_busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0., 0., 0., 0.]
    }
}
