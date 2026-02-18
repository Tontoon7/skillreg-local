use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn env_dir(org: &str) -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".skillreg").join("env").join(org)
}

fn env_file(org: &str, skill: &str) -> PathBuf {
    env_dir(org).join(format!("{}.env", skill))
}

fn parse_env_file(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let key = trimmed[..eq].trim().to_string();
            let val = trimmed[eq + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !key.is_empty() {
                map.insert(key, val);
            }
        }
    }
    map
}

fn serialize_env(vars: &HashMap<String, String>) -> String {
    let mut lines: Vec<String> = vars
        .iter()
        .map(|(k, v)| {
            if v.contains(' ') || v.contains('"') || v.contains('\'') {
                format!("{}=\"{}\"", k, v.replace('"', "\\\""))
            } else {
                format!("{}={}", k, v)
            }
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

#[tauri::command]
pub fn get_env_vars(org: String, skill: String) -> Result<HashMap<String, String>, String> {
    let path = env_file(&org, &skill);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(parse_env_file(&content))
}

#[tauri::command]
pub fn set_env_vars(
    org: String,
    skill: String,
    vars: HashMap<String, String>,
) -> Result<(), String> {
    let dir = env_dir(&org);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let path = env_file(&org, &skill);

    // Merge with existing
    let mut existing = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        parse_env_file(&content)
    } else {
        HashMap::new()
    };

    for (k, v) in vars {
        existing.insert(k, v);
    }

    let content = serialize_env(&existing);
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_env_vars(org: String, skill: String, keys: Vec<String>) -> Result<(), String> {
    let path = env_file(&org, &skill);
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut vars = parse_env_file(&content);

    for key in &keys {
        vars.remove(key);
    }

    if vars.is_empty() {
        fs::remove_file(&path).map_err(|e| e.to_string())
    } else {
        let content = serialize_env(&vars);
        fs::write(&path, content).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillEnvVars {
    pub skill: String,
    pub vars: HashMap<String, String>,
}

#[tauri::command]
pub fn list_all_env_vars(org: String) -> Result<Vec<SkillEnvVars>, String> {
    let dir = env_dir(&org);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().into_owned();
        if !fname.ends_with(".env") {
            continue;
        }
        let skill = fname.trim_end_matches(".env").to_string();
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let vars = parse_env_file(&content);
        if !vars.is_empty() {
            results.push(SkillEnvVars { skill, vars });
        }
    }

    Ok(results)
}

#[tauri::command]
pub fn import_env_file(org: String, skill: String, file_path: String) -> Result<HashMap<String, String>, String> {
    let content = fs::read_to_string(&file_path).map_err(|e| format!("Cannot read file: {}", e))?;
    let vars = parse_env_file(&content);

    if !vars.is_empty() {
        set_env_vars(org, skill, vars.clone())?;
    }

    Ok(vars)
}
