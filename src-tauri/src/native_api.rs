//! Tauri-independent entry points for the native desktop UI.
use super::{
    atomic_json, bridge, codex::CodexState, read_settings, ResourceMode, Settings, SystemMonitor,
    Usage,
};
use serde_json::Value;
use std::path::PathBuf;

pub struct NativeBackend {
    config_dir: PathBuf,
    bridge_dir: PathBuf,
    monitor: SystemMonitor,
    codex: CodexState,
}

impl NativeBackend {
    pub fn new(config_dir: PathBuf) -> Result<Self, String> {
        if config_dir.as_os_str().is_empty() {
            return Err("App config directory is empty".into());
        }
        // Claude hooks are installed once per account and their CLI writes to
        // the shared app directory. Only preferences use the preview subfolder.
        let bridge_dir = if config_dir
            .file_name()
            .is_some_and(|n| n == "native-preview")
        {
            config_dir.parent().unwrap_or(&config_dir).to_path_buf()
        } else {
            config_dir.clone()
        };
        Ok(Self {
            config_dir,
            bridge_dir,
            monitor: SystemMonitor::new(),
            codex: CodexState::default(),
        })
    }

    pub fn load_profile(&self) -> Result<Value, String> {
        let path = self.config_dir.join("settings.json");
        let mut value = serde_json::to_value(read_settings(&path)?).map_err(|e| e.to_string())?;
        if !path.exists() {
            value["size"] = Value::String("compressed".into());
        }
        Ok(value)
    }

    pub fn save_profile(&self, value: Value) -> Result<(), String> {
        let mut settings: Settings = serde_json::from_value(value).map_err(|e| e.to_string())?;
        super::normalize_settings(&mut settings);
        atomic_json(&self.config_dir.join("settings.json"), &settings)
    }

    pub fn system(&self, mode: &str, sampling_ms: u64) -> Result<Value, String> {
        serde_json::to_value(
            self.monitor
                .snapshot(ResourceMode::parse(Some(mode)), sampling_ms)?,
        )
        .map_err(|e| e.to_string())
    }

    pub fn providers(&mut self, _mode: &str) -> Result<Value, String> {
        let mut usages = vec![Usage::unavailable(
            "chatgpt",
            "No supported API",
            "ChatGPT chat allowance is unavailable through a supported local source.",
        )];
        usages.push(self.codex.usage());
        let (claude, attention) = bridge::read_provider(&self.bridge_dir);
        usages.push(claude.clone());
        usages.push(Usage {
            surface: "claude-code".into(),
            message: "Shares the Claude account allowance; no separate total is added.".into(),
            ..claude
        });
        serde_json::to_value(super::ProviderSnapshot {
            usages,
            attention,
            claude_bridge_enabled: bridge::is_enabled(),
        })
        .map_err(|e| e.to_string())
    }

    pub fn connect_codex(&mut self, login: bool) -> Result<String, String> {
        self.codex
            .connect(login, |url| open::that(url).map_err(|e| e.to_string()))
    }

    pub fn disconnect_codex(&mut self) {
        self.codex.disconnect();
    }

    pub fn install_claude_bridge(&self) -> Result<String, String> {
        bridge::install(&self.bridge_dir)
    }

    pub fn remove_claude_bridge(&self) -> Result<String, String> {
        bridge::remove(&self.bridge_dir)
    }

    pub fn dismiss_attention(&self, id: &str) -> Result<(), String> {
        bridge::dismiss(&self.bridge_dir, id)
    }
}

pub fn run_bridge_cli() -> bool {
    bridge::run_cli_if_requested()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_roundtrip_and_provider_shape_without_tauri() {
        let dir = std::env::temp_dir().join(format!(
            "neon-native-api-{}-{}",
            std::process::id(),
            super::super::now().to_bits()
        ));
        let mut backend = NativeBackend::new(dir.clone()).unwrap();
        let mut profile = backend.load_profile().unwrap();
        assert_eq!(profile["size"], "compressed");
        profile["theme"] = Value::String("aurora".into());
        backend.save_profile(profile).unwrap();
        assert_eq!(backend.load_profile().unwrap()["theme"], "aurora");
        let providers = backend.providers("normal").unwrap();
        let usages = providers["usages"].as_array().unwrap();
        assert_eq!(usages.len(), 4);
        assert_eq!(usages[0]["surface"], "chatgpt");
        assert_eq!(usages[1]["surface"], "codex");
        assert_eq!(usages[2]["surface"], "claude");
        assert_eq!(usages[3]["surface"], "claude-code");
        assert!(providers["attention"].is_array());
        assert!(providers["claudeBridgeEnabled"].is_boolean());
        let original = std::fs::read(dir.join("settings.json")).unwrap();
        let preview = NativeBackend::new(dir.join("native-preview")).unwrap();
        assert_eq!(preview.bridge_dir, dir);
        let mut profile = preview.load_profile().unwrap();
        profile["theme"] = Value::String("cyberpunk".into());
        preview.save_profile(profile).unwrap();
        assert_eq!(std::fs::read(dir.join("settings.json")).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
