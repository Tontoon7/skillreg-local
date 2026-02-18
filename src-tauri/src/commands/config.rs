use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillregConfig {
    pub token: Option<String>,
    pub api_url: Option<String>,
    pub org: Option<String>,
    pub default_agent: Option<String>,
    pub default_scope: Option<String>,
    pub setup_done: Option<bool>,
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
