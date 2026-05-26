use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub const DEFAULT_AUTO_UPDATE_ENABLED: bool = true;
pub const DEFAULT_AUTO_UPDATE_INTERVAL_MINUTES: u64 = 60;
pub const DEFAULT_LAUNCH_AT_LOGIN: bool = true;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillregConfig {
    pub token: Option<String>,
    pub api_url: Option<String>,
    pub org: Option<String>,
    pub default_agent: Option<String>,
    pub default_scope: Option<String>,
    pub setup_done: Option<bool>,
    pub auto_update_enabled: Option<bool>,
    pub auto_update_interval_minutes: Option<u64>,
    pub launch_at_login: Option<bool>,
}

impl SkillregConfig {
    pub fn auto_update_enabled_value(&self) -> bool {
        self.auto_update_enabled
            .unwrap_or(DEFAULT_AUTO_UPDATE_ENABLED)
    }

    pub fn auto_update_interval_minutes_value(&self) -> u64 {
        self.auto_update_interval_minutes
            .unwrap_or(DEFAULT_AUTO_UPDATE_INTERVAL_MINUTES)
            .clamp(15, 24 * 60)
    }

    pub fn launch_at_login_value(&self) -> bool {
        self.launch_at_login.unwrap_or(DEFAULT_LAUNCH_AT_LOGIN)
    }
}

fn config_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".skillreg").join("config.json")
}

#[tauri::command]
pub fn read_config() -> Result<SkillregConfig, String> {
    let path = config_path();
    if !path.exists() {
        return Ok(SkillregConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let config: SkillregConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn write_config(config: SkillregConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn apply_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())?;
    } else {
        autolaunch.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut config = read_config()?;
    apply_launch_at_login(&app, enabled)?;
    config.launch_at_login = Some(enabled);
    write_config(config)
}
