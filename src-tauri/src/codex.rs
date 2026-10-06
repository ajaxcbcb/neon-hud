use super::{now, Usage, UsageWindow};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

pub struct CodexState {
    child: Option<Child>,
    input: Option<ChildStdin>,
    lines: Option<Receiver<Value>>,
    last: Option<Instant>,
    usage: Usage,
    sequence: u64,
}
impl Default for CodexState {
    fn default() -> Self {
        Self {
            child: None,
            input: None,
            lines: None,
            last: None,
            usage: Usage::unavailable(
                "codex",
                "Codex app-server",
                "Connect the local Codex CLI to read allowance.",
            ),
            sequence: 0,
        }
    }
}
impl CodexState {
    pub fn disconnect(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.input = None;
        self.lines = None;
        self.last = None;
        self.usage = Usage::unavailable("codex", "Codex app-server", "Disconnected.");
    }
    fn executable() -> Option<std::path::PathBuf> {
        let mut candidates = Vec::new();
        if let Ok(root) = std::env::var("LOCALAPPDATA") {
            let base = std::path::Path::new(&root).join("hermes/node/node_modules");
            candidates.push(base.join("@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe"));
            candidates.push(
                base.join("@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe"),
            );
            candidates.push(base.join("@openai/codex/vendor/x86_64-pc-windows-msvc/bin/codex.exe"));
        }
        if let Ok(path) = std::env::var("PATH") {
            for part in std::env::split_paths(&path) {
                candidates.push(part.join(if cfg!(windows) { "codex.exe" } else { "codex" }));
            }
        }
        candidates.into_iter().find(|p| p.is_file())
    }
    fn start(&mut self) -> Result<(), String> {
        let exe = Self::executable().ok_or("Codex native CLI executable was not found")?;
        let mut cmd = Command::new(exe);
        cmd.arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        let output = child.stdout.take().ok_or("Codex stdout unavailable")?;
        let input = child.stdin.take().ok_or("Codex stdin unavailable")?;
        let (tx, rx) = mpsc::sync_channel(64);
        thread::spawn(move || {
            let mut reader = BufReader::new(output);
            while let Ok(Some(line)) = read_bounded_line(&mut reader) {
                if let Ok(v) = serde_json::from_slice::<Value>(&line) {
                    if tx.send(v).is_err() {
                        break;
                    }
                }
            }
        });
        self.child = Some(child);
        self.input = Some(input);
        self.lines = Some(rx);
        self.send(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"clientInfo":{"name":"neon_hud","title":"Neon HUD","version":"0.1.0"}}}))?;
        self.response(0, Duration::from_secs(8))?;
        self.send(json!({"jsonrpc":"2.0","method":"initialized"}))?;
        Ok(())
    }
    fn send(&mut self, value: Value) -> Result<(), String> {
        let input = self.input.as_mut().ok_or("Codex not running")?;
        writeln!(input, "{}", value).map_err(|e| e.to_string())?;
        input.flush().map_err(|e| e.to_string())
    }
    fn response(&mut self, id: u64, timeout: Duration) -> Result<Value, String> {
        let lines = self.lines.as_ref().ok_or("Codex reader unavailable")?;
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err("Codex RPC timed out".into());
            }
            let line = lines
                .recv_timeout(left)
                .map_err(|_| "Codex RPC timed out or closed".to_string())?;
            if line.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = line.get("error") {
                    return Err(format!(
                        "Codex RPC error: {}",
                        error
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                    ));
                }
                return Ok(line.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }
    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.sequence += 1;
        let id = self.sequence;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        self.response(id, Duration::from_secs(8))
    }
    pub fn connect(&mut self, app: &AppHandle, login: bool) -> Result<String, String> {
        self.disconnect();
        if let Err(e) = self.start() {
            self.disconnect();
            return Err(e);
        }
        match self.request("account/read", json!({"refreshToken":false})) {
            Ok(account) if !account.get("account").unwrap_or(&Value::Null).is_null() => {
                self.refresh();
                Ok("Connected to the local Codex account.".into())
            }
            _ if login => {
                let login = self.request(
                    "account/login/start",
                    json!({"type":"chatgpt","useHostedLoginSuccessPage":true,"appBrand":"chatgpt"}),
                )?;
                let url = login
                    .get("authUrl")
                    .and_then(Value::as_str)
                    .ok_or("Codex did not return an authorization URL")?;
                if !url.starts_with("https://") {
                    return Err("Codex returned an unsafe authorization URL".into());
                }
                app.opener()
                    .open_url(url, None::<&str>)
                    .map_err(|e| e.to_string())?;
                self.usage = Usage {
                    surface: "codex".into(),
                    source: "Codex app-server".into(),
                    state: "needs-login".into(),
                    message: "Finish Codex sign-in in your browser.".into(),
                    fetched_at: None,
                    windows: vec![],
                };
                self.last = Some(Instant::now());
                Ok("Finish Codex sign-in in your browser.".into())
            }
            _ => {
                self.usage = Usage {
                    surface: "codex".into(),
                    source: "Codex app-server".into(),
                    state: "needs-login".into(),
                    message: "Sign in to Codex from Connections.".into(),
                    fetched_at: None,
                    windows: vec![],
                };
                self.last = Some(Instant::now());
                Ok("Codex sign-in is required.".into())
            }
        }
    }
    fn refresh(&mut self) {
        self.last = Some(Instant::now());
        match self.request("account/rateLimits/read", json!({})) {
            Ok(value) => {
                let windows = parse_limits(&value);
                self.usage = if windows.is_empty() {
                    Usage::unavailable(
                        "codex",
                        "Codex app-server",
                        "Codex returned no supported allowance windows.",
                    )
                } else {
                    Usage {
                        surface: "codex".into(),
                        source: "Codex app-server".into(),
                        state: "connected".into(),
                        message: "Codex account allowance".into(),
                        fetched_at: Some(now()),
                        windows,
                    }
                };
            }
            Err(e) => {
                self.usage = Usage {
                    surface: "codex".into(),
                    source: "Codex app-server".into(),
                    state: "error".into(),
                    message: e,
                    fetched_at: None,
                    windows: vec![],
                };
            }
        }
    }
    pub fn usage(&mut self) -> Usage {
        if self.child.is_some()
            && self
                .last
                .map_or(true, |t| t.elapsed() >= Duration::from_secs(60))
        {
            if self.usage.state == "needs-login" {
                self.last = Some(Instant::now());
                if self
                    .request("account/read", json!({"refreshToken":false}))
                    .ok()
                    .and_then(|v| v.get("account").cloned())
                    .is_some_and(|v| !v.is_null())
                {
                    self.refresh();
                }
            } else {
                self.refresh();
            }
        }
        self.usage.clone()
    }
}
fn read_bounded_line<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let mut too_large = false;
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            return Ok(if line.is_empty() { None } else { Some(line) });
        }
        let end = chunk.iter().position(|b| *b == b'\n').map(|i| i + 1);
        let take = end.unwrap_or(chunk.len());
        if !too_large && line.len() + take <= 1024 * 1024 {
            line.extend_from_slice(&chunk[..take]);
        } else {
            too_large = true;
            line.clear();
        }
        reader.consume(take);
        if end.is_some() {
            return Ok(Some(line));
        }
    }
}
impl Drop for CodexState {
    fn drop(&mut self) {
        self.disconnect();
    }
}
fn parse_limits(result: &Value) -> Vec<UsageWindow> {
    let limits = result
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
        .and_then(|m| m.get("codex"))
        .or_else(|| result.get("rateLimits"));
    let Some(limits) = limits else { return vec![] };
    ["primary", "secondary"]
        .iter()
        .filter_map(|key| {
            let w = limits.get(*key)?;
            let used = w.get("usedPercent")?.as_f64()?;
            let mins = w.get("windowDurationMins")?.as_u64()?;
            if !(0.0..=100.0).contains(&used) {
                return None;
            }
            Some(UsageWindow {
                label: if mins == 300 {
                    "5 hours".into()
                } else if mins == 10080 {
                    "7 days".into()
                } else {
                    format!("{} minutes", mins)
                },
                minutes: mins,
                used_percent: used,
                resets_at: w.get("resetsAt").and_then(Value::as_f64),
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limit_parsing_ignores_missing_and_out_of_range() {
        assert!(parse_limits(&json!({})).is_empty());
        assert!(parse_limits(
            &json!({"rateLimits":{"primary":{"usedPercent":120,"windowDurationMins":300}}})
        )
        .is_empty());
        assert_eq!(parse_limits(&json!({"rateLimits":{"primary":{"usedPercent":42,"windowDurationMins":300,"resetsAt":1000}}})).len(), 1);
    }
    #[test]
    fn oversized_rpc_line_is_dropped_without_losing_next_message() {
        let mut bytes = vec![b'x'; 1024 * 1024 + 1];
        bytes.extend_from_slice(b"\n{\"id\":1}\n");
        let mut reader = BufReader::new(bytes.as_slice());
        assert!(read_bounded_line(&mut reader).unwrap().unwrap().is_empty());
        assert_eq!(
            read_bounded_line(&mut reader).unwrap().unwrap(),
            b"{\"id\":1}\n"
        );
    }
}
