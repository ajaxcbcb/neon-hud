mod bridge;
mod codex;

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    sync::Mutex,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{Components, CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}
fn atomic_json(path: &std::path::Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("Local data exceeds 1 MB".into());
    }
    let parent = path.parent().ok_or("Invalid app data path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let src: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let dst: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // Replaces an existing file on Windows without a remove-then-write gap.
        if unsafe {
            MoveFileExW(
                src.as_ptr(),
                dst.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(temp, path).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    version: u8,
    completed: bool,
    step: u8,
    theme: String,
    #[serde(default = "default_motion")]
    motion: String,
    size: String,
    corner: String,
    monitor: usize,
    always_on_top: bool,
    opacity: f64,
    text_scale: f64,
    reduced_motion: bool,
    launch_at_login: bool,
    notifications: bool,
    #[serde(rename = "interface")]
    interface_name: String,
    warning: f64,
    critical: f64,
    metrics: Metrics,
    codex_enabled: bool,
}
fn default_motion() -> String {
    "chaotic".into()
}
#[derive(Clone, Serialize, Deserialize)]
struct Metrics {
    cpu: bool,
    ram: bool,
    network: bool,
    ai: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            completed: false,
            step: 1,
            theme: "circuit".into(),
            motion: default_motion(),
            size: "compact".into(),
            corner: "bottom-right".into(),
            monitor: 0,
            always_on_top: true,
            opacity: 1.0,
            text_scale: 1.0,
            reduced_motion: false,
            launch_at_login: false,
            notifications: false,
            interface_name: "auto".into(),
            warning: 20.0,
            critical: 10.0,
            metrics: Metrics {
                cpu: true,
                ram: true,
                network: true,
                ai: true,
            },
            codex_enabled: false,
        }
    }
}
#[tauri::command]
fn load_settings(app: AppHandle) -> Settings {
    data_dir(&app)
        .ok()
        .and_then(|p| fs::read(p.join("settings.json")).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let mut settings = settings;
    settings.version = 1;
    settings.step = settings.step.clamp(1, 3);
    settings.opacity = if settings.opacity.is_finite() {
        settings.opacity.clamp(0.85, 1.0)
    } else {
        1.0
    };
    settings.text_scale = if settings.text_scale.is_finite() {
        settings.text_scale.clamp(0.9, 1.15)
    } else {
        1.0
    };
    settings.warning = if settings.warning.is_finite() {
        settings.warning.clamp(1.0, 100.0)
    } else {
        20.0
    };
    settings.critical = if settings.critical.is_finite() {
        settings.critical.clamp(0.0, settings.warning)
    } else {
        10.0
    };
    if !["circuit", "cyberpunk", "aurora"].contains(&settings.theme.as_str()) {
        settings.theme = "circuit".into();
    }
    if !["chaotic", "playful", "quiet"].contains(&settings.motion.as_str()) {
        settings.motion = default_motion();
    }
    if !["compact", "expanded"].contains(&settings.size.as_str()) {
        settings.size = "compact".into();
    }
    atomic_json(&data_dir(&app)?.join("settings.json"), &settings)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Core {
    usage: f32,
    frequency_mhz: u64,
}
#[derive(Serialize)]
struct Memory {
    used: u64,
    total: u64,
    available: u64,
}
#[derive(Serialize)]
struct Network {
    name: String,
    down: f64,
    up: f64,
    received: u64,
    transmitted: u64,
}
#[derive(Serialize)]
struct Temperature {
    label: String,
    celsius: f32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemSnapshot {
    sampled_at: f64,
    cpu: Option<f32>,
    cores: Vec<Core>,
    memory: Memory,
    networks: Vec<Network>,
    default_interface: Option<String>,
    temperatures: Vec<Temperature>,
}
struct SystemState {
    system: System,
    networks: Networks,
    components: Components,
    last: Option<Instant>,
    totals: HashMap<String, (u64, u64)>,
}
impl SystemState {
    fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything()),
            ),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last: None,
            totals: HashMap::new(),
        }
    }
    fn snapshot(&mut self) -> SystemSnapshot {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.components.refresh(true);
        let elapsed = self
            .last
            .map(|last| last.elapsed().as_secs_f64())
            .filter(|v| *v > 0.0);
        self.last = Some(Instant::now());
        let mut networks = Vec::new();
        for (name, data) in &self.networks {
            let received = data.total_received();
            let transmitted = data.total_transmitted();
            let prior = self.totals.insert(name.clone(), (received, transmitted));
            let (down, up) = match (elapsed, prior) {
                (Some(secs), Some((r, t))) => (
                    received.saturating_sub(r) as f64 / secs,
                    transmitted.saturating_sub(t) as f64 / secs,
                ),
                _ => (0.0, 0.0),
            };
            networks.push(Network {
                name: name.clone(),
                down,
                up,
                received,
                transmitted,
            });
        }
        let default_interface = default_net::get_default_interface()
            .ok()
            .map(|i| i.name)
            .filter(|name| networks.iter().any(|n| &n.name == name));
        SystemSnapshot {
            sampled_at: now(),
            cpu: elapsed.map(|_| self.system.global_cpu_usage()),
            cores: self
                .system
                .cpus()
                .iter()
                .map(|c| Core {
                    usage: c.cpu_usage(),
                    frequency_mhz: c.frequency(),
                })
                .collect(),
            memory: Memory {
                used: self.system.used_memory(),
                total: self.system.total_memory(),
                available: self.system.available_memory(),
            },
            networks,
            default_interface,
            temperatures: self
                .components
                .iter()
                .filter_map(|c| {
                    c.temperature().map(|v| Temperature {
                        label: c.label().into(),
                        celsius: v,
                    })
                })
                .collect(),
        }
    }
}
#[tauri::command]
fn system_snapshot(state: tauri::State<'_, Mutex<SystemState>>) -> Result<SystemSnapshot, String> {
    Ok(state.lock().map_err(|e| e.to_string())?.snapshot())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    label: String,
    minutes: u64,
    used_percent: f64,
    resets_at: Option<f64>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    surface: String,
    source: String,
    state: String,
    message: String,
    fetched_at: Option<f64>,
    windows: Vec<UsageWindow>,
}
impl Usage {
    fn unavailable(surface: &str, source: &str, message: &str) -> Self {
        Self {
            surface: surface.into(),
            source: source.into(),
            state: "unavailable".into(),
            message: message.into(),
            fetched_at: None,
            windows: vec![],
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attention {
    id: String,
    surface: String,
    reason: String,
    occurred_at: f64,
    session_id: String,
}
#[derive(Serialize)]
struct ProviderSnapshot {
    usages: Vec<Usage>,
    attention: Vec<Attention>,
}
#[tauri::command]
fn provider_snapshot(
    app: AppHandle,
    codex: tauri::State<'_, Mutex<codex::CodexState>>,
) -> ProviderSnapshot {
    let mut usages = vec![Usage::unavailable(
        "chatgpt",
        "No supported API",
        "ChatGPT chat allowance is unavailable through a supported local source.",
    )];
    usages.push(
        codex.lock().map(|mut c| c.usage()).unwrap_or_else(|_| {
            Usage::unavailable("codex", "Codex app-server", "Reader unavailable")
        }),
    );
    let (claude, attention) = bridge::read_provider(&app);
    usages.push(claude.clone());
    usages.push(Usage {
        surface: "claude-code".into(),
        message: "Shares the Claude account allowance; no separate total is added.".into(),
        ..claude
    });
    ProviderSnapshot { usages, attention }
}
#[tauri::command]
fn connect_codex(
    app: AppHandle,
    state: tauri::State<'_, Mutex<codex::CodexState>>,
    login: Option<bool>,
) -> Result<String, String> {
    state
        .lock()
        .map_err(|e| e.to_string())?
        .connect(&app, login.unwrap_or(true))
}
#[tauri::command]
fn disconnect_codex(state: tauri::State<'_, Mutex<codex::CodexState>>) -> Result<(), String> {
    state.lock().map_err(|e| e.to_string())?.disconnect();
    Ok(())
}
#[tauri::command]
fn install_claude_bridge(app: AppHandle) -> Result<String, String> {
    bridge::install(&app)
}
#[tauri::command]
fn remove_claude_bridge(app: AppHandle) -> Result<String, String> {
    bridge::remove(&app)
}
#[tauri::command]
fn set_preferences(
    app: AppHandle,
    launch_at_login: bool,
    notifications: bool,
) -> Result<(), String> {
    if launch_at_login {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    let mut settings = load_settings(app.clone());
    settings.launch_at_login = launch_at_login;
    settings.notifications = notifications;
    save_settings(app, settings)
}
#[tauri::command]
fn notify_attention(app: AppHandle, surfaces: Vec<String>) -> Result<(), String> {
    if !load_settings(app.clone()).notifications {
        return Ok(());
    }
    let valid: Vec<_> = surfaces
        .into_iter()
        .filter(|s| ["codex", "claude", "claude-code"].contains(&s.as_str()))
        .collect();
    if valid.is_empty() {
        return Ok(());
    }
    app.notification()
        .builder()
        .title("Neon HUD")
        .body("A connected session needs your attention")
        .show()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn dismiss_attention(app: AppHandle, id: String) -> Result<(), String> {
    bridge::dismiss(&app, &id)
}
#[tauri::command]
fn open_link(app: AppHandle, kind: String) -> Result<(), String> {
    let url = match kind.as_str() {
        "chatgpt" => "https://chatgpt.com",
        "codex" => "https://chatgpt.com/codex/settings/usage",
        "claude" => "https://claude.ai/settings/usage",
        "claude-code" => "https://code.claude.com/docs/en/setup",
        "codex-install" => "https://developers.openai.com/codex/cli",
        _ => return Err("Unknown link".into()),
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

pub fn run() {
    if bridge::run_cli_if_requested() {
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .manage(Mutex::new(SystemState::new()))
        .manage(Mutex::new(codex::CodexState::default()))
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show HUD", true, None::<&str>)?;
            let config = MenuItem::with_id(app, "configure", "Configure", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &config, &quit])?;
            TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .ok_or("App icon unavailable")?
                        .clone(),
                )
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        if let Some(state) = app.try_state::<Mutex<codex::CodexState>>() {
                            if let Ok(mut codex) = state.lock() {
                                codex.disconnect();
                            }
                        }
                        app.exit(0);
                    }
                    "show" | "configure" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                            if event.id().as_ref() == "configure" {
                                let _ = win.emit("open-settings", ());
                            }
                        }
                    }
                    _ => {}
                })
                .build(app)?;
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            system_snapshot,
            provider_snapshot,
            set_preferences,
            notify_attention,
            connect_codex,
            disconnect_codex,
            install_claude_bridge,
            remove_claude_bridge,
            open_link,
            dismiss_attention
        ])
        .run(tauri::generate_context!())
        .expect("Neon HUD failed to start");
}
