//! Signed, opt-in updates for the separate native preview application.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use fs2::FileExt;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use ureq::ResponseExt;

const API: &str = "https://api.github.com/repos/ajaxcbcb/neon-hud/releases?per_page=30";
const MAX_MANIFEST: u64 = 64 * 1024;
const MAX_ARCHIVE: u64 = 64 * 1024 * 1024;
const MAX_UNPACKED: u64 = 192 * 1024 * 1024;
const MAX_FILES: usize = 1024;

#[derive(Clone, Debug)]
pub struct Offer {
    pub version: String,
    pub manifest: Vec<u8>,
    pub signature: Vec<u8>,
    pub platform: String,
}
#[derive(Clone, Debug)]
pub struct VerifiedStage {
    pub ticket: PathBuf,
    pub version: String,
}
#[derive(Clone, Debug)]
pub enum Status {
    Checking,
    Downloading { done: u64, total: u64 },
    Available(Offer),
    Current,
}
#[derive(Clone, Debug)]
pub enum Event {
    State(Status),
    Ready(VerifiedStage),
    Error(String),
}
pub enum Command {
    Check,
    Download(Offer),
}
pub struct Worker {
    pub tx: Sender<Command>,
    pub rx: Receiver<Event>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema: u32,
    channel: String,
    version: String,
    source_commit: String,
    platforms: HashMap<String, Artifact>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    url: String,
    size: u64,
    sha256: String,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}
#[derive(Serialize, Deserialize)]
struct Ticket {
    profile: PathBuf,
    target: PathBuf,
    old_sha256: String,
    archive: PathBuf,
    manifest: PathBuf,
    signature: PathBuf,
    platform: String,
    version: String,
}

impl Worker {
    pub fn start(profile_dir: PathBuf, ctx: eframe::egui::Context) -> Self {
        let (tx, commands) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = commands.recv() {
                let result = match command {
                    Command::Check => {
                        let _ = events.send(Event::State(Status::Checking));
                        ctx.request_repaint();
                        check().map(|s| Event::State(s))
                    }
                    Command::Download(offer) => {
                        download(&profile_dir, &offer, &events, Some(&ctx)).map(Event::Ready)
                    }
                };
                let _ = events.send(result.unwrap_or_else(Event::Error));
                ctx.request_repaint();
            }
        });
        Self { tx, rx }
    }
}

fn platform() -> Result<&'static str, String> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Ok("windows-x86_64")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Ok("macos-aarch64")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Ok("macos-x86_64")
    } else {
        Err("Native updater is unsupported on this platform".into())
    }
}
fn version(s: &str) -> Result<Version, String> {
    Version::parse(s.trim_start_matches('v')).map_err(|e| format!("Invalid update version: {e}"))
}
fn newer(s: &str) -> Result<bool, String> {
    let candidate = version(s)?;
    Ok(candidate > version(env!("CARGO_PKG_VERSION"))? && candidate >= version("0.2.0-alpha")?)
}
fn public_key() -> Result<PublicKey, String> {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../src-tauri/tauri.conf.json"))
            .map_err(|e| e.to_string())?;
    let outer = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .ok_or("Updater public key missing")?;
    let text = STANDARD.decode(outer).map_err(|e| e.to_string())?;
    let text = std::str::from_utf8(&text).map_err(|e| e.to_string())?;
    PublicKey::decode(text).map_err(|e| format!("Updater public key invalid: {e}"))
}
fn verify_manifest(bytes: &[u8], sig_bytes: &[u8]) -> Result<Manifest, String> {
    let manifest = verify_signature_and_parse(bytes, sig_bytes)?;
    validate_manifest(&manifest, true)?;
    Ok(manifest)
}
fn validate_manifest(m: &Manifest, require_newer: bool) -> Result<(), String> {
    if m.schema != 1
        || m.channel != "native-preview"
        || (require_newer && !newer(&m.version)?)
        || version(&m.version)? < version("0.2.0-alpha")?
        || m.source_commit.len() != 40
        || !m
            .source_commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Invalid native manifest schema, channel, version or source commit".into());
    }
    for (key, item) in &m.platforms {
        if !["windows-x86_64", "macos-aarch64", "macos-x86_64"].contains(&key.as_str())
            || item.size == 0
            || item.size > MAX_ARCHIVE
            || item.sha256.len() != 64
            || !item.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || !asset_url(&item.url)
        {
            return Err("Invalid native artifact metadata".into());
        }
    }
    if m.platforms.is_empty() {
        return Err("Native manifest contains no platforms".into());
    }
    Ok(())
}
/// CI verification accepts historical releases; the update path still rejects downgrades.
pub fn verify_manifest_bytes(bytes: &[u8], signature: &[u8]) -> Result<String, String> {
    let m = verify_signature_and_parse(bytes, signature)?;
    validate_manifest(&m, false)?;
    Ok(m.version)
}
fn verify_signature_and_parse(bytes: &[u8], sig_bytes: &[u8]) -> Result<Manifest, String> {
    if bytes.len() as u64 > MAX_MANIFEST || sig_bytes.len() > 8192 {
        return Err("Manifest or signature too large".into());
    }
    let outer = std::str::from_utf8(sig_bytes).map_err(|e| e.to_string())?;
    let decoded = STANDARD
        .decode(outer.trim())
        .map_err(|e| format!("Invalid signature encoding: {e}"))?;
    let sig_text = std::str::from_utf8(&decoded).map_err(|e| e.to_string())?;
    let sig =
        Signature::decode(sig_text).map_err(|e| format!("Invalid manifest signature: {e}"))?;
    public_key()?
        .verify(bytes, &sig, false)
        .map_err(|e| format!("Manifest signature verification failed: {e}"))?;
    serde_json::from_slice(bytes).map_err(|e| format!("Invalid native manifest: {e}"))
}
pub fn verify_archive_file(
    bytes: &[u8],
    signature: &[u8],
    platform: &str,
    archive: &Path,
) -> Result<(), String> {
    verify_manifest_bytes(bytes, signature)?;
    let m: Manifest = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let a = m
        .platforms
        .get(platform)
        .ok_or("Platform absent from manifest")?;
    if fs::metadata(archive).map_err(|e| e.to_string())?.len() != a.size
        || file_sha256(archive)? != a.sha256.to_ascii_lowercase()
    {
        return Err("Archive size or SHA-256 mismatch".into());
    }
    Ok(())
}
pub fn check_now() -> Result<Option<Offer>, String> {
    match check()? {
        Status::Available(offer) => Ok(Some(offer)),
        Status::Current => Ok(None),
        _ => unreachable!(),
    }
}
pub fn download_now(profile: &Path, offer: Offer) -> Result<VerifiedStage, String> {
    let (tx, _) = mpsc::channel();
    download(profile, &offer, &tx, None)
}
fn asset_url(url: &str) -> bool {
    let prefix = "https://github.com/ajaxcbcb/neon-hud/releases/download/";
    url.starts_with(prefix)
        && !url[prefix.len()..].is_empty()
        && !url.chars().any(|c| ['?', '#', '\\', '@'].contains(&c))
        && !url.contains("..")
        && !url.bytes().any(|b| b < 0x20)
}
fn http(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    http_progress(url, limit, |_| {})
}
fn http_progress(url: &str, limit: u64, mut progress: impl FnMut(u64)) -> Result<Vec<u8>, String> {
    if !(url == API || asset_url(url)) {
        return Err("Update URL outside trusted repository".into());
    }
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(90)))
        .https_only(true)
        .max_redirects(3)
        .save_redirect_history(true)
        .build();
    let agent = config.new_agent();
    let mut response = agent
        .get(url)
        .header("User-Agent", "neon-hud-native-updater")
        .call()
        .map_err(|e| format!("Update request failed: {e}"))?;
    let allowed = |uri: &ureq::http::Uri| {
        uri.scheme_str() == Some("https")
            && uri.port_u16().is_none_or(|port| port == 443)
            && matches!(
                uri.host(),
                Some(
                    "api.github.com"
                        | "github.com"
                        | "release-assets.githubusercontent.com"
                        | "objects.githubusercontent.com"
                )
            )
    };
    if !allowed(response.get_uri())
        || !response
            .get_redirect_history()
            .is_some_and(|history| history.iter().all(allowed))
    {
        return Err("Update redirected outside trusted HTTPS origins".into());
    }
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    let mut reader = response.body_mut().as_reader();
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| format!("Update read failed: {e}"))?;
        if n == 0 {
            break;
        }
        if bytes.len() as u64 + n as u64 > limit {
            return Err("Update response exceeds size limit".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
        progress(bytes.len() as u64);
    }
    Ok(bytes)
}
fn check() -> Result<Status, String> {
    let target = platform()?;
    let releases: Vec<Release> = serde_json::from_slice(&http(API, 512 * 1024)?)
        .map_err(|e| format!("Invalid release feed: {e}"))?;
    let mut candidates = releases
        .into_iter()
        .filter_map(|r| {
            let v = version(&r.tag_name).ok()?;
            if !r.prerelease || !newer(&r.tag_name).ok()? {
                return None;
            }
            Some((v, r))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, release) in candidates {
        let asset = |name: &str| {
            release
                .assets
                .iter()
                .find(|a| a.name == name && asset_url(&a.browser_download_url))
                .map(|a| a.browser_download_url.as_str())
        };
        let (Some(m_url), Some(s_url)) =
            (asset("native-update.json"), asset("native-update.json.sig"))
        else {
            continue;
        };
        let manifest = http(m_url, MAX_MANIFEST)?;
        let signature = http(s_url, 8192)?;
        let m = verify_manifest(&manifest, &signature)?;
        if version(&m.version)? != version(&release.tag_name)? {
            return Err("Manifest version differs from release tag".into());
        }
        if m.platforms.contains_key(target) {
            return Ok(Status::Available(Offer {
                version: m.version,
                manifest,
                signature,
                platform: target.into(),
            }));
        }
    }
    Ok(Status::Current)
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn file_sha256(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn stage_base(profile: &Path) -> Result<PathBuf, String> {
    let profile = profile
        .canonicalize()
        .map_err(|e| format!("Profile missing: {e}"))?;
    let base = profile.join("native-updates");
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    if base
        .symlink_metadata()
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Update directory is a link".into());
    }
    Ok(base)
}
fn download(
    profile: &Path,
    offer: &Offer,
    events: &Sender<Event>,
    repaint: Option<&eframe::egui::Context>,
) -> Result<VerifiedStage, String> {
    if offer.platform != platform()? {
        return Err("Update platform mismatch".into());
    }
    let m = verify_manifest(&offer.manifest, &offer.signature)?;
    if m.version != offer.version {
        return Err("Update offer changed".into());
    }
    let artifact = m
        .platforms
        .get(&offer.platform)
        .ok_or("Platform archive missing")?;
    let _ = events.send(Event::State(Status::Downloading {
        done: 0,
        total: artifact.size,
    }));
    if let Some(ctx) = repaint {
        ctx.request_repaint();
    }
    let archive = http_progress(&artifact.url, artifact.size, |done| {
        let _ = events.send(Event::State(Status::Downloading {
            done,
            total: artifact.size,
        }));
        if let Some(ctx) = repaint {
            ctx.request_repaint();
        }
    })?;
    if archive.len() as u64 != artifact.size
        || sha256(&archive) != artifact.sha256.to_ascii_lowercase()
    {
        return Err("Archive size or SHA-256 mismatch".into());
    }
    let _ = events.send(Event::State(Status::Downloading {
        done: artifact.size,
        total: artifact.size,
    }));
    let base = stage_base(profile)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let id = format!("{}-{}-{}", sha256(&offer.manifest), std::process::id(), now);
    let dir = base.join(id);
    fs::create_dir(&dir)
        .map_err(|e| format!("Update stage already exists or cannot be created: {e}"))?;
    let archive_path = dir.join("archive.zip");
    let manifest_path = dir.join("native-update.json");
    let signature_path = dir.join("native-update.json.sig");
    write_new(&archive_path, &archive)?;
    write_new(&manifest_path, &offer.manifest)?;
    write_new(&signature_path, &offer.signature)?;
    let target = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let ticket = Ticket {
        profile: profile.canonicalize().map_err(|e| e.to_string())?,
        old_sha256: file_sha256(&target)?,
        target,
        archive: archive_path,
        manifest: manifest_path,
        signature: signature_path,
        platform: offer.platform.clone(),
        version: offer.version.clone(),
    };
    let ticket_path = dir.join("ticket.json");
    write_new(
        &ticket_path,
        &serde_json::to_vec(&ticket).map_err(|e| e.to_string())?,
    )?;
    Ok(VerifiedStage {
        ticket: ticket_path,
        version: offer.version.clone(),
    })
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

/// Acquire for the entire GUI lifetime. The helper waits for this file lock to clear.
pub fn acquire_instance(profile: &Path) -> Result<File, String> {
    let path = stage_base(profile)?.join("instance.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock_exclusive()
        .map_err(|e| format!("Native instance already running: {e}"))?;
    Ok(file)
}

/// Called only after profile persistence and backend shutdown.
pub fn launch_helper(stage: &VerifiedStage) -> Result<(), String> {
    let ticket = read_ticket(&stage.ticket)?;
    verify_ticket(&stage.ticket, &ticket)?;
    let helper = stage
        .ticket
        .parent()
        .ok_or("Invalid stage")?
        .join(if cfg!(windows) {
            "update-helper.exe"
        } else {
            "update-helper"
        });
    let mut from = File::open(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut to = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&helper)
        .map_err(|e| format!("Helper already exists or cannot be created: {e}"))?;
    std::io::copy(&mut from, &mut to).map_err(|e| format!("Helper copy failed: {e}"))?;
    to.sync_all().map_err(|e| e.to_string())?;
    drop(to);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("Helper is not executable: {e}"))?;
    }
    let mut cmd = ProcessCommand::new(helper);
    cmd.arg("--apply-update")
        .arg(&stage.ticket)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.spawn()
        .map_err(|e| format!("Helper launch failed: {e}"))?;
    Ok(())
}
pub fn run_helper_cli() -> Option<Result<(), String>> {
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--apply-update")) {
        return None;
    }
    let Some(path) = args.next() else {
        return Some(Err("Missing update ticket".into()));
    };
    if args.next().is_some() {
        return Some(Err("Unexpected helper arguments".into()));
    }
    let path = Path::new(&path);
    Some(match apply(path) {
        Ok(()) => {
            clear_last_error();
            Ok(())
        }
        Err(error) => {
            save_last_error(&error);
            let error = match restart_original_if_safe(path) {
                Ok(true) => format!("{error}; previous app restarted"),
                Ok(false) => error,
                Err(restart) => format!("{error}; previous app restart failed: {restart}"),
            };
            save_last_error(&error);
            Err(error)
        }
    })
}
fn last_error_path() -> Result<PathBuf, String> {
    let profile = crate::desktop::profile_dir(false)?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    Ok(stage_base(&profile)?.join("last-update-error.txt"))
}
fn save_last_error(error: &str) {
    if let Ok(path) = last_error_path() {
        let bounded: String = error.chars().take(2048).collect();
        if let Ok(mut file) = File::create(path) {
            let _ = file.write_all(bounded.as_bytes());
            let _ = file.sync_all();
        }
    }
}
fn clear_last_error() {
    if let Ok(path) = last_error_path() {
        let _ = fs::remove_file(path);
    }
}
fn relaunch_marker(ticket: &Path) -> Result<PathBuf, String> {
    Ok(ticket
        .parent()
        .ok_or("Invalid update stage")?
        .join("previous-app-relaunched"))
}
fn spawn_previous(executable: &Path, ticket: &Path) -> Result<(), String> {
    let marker = relaunch_marker(ticket)?;
    write_new(&marker, b"1")?;
    if let Err(e) = ProcessCommand::new(executable).stdin(Stdio::null()).spawn() {
        let _ = fs::remove_file(&marker);
        return Err(format!("Previous app could not relaunch: {e}"));
    }
    Ok(())
}
fn restart_original_if_safe(path: &Path) -> Result<bool, String> {
    let Ok(t) = read_ticket(path) else {
        return Ok(false);
    };
    let own_profile = crate::desktop::profile_dir(false)?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if t.profile.canonicalize().ok().as_deref() != Some(own_profile.as_path()) {
        return Ok(false);
    }
    if verify_ticket(path, &t).is_err() {
        return Ok(false);
    }
    let helper = path
        .parent()
        .ok_or("Invalid update stage")?
        .join(if cfg!(windows) {
            "update-helper.exe"
        } else {
            "update-helper"
        });
    if std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?
        != helper
    {
        return Ok(false);
    }
    if relaunch_marker(path)?.exists() {
        return Ok(false);
    }
    let lease = match acquire_instance(&own_profile) {
        Ok(file) => file,
        Err(_) => return Ok(false),
    };
    drop(lease);
    spawn_previous(&t.target, path)?;
    Ok(true)
}
fn read_ticket(path: &Path) -> Result<Ticket, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Invalid update ticket: {e}"))
}
fn verify_ticket(path: &Path, t: &Ticket) -> Result<(), String> {
    let profile = t.profile.canonicalize().map_err(|e| e.to_string())?;
    let base = stage_base(&profile)?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let dir = path
        .parent()
        .ok_or("Invalid ticket path")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if dir.parent() != Some(base.as_path())
        || path.canonicalize().map_err(|e| e.to_string())? != dir.join("ticket.json")
        || t.archive != dir.join("archive.zip")
        || t.manifest != dir.join("native-update.json")
        || t.signature != dir.join("native-update.json.sig")
    {
        return Err("Ticket outside native staging area".into());
    }
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if exe != t.target
        && exe
            != dir.join(if cfg!(windows) {
                "update-helper.exe"
            } else {
                "update-helper"
            })
    {
        return Err("Helper binary is not the installed application".into());
    }
    if t.platform == "windows-x86_64"
        && t.target.file_name().and_then(|x| x.to_str()) != Some("neon-hud-native.exe")
    {
        return Err("Unexpected installed executable".into());
    }
    if exe != t.target && file_sha256(&exe)? != t.old_sha256 {
        return Err("Helper binary differs from installed application".into());
    }
    if t.platform != platform()? {
        return Err("Ticket platform mismatch".into());
    }
    let manifest_bytes = fs::read(&t.manifest).map_err(|e| e.to_string())?;
    let m = verify_manifest(
        &manifest_bytes,
        &fs::read(&t.signature).map_err(|e| e.to_string())?,
    )?;
    if t.version != m.version
        || !dir
            .file_name()
            .and_then(|x| x.to_str())
            .is_some_and(|name| name.starts_with(&format!("{}-", sha256(&manifest_bytes))))
    {
        return Err("Ticket manifest mismatch".into());
    }
    let a = m
        .platforms
        .get(&t.platform)
        .ok_or("Ticket platform absent")?;
    if fs::metadata(&t.archive).map_err(|e| e.to_string())?.len() != a.size
        || file_sha256(&t.archive)? != a.sha256.to_ascii_lowercase()
    {
        return Err("Staged archive mismatch".into());
    }
    if file_sha256(&t.target)? != t.old_sha256 {
        return Err("Installed executable changed since download".into());
    }
    Ok(())
}
fn wait_for_exit(profile: &Path) -> Result<File, String> {
    let path = stage_base(profile)?.join("instance.lock");
    let start = Instant::now();
    loop {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        if f.try_lock_exclusive().is_ok() {
            return Ok(f);
        }
        if start.elapsed() > Duration::from_secs(90) {
            return Err("Timed out waiting for native application to exit".into());
        }
        thread::sleep(Duration::from_millis(250));
    }
}
fn apply(path: &Path) -> Result<(), String> {
    let t = read_ticket(path)?;
    let own_profile = crate::desktop::profile_dir(false)?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if t.profile.canonicalize().map_err(|e| e.to_string())? != own_profile {
        return Err("Helper ticket does not belong to this native profile".into());
    }
    let lock = wait_for_exit(&t.profile)?;
    verify_ticket(path, &t)?;
    let dir = path.parent().ok_or("Invalid update stage")?;
    let unpack = dir.join("unpacked");
    fs::create_dir(&unpack).map_err(|e| e.to_string())?;
    extract(&t.archive, &unpack, &t.platform)?;
    let (source, target) = install_paths(&unpack, &t.target, &t.platform)?;
    let replacement = unique_sibling(&target, "new")?;
    let backup = unique_sibling(&target, "previous")?;
    copy_replacement(&source, &replacement, &t.platform)?;
    fs::rename(&target, &backup).map_err(|e| format!("Unable to back up installed app: {e}"))?;
    if let Err(e) = fs::rename(&replacement, &target) {
        fs::rename(&backup, &target).map_err(|restore| {
            format!("Install failed: {e}; previous app restore failed: {restore}")
        })?;
        return Err(format!(
            "Unable to install native update; previous app restored: {e}"
        ));
    }
    let ack = dir.join("startup.ack");
    let _ = fs::remove_file(&ack);
    drop(lock);
    let mut child = match ProcessCommand::new(&t.target)
        .arg("--update-ack")
        .arg(path)
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            rollback_and_relaunch(&target, &backup, &t.target, path)?;
            return Err(format!(
                "Updated app could not launch; backup restored: {e}"
            ));
        }
    };
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(35) {
        if ack.is_file() {
            return Ok(());
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            rollback_and_relaunch(&target, &backup, &t.target, path)?;
            return Err(
                "Updated app exited before startup acknowledgement; backup restored".into(),
            );
        }
        thread::sleep(Duration::from_millis(250));
    }
    // The new process is still running. Never replace or kill its executable.
    Err("Updated app did not acknowledge startup; backup retained for manual recovery".into())
}
fn rollback_and_relaunch(
    target: &Path,
    backup: &Path,
    executable: &Path,
    ticket: &Path,
) -> Result<(), String> {
    if target.parent() != backup.parent() || !backup.exists() {
        return Err("Rollback backup is invalid".into());
    }
    let failed = unique_sibling(target, "failed")?;
    fs::rename(target, &failed)
        .map_err(|e| format!("Rollback could not retain failed app: {e}"))?;
    fs::rename(backup, target)
        .map_err(|e| format!("Rollback could not restore previous app: {e}"))?;
    spawn_previous(executable, ticket)
}
fn unique_sibling(target: &Path, label: &str) -> Result<PathBuf, String> {
    let parent = target.parent().ok_or("Installed app has no parent")?;
    let name = target
        .file_name()
        .ok_or("Installed app has no file name")?
        .to_string_lossy();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let sibling = parent.join(format!(
        ".{name}.native-{label}-{}-{now}",
        std::process::id()
    ));
    if sibling.exists() {
        return Err("Native update sibling already exists".into());
    }
    Ok(sibling)
}
fn copy_replacement(source: &Path, sibling: &Path, platform: &str) -> Result<(), String> {
    if platform == "windows-x86_64" {
        if !source.is_file()
            || source
                .symlink_metadata()
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err("Unsafe replacement executable".into());
        }
        let mut from = File::open(source).map_err(|e| e.to_string())?;
        let mut to = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(sibling)
            .map_err(|e| format!("Installed folder is not writable: {e}"))?;
        std::io::copy(&mut from, &mut to).map_err(|e| e.to_string())?;
        return to.sync_all().map_err(|e| e.to_string());
    }
    let mut count = 0usize;
    let mut total = 0u64;
    copy_tree(source, sibling, &mut count, &mut total)
}
fn copy_tree(source: &Path, dest: &Path, count: &mut usize, total: &mut u64) -> Result<(), String> {
    let metadata = source.symlink_metadata().map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err("Replacement contains a link".into());
    }
    *count += 1;
    if *count > MAX_FILES {
        return Err("Replacement contains too many files".into());
    }
    if metadata.is_dir() {
        fs::create_dir(dest).map_err(|e| format!("Installed folder is not writable: {e}"))?;
        for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            copy_tree(&entry.path(), &dest.join(entry.file_name()), count, total)?;
        }
    } else if metadata.is_file() {
        *total = total
            .checked_add(metadata.len())
            .filter(|n| *n <= MAX_UNPACKED)
            .ok_or("Replacement exceeds size limit")?;
        let mut from = File::open(source).map_err(|e| e.to_string())?;
        let mut to = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)
            .map_err(|e| format!("Installed folder is not writable: {e}"))?;
        let copied = std::io::copy(&mut (&mut from).take(metadata.len() + 1), &mut to)
            .map_err(|e| e.to_string())?;
        if copied != metadata.len() {
            return Err("Replacement changed during copy".into());
        }
        to.sync_all().map_err(|e| e.to_string())?;
    } else {
        return Err("Replacement contains a special file".into());
    }
    fs::set_permissions(dest, metadata.permissions()).map_err(|e| e.to_string())?;
    Ok(())
}
/// Call after the profile Loaded event and first successful frame. No-op for ordinary starts.
pub fn acknowledge_from_args(profile: &Path) -> Result<(), String> {
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--update-ack")) {
        return Ok(());
    }
    let path = PathBuf::from(args.next().ok_or("Missing update acknowledgement ticket")?);
    let t = read_ticket(&path)?;
    if t.profile.canonicalize().map_err(|e| e.to_string())?
        != profile.canonicalize().map_err(|e| e.to_string())?
    {
        return Err("Acknowledgement profile mismatch".into());
    }
    let base = stage_base(profile)?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let dir = path
        .parent()
        .ok_or("Invalid acknowledgement path")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if dir.parent() != Some(base.as_path())
        || path.canonicalize().map_err(|e| e.to_string())? != dir.join("ticket.json")
        || std::env::current_exe()
            .map_err(|e| e.to_string())?
            .canonicalize()
            .map_err(|e| e.to_string())?
            != t.target
    {
        return Err("Invalid acknowledgement target".into());
    }
    File::create(dir.join("startup.ack")).map_err(|e| e.to_string())?;
    Ok(())
}
fn install_paths(
    unpack: &Path,
    current: &Path,
    platform: &str,
) -> Result<(PathBuf, PathBuf), String> {
    if platform == "windows-x86_64" {
        let source = unpack
            .join("neon-hud-native-windows-x64-unsigned-preview")
            .join("neon-hud-native.exe");
        if !source.is_file() {
            return Err("Windows archive lacks native executable".into());
        }
        return Ok((source, current.to_owned()));
    }
    let source = unpack.join("Neon HUD Native.app");
    let binary = source.join("Contents/MacOS/neon-hud-native");
    if !binary.is_file() {
        return Err("Mac archive lacks native app executable".into());
    }
    let target = current
        .ancestors()
        .find(|p| p.extension().is_some_and(|x| x == "app"))
        .ok_or("Running executable is not inside a Mac app bundle")?
        .to_owned();
    if !target.is_dir() {
        return Err("Mac app bundle is unavailable".into());
    }
    Ok((source, target))
}
fn extract(archive: &Path, dest: &Path, platform: &str) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if zip.len() > MAX_FILES {
        return Err("Archive contains too many entries".into());
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        let path = Path::new(&name);
        if name.starts_with('/')
            || name.contains(':')
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Unsafe archive entry".into());
        }
        let normalized = name.trim_end_matches('/').to_ascii_lowercase();
        if !names.insert(normalized) {
            return Err("Duplicate archive entry".into());
        }
        if entry.size() > MAX_UNPACKED
            || total
                .checked_add(entry.size())
                .filter(|v| *v <= MAX_UNPACKED)
                .is_none()
        {
            return Err("Archive expansion exceeds limit".into());
        }
        total += entry.size();
        let root = if platform == "windows-x86_64" {
            "neon-hud-native-windows-x64-unsigned-preview"
        } else {
            "Neon HUD Native.app"
        };
        if path
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            != Some(root)
        {
            return Err("Unexpected archive root".into());
        }
        let output = dest.join(path);
        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = File::create(&output).map_err(|e| e.to_string())?;
        let expected = entry.size();
        let copied = std::io::copy(&mut (&mut entry).take(expected + 1), &mut f)
            .map_err(|e| e.to_string())?;
        if copied != expected {
            return Err("Archive entry size mismatch".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if entry.unix_mode().is_some_and(|m| m & 0o111 != 0) {
                fs::set_permissions(&output, fs::Permissions::from_mode(0o755))
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    #[cfg(unix)]
    if platform.starts_with("macos-") {
        use std::os::unix::fs::PermissionsExt;
        let binary = dest.join("Neon HUD Native.app/Contents/MacOS/neon-hud-native");
        if binary.is_file() {
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_downgrade_and_external_asset() {
        assert!(!newer("0.1.9").unwrap());
        assert!(!asset_url("https://example.com/evil.zip"));
        assert!(!asset_url(
            "https://github.com/ajaxcbcb/neon-hud/releases/download/../evil"
        ));
    }
    #[test]
    fn refuses_unsigned_manifest() {
        assert!(verify_manifest(br#"{"schema":1}"#, b"invalid").is_err());
    }
    #[test]
    fn rejects_bad_manifest_metadata() {
        let mut platforms = HashMap::new();
        platforms.insert(
            "windows-x86_64".into(),
            Artifact {
                url: "https://evil.test/a.zip".into(),
                size: 10,
                sha256: "a".repeat(64),
            },
        );
        let m = Manifest {
            schema: 1,
            channel: "native-preview".into(),
            version: "0.2.0-alpha.99".into(),
            source_commit: "a".repeat(40),
            platforms,
        };
        assert!(validate_manifest(&m, true).is_err());
    }
    #[test]
    fn accepts_well_formed_native_manifest_metadata() {
        let mut platforms = HashMap::new();
        platforms.insert("windows-x86_64".into(), Artifact { url: "https://github.com/ajaxcbcb/neon-hud/releases/download/v0.2.0-alpha.4/neon-hud-native-windows-x64-unsigned-preview.zip".into(), size: 10, sha256: "a".repeat(64) });
        let m = Manifest {
            schema: 1,
            channel: "native-preview".into(),
            version: "0.2.0-alpha.4".into(),
            source_commit: "a".repeat(40),
            platforms,
        };
        assert!(validate_manifest(&m, false).is_ok());
    }
    #[test]
    fn rejects_traversal_zip() {
        let dir = std::env::temp_dir().join(format!("neon-updater-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.zip");
        let mut writer = zip::ZipWriter::new(File::create(&path).unwrap());
        writer
            .start_file("../escape", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"oops").unwrap();
        writer.finish().unwrap();
        assert!(extract(&path, &dir.join("out"), "windows-x86_64").is_err());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn extracts_expected_windows_entry() {
        let dir =
            std::env::temp_dir().join(format!("neon-updater-valid-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("valid.zip");
        let mut writer = zip::ZipWriter::new(File::create(&path).unwrap());
        writer
            .start_file(
                "neon-hud-native-windows-x64-unsigned-preview/neon-hud-native.exe",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"fixture executable").unwrap();
        writer.finish().unwrap();
        let output = dir.join("out");
        extract(&path, &output, "windows-x86_64").unwrap();
        let (binary, _) = install_paths(
            &output,
            Path::new("C:/neon-hud-native.exe"),
            "windows-x86_64",
        )
        .unwrap();
        assert_eq!(fs::read(binary).unwrap(), b"fixture executable");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn rejects_case_colliding_zip_entries() {
        let dir = std::env::temp_dir().join(format!(
            "neon-updater-duplicate-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("duplicate.zip");
        let mut writer = zip::ZipWriter::new(File::create(&path).unwrap());
        for name in [
            "neon-hud-native-windows-x64-unsigned-preview/a.txt",
            "neon-hud-native-windows-x64-unsigned-preview/A.txt",
        ] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap();
        assert!(extract(&path, &dir.join("out"), "windows-x86_64").is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
