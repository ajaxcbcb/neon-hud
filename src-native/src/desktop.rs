use auto_launch::AutoLaunch;
use eframe::egui;
use std::path::PathBuf;

pub fn app_icon() -> std::sync::Arc<egui::IconData> {
    static ICON: std::sync::OnceLock<std::sync::Arc<egui::IconData>> = std::sync::OnceLock::new();
    ICON.get_or_init(|| {
        eframe::icon_data::from_png_bytes(include_bytes!("../../src-tauri/icons/128x128.png"))
            .expect("Bundled Neon HUD icon must be a valid PNG")
            .into()
    })
    .clone()
}

pub fn coordinate_scale(native_pixels_per_point: f32) -> f64 {
    if cfg!(target_os = "macos") {
        1.
    } else {
        native_pixels_per_point as f64
    }
}

pub fn screens() -> Vec<crate::model::Screen> {
    display_info::DisplayInfo::all()
        .unwrap_or_default()
        .into_iter()
        .map(|d| crate::model::Screen {
            id: format!("display:{}", d.id),
            name: d.name,
            origin: [d.x as f64, d.y as f64],
            size: [d.width as f64, d.height as f64],
            // CoreGraphics reports points; Windows reports physical pixels.
            scale: if cfg!(target_os = "macos") {
                1.
            } else {
                (d.scale_factor as f64).max(1.)
            },
            primary: d.is_primary,
        })
        .collect()
}
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

pub fn profile_dir(smoke: bool) -> Result<PathBuf, String> {
    if smoke {
        let profile = std::env::temp_dir().join(format!("neon-native-smoke-{}", std::process::id()));
        std::fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
        return Ok(profile);
    }
    let base = directories::BaseDirs::new().ok_or("OS configuration directory unavailable")?;
    let original = base.config_dir().join("io.github.ajaxcbcb.neonhud");
    let preview = original.join("native-preview");
    std::fs::create_dir_all(&preview).map_err(|e| e.to_string())?;
    let target = preview.join("settings.json");
    if !target.exists() {
        let source = original.join("settings.json");
        if source.is_file() {
            // Copy bytes only. Invalid legacy data remains available for recovery.
            std::fs::copy(source, target).map_err(|e| format!("Could not import profile: {e}"))?;
        }
    }
    Ok(preview)
}

pub fn startup() -> Result<AutoLaunch, String> {
    let path = std::env::current_exe().map_err(|e| e.to_string())?;
    let path = path.to_str().ok_or("Executable path is not UTF-8")?;
    #[cfg(windows)]
    let app = AutoLaunch::new(
        "Neon HUD Native Preview",
        path,
        auto_launch::WindowsEnableMode::CurrentUser,
        &["--minimized"],
    );
    #[cfg(target_os = "macos")]
    let app = AutoLaunch::new(
        "Neon HUD Native Preview",
        path,
        auto_launch::MacOSLaunchMode::LaunchAgent,
        &["--minimized"],
        &[] as &[&str],
        "",
    );
    #[cfg(not(any(windows, target_os = "macos")))]
    let app = AutoLaunch::new("Neon HUD Native Preview", path, &["--minimized"]);
    Ok(app)
}

pub enum TrayAction {
    Show,
    Settings,
    Compress,
    Quit,
}
pub struct Tray {
    _icon: TrayIcon,
    show: MenuItem,
    settings: MenuItem,
    compress: MenuItem,
    quit: MenuItem,
}
impl Tray {
    pub fn new(ctx: egui::Context) -> Result<Self, String> {
        let menu = Menu::new();
        let show = MenuItem::new("Show HUD", true, None);
        let settings = MenuItem::new("Settings…", true, None);
        let compress = MenuItem::new("Compress / expand", true, None);
        let quit = MenuItem::new("Quit Neon HUD Native", true, None);
        menu.append_items(&[&show, &settings, &compress, &quit])
            .map_err(|e| e.to_string())?;
        let image =
            eframe::icon_data::from_png_bytes(include_bytes!("../../src-tauri/icons/32x32.png"))
                .map_err(|e| e.to_string())?;
        let icon =
            Icon::from_rgba(image.rgba, image.width, image.height).map_err(|e| e.to_string())?;
        let _icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_tooltip("Neon HUD · native preview")
            .with_menu_on_left_click(true)
            .build()
            .map_err(|e| e.to_string())?;
        // Event handlers notify the native event loop; events remain queued for the UI.
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let _ = TRAY_EVENTS
                .get_or_init(|| std::sync::Mutex::new(Vec::new()))
                .lock()
                .map(|mut q| q.push(event.id));
            menu_ctx.request_repaint();
        }));
        TrayIconEvent::set_event_handler(Some(move |_: TrayIconEvent| ctx.request_repaint()));
        Ok(Self {
            _icon,
            show,
            settings,
            compress,
            quit,
        })
    }
    pub fn actions(&self) -> Vec<TrayAction> {
        let events = TRAY_EVENTS
            .get_or_init(|| std::sync::Mutex::new(Vec::new()))
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        events
            .into_iter()
            .filter_map(|id| {
                if id == *self.show.id() {
                    Some(TrayAction::Show)
                } else if id == *self.settings.id() {
                    Some(TrayAction::Settings)
                } else if id == *self.compress.id() {
                    Some(TrayAction::Compress)
                } else if id == *self.quit.id() {
                    Some(TrayAction::Quit)
                } else {
                    None
                }
            })
            .collect()
    }
}
static TRAY_EVENTS: std::sync::OnceLock<std::sync::Mutex<Vec<tray_icon::menu::MenuId>>> =
    std::sync::OnceLock::new();

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_icons_decode_and_share_the_application_mark() {
        let app = super::app_icon();
        assert_eq!((app.width, app.height), (128, 128));
        assert!(std::sync::Arc::ptr_eq(&app, &super::app_icon()));
        let tray =
            eframe::icon_data::from_png_bytes(include_bytes!("../../src-tauri/icons/32x32.png"))
                .unwrap();
        assert_eq!((tray.width, tray.height), (32, 32));
        for icon in [&*app, &tray] {
            assert!(icon
                .rgba
                .chunks_exact(4)
                .any(|p| p[1] > 180 && p[2] > 180 && p[3] == 255));
        }
    }
}
