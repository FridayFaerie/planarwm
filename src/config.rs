use crate::wm::slide::SlideType;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub bindings: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub startup: Vec<String>,
    #[serde(default)]
    pub window: WindowConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WindowConfig {
    #[serde(default)]
    pub force_ssd: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WorkspaceConfig {
    pub id: String,
    #[serde(default)]
    pub coord: (i32, i32),
    #[serde(default)]
    pub slides: Vec<SlideConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SlideConfig {
    #[serde(default)]
    pub slide_type: SlideType,
}

fn config_path() -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_else(|| ".".into());
    if cfg!(debug_assertions) {
        PathBuf::from(home)
            .join(".config")
            .join("river")
            .join("planarwm-debug.conf")
    } else {
        PathBuf::from(home)
            .join(".config")
            .join("river")
            .join("planarwm.conf")
    }
}

pub fn load_config() -> Config {
    let path = config_path();

    if !path.exists() {
        return Config::default();
    }

    match std::fs::read_to_string(path.to_string_lossy().as_ref()) {
        Ok(content) => match hocon_rs::Config::parse_str(&content, None) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Failed to parse config: {e}");
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("Failed to read config: {e}");
            Config::default()
        }
    }
}
