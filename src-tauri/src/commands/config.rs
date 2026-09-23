use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

pub const DEFAULT_AUTO_UPDATE_ENABLED: bool = true;
pub const DEFAULT_AUTO_UPDATE_INTERVAL_MINUTES: u64 = 60;
pub const DEFAULT_LAUNCH_AT_LOGIN: bool = false;
static CONFIG_WRITE_LOCK: Mutex<()> = Mutex::new(());

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
    read_config_at(&config_path())
}

fn read_config_at(path: &Path) -> Result<SkillregConfig, String> {
    if !path.exists() {
        return Ok(SkillregConfig::default());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config: SkillregConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn write_config(config: SkillregConfig) -> Result<(), String> {
    write_config_at(&config_path(), &config)
}

fn write_config_at(path: &Path, config: &SkillregConfig) -> Result<(), String> {
    let temporary = config_temp_path(path);
    write_config_at_with_temp(path, &temporary, config)
}

fn write_config_at_with_temp(
    path: &Path,
    temporary: &Path,
    config: &SkillregConfig,
) -> Result<(), String> {
    let _guard = CONFIG_WRITE_LOCK
        .lock()
        .map_err(|_| "Configuration write lock is unavailable".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "Configuration path has no parent".to_string())?;
    create_private_directory(parent)?;
    let content = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;

    let write_result = (|| {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(temporary).map_err(|error| error.to_string())?;
        file.write_all(&content)
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        replace_config_file(temporary, path).map_err(|error| error.to_string())?;
        sync_parent_directory(parent)?;
        Ok(())
    })();

    if write_result.is_err() && temporary.is_file() {
        let _ = fs::remove_file(temporary);
    }
    write_result
}

fn config_temp_path(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("config.json");
    path.with_file_name(format!(".{filename}.{}.tmp", Uuid::new_v4()))
}

fn create_private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_config_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(target_os = "windows")]
fn replace_config_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), String> {
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

#[tauri::command]
pub fn set_auto_update_enabled(enabled: bool) -> Result<(), String> {
    let mut config = read_config()?;
    config.auto_update_enabled = Some(enabled);
    write_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_at_login_defaults_to_opt_in_disabled() {
        let config = SkillregConfig::default();
        assert!(!config.launch_at_login_value());
    }

    #[test]
    fn launch_at_login_honors_explicit_value() {
        let enabled = SkillregConfig {
            launch_at_login: Some(true),
            ..Default::default()
        };
        assert!(enabled.launch_at_login_value());

        let disabled = SkillregConfig {
            launch_at_login: Some(false),
            ..Default::default()
        };
        assert!(!disabled.launch_at_login_value());
    }

    #[test]
    fn auto_update_is_global_and_defaults_to_enabled() {
        assert!(SkillregConfig::default().auto_update_enabled_value());
        assert!(!SkillregConfig {
            auto_update_enabled: Some(false),
            ..Default::default()
        }
        .auto_update_enabled_value());
    }

    #[test]
    fn config_write_is_atomic_and_private() {
        let root = std::env::temp_dir().join(format!("skillreg-config-atomic-{}", Uuid::new_v4()));
        let path = root.join(".skillreg/config.json");
        let first = SkillregConfig {
            token: Some("first-token".to_string()),
            org: Some("acme".to_string()),
            ..Default::default()
        };
        write_config_at(&path, &first).unwrap();

        let second = SkillregConfig {
            token: Some("second-token".to_string()),
            org: Some("globex".to_string()),
            ..Default::default()
        };
        write_config_at(&path, &second).unwrap();

        assert_eq!(
            read_config_at(&path).unwrap().org.as_deref(),
            Some("globex")
        );
        assert!(fs::read_dir(path.parent().unwrap())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_write_failure_preserves_the_previous_file() {
        let root = std::env::temp_dir().join(format!("skillreg-config-failure-{}", Uuid::new_v4()));
        let path = root.join(".skillreg/config.json");
        let original = SkillregConfig {
            token: Some("keep-this-token".to_string()),
            org: Some("acme".to_string()),
            ..Default::default()
        };
        write_config_at(&path, &original).unwrap();
        let original_bytes = fs::read(&path).unwrap();
        let blocked_temp = path.parent().unwrap().join(".blocked-config.tmp");
        fs::create_dir_all(&blocked_temp).unwrap();

        let error = write_config_at_with_temp(
            &path,
            &blocked_temp,
            &SkillregConfig {
                org: Some("globex".to_string()),
                ..Default::default()
            },
        );

        assert!(error.is_err());
        assert_eq!(fs::read(&path).unwrap(), original_bytes);
        fs::remove_dir_all(root).unwrap();
    }
}
