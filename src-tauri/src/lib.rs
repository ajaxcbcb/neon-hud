mod bridge;
mod codex;
mod gpu;

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    sync::atomic::{AtomicU64, Ordering},
    sync::mpsc::{self, Sender, SyncSender},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System,
};
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
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("Local data exceeds 1 MB".into());
    }
    let parent = path.parent().ok_or("Invalid app data path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp_name = path
        .file_name()
        .ok_or("Invalid app data file")?
        .to_os_string();
    temp_name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let temp = path.with_file_name(temp_name);
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    let mut result = Err("Could not replace local data".to_string());
    for _ in 0..5 {
        match replace_file(&temp, path) {
            Ok(()) => {
                result = Ok(());
                break;
            }
            Err(error) => {
                result = Err(error.to_string());
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
fn replace_file(temp: &std::path::Path, path: &std::path::Path) -> std::io::Result<()> {
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
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(temp, path)
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
    #[serde(default)]
    storage_drive_ids: Vec<String>,
    #[serde(default = "default_gpu_id")]
    gpu_id: String,
    #[serde(default)]
    performance: Performance,
    #[serde(default)]
    resources: Resources,
    codex_enabled: bool,
    #[serde(default = "default_true")]
    auto_updates: bool,
}
fn default_motion() -> String {
    "chaotic".into()
}
fn default_gpu_id() -> String {
    "auto".into()
}
fn default_sampling_ms() -> u64 {
    250
}
fn default_gpu_threshold() -> f64 {
    90.0
}
#[derive(Clone, Serialize, Deserialize)]
struct Metrics {
    cpu: bool,
    #[serde(default = "default_true")]
    gpu: bool,
    ram: bool,
    network: bool,
    ai: bool,
    #[serde(default = "default_true")]
    storage: bool,
}
fn default_true() -> bool {
    true
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Performance {
    cpu_percent: f64,
    #[serde(default = "default_gpu_threshold")]
    gpu_percent: f64,
    memory_percent: f64,
    temperature_celsius: f64,
    storage_percent: f64,
}
impl Default for Performance {
    fn default() -> Self {
        Self {
            cpu_percent: 90.0,
            gpu_percent: 90.0,
            memory_percent: 90.0,
            temperature_celsius: 85.0,
            storage_percent: 90.0,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Resources {
    #[serde(default = "default_true")]
    adaptive: bool,
    #[serde(default = "default_sampling_ms", rename = "samplingMs")]
    sampling_ms: u64,
}
impl Default for Resources {
    fn default() -> Self {
        Self {
            adaptive: true,
            sampling_ms: default_sampling_ms(),
        }
    }
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
            corner: "middle-right".into(),
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
                gpu: true,
                ram: true,
                network: true,
                ai: true,
                storage: true,
            },
            storage_drive_ids: vec![],
            gpu_id: default_gpu_id(),
            performance: Performance::default(),
            resources: Resources::default(),
            codex_enabled: false,
            auto_updates: true,
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
    normalize_settings(&mut settings);
    atomic_json(&data_dir(&app)?.join("settings.json"), &settings)
}
fn normalize_settings(settings: &mut Settings) {
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
    if ![
        "top-left",
        "top-right",
        "middle-left",
        "middle-right",
        "bottom-left",
        "bottom-right",
    ]
    .contains(&settings.corner.as_str())
    {
        settings.corner = "middle-right".into();
    }
    settings
        .storage_drive_ids
        .retain(|id| !id.trim().is_empty() && id.len() <= 1024);
    settings.storage_drive_ids.sort();
    settings.storage_drive_ids.dedup();
    settings.storage_drive_ids.truncate(64);
    if settings.gpu_id.trim().is_empty() || settings.gpu_id.len() > 1024 {
        settings.gpu_id = default_gpu_id();
    }
    if ![250, 500, 1000, 2000].contains(&settings.resources.sampling_ms) {
        settings.resources.sampling_ms = default_sampling_ms();
    }
    fn bounded(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
        if value.is_finite() {
            value.clamp(min, max)
        } else {
            fallback
        }
    }
    settings.performance.cpu_percent = bounded(settings.performance.cpu_percent, 50.0, 100.0, 90.0);
    settings.performance.gpu_percent = bounded(settings.performance.gpu_percent, 50.0, 100.0, 90.0);
    settings.performance.memory_percent =
        bounded(settings.performance.memory_percent, 50.0, 100.0, 90.0);
    settings.performance.storage_percent =
        bounded(settings.performance.storage_percent, 50.0, 100.0, 90.0);
    settings.performance.temperature_celsius =
        bounded(settings.performance.temperature_celsius, 40.0, 120.0, 85.0);
}
#[cfg(test)]
mod settings_tests {
    use super::*;
    #[test]
    fn corner_defaults_and_normalizes_to_supported_positions() {
        let mut settings = Settings::default();
        assert_eq!(settings.corner, "middle-right");
        settings.corner = "middle-left".into();
        normalize_settings(&mut settings);
        assert_eq!(settings.corner, "middle-left");
        settings.corner = "off-screen".into();
        normalize_settings(&mut settings);
        assert_eq!(settings.corner, "middle-right");
    }
    #[test]
    fn older_settings_get_storage_and_performance_defaults() {
        let mut old = serde_json::to_value(Settings::default()).unwrap();
        let root = old.as_object_mut().unwrap();
        root.remove("storageDriveIds");
        root.remove("performance");
        root.remove("resources");
        root.remove("gpuId");
        root.get_mut("metrics")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("gpu");
        root.get_mut("metrics")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("storage");
        let restored: Settings = serde_json::from_value(old).unwrap();
        assert!(restored.metrics.storage);
        assert!(restored.storage_drive_ids.is_empty());
        assert_eq!(restored.performance.temperature_celsius, 85.0);
        assert!(restored.resources.adaptive);
        assert!(restored.metrics.gpu);
        assert_eq!(restored.gpu_id, "auto");
        assert_eq!(restored.resources.sampling_ms, 250);
        assert_eq!(restored.performance.gpu_percent, 90.0);
    }
    #[test]
    fn performance_thresholds_and_drive_ids_are_bounded() {
        let mut settings = Settings::default();
        settings.resources.sampling_ms = 1;
        settings.gpu_id = " ".into();
        settings.performance.gpu_percent = f64::NAN;
        settings.performance.cpu_percent = 12.0;
        settings.performance.memory_percent = f64::NAN;
        settings.performance.storage_percent = 125.0;
        settings.performance.temperature_celsius = 200.0;
        settings.storage_drive_ids = vec!["C:\\".into(), "C:\\".into(), "".into()];
        normalize_settings(&mut settings);
        assert_eq!(settings.resources.sampling_ms, 250);
        assert_eq!(settings.gpu_id, "auto");
        assert_eq!(settings.performance.gpu_percent, 90.0);
        assert_eq!(settings.performance.cpu_percent, 50.0);
        assert_eq!(settings.performance.memory_percent, 90.0);
        assert_eq!(settings.performance.storage_percent, 100.0);
        assert_eq!(settings.performance.temperature_celsius, 120.0);
        assert_eq!(settings.storage_drive_ids, vec!["C:\\"]);
    }
    #[test]
    fn resource_mode_is_bounded_and_slows_expensive_refreshes() {
        assert_eq!(ResourceMode::parse(None), ResourceMode::Normal);
        assert_eq!(
            ResourceMode::parse(Some("unexpected")),
            ResourceMode::Normal
        );
        assert_eq!(
            ResourceMode::parse(Some("pressure")).disk_interval(),
            Duration::from_secs(90)
        );
        assert_eq!(
            ResourceMode::parse(Some("critical")).route_interval(),
            Duration::from_secs(120)
        );
        assert_eq!(
            ResourceMode::parse(Some("critical")).frequency_interval(),
            Duration::from_secs(60)
        );
    }
    #[test]
    fn additive_gpu_preferences_preserve_existing_resource_settings() {
        let mut old = serde_json::to_value(Settings::default()).unwrap();
        old["resources"]
            .as_object_mut()
            .unwrap()
            .remove("samplingMs");
        old["resources"]["adaptive"] = serde_json::json!(false);
        old["performance"]
            .as_object_mut()
            .unwrap()
            .remove("gpuPercent");
        let restored: Settings = serde_json::from_value(old).unwrap();
        assert!(!restored.resources.adaptive);
        assert_eq!(restored.resources.sampling_ms, 250);
        assert_eq!(restored.performance.gpu_percent, 90.0);
        assert_eq!(
            ResourceMode::Normal.system_interval(250),
            Duration::from_millis(250)
        );
        assert_eq!(
            ResourceMode::Normal.system_interval(1),
            Duration::from_millis(250)
        );
        assert_eq!(
            ResourceMode::Pressure.system_interval(250),
            Duration::from_secs(4)
        );
        assert_eq!(
            ResourceMode::Critical.system_interval(250),
            Duration::from_secs(8)
        );
        assert_eq!(ResourceMode::Normal.gpu_interval(), Duration::from_secs(1));
        assert_eq!(
            ResourceMode::Critical.sensor_interval(),
            Duration::from_secs(8)
        );
    }
    #[test]
    fn background_monitor_keeps_sampling_across_resource_modes() {
        let monitor = SystemMonitor::new();
        let first = monitor.snapshot(ResourceMode::Normal, 250).unwrap();
        assert!(first.memory.total > 0);
        assert!(first.cpu.is_none());
        let cached = monitor.snapshot(ResourceMode::Normal, 250).unwrap();
        assert_eq!(cached.sampled_at, first.sampled_at);
        thread::sleep(Duration::from_millis(275));
        let next = monitor.snapshot(ResourceMode::Normal, 250).unwrap();
        assert!(next.sampled_at >= first.sampled_at);
        assert!(next
            .cpu
            .is_some_and(|usage| usage.is_finite() && (0.0..=100.0).contains(&usage)));
        assert!(!next.cores.is_empty());
        let pressure_cache = monitor.snapshot(ResourceMode::Critical, 250).unwrap();
        assert_eq!(pressure_cache.sampled_at, next.sampled_at);
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Core {
    usage: f32,
    frequency_mhz: u64,
}
#[derive(Clone, Serialize)]
struct Memory {
    used: u64,
    total: u64,
    available: u64,
}
#[derive(Clone, Serialize)]
struct Network {
    name: String,
    down: f64,
    up: f64,
    received: u64,
    transmitted: u64,
}
#[derive(Clone, Serialize)]
struct Temperature {
    label: String,
    celsius: f32,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Drive {
    id: String,
    name: String,
    mount: String,
    total_bytes: u64,
    available_bytes: u64,
    used_bytes: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemSnapshot {
    sampled_at: f64,
    cpu: Option<f32>,
    cores: Vec<Core>,
    memory: Memory,
    networks: Vec<Network>,
    default_interface: Option<String>,
    temperatures: Vec<Temperature>,
    drives: Vec<Drive>,
    gpus: Vec<gpu::GpuSnapshot>,
}
struct SystemState {
    system: System,
    networks: Networks,
    components: Components,
    disks: Disks,
    drive_cache: Vec<Drive>,
    drives_last: Option<Instant>,
    route_last: Option<Instant>,
    route_cache: Option<String>,
    frequency_last: Option<Instant>,
    sensors_last: Option<Instant>,
    gpu: gpu::GpuMonitor,
    gpu_last: Option<Instant>,
    gpu_cache: Vec<gpu::GpuSnapshot>,
    snapshot_cache: Option<SystemSnapshot>,
    last: Option<Instant>,
    totals: HashMap<String, (u64, u64)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResourceMode {
    Normal,
    Pressure,
    Critical,
}
impl ResourceMode {
    fn parse(value: Option<&str>) -> Self {
        match value {
            Some("pressure") => Self::Pressure,
            Some("critical") => Self::Critical,
            _ => Self::Normal,
        }
    }
    fn disk_interval(self) -> Duration {
        Duration::from_secs(match self {
            Self::Normal => 30,
            Self::Pressure => 90,
            Self::Critical => 180,
        })
    }
    fn route_interval(self) -> Duration {
        Duration::from_secs(match self {
            Self::Normal => 30,
            Self::Pressure => 60,
            Self::Critical => 120,
        })
    }
    fn frequency_interval(self) -> Duration {
        Duration::from_secs(match self {
            Self::Normal => 15,
            Self::Pressure => 30,
            Self::Critical => 60,
        })
    }
    fn system_interval(self, sampling_ms: u64) -> Duration {
        Duration::from_millis(match self {
            Self::Normal => {
                if [250, 500, 1000, 2000].contains(&sampling_ms) {
                    sampling_ms
                } else {
                    250
                }
            }
            Self::Pressure => 4000,
            Self::Critical => 8000,
        })
    }
    fn gpu_interval(self) -> Duration {
        Duration::from_secs(match self {
            Self::Normal => 1,
            Self::Pressure => 4,
            Self::Critical => 8,
        })
    }
    fn sensor_interval(self) -> Duration {
        Duration::from_secs(match self {
            Self::Normal => 2,
            Self::Pressure => 4,
            Self::Critical => 8,
        })
    }
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
            disks: Disks::new_with_refreshed_list(),
            drive_cache: Vec::new(),
            drives_last: None,
            route_last: None,
            route_cache: None,
            frequency_last: None,
            sensors_last: None,
            gpu: gpu::GpuMonitor::new(),
            gpu_last: None,
            gpu_cache: Vec::new(),
            snapshot_cache: None,
            last: None,
            totals: HashMap::new(),
        }
    }
    fn snapshot(&mut self, mode: ResourceMode, sampling_ms: u64) -> SystemSnapshot {
        // Extra UI/manual requests return the same timestamped reading. They cannot
        // bypass the sampling budget or turn cached GPU data into fresh telemetry.
        if self
            .last
            .is_some_and(|last| last.elapsed() < mode.system_interval(sampling_ms))
        {
            if let Some(snapshot) = &self.snapshot_cache {
                return snapshot.clone();
            }
        }
        let frequency_due = self
            .frequency_last
            .is_none_or(|last| last.elapsed() >= mode.frequency_interval());
        let mut cpu_kind = CpuRefreshKind::nothing().with_cpu_usage();
        if frequency_due {
            cpu_kind = cpu_kind.with_frequency();
            self.frequency_last = Some(Instant::now());
        }
        self.system.refresh_cpu_specifics(cpu_kind);
        self.system.refresh_memory();
        self.networks.refresh(true);
        if self
            .sensors_last
            .is_none_or(|last| last.elapsed() >= mode.sensor_interval())
        {
            self.components.refresh(true);
            self.sensors_last = Some(Instant::now());
        }
        if self
            .gpu_last
            .is_none_or(|last| last.elapsed() >= mode.gpu_interval())
        {
            self.gpu_cache = self.gpu.refresh(now());
            self.gpu_last = Some(Instant::now());
        }
        if self
            .drives_last
            .is_none_or(|last| last.elapsed() >= mode.disk_interval())
        {
            self.disks.refresh(true);
            self.drive_cache = self
                .disks
                .iter()
                .filter_map(|disk| {
                    let total_bytes = disk.total_space();
                    if total_bytes == 0 {
                        return None;
                    }
                    let mount = disk.mount_point().to_string_lossy().to_string();
                    let available_bytes = disk.available_space().min(total_bytes);
                    Some(Drive {
                        id: mount.clone(),
                        name: disk.name().to_string_lossy().to_string(),
                        mount,
                        total_bytes,
                        available_bytes,
                        used_bytes: total_bytes.saturating_sub(available_bytes),
                    })
                })
                .collect();
            self.drives_last = Some(Instant::now());
        }
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
        let route_missing = self
            .route_cache
            .as_ref()
            .is_some_and(|name| !networks.iter().any(|n| &n.name == name));
        if route_missing
            || self
                .route_last
                .is_none_or(|last| last.elapsed() >= mode.route_interval())
        {
            self.route_cache = default_net::get_default_interface().ok().map(|i| i.name);
            self.route_last = Some(Instant::now());
        }
        let default_interface = self
            .route_cache
            .clone()
            .filter(|name| networks.iter().any(|n| &n.name == name));
        let snapshot = SystemSnapshot {
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
            drives: self.drive_cache.clone(),
            gpus: self.gpu_cache.clone(),
        };
        self.snapshot_cache = Some(snapshot.clone());
        snapshot
    }
}
#[derive(Clone)]
struct SystemMonitor {
    requests: SyncSender<(ResourceMode, u64, Sender<SystemSnapshot>)>,
}
impl SystemMonitor {
    fn new() -> Self {
        let (requests, receiver) =
            mpsc::sync_channel::<(ResourceMode, u64, Sender<SystemSnapshot>)>(1);
        std::thread::Builder::new()
            .name("neon-hud-monitor".into())
            .spawn(move || {
                // Windows temperature sensors initialize MTA COM. Keep their creation,
                // refresh and destruction on this thread, away from the STA UI thread.
                let mut state = None;
                while let Ok((mode, sampling_ms, response)) = receiver.recv() {
                    let snapshot = state
                        .get_or_insert_with(SystemState::new)
                        .snapshot(mode, sampling_ms);
                    let _ = response.send(snapshot);
                }
            })
            .expect("Could not start system monitor");
        Self { requests }
    }
    fn snapshot(&self, mode: ResourceMode, sampling_ms: u64) -> Result<SystemSnapshot, String> {
        let (response, receiver) = mpsc::channel();
        self.requests
            .try_send((mode, sampling_ms, response))
            .map_err(|error| format!("System monitor unavailable or busy: {error}"))?;
        receiver
            .recv_timeout(Duration::from_secs(15))
            .map_err(|error| format!("System monitor did not respond: {error}"))
    }
}
#[tauri::command]
async fn system_snapshot(
    app: AppHandle,
    resource_mode: Option<String>,
    sampling_ms: Option<u64>,
) -> Result<SystemSnapshot, String> {
    let monitor = app.state::<SystemMonitor>().inner().clone();
    let mode = ResourceMode::parse(resource_mode.as_deref());
    tauri::async_runtime::spawn_blocking(move || monitor.snapshot(mode, sampling_ms.unwrap_or(250)))
        .await
        .map_err(|error| format!("System monitor task failed: {error}"))?
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
#[serde(rename_all = "camelCase")]
struct ProviderSnapshot {
    usages: Vec<Usage>,
    attention: Vec<Attention>,
    claude_bridge_enabled: bool,
}
#[tauri::command]
async fn provider_snapshot(
    app: AppHandle,
    codex: tauri::State<'_, Arc<Mutex<codex::CodexState>>>,
) -> Result<ProviderSnapshot, String> {
    let codex = Arc::clone(codex.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let mut usages = vec![Usage::unavailable(
            "chatgpt",
            "No supported API",
            "ChatGPT chat allowance is unavailable through a supported local source.",
        )];
        usages.push(codex.lock().map(|mut c| c.usage()).unwrap_or_else(|_| {
            Usage::unavailable("codex", "Codex app-server", "Reader unavailable")
        }));
        let (claude, attention) = bridge::read_provider(&app);
        usages.push(claude.clone());
        usages.push(Usage {
            surface: "claude-code".into(),
            message: "Shares the Claude account allowance; no separate total is added.".into(),
            ..claude
        });
        ProviderSnapshot {
            usages,
            attention,
            claude_bridge_enabled: bridge::is_enabled(&app),
        }
    })
    .await
    .map_err(|e| e.to_string())
}
#[tauri::command]
async fn connect_codex(
    app: AppHandle,
    state: tauri::State<'_, Arc<Mutex<codex::CodexState>>>,
    login: Option<bool>,
) -> Result<String, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        state
            .lock()
            .map_err(|e| e.to_string())?
            .connect(&app, login.unwrap_or(true))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn disconnect_codex(
    state: tauri::State<'_, Arc<Mutex<codex::CodexState>>>,
) -> Result<(), String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        state.lock().map_err(|e| e.to_string())?.disconnect();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn install_claude_bridge(app: AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || bridge::install(&app))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn remove_claude_bridge(app: AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || bridge::remove(&app))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn restart_app(
    app: AppHandle,
    state: tauri::State<'_, Arc<Mutex<codex::CodexState>>>,
) -> Result<(), String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        state.lock().map_err(|e| e.to_string())?.disconnect();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    app.restart()
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(SystemMonitor::new())
        .manage(Arc::new(Mutex::new(codex::CodexState::default())))
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
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        if let Some(state) = app.try_state::<Arc<Mutex<codex::CodexState>>>() {
                            if let Ok(mut codex) = state.lock() {
                                codex.disconnect();
                            }
                        }
                        app.exit(0);
                    }
                    "show" | "configure" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.emit("hud-visible", true);
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
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let _ = window.emit("hud-visible", false);
            }
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
            restart_app,
            open_link,
            dismiss_attention
        ])
        .run(tauri::generate_context!())
        .expect("Neon HUD failed to start");
}
