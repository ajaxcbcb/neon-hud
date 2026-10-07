use super::{atomic_json, now, Attention, Usage, UsageWindow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const MAX_INPUT: u64 = 1024 * 1024;
const OWN: &str = "--bridge";
const BRIDGE_EVENTS: [&str; 7] = [
    "Notification",
    "PermissionRequest",
    "PreToolUse",
    "UserPromptSubmit",
    "PostToolUse",
    "Stop",
    "SessionEnd",
];
fn home() -> Result<PathBuf, String> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .ok_or("Home folder unavailable".into())
}
fn settings_path() -> Result<PathBuf, String> {
    Ok(home()?.join(".claude/settings.json"))
}
fn cli_data_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("io.github.ajaxcbcb.neonhud"))
            .ok_or("App data unavailable".into());
    }
    #[cfg(target_os = "macos")]
    {
        return Ok(home()?.join("Library/Application Support/io.github.ajaxcbcb.neonhud"));
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Ok(home()?.join(".config/io.github.ajaxcbcb.neonhud"))
    }
}
fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))[..24].to_string()
}
#[derive(Default, Serialize, Deserialize)]
struct ProviderIndex {
    quota: HashMap<String, IndexedQuota>,
    attention: HashMap<String, Attention>,
}
#[derive(Serialize, Deserialize)]
struct IndexedQuota {
    sampled_at: f64,
    windows: Vec<UsageWindow>,
}
fn index_path(dir: &Path) -> PathBuf {
    dir.join("claude-session-index.json")
}
fn dirty_path(dir: &Path, session_hash: &str) -> PathBuf {
    dir.join("claude-index-dirty").join(session_hash)
}
fn mark_dirty(dir: &Path, session_hash: &str) -> Result<(), String> {
    let path = dirty_path(dir, session_hash);
    fs::create_dir_all(path.parent().ok_or("Invalid dirty path")?).map_err(|e| e.to_string())?;
    fs::File::create(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())
}
fn clear_dirty(dir: &Path, session_hash: &str) -> Result<(), String> {
    match fs::remove_file(dirty_path(dir, session_hash)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
fn index_from_session(index: &mut ProviderIndex, session_hash: &str, value: &Value) {
    if let Some(attention) = value
        .get("attention")
        .and_then(|v| serde_json::from_value::<Attention>(v.clone()).ok())
    {
        index.attention.insert(session_hash.into(), attention);
    } else {
        index.attention.remove(session_hash);
    }
    let quota_at = value
        .get("quotaSampledAt")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let windows = value
        .get("windows")
        .and_then(|v| serde_json::from_value::<Vec<UsageWindow>>(v.clone()).ok())
        .unwrap_or_default();
    if (0.0..=600.0).contains(&(now() - quota_at)) && !windows.is_empty() {
        index.quota.insert(
            session_hash.into(),
            IndexedQuota {
                sampled_at: quota_at,
                windows,
            },
        );
    } else {
        index.quota.remove(session_hash);
    }
}
fn read_index(dir: &Path) -> Result<ProviderIndex, String> {
    serde_json::from_slice(&read_limited(&index_path(dir))?).map_err(|e| e.to_string())
}
fn build_index(dir: &Path) -> ProviderIndex {
    let mut index = ProviderIndex::default();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(session_hash) = name
                .strip_prefix("session-")
                .and_then(|s| s.strip_suffix(".json"))
            else {
                continue;
            };
            if session_hash.len() != 24 || !session_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            if let Ok(value) = read_settings(&entry.path()) {
                index_from_session(&mut index, session_hash, &value);
            }
        }
    }
    index
}
fn load_or_build_index(dir: &Path) -> Result<ProviderIndex, String> {
    reconcile_dirty(dir);
    if let Ok(index) = read_index(dir) {
        return Ok(index);
    }
    let path = index_path(dir);
    with_session_lock(&path, || {
        if let Ok(index) = read_index(dir) {
            return Ok(index);
        }
        let index = build_index(dir);
        atomic_json(&path, &index)?;
        Ok(index)
    })
}
fn reconcile_dirty(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir.join("claude-index-dirty")) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.len() != 24 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let session_path = dir.join(format!("session-{name}.json"));
        let _ = with_session_lock(&session_path, || {
            if session_path.exists() {
                let value = read_settings(&session_path)?;
                update_index(dir, &name, &value)?;
            }
            clear_dirty(dir, &name)
        });
    }
}
fn update_index(dir: &Path, session_hash: &str, value: &Value) -> Result<(), String> {
    let path = index_path(dir);
    with_session_lock(&path, || {
        let mut index = read_index(dir).unwrap_or_else(|_| build_index(dir));
        index_from_session(&mut index, session_hash, value);
        index
            .quota
            .retain(|_, q| (0.0..=600.0).contains(&(now() - q.sampled_at)));
        atomic_json(&path, &index)
    })
}
fn read_limited(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_INPUT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_INPUT {
        return Err("Claude settings exceed 1 MB".into());
    }
    Ok(bytes)
}
fn read_settings(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let bytes = read_limited(path)?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
fn executable_command(event: &str, previous: Option<&str>) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    executable_command_for(&exe, event, previous)
}
fn executable_command_for(
    exe: &Path,
    event: &str,
    previous: Option<&str>,
) -> Result<String, String> {
    let exe = exe.to_string_lossy();
    if exe.contains('"') {
        return Err("Executable path contains quote".into());
    }
    let mut cmd = format!("\"{exe}\" {OWN} {event}");
    if let Some(prior) = previous {
        cmd.push_str(" --previous ");
        cmd.push_str(
            &prior
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        );
    }
    Ok(cmd)
}
fn statusline_command_parts(command: &str) -> Option<(PathBuf, Option<String>)> {
    let (exe, args) = command.strip_prefix('"')?.split_once('"')?;
    let path = PathBuf::from(exe);
    let name = path.file_name()?.to_str()?;
    if !name.eq_ignore_ascii_case("neon-hud")
        && !name.eq_ignore_ascii_case("neon-hud.exe")
        && !name.eq_ignore_ascii_case("neon-hud-native")
        && !name.eq_ignore_ascii_case("neon-hud-native.exe") {
        return None;
    }
    let parts: Vec<_> = args.split_whitespace().collect();
    match parts.as_slice() {
        ["--bridge", "statusline"] => Some((path, None)),
        ["--bridge", "statusline", "--previous", hex]
            if !hex.is_empty() && hex.len() <= (MAX_INPUT as usize) * 2 && hex.len() % 2 == 0 =>
        {
            let bytes: Option<Vec<u8>> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
                .collect();
            Some((path, Some(String::from_utf8(bytes?).ok()?)))
        }
        _ => None,
    }
}
fn unlink_statusline_chain(
    command: &str,
    owners: &[PathBuf],
    depth: usize,
) -> Result<(Option<String>, bool), String> {
    if depth >= 16 {
        return Err("Claude statusline chain exceeds limit".into());
    }
    let Some((exe, previous)) = statusline_command_parts(command) else {
        return Ok((Some(command.into()), false));
    };
    let owned = owners.iter().any(|owner| same_executable(&exe, owner));
    if owned {
        return match previous {
            Some(previous) => {
                let (next, _) = unlink_statusline_chain(&previous, owners, depth + 1)?;
                Ok((next, true))
            }
            None => Ok((None, true)),
        };
    }
    let Some(previous) = previous else {
        return Ok((Some(command.into()), false));
    };
    let (next, changed) = unlink_statusline_chain(&previous, owners, depth + 1)?;
    if changed {
        Ok((
            Some(executable_command_for(&exe, "statusline", next.as_deref())?),
            true,
        ))
    } else {
        Ok((Some(command.into()), false))
    }
}
fn unlink_status_field(root: &mut Value, key: &str, owners: &[PathBuf]) -> Result<(), String> {
    let Some(command) = root
        .get(key)
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
    else {
        return Ok(());
    };
    let (replacement, changed) = unlink_statusline_chain(command, owners, 0)?;
    if changed {
        if let Some(command) = replacement {
            root[key]["command"] = json!(command);
        } else if let Some(object) = root.as_object_mut() {
            object.remove(key);
        }
    }
    Ok(())
}
fn same_executable(path: &Path, owner: &Path) -> bool {
    #[cfg(windows)]
    {
        return path
            .to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&owner.to_string_lossy().replace('/', "\\"));
    }
    #[cfg(not(windows))]
    {
        path == owner
    }
}
fn same_install_lineage(recorded: &Path, current: &Path) -> bool {
    if !recorded.is_absolute() || !current.is_absolute() { return false; }
    fn version_prefix(dir: &str) -> Option<&str> {
        let index = dir.find(|c: char| c.is_ascii_digit())?;
        let suffix = &dir[index..];
        if !suffix.chars().any(|c| c.is_ascii_digit())
            || !suffix.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+')) {
            return None;
        }
        Some(dir[..index].trim_end_matches(['-', '_', ' ']))
    }
    fn app_bundle(exe: &Path) -> Option<&Path> {
        let macos = exe.parent()?;
        if macos.file_name()?.to_str()? != "MacOS" { return None; }
        let contents = macos.parent()?;
        if contents.file_name()?.to_str()? != "Contents" { return None; }
        let bundle = contents.parent()?;
        if bundle.extension()?.to_str()? != "app" { return None; }
        Some(bundle)
    }
    if let (Some(old_bundle), Some(new_bundle)) = (app_bundle(recorded), app_bundle(current)) {
        let (Some(old_root), Some(new_root)) = (old_bundle.parent(), new_bundle.parent()) else {
            return false;
        };
        if !same_executable(old_root, new_root) { return false; }
        let (Some(old_name), Some(new_name)) = (old_bundle.file_name(), new_bundle.file_name()) else {
            return false;
        };
        let (old_name, new_name) = (old_name.to_string_lossy().to_ascii_lowercase(), new_name.to_string_lossy().to_ascii_lowercase());
        return matches!((version_prefix(&old_name), version_prefix(&new_name)),
            (Some(old), Some(new)) if old == new && (old.contains("neon-hud") || old.contains("neon hud")));
    }
    let Some(old_parent) = recorded.parent() else { return false; };
    let Some(new_parent) = current.parent() else { return false; };
    if same_executable(old_parent, new_parent) { return true; }
    let (Some(old_root), Some(new_root)) = (old_parent.parent(), new_parent.parent()) else {
        return false;
    };
    if !same_executable(old_root, new_root) { return false; }
    let (Some(old_dir), Some(new_dir)) = (old_parent.file_name(), new_parent.file_name()) else {
        return false;
    };
    let (old_dir, new_dir) = (old_dir.to_string_lossy().to_ascii_lowercase(), new_dir.to_string_lossy().to_ascii_lowercase());
    let (Some(old_prefix), Some(new_prefix)) = (version_prefix(&old_dir), version_prefix(&new_dir)) else {
        return false;
    };
    if old_prefix != new_prefix { return false; }
    let root_name = old_root.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_ascii_lowercase();
    let product_root = root_name.contains("neon-hud") || root_name.contains("neon hud");
    product_root || old_prefix.contains("neon-hud") || old_prefix.contains("neon hud")
}
fn is_ours(command: &str, owners: &[PathBuf]) -> bool {
    let Some(rest) = command.strip_prefix('"') else {
        return false;
    };
    let Some((exe, args)) = rest.split_once('"') else {
        return false;
    };
    if !owners
        .iter()
        .any(|owner| same_executable(Path::new(exe), owner))
    {
        return false;
    }
    let parts: Vec<_> = args.split_whitespace().collect();
    match parts.as_slice() {
        ["--bridge", "statusline"] => true,
        ["--bridge", "statusline", "--previous", hex] => {
            !hex.is_empty() && hex.len() % 2 == 0 && hex.bytes().all(|b| b.is_ascii_hexdigit())
        }
        ["--bridge", "hook", event] => BRIDGE_EVENTS.contains(event),
        _ => false,
    }
}
fn owns_status_line(settings: &Value, owners: &[PathBuf]) -> bool {
    settings
        .get("statusLine")
        .and_then(|v| v.get("command"))
        .and_then(Value::as_str)
        .is_some_and(|command| is_ours(command, owners))
}
fn settings_bridge_enabled(settings: &Value, owner: &Path) -> bool {
    let status_owned = settings
        .get("statusLine")
        .and_then(|status| status.get("command"))
        .and_then(Value::as_str)
        .and_then(statusline_command_parts)
        .is_some_and(|(path, _)| same_executable(&path, owner));
    if !status_owned {
        return false;
    }
    BRIDGE_EVENTS.iter().all(|event| {
        let Ok(command) = executable_command_for(owner, &format!("hook {event}"), None) else {
            return false;
        };
        settings
            .get("hooks")
            .and_then(|hooks| hooks.get(*event))
            .and_then(Value::as_array)
            .is_some_and(|groups| {
                groups.iter().any(|group| {
                    group
                        .get("hooks")
                        .and_then(Value::as_array)
                        .is_some_and(|hooks| {
                            hooks.iter().any(|hook| {
                                hook.get("command").and_then(Value::as_str)
                                    == Some(command.as_str())
                            })
                        })
                })
            })
    })
}
pub fn is_enabled() -> bool {
    let Ok(owner) = std::env::current_exe() else {
        return false;
    };
    settings_path()
        .and_then(|path| read_settings(&path))
        .is_ok_and(|settings| settings_bridge_enabled(&settings, &owner))
}
/// Repair only a complete bridge previously installed by this app at a missing
/// executable or a proven version sibling. A foreign or partial configuration is data,
/// not an instruction to enable the bridge.
fn relocated_settings(
    settings: Value, manifest: &Value, current: &Path, eligible: bool,
) -> Result<Option<(Value, Value)>, String> {
    let Some(recorded) = manifest.get("installedExecutable").and_then(Value::as_str) else {
        return Ok(None);
    };
    let recorded = PathBuf::from(recorded);
    if !eligible || !recorded.is_absolute() || same_executable(&recorded, current) {
        return Ok(None);
    }
    let previous = manifest.get("previousStatusLine")
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str);
    let mut updated_manifest = manifest.clone();
    if settings_bridge_enabled(&settings, current) {
        // This may be the second half of our interrupted settings-then-manifest
        // migration. Do not claim another installation with a different chain.
        let expected = executable_command_for(current, "statusline", previous)?;
        if settings["statusLine"]["command"].as_str() != Some(expected.as_str()) {
            return Ok(None);
        }
        updated_manifest["installedExecutable"] = json!(current);
        return Ok(Some((settings, updated_manifest)));
    }
    if !settings_bridge_enabled(&settings, &recorded) {
        return Ok(None);
    }
    let status = executable_command_for(current, "statusline", previous)?;
    let commands: Vec<_> = BRIDGE_EVENTS.iter().map(|event| {
        Ok(((*event).to_string(), executable_command_for(current, &format!("hook {event}"), None)?))
    }).collect::<Result<_, String>>()?;
    let updated_settings = merge(strip(settings, &[recorded]), &status, &commands)?;
    updated_manifest["installedExecutable"] = json!(current);
    Ok(Some((updated_settings, updated_manifest)))
}

pub fn restore_relocated(dir: &Path) -> Result<bool, String> {
    let manifest_path = dir.join("claude-bridge-manifest.json");
    if !manifest_path.exists() { return Ok(false); }
    let manifest = read_settings(&manifest_path)?;
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let eligible = manifest.get("installedExecutable")
        .and_then(Value::as_str)
        .is_some_and(|path| {
            let recorded = Path::new(path);
            !recorded.exists() || same_install_lineage(recorded, &current)
        });
    let settings_path = settings_path()?;
    let settings = read_settings(&settings_path)?;
    let Some((updated_settings, updated_manifest)) =
        relocated_settings(settings.clone(), &manifest, &current, eligible)? else {
            return Ok(false);
        };
    if updated_settings != settings { atomic_json(&settings_path, &updated_settings)?; }
    atomic_json(&manifest_path, &updated_manifest)?;
    Ok(true)
}
fn restore_status_line(settings: &mut Value, manifest: &Value, owners: &[PathBuf]) {
    if !owns_status_line(settings, owners) {
        return;
    }
    if let Some(root) = settings.as_object_mut() {
        let previous = manifest
            .get("previousStatusLine")
            .cloned()
            .unwrap_or(Value::Null);
        if previous.is_null() {
            root.remove("statusLine");
        } else {
            root.insert("statusLine".into(), previous);
        }
    }
}
fn merge(
    mut settings: Value,
    status_command: &str,
    hook_commands: &[(String, String)],
) -> Result<Value, String> {
    let root = settings
        .as_object_mut()
        .ok_or("Claude settings are not a JSON object")?;
    root.insert(
        "statusLine".into(),
        json!({"type":"command","command":status_command}),
    );
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("Claude hooks are not an object")?;
    for (event, command) in hook_commands {
        let items = hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or("Claude hook event is not an array")?;
        if !items.iter().any(|v| {
            v.get("hooks").and_then(Value::as_array).is_some_and(|arr| {
                arr.iter()
                    .any(|h| h.get("command").and_then(Value::as_str) == Some(command))
            })
        }) {
            items.push(json!({"hooks":[{"type":"command","command":command}]}));
        }
    }
    Ok(settings)
}
fn strip(mut settings: Value, owners: &[PathBuf]) -> Value {
    if let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) {
        for value in hooks.values_mut() {
            if let Some(groups) = value.as_array_mut() {
                for group in groups.iter_mut() {
                    if let Some(commands) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                        commands.retain(|h| {
                            !h.get("command")
                                .and_then(Value::as_str)
                                .is_some_and(|command| is_ours(command, owners))
                        });
                    }
                }
                groups.retain(|g| {
                    g.get("hooks")
                        .and_then(Value::as_array)
                        .is_none_or(|h| !h.is_empty())
                });
            }
        }
    }
    settings
}
fn executable_owners_for(manifest: &Value, current: &Path) -> Vec<PathBuf> {
    let mut owners = vec![current.to_path_buf()];
    if let Some(recorded) = manifest.get("installedExecutable").and_then(Value::as_str) {
        let recorded = PathBuf::from(recorded);
        // A moved installation may leave a stale command. A still-existing executable
        // belongs to a distinct installation and must be preserved.
        if recorded.is_absolute() && !recorded.exists() && !same_executable(&recorded, &current) {
            owners.push(recorded);
        }
    }
    owners
}
fn executable_owners(manifest: &Value) -> Result<Vec<PathBuf>, String> {
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(executable_owners_for(manifest, &current))
}
fn remove_settings(
    settings: Value,
    manifest: &Value,
    current_exe: &Path,
) -> Result<(Value, Value, bool), String> {
    let owners = executable_owners_for(manifest, current_exe);
    let mut settings = strip(settings, &owners);
    restore_status_line(&mut settings, manifest, &owners);
    unlink_status_field(&mut settings, "statusLine", &owners)?;
    let mut updated_manifest = manifest.clone();
    unlink_status_field(&mut updated_manifest, "previousStatusLine", &owners)?;
    // The app identifier shares one manifest across installations. Only the
    // installation recorded there may remove it; a missing path alone does
    // not establish that a different installation owns the record.
    let owns_manifest = manifest
        .get("installedExecutable")
        .and_then(Value::as_str)
        .is_some_and(|path| same_executable(Path::new(path), current_exe));
    Ok((settings, updated_manifest, owns_manifest))
}
pub fn install(dir: &Path) -> Result<String, String> {
    let path = settings_path()?;
    let current = read_settings(&path)?;
    let saved = dir.join("claude-bridge-manifest.json");
    let manifest = read_settings(&saved).unwrap_or(Value::Null);
    let owners = executable_owners(&manifest)?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let backup = dir.join("claude-settings.backup.json");
    if !backup.exists() {
        atomic_json(&backup, &current)?;
    }
    let previous = current
        .get("statusLine")
        .and_then(|v| v.get("command"))
        .and_then(Value::as_str)
        .filter(|s| !is_ours(s, &owners))
        .map(str::to_owned);
    let already_installed = owns_status_line(&current, &owners);
    if !already_installed {
        atomic_json(
            &saved,
            &json!({"previousStatusLine":current.get("statusLine"),"installedExecutable":std::env::current_exe().map_err(|e| e.to_string())?}),
        )?;
    }
    let previous = previous.or_else(|| {
        read_settings(&saved).ok().and_then(|v| {
            v.get("previousStatusLine")?
                .get("command")?
                .as_str()
                .map(str::to_owned)
        })
    });
    let status = executable_command("statusline", previous.as_deref())?;
    let commands: Vec<_> = BRIDGE_EVENTS
        .iter()
        .map(|e| {
            Ok((
                (*e).to_string(),
                executable_command(&format!("hook {e}"), None)?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let merged = merge(strip(current, &owners), &status, &commands)?;
    atomic_json(&path, &merged)?;
    Ok(
        "Claude Code bridge enabled. New sessions will report supported allowance and questions."
            .into(),
    )
}
pub fn remove(dir: &Path) -> Result<String, String> {
    let path = settings_path()?;
    let manifest_path = dir.join("claude-bridge-manifest.json");
    let manifest = read_settings(&manifest_path).unwrap_or(Value::Null);
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let (current, updated_manifest, owns_manifest) =
        remove_settings(read_settings(&path)?, &manifest, &current_exe)?;
    persist_removal(
        &path,
        &manifest_path,
        &current,
        &manifest,
        &updated_manifest,
        owns_manifest,
    )?;
    Ok("Claude Code bridge removed.".into())
}
fn persist_removal(
    settings_path: &Path,
    manifest_path: &Path,
    settings: &Value,
    original_manifest: &Value,
    updated_manifest: &Value,
    owns_manifest: bool,
) -> Result<(), String> {
    // A foreign installation may still be active. Persist its revised restore
    // target first, so a failed settings write cannot leave a stale target.
    if !owns_manifest && updated_manifest != original_manifest {
        atomic_json(manifest_path, updated_manifest)?;
    }
    atomic_json(settings_path, settings)?;
    if owns_manifest && manifest_path.exists() {
        fs::remove_file(manifest_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn bounded_stdin() -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(MAX_INPUT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_INPUT {
        return Err("Input exceeds 1 MB".into());
    }
    Ok(bytes)
}
fn quota_windows(input: &Value) -> Vec<UsageWindow> {
    let limits = input.get("rate_limits").unwrap_or(&Value::Null);
    [
        ("five_hour", "5 hours", 300),
        ("seven_day", "7 days", 10080),
    ]
    .iter()
    .filter_map(|(key, label, minutes)| {
        let value = limits.get(*key)?;
        let used = value
            .get("used_percentage")?
            .as_f64()
            .filter(|u| (0.0..=100.0).contains(u))?;
        Some(UsageWindow {
            label: (*label).into(),
            minutes: *minutes,
            used_percent: used,
            resets_at: value.get("resets_at").and_then(Value::as_f64),
        })
    })
    .collect()
}
fn with_session_lock<T>(
    path: &Path,
    action: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let lock_path = path.with_extension("lock");
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(lock_path)
        .map_err(|e| e.to_string())?;
    let mut acquired = false;
    for _ in 0..50 {
        if lock.try_lock().is_ok() {
            acquired = true;
            break;
        }
        thread::sleep(Duration::from_millis(40));
    }
    if !acquired {
        return Err("Claude session is busy".into());
    }
    let result = action();
    let _ = lock.unlock();
    result
}
fn write_session(dir: &Path, input: &Value, event: &str) -> Result<(), String> {
    let session = input
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or("No session ID")?;
    if session.is_empty() || session.len() > 256 {
        return Err("Invalid session ID".into());
    }
    let path = dir.join(format!("session-{}.json", hash(session)));
    with_session_lock(&path, || write_session_locked(&path, input, event, session))
}
fn write_session_locked(
    path: &Path,
    input: &Value,
    event: &str,
    session: &str,
) -> Result<(), String> {
    let mut value = read_settings(&path).unwrap_or_else(|_| json!({}));
    if !value.is_object() {
        value = json!({});
    }
    let root = value.as_object_mut().unwrap();
    root.insert("sessionId".into(), json!(session));
    root.insert("sampledAt".into(), json!(now()));
    if event == "statusline" {
        root.insert("quotaSampledAt".into(), json!(now()));
        root.insert(
            "windows".into(),
            serde_json::to_value(quota_windows(input)).unwrap_or(Value::Null),
        );
    } else if ["UserPromptSubmit", "PostToolUse", "Stop", "SessionEnd"].contains(&event) {
        root.remove("attention");
    } else {
        let notification = input.get("notification_type").and_then(Value::as_str);
        let tool = input.get("tool_name").and_then(Value::as_str);
        let attention = event == "PermissionRequest"
            || (event == "Notification"
                && matches!(
                    notification,
                    Some(
                        "permission_prompt"
                            | "agent_needs_input"
                            | "elicitation_dialog"
                            | "url_dialog"
                    )
                ))
            || (event == "PreToolUse"
                && matches!(tool, Some("AskUserQuestion" | "PermissionRequest")));
        if attention {
            root.insert("attention".into(), json!({"id":format!("{}-{}",hash(session),hash(&format!("{}:{}",event,now()))),"surface":"claude-code","reason":"A Claude Code session is asking for input","occurredAt":now(),"sessionId":session}));
        }
    }
    let dir = path.parent().ok_or("Invalid session path")?;
    let session_hash = hash(session);
    mark_dirty(dir, &session_hash)?;
    atomic_json(&path, &value)?;
    update_index(dir, &session_hash, &value)?;
    clear_dirty(dir, &session_hash)
}
fn run_previous(hex: &str, input: &[u8]) {
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect();
    let Some(bytes) = bytes else {
        return;
    };
    let Ok(command) = String::from_utf8(bytes) else {
        return;
    };
    let mut shell = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", &command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &command]);
        c
    };
    shell
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        shell.creation_flags(0x0800_0000);
    }
    if let Ok(mut child) = shell.spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let bytes = input.to_vec();
            thread::spawn(move || {
                let _ = stdin.write_all(&bytes);
            });
        }
        let (tx, rx) = mpsc::channel();
        if let Some(mut stdout) = child.stdout.take() {
            thread::spawn(move || {
                let mut output = Vec::new();
                let _ = stdout.take(MAX_INPUT).read_to_end(&mut output);
                let _ = tx.send(output);
            });
        }
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if child.try_wait().ok().flatten().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
        }
        if let Ok(output) = rx.recv_timeout(Duration::from_millis(200)) {
            let _ = io::stdout().write_all(&output);
        }
        let _ = child.wait();
    }
}
pub fn run_cli_if_requested() -> bool {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--bridge") {
        return false;
    }
    let mode = args.get(2).map(String::as_str).unwrap_or("");
    if let Ok(bytes) = bounded_stdin() {
        if let Ok(input) = serde_json::from_slice::<Value>(&bytes) {
            if let Ok(dir) = cli_data_dir() {
                let _ = write_session(
                    &dir,
                    &input,
                    if mode == "hook" {
                        args.get(3).map(String::as_str).unwrap_or("")
                    } else {
                        mode
                    },
                );
            }
        }
        if mode == "statusline" {
            if let Some(i) = args.iter().position(|s| s == "--previous") {
                if let Some(hex) = args.get(i + 1) {
                    run_previous(hex, &bytes);
                }
            }
        }
    }
    true
}
pub fn read_provider(dir: &Path) -> (Usage, Vec<Attention>) {
    read_provider_dir(dir)
}
fn read_provider_dir(dir: &Path) -> (Usage, Vec<Attention>) {
    let Ok(index) = load_or_build_index(dir) else {
        return (
            Usage::unavailable(
                "claude",
                "Claude Code statusline",
                "Local bridge index unavailable",
            ),
            vec![],
        );
    };
    let latest = index
        .quota
        .into_values()
        .filter(|q| (0.0..=600.0).contains(&(now() - q.sampled_at)))
        .max_by(|a, b| a.sampled_at.total_cmp(&b.sampled_at));
    let usage = match latest {
        Some(quota) => Usage { surface:"claude".into(), source:"Claude Code statusline".into(), state:"connected".into(), message:"Shared Claude account allowance".into(), fetched_at:Some(quota.sampled_at), windows:quota.windows },
        None => Usage::unavailable("claude", "Claude Code statusline", "No recent supported allowance reading; start a Claude Code session with the bridge enabled."),
    };
    let mut attention: Vec<_> = index.attention.into_values().collect();
    attention.sort_by(|a, b| b.occurred_at.total_cmp(&a.occurred_at));
    (usage, attention)
}
pub fn dismiss(dir: &Path, id: &str) -> Result<(), String> {
    let Some((session_hash, _)) = id.split_once('-') else {
        return Ok(());
    };
    if session_hash.len() != 24 || !session_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(());
    }
    let path = dir.join(format!("session-{session_hash}.json"));
    if path.exists() {
        with_session_lock(&path, || {
            let mut v = read_settings(&path)?;
            if v.get("attention")
                .and_then(|a| a.get("id"))
                .and_then(Value::as_str)
                == Some(id)
            {
                if let Some(o) = v.as_object_mut() {
                    o.remove("attention");
                }
                mark_dirty(&dir, session_hash)?;
                atomic_json(&path, &v)?;
                update_index(&dir, session_hash, &v)?;
                clear_dirty(&dir, session_hash)?;
            }
            Ok(())
        })?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn abs(path: &str) -> PathBuf {
        if cfg!(windows) { PathBuf::from(format!("C:{path}")) }
        else { PathBuf::from(path) }
    }
    fn temp_dir() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "neon-hud-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
    #[test]
    fn merge_strip_preserves_foreign() {
        let source = json!({"theme":"dark","hooks":{"Stop":[{"hooks":[{"type":"command","command":"foreign"}]}]}});
        let command = "\"/tmp/neon-hud\" --bridge hook Stop".to_string();
        let hooks = [("Stop".into(), command)];
        let merged = merge(source.clone(), "own", &hooks).unwrap();
        assert_eq!(merge(merged.clone(), "own", &hooks).unwrap(), merged);
        assert_eq!(
            strip(merged, &[PathBuf::from("/tmp/neon-hud")]).get("hooks"),
            source.get("hooks")
        );
    }
    #[test]
    fn malformed_settings_rejected() {
        assert!(merge(json!([]), "own", &[]).is_err());
        assert!(merge(json!({"hooks":[]}), "own", &[]).is_err());
    }
    #[test]
    fn bridge_status_requires_current_executable_and_all_hooks() {
        let own = PathBuf::from("/install/one/neon-hud");
        let foreign = PathBuf::from("/install/two/neon-hud");
        let status = executable_command_for(&own, "statusline", None).unwrap();
        let hooks: Vec<_> = BRIDGE_EVENTS
            .iter()
            .map(|event| {
                (
                    (*event).to_owned(),
                    executable_command_for(&own, &format!("hook {event}"), None).unwrap(),
                )
            })
            .collect();
        let installed = merge(json!({}), &status, &hooks).unwrap();
        assert!(settings_bridge_enabled(&installed, &own));
        assert!(!settings_bridge_enabled(&installed, &foreign));
        let mut wrong_status = installed.clone();
        wrong_status["statusLine"]["command"] = json!(hooks[0].1.clone());
        assert!(!settings_bridge_enabled(&wrong_status, &own));
        let mut missing_hook = installed.clone();
        missing_hook["hooks"]["Stop"] = json!([]);
        assert!(!settings_bridge_enabled(&missing_hook, &own));
        let removed = strip(installed, &[own.clone()]);
        assert!(!settings_bridge_enabled(&removed, &own));
    }
    #[test]
    fn relocation_repairs_only_manifest_owned_complete_bridge() {
        let old = abs("/retired/neon-hud");
        let current = abs("/updated/neon-hud");
        let previous = json!({"type":"command","command":"foreign status"});
        let manifest = json!({"installedExecutable":old,"previousStatusLine":previous});
        let status = executable_command_for(&old, "statusline", Some("foreign status")).unwrap();
        let hooks: Vec<_> = BRIDGE_EVENTS.iter().map(|event| (
            (*event).into(), executable_command_for(&old, &format!("hook {event}"), None).unwrap()
        )).collect();
        let settings = merge(json!({"theme":"dark","hooks":{"Stop":[{"hooks":[{"type":"command","command":"foreign hook"}]}]}}),
            &status, &hooks).unwrap();
        let (updated, updated_manifest) = relocated_settings(settings.clone(), &manifest, &current, true)
            .unwrap().unwrap();
        assert!(settings_bridge_enabled(&updated, &current));
        assert!(!settings_bridge_enabled(&updated, &old));
        assert_eq!(updated["theme"], "dark");
        assert_eq!(updated["hooks"]["Stop"][0]["hooks"][0]["command"], "foreign hook");
        assert_eq!(updated_manifest["previousStatusLine"], previous);
        assert_eq!(updated_manifest["installedExecutable"], json!(current));
        let (_, resumed_manifest) = relocated_settings(updated, &manifest, &current, true)
            .unwrap().unwrap();
        assert_eq!(resumed_manifest["installedExecutable"], json!(current));
        assert!(relocated_settings(settings.clone(), &manifest, &current, false).unwrap().is_none());
        let mut foreign = settings;
        foreign["statusLine"]["command"] = json!("foreign status");
        assert!(relocated_settings(foreign, &manifest, &current, true).unwrap().is_none());
        let unrelated_status = executable_command_for(&current, "statusline", None).unwrap();
        let unrelated_hooks: Vec<_> = BRIDGE_EVENTS.iter().map(|event| (
            (*event).into(), executable_command_for(&current, &format!("hook {event}"), None).unwrap()
        )).collect();
        let other_install = merge(json!({}), &unrelated_status, &unrelated_hooks).unwrap();
        assert!(relocated_settings(other_install, &manifest, &current, true).unwrap().is_none());
    }
    #[test]
    fn retained_old_version_migrates_only_with_full_manifest_ownership() {
        let old = abs("/Applications/Neon HUD/app-1.4/neon-hud");
        let current = abs("/Applications/Neon HUD/app-1.5/neon-hud");
        assert!(same_install_lineage(&old, &current));
        let manifest = json!({"installedExecutable":old,"previousStatusLine":null});
        let status = executable_command_for(&old, "statusline", None).unwrap();
        let hooks: Vec<_> = BRIDGE_EVENTS.iter().map(|event| (
            (*event).into(), executable_command_for(&old, &format!("hook {event}"), None).unwrap()
        )).collect();
        let settings = merge(json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":"foreign hook"}]}]}}), &status, &hooks).unwrap();
        let (updated, _) = relocated_settings(settings.clone(), &manifest, &current,
            same_install_lineage(&old, &current)).unwrap().unwrap();
        assert!(settings_bridge_enabled(&updated, &current));
        assert_eq!(updated["hooks"]["Stop"][0]["hooks"][0]["command"], "foreign hook");
        let mut foreign = settings;
        foreign["statusLine"]["command"] = json!("foreign status");
        assert!(relocated_settings(foreign, &manifest, &current, true).unwrap().is_none());
    }
    #[test]
    fn distinct_live_installation_has_no_migration_lineage() {
        let old = abs("/Applications/one/neon-hud");
        let current = abs("/Applications/two/neon-hud");
        assert!(!same_install_lineage(&old, &current));
        let old = abs("/Applications/Neon HUD/app-1.4/neon-hud");
        let current = abs("/Applications/Other HUD/app-1.5/neon-hud");
        assert!(!same_install_lineage(&old, &current));
    }
    #[test]
    fn retained_macos_app_version_sibling_migrates_without_claiming_foreign_bundle() {
        let old = abs("/Applications/Neon HUD 1.4.app/Contents/MacOS/neon-hud");
        let current = abs("/Applications/Neon HUD 1.5.app/Contents/MacOS/neon-hud");
        assert!(same_install_lineage(&old, &current));
        let manifest = json!({"installedExecutable":old,"previousStatusLine":null});
        let status = executable_command_for(&old, "statusline", None).unwrap();
        let hooks: Vec<_> = BRIDGE_EVENTS.iter().map(|event| (
            (*event).into(), executable_command_for(&old, &format!("hook {event}"), None).unwrap()
        )).collect();
        let settings = merge(json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":"foreign hook"}]}]}}), &status, &hooks).unwrap();
        let (updated, _) = relocated_settings(settings.clone(), &manifest, &current,
            same_install_lineage(&old, &current)).unwrap().unwrap();
        assert!(settings_bridge_enabled(&updated, &current));
        assert_eq!(updated["hooks"]["Stop"][0]["hooks"][0]["command"], "foreign hook");
        let foreign = abs("/Applications/Other HUD 1.5.app/Contents/MacOS/neon-hud");
        assert!(!same_install_lineage(&old, &foreign));
        let separate = abs("/Another/Neon HUD 1.5.app/Contents/MacOS/neon-hud");
        assert!(!same_install_lineage(&old, &separate));
        let mut foreign_settings = settings;
        foreign_settings["statusLine"]["command"] = json!("foreign status");
        assert!(relocated_settings(foreign_settings, &manifest, &current, true).unwrap().is_none());
    }
    #[test]
    fn manifest_restores_foreign_status_without_touching_other_settings() {
        let foreign = json!({"type":"command","command":"foreign status","padding":2});
        let manifest = json!({"previousStatusLine":foreign});
        let owner = [PathBuf::from("/tmp/neon-hud")];
        let mut installed = json!({"theme":"dark","statusLine":{"type":"command","command":"\"/tmp/neon-hud\" --bridge statusline"}});
        restore_status_line(&mut installed, &manifest, &owner);
        assert_eq!(installed["statusLine"], manifest["previousStatusLine"]);
        assert_eq!(installed["theme"], "dark");
        installed["statusLine"] = json!({"type":"command","command":"new foreign"});
        restore_status_line(&mut installed, &manifest, &owner);
        assert_eq!(installed["statusLine"]["command"], "new foreign");
    }
    #[test]
    fn foreign_wrapper_mentioning_bridge_is_preserved() {
        let wrapper = "\"/tmp/wrapper\" --run neon-hud --bridge statusline";
        let owner = [PathBuf::from("/tmp/neon-hud")];
        assert!(!is_ours(wrapper, &owner));
        assert!(!is_ours(
            "\"/tmp/neon-hud\" --bridge statusline && foreign",
            &owner
        ));
        assert!(is_ours("\"/tmp/neon-hud\" --bridge statusline", &owner));
        assert!(is_ours("\"/tmp/neon-hud\" --bridge hook Stop", &owner));
        let settings = json!({"statusLine":{"type":"command","command":wrapper},"hooks":{"Stop":[{"hooks":[{"type":"command","command":wrapper}]}]}});
        assert_eq!(strip(settings.clone(), &owner), settings);
    }
    #[test]
    fn distinct_install_with_same_filename_is_foreign() {
        let owner = [PathBuf::from("/install/one/neon-hud")];
        let other = "\"/install/two/neon-hud\" --bridge hook Stop";
        let own = "\"/install/one/neon-hud\" --bridge hook Stop";
        assert!(!is_ours(other, &owner));
        let settings = json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":other},{"type":"command","command":own}]}]}});
        let stripped = strip(settings, &owner);
        assert_eq!(
            stripped["hooks"]["Stop"][0]["hooks"],
            json!([{"type":"command","command":other}])
        );
    }
    #[test]
    fn removing_first_install_preserves_second_install_manifest() {
        let dir = temp_dir();
        let a = dir.join("a").join("neon-hud");
        let b = dir.join("b").join("neon-hud");
        fs::create_dir_all(a.parent().unwrap()).unwrap();
        fs::create_dir_all(b.parent().unwrap()).unwrap();
        fs::write(&a, b"").unwrap();
        fs::write(&b, b"").unwrap();
        let a_status = executable_command_for(&a, "statusline", Some("foreign status")).unwrap();
        let b_status = executable_command_for(&b, "statusline", Some(&a_status)).unwrap();
        let a_hook = format!("\"{}\" --bridge hook Stop", a.display());
        let b_hook = format!("\"{}\" --bridge hook Stop", b.display());
        let original = json!({"type":"command","command":"foreign status"});
        let a_manifest = json!({"installedExecutable":a,"previousStatusLine":original});
        let b_manifest = json!({"installedExecutable":b,"previousStatusLine":{"type":"command","command":a_status}});
        let settings = json!({"statusLine":{"type":"command","command":b_status},"hooks":{"Stop":[{"hooks":[{"type":"command","command":a_hook},{"type":"command","command":b_hook}]}]}});
        let (after_a, updated_b_manifest, delete_a_manifest) =
            remove_settings(settings, &b_manifest, &a).unwrap();
        assert!(!delete_a_manifest);
        assert_eq!(
            after_a["statusLine"]["command"],
            executable_command_for(&b, "statusline", Some("foreign status")).unwrap()
        );
        assert_eq!(
            updated_b_manifest["previousStatusLine"]["command"],
            "foreign status"
        );
        assert_eq!(
            after_a["hooks"]["Stop"][0]["hooks"],
            json!([{"type":"command","command":b_hook}])
        );
        let (after_b, _, delete_b_manifest) =
            remove_settings(after_a, &updated_b_manifest, &b).unwrap();
        assert!(delete_b_manifest);
        assert_eq!(after_b["statusLine"]["command"], "foreign status");
        assert!(after_b["hooks"]["Stop"].as_array().unwrap().is_empty());
        assert_eq!(a_manifest["previousStatusLine"], original);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn removing_prior_install_without_chain_clears_later_restore_target() {
        let dir = temp_dir();
        let a = dir.join("one").join("neon-hud");
        let b = dir.join("two").join("neon-hud");
        fs::create_dir_all(a.parent().unwrap()).unwrap();
        fs::create_dir_all(b.parent().unwrap()).unwrap();
        fs::write(&a, b"").unwrap();
        fs::write(&b, b"").unwrap();
        let a_status = executable_command_for(&a, "statusline", None).unwrap();
        let b_status = executable_command_for(&b, "statusline", Some(&a_status)).unwrap();
        let manifest = json!({"installedExecutable":b,"previousStatusLine":{"type":"command","command":a_status}});
        let settings = json!({"statusLine":{"type":"command","command":b_status}});
        let (after_a, updated_manifest, _) = remove_settings(settings, &manifest, &a).unwrap();
        assert_eq!(
            after_a["statusLine"]["command"],
            executable_command_for(&b, "statusline", None).unwrap()
        );
        assert!(updated_manifest.get("previousStatusLine").is_none());
        let (after_b, _, _) = remove_settings(after_a, &updated_manifest, &b).unwrap();
        assert!(after_b.get("statusLine").is_none());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn failed_second_write_keeps_foreign_manifest_unlinked() {
        let dir = temp_dir();
        let a = dir.join("a").join("neon-hud");
        let b = dir.join("b").join("neon-hud");
        fs::create_dir_all(a.parent().unwrap()).unwrap();
        fs::create_dir_all(b.parent().unwrap()).unwrap();
        fs::write(&a, b"").unwrap();
        fs::write(&b, b"").unwrap();
        let a_status = executable_command_for(&a, "statusline", Some("foreign status")).unwrap();
        let b_status = executable_command_for(&b, "statusline", Some(&a_status)).unwrap();
        let original_manifest = json!({"installedExecutable":b,"previousStatusLine":{"type":"command","command":a_status}});
        let settings = json!({"statusLine":{"type":"command","command":b_status}});
        let (revised_settings, revised_manifest, owns_manifest) =
            remove_settings(settings, &original_manifest, &a).unwrap();
        assert!(!owns_manifest);
        let manifest_path = dir.join("manifest.json");
        atomic_json(&manifest_path, &original_manifest).unwrap();
        // Replacing a directory as if it were settings.json fails on every platform.
        let settings_path = dir.join("settings.json");
        fs::create_dir(&settings_path).unwrap();
        assert!(persist_removal(
            &settings_path,
            &manifest_path,
            &revised_settings,
            &original_manifest,
            &revised_manifest,
            owns_manifest,
        )
        .is_err());
        let persisted = read_settings(&manifest_path).unwrap();
        assert_eq!(persisted, revised_manifest);
        assert_eq!(persisted["previousStatusLine"]["command"], "foreign status");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn failed_index_update_reconciles_from_dirty_marker() {
        let dir = temp_dir();
        atomic_json(&index_path(&dir), &ProviderIndex::default()).unwrap();
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(index_path(&dir).with_extension("lock"))
            .unwrap();
        lock.lock().unwrap();
        let input = json!({"session_id":"pending","notification_type":"permission_prompt"});
        assert!(write_session(&dir, &input, "Notification").is_err());
        let session_hash = hash("pending");
        assert!(dirty_path(&dir, &session_hash).exists());
        assert!(dir.join(format!("session-{session_hash}.json")).exists());
        lock.unlock().unwrap();
        assert_eq!(read_provider_dir(&dir).1.len(), 1);
        assert!(!dirty_path(&dir, &session_hash).exists());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn session_writers_preserve_quota_and_attention() {
        let dir = temp_dir();
        let quota =
            json!({"session_id":"same","rate_limits":{"five_hour":{"used_percentage":42.0}}});
        let question = json!({"session_id":"same","notification_type":"permission_prompt"});
        std::thread::scope(|scope| {
            scope.spawn(|| write_session(&dir, &quota, "statusline").unwrap());
            scope.spawn(|| write_session(&dir, &question, "Notification").unwrap());
        });
        let (usage, attention) = read_provider_dir(&dir);
        assert_eq!(usage.windows.len(), 1);
        assert_eq!(attention.len(), 1);
        let session_path = dir.join(format!("session-{}.json", hash("same")));
        let mut session = read_settings(&session_path).unwrap();
        session["quotaSampledAt"] = json!(now() - 601.0);
        atomic_json(&session_path, &session).unwrap();
        update_index(&dir, &hash("same"), &session).unwrap();
        let (stale_usage, pending) = read_provider_dir(&dir);
        assert!(stale_usage.windows.is_empty());
        assert_eq!(pending.len(), 1);
        write_session(&dir, &json!({"session_id":"same"}), "UserPromptSubmit").unwrap();
        assert!(read_provider_dir(&dir).1.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn provider_reads_recent_session_after_many_old_files() {
        let dir = temp_dir();
        for n in 0..520 {
            fs::write(dir.join(format!("session-{n:024x}.json")), b"{}").unwrap();
        }
        let recent = json!({"sessionId":"recent","quotaSampledAt":now(),"windows":[{"label":"5 hours","minutes":300,"usedPercent":65.0,"resetsAt":null}]});
        atomic_json(
            &dir.join(format!("session-{}.json", hash("recent"))),
            &recent,
        )
        .unwrap();
        assert_eq!(read_provider_dir(&dir).0.windows.len(), 1);
        assert!(index_path(&dir).exists());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn quota_windows_require_real_supported_readings() {
        assert!(quota_windows(&json!({})).is_empty());
        assert!(
            quota_windows(&json!({"rate_limits":{"five_hour":{"used_percentage":-1}}})).is_empty()
        );
        let windows = quota_windows(
            &json!({"rate_limits":{"five_hour":{"used_percentage":42.0,"resets_at":1234.0},"seven_day":{"used_percentage":null}}}),
        );
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].minutes, 300);
        assert_eq!(windows[0].resets_at, Some(1234.0));
    }
}
