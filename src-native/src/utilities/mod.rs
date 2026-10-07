//! Native utilities share one bounded worker. Paint reads only its latest snapshot.
use std::{
    path::PathBuf,
    sync::{atomic::{AtomicBool, AtomicU8, Ordering}, mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
mod calendar;
#[cfg(target_os = "windows")]
use windows::Platform;
#[cfg(target_os = "macos")]
use macos::Platform;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Availability {
    Ready,
    #[default]
    Disconnected,
    Unavailable(String),
    Denied(String),
    Error(String),
}
impl Availability {
    pub fn label(&self) -> &str {
        match self {
            Self::Ready => "Connected",
            Self::Disconnected => "Not connected",
            Self::Unavailable(s) | Self::Denied(s) | Self::Error(s) => s,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Media {
    pub status: Availability,
    pub title: String,
    pub artist: String,
    pub source: String,
    pub artwork: Option<Arc<CameraFrame>>,
    pub playing: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub can_seek: bool,
    pub can_toggle: bool,
    pub can_next: bool,
    pub can_previous: bool,
}
#[derive(Clone, Debug)]
pub enum MediaCommand { Toggle, Next, Previous, Seek(f64) }

#[derive(Clone, Debug, Default)]
pub struct Power {
    pub status: Availability,
    pub charge_percent: Option<u8>,
    pub charging: Option<bool>,
    pub on_ac: Option<bool>,
    pub remaining_seconds: Option<u64>,
}
#[derive(Clone, Debug, Default)]
pub struct CalendarEvent {
    pub title: String,
    pub starts_at_ms: i64,
    pub ends_at_ms: i64,
    pub all_day: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Calendar {
    pub status: Availability,
    pub source: String,
    pub events: Vec<CalendarEvent>,
}
#[derive(Clone, Debug)]
pub struct CameraFrame {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}
impl CameraFrame {
    pub fn valid(&self) -> bool {
        self.width > 0 && self.height > 0 && self.width <= 1920 && self.height <= 1080
            && self.width.checked_mul(self.height).and_then(|n| n.checked_mul(4)) == Some(self.rgba.len())
    }
}
#[derive(Clone, Debug, Default)]
pub struct Mirror {
    pub status: Availability,
    pub frame: Option<Arc<CameraFrame>>,
}
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub media: Media,
    pub power: Power,
    pub calendar: Calendar,
    pub mirror: Mirror,
    pub revision: u64,
    pub sampled_at_ms: i64,
    pub message: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub media_enabled: bool,
    pub calendar_path: Option<PathBuf>,
    pub native_calendar: bool,
    pub note_bold: bool,
    pub note_italic: bool,
    pub note_underline: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self { media_enabled: !cfg!(target_os = "macos"), calendar_path: None,
            native_calendar: false, note_bold: false, note_italic: false, note_underline: false }
    }
}

enum Command {
    Media(MediaCommand),
    Configure(Preferences),
    ConnectCalendar,
    Stop,
}
pub struct Service {
    tx: mpsc::SyncSender<Command>,
    activity: Arc<AtomicU8>,
    latest: Arc<Mutex<Snapshot>>,
    cancel: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}
impl Service {
    pub fn new(ctx: eframe::egui::Context) -> Self {
        let (tx, rx) = mpsc::sync_channel(8);
        let activity = Arc::new(AtomicU8::new(0));
        let latest = Arc::new(Mutex::new(Snapshot::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        let stopped = Arc::new(AtomicBool::new(false));
        let cancelled = cancel.clone();
        let stop_ack = stopped.clone();
        let active = activity.clone();
        let published = latest.clone();
        let join = thread::spawn(move || {
            let mut platform = Platform::new();
            let mut state = Snapshot::default();
            let mut preferences = Preferences::default();
            let mut poll = Instant::now() - Duration::from_secs(60);
            let mut calendar_poll = poll;
            let mut mirror_requested = false;
            let mut last_activity = 0;
            loop {
                if cancelled.load(Ordering::Acquire) { break; }
                let command = rx.recv_timeout(Duration::from_millis(if mirror_requested { 100 } else { 250 }));
                let mut changed = false;
                match command {
                    Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Ok(Command::Configure(p)) => {
                        if preferences != p {
                            calendar_poll = Instant::now() - Duration::from_secs(60);
                            poll = Instant::now() - Duration::from_secs(60);
                        }
                        preferences = p;
                    }
                    Ok(Command::ConnectCalendar) => {
                        state.calendar = platform.calendar(epoch_ms(), epoch_ms() + 7 * 86_400_000, true);
                        calendar_poll = Instant::now();
                        changed = true;
                    }
                    Ok(Command::Media(c)) => {
                        state.message = platform.media_command(c).err().unwrap_or_default();
                        poll = Instant::now() - Duration::from_secs(60);
                        changed = true;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                if cancelled.load(Ordering::Acquire) { break; }
                let current = active.load(Ordering::Relaxed);
                let visible = current & 1 != 0;
                let requested_mirror = visible && current & 2 != 0 && current & 4 == 0;
                changed |= current != last_activity;
                last_activity = current;
                // A denial is latched until the user turns the camera off and on.
                // Polling must never repeat a permission request.
                if requested_mirror != mirror_requested {
                    mirror_requested = requested_mirror;
                    state.mirror = platform.set_mirror(requested_mirror);
                    if !requested_mirror { state.mirror.frame = None; }
                    changed = true;
                }
                if mirror_requested {
                    if let Some(frame) = platform.next_frame() {
                        match frame {
                            Ok(frame) if frame.valid() => {
                                state.mirror.status = Availability::Ready;
                                state.mirror.frame = Some(Arc::new(frame));
                            }
                            Ok(_) => {
                                state.mirror.status = Availability::Error("Invalid camera frame".into());
                                state.mirror.frame = None;
                                platform.set_mirror(false);
                            }
                            Err(e) => {
                                state.mirror.status = if e == "Camera access denied" {
                                    Availability::Denied(e)
                                } else { Availability::Error(e) };
                                state.mirror.frame = None;
                                platform.set_mirror(false);
                            }
                        }
                        changed = true;
                    }
                }
                if visible && poll.elapsed() >= Duration::from_secs(if current & 4 != 0 { 10 } else { 2 }) {
                    state.media = if preferences.media_enabled { platform.refresh_media() } else { Media::default() };
                    state.power = platform.power();
                    poll = Instant::now();
                    changed = true;
                }
                if visible && calendar_poll.elapsed() >= Duration::from_secs(60) {
                    state.calendar = if let Some(path) = &preferences.calendar_path {
                        calendar::read(path, epoch_ms(), epoch_ms() + 7 * 86_400_000)
                    } else if preferences.native_calendar {
                        platform.calendar(epoch_ms(), epoch_ms() + 7 * 86_400_000, false)
                    } else { Calendar::default() };
                    calendar_poll = Instant::now();
                    changed = true;
                }
                if changed {
                    state.revision = state.revision.wrapping_add(1);
                    state.sampled_at_ms = epoch_ms();
                    if let Ok(mut target) = published.lock() { *target = state.clone(); }
                    ctx.request_repaint();
                }
            }
            platform.stop();
            stop_ack.store(true, Ordering::Release);
            ctx.request_repaint();
        });
        Self { tx, activity, latest, cancel, stopped, join: Some(join) }
    }
    pub fn activity(&self, visible: bool, mirror: bool, pressure: bool) {
        self.activity.store(u8::from(visible) | (u8::from(mirror) << 1) | (u8::from(pressure) << 2), Ordering::Relaxed);
    }
    pub fn snapshot(&self) -> Option<Snapshot> { self.latest.try_lock().ok().map(|s| s.clone()) }
    pub fn configure(&self, preferences: Preferences) -> bool { self.tx.try_send(Command::Configure(preferences)).is_ok() }
    pub fn connect_calendar(&self) -> bool { self.tx.try_send(Command::ConnectCalendar).is_ok() }
    pub fn media(&self, command: MediaCommand) -> bool { self.tx.try_send(Command::Media(command)).is_ok() }
    pub fn stop(&self) {
        self.activity(false, false, true);
        self.cancel.store(true, Ordering::Release);
        let _ = self.tx.try_send(Command::Stop);
    }
    pub fn ready_to_stop(&self) -> bool { self.stopped.load(Ordering::Acquire) }
}
impl Drop for Service {
    fn drop(&mut self) {
        // Application drop happens after the eframe paint/event loop has ended.
        self.stop();
        if let Some(join) = self.join.take() { let _ = join.join(); }
    }
}
fn epoch_ms() -> i64 { (crate::now() * 1000.) as i64 }

#[cfg(test)]
mod tests {
    use super::CameraFrame;
    #[test]
    fn frame_validation_rejects_invalid_dimensions_and_buffers() {
        assert!(CameraFrame { width: 2, height: 2, rgba: vec![0; 16] }.valid());
        assert!(!CameraFrame { width: 0, height: 2, rgba: vec![] }.valid());
        assert!(!CameraFrame { width: 2, height: 2, rgba: vec![0; 15] }.valid());
        assert!(!CameraFrame { width: 1921, height: 1, rgba: vec![0; 1921 * 4] }.valid());
        assert!(!CameraFrame { width: usize::MAX, height: usize::MAX, rgba: vec![] }.valid());
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
struct Platform;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
impl Platform {
    fn new() -> Self { Self }
    fn refresh_media(&mut self) -> Media { Media { status: Availability::Unavailable("Media requires Windows or macOS".into()), ..Media::default() } }
    fn media_command(&mut self, _: MediaCommand) -> Result<(), String> { Err("Media unavailable".into()) }
    fn power(&mut self) -> Power { Power::default() }
    fn calendar(&mut self, _: i64, _: i64, _: bool) -> Calendar { Calendar::default() }
    fn set_mirror(&mut self, _: bool) -> Mirror { Mirror::default() }
    fn next_frame(&mut self) -> Option<Result<CameraFrame, String>> { None }
    fn stop(&mut self) {}
}
