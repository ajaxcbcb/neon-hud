use super::{atomic_json, now, Attention, Usage, UsageWindow};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

const MAX_INPUT: u64 = 1024 * 1024;
const OWN: &str = "--bridge";
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
fn is_ours(command: &str) -> bool {
    command.contains(" --bridge ") && command.contains("neon-hud")
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
fn strip(mut settings: Value) -> Value {
    if let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) {
        for value in hooks.values_mut() {
            if let Some(groups) = value.as_array_mut() {
                for group in groups.iter_mut() {
                    if let Some(commands) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                        commands.retain(|h| {
                            !h.get("command")
                                .and_then(Value::as_str)
                                .is_some_and(is_ours)
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
pub fn install(app: &AppHandle) -> Result<String, String> {
    let path = settings_path()?;
    let current = read_settings(&path)?;
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let backup = dir.join("claude-settings.backup.json");
    if !backup.exists() {
        atomic_json(&backup, &current)?;
    }
    let previous = current
        .get("statusLine")
        .and_then(|v| v.get("command"))
        .and_then(Value::as_str)
        .filter(|s| !is_ours(s))
        .map(str::to_owned);
    let saved = dir.join("claude-bridge-manifest.json");
    if !saved.exists() {
        atomic_json(
            &saved,
            &json!({"previousStatusLine":current.get("statusLine")}),
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
    let events = [
        "Notification",
        "PermissionRequest",
        "PreToolUse",
        "UserPromptSubmit",
        "PostToolUse",
        "Stop",
        "SessionEnd",
    ];
    let commands: Vec<_> = events
        .iter()
        .map(|e| {
            Ok((
                (*e).to_string(),
                executable_command(&format!("hook {e}"), None)?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let merged = merge(strip(current), &status, &commands)?;
    atomic_json(&path, &merged)?;
    Ok(
        "Claude Code bridge enabled. New sessions will report supported allowance and questions."
            .into(),
    )
}
pub fn remove(app: &AppHandle) -> Result<String, String> {
    let path = settings_path()?;
    let mut current = strip(read_settings(&path)?);
    if current
        .get("statusLine")
        .and_then(|v| v.get("command"))
        .and_then(Value::as_str)
        .is_some_and(is_ours)
    {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        let manifest =
            read_settings(&dir.join("claude-bridge-manifest.json")).unwrap_or(Value::Null);
        let previous = manifest
            .get("previousStatusLine")
            .cloned()
            .unwrap_or(Value::Null);
        if let Some(root) = current.as_object_mut() {
            if previous.is_null() {
                root.remove("statusLine");
            } else {
                root.insert("statusLine".into(), previous);
            }
        }
    }
    atomic_json(&path, &current)?;
    Ok("Claude Code bridge removed.".into())
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
fn write_session(dir: &Path, input: &Value, event: &str) -> Result<(), String> {
    let session = input
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or("No session ID")?;
    if session.is_empty() || session.len() > 256 {
        return Err("Invalid session ID".into());
    }
    let path = dir.join(format!("session-{}.json", hash(session)));
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
    atomic_json(&path, &value)
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
pub fn read_provider(app: &AppHandle) -> (Usage, Vec<Attention>) {
    let dir = match app.path().app_config_dir() {
        Ok(v) => v,
        Err(_) => {
            return (
                Usage::unavailable("claude", "Claude Code bridge", "App data unavailable"),
                vec![],
            )
        }
    };
    let mut latest: Option<(f64, Vec<UsageWindow>)> = None;
    let mut attention = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten().take(500) {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("session-") || !name.ends_with(".json") {
                continue;
            }
            let Ok(bytes) = read_limited(&entry.path()) else {
                continue;
            };
            let Ok(v) = serde_json::from_slice::<Value>(&bytes) else {
                continue;
            };
            let t = v.get("sampledAt").and_then(Value::as_f64).unwrap_or(0.0);
            if now() - t > 600.0 {
                continue;
            }
            if let Some(a) = v
                .get("attention")
                .and_then(|x| serde_json::from_value::<Attention>(x.clone()).ok())
            {
                attention.push(a);
            }
            if let Some(w) = v
                .get("windows")
                .and_then(|x| serde_json::from_value::<Vec<UsageWindow>>(x.clone()).ok())
                .filter(|w| !w.is_empty())
            {
                let quota_at = v
                    .get("quotaSampledAt")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                if now() - quota_at <= 600.0
                    && latest.as_ref().is_none_or(|(old, _)| quota_at > *old)
                {
                    latest = Some((quota_at, w));
                }
            }
        }
    }
    let usage=match latest {Some((t,windows))=>Usage{surface:"claude".into(),source:"Claude Code statusline".into(),state:"connected".into(),message:"Shared Claude account allowance".into(),fetched_at:Some(t),windows},None=>Usage::unavailable("claude","Claude Code statusline","No recent supported allowance reading; start a Claude Code session with the bridge enabled.")};
    (usage, attention)
}
pub fn dismiss(app: &AppHandle, id: &str) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten().take(500) {
            let path = entry.path();
            if !entry.file_name().to_string_lossy().starts_with("session-") {
                continue;
            }
            let Ok(mut v) = read_settings(&path) else {
                continue;
            };
            if v.get("attention")
                .and_then(|a| a.get("id"))
                .and_then(Value::as_str)
                == Some(id)
            {
                if let Some(o) = v.as_object_mut() {
                    o.remove("attention");
                }
                atomic_json(&path, &v)?;
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merge_strip_preserves_foreign() {
        let source = json!({"theme":"dark","hooks":{"Stop":[{"hooks":[{"type":"command","command":"foreign"}]}]}});
        let command = "\"/tmp/neon-hud\" --bridge hook Stop".to_string();
        let hooks = [("Stop".into(), command)];
        let merged = merge(source.clone(), "own", &hooks).unwrap();
        assert_eq!(merge(merged.clone(), "own", &hooks).unwrap(), merged);
        assert_eq!(strip(merged).get("hooks"), source.get("hooks"));
    }
    #[test]
    fn malformed_settings_rejected() {
        assert!(merge(json!([]), "own", &[]).is_err());
        assert!(merge(json!({"hooks":[]}), "own", &[]).is_err());
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
