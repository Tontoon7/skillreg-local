use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub const INSTALLED_MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledManifest {
    pub version: u32,
    pub installations: Vec<TrackedInstallation>,
}

impl Default for InstalledManifest {
    fn default() -> Self {
        Self {
            version: INSTALLED_MANIFEST_VERSION,
            installations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedInstallation {
    pub org: String,
    pub name: String,
    pub version: String,
    pub agent: String,
    pub scope: String,
    pub project_dir: Option<String>,
    pub install_path: String,
    pub content_hash: String,
    /// Publisher org when the skill came from the public catalog rather than
    /// the user's own registry. `None` for private installs.
    #[serde(default)]
    pub source_org: Option<String>,
    pub sha256: Option<String>,
    pub auto_update_enabled: Option<bool>,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<String>,
}

pub fn manifest_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".skillreg").join("installed.json")
}

pub fn read_installed_manifest() -> Result<InstalledManifest, String> {
    read_installed_manifest_from_path(&manifest_path())
}

pub fn write_installed_manifest(manifest: InstalledManifest) -> Result<(), String> {
    write_installed_manifest_to_path(&manifest_path(), manifest)
}

fn read_installed_manifest_from_path(path: &PathBuf) -> Result<InstalledManifest, String> {
    if !path.exists() {
        return Ok(InstalledManifest::default());
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut manifest: InstalledManifest =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;
    if manifest.version == 0 {
        manifest.version = INSTALLED_MANIFEST_VERSION;
    }
    Ok(manifest)
}

fn write_installed_manifest_to_path(
    path: &PathBuf,
    manifest: InstalledManifest,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(&InstalledManifest {
        version: INSTALLED_MANIFEST_VERSION,
        installations: manifest.installations,
    })
    .map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

fn matches_installation_key(left: &TrackedInstallation, right: &TrackedInstallation) -> bool {
    left.org == right.org
        && left.name == right.name
        && left.agent == right.agent
        && left.scope == right.scope
        && left.project_dir == right.project_dir
}

fn matches_uninstall_key(
    installation: &TrackedInstallation,
    name: &str,
    agent: &str,
    scope: &str,
    project_dir: Option<&str>,
) -> bool {
    installation.name == name
        && installation.agent == agent
        && installation.scope == scope
        && installation.project_dir.as_deref() == project_dir
}

pub fn upsert_tracked_installation(entry: TrackedInstallation) -> Result<(), String> {
    upsert_tracked_installation_in_path(&manifest_path(), entry)
}

fn upsert_tracked_installation_in_path(
    path: &PathBuf,
    entry: TrackedInstallation,
) -> Result<(), String> {
    let mut manifest = read_installed_manifest_from_path(path)?;
    if let Some(existing) = manifest
        .installations
        .iter_mut()
        .find(|installation| matches_installation_key(installation, &entry))
    {
        *existing = entry;
    } else {
        manifest.installations.push(entry);
    }
    write_installed_manifest_to_path(path, manifest)
}

pub fn remove_tracked_installation(
    name: &str,
    agent: &str,
    scope: &str,
    project_dir: Option<&str>,
) -> Result<(), String> {
    remove_tracked_installation_in_path(&manifest_path(), name, agent, scope, project_dir)
}

fn remove_tracked_installation_in_path(
    path: &PathBuf,
    name: &str,
    agent: &str,
    scope: &str,
    project_dir: Option<&str>,
) -> Result<(), String> {
    let mut manifest = read_installed_manifest_from_path(path)?;
    manifest.installations.retain(|installation| {
        !matches_uninstall_key(installation, name, agent, scope, project_dir)
    });
    write_installed_manifest_to_path(path, manifest)
}

#[tauri::command]
pub fn list_tracked_installations() -> Result<Vec<TrackedInstallation>, String> {
    Ok(read_installed_manifest()?.installations)
}

#[tauri::command]
pub fn set_skill_auto_update(
    org: String,
    name: String,
    agent: String,
    scope: String,
    project_dir: Option<String>,
    enabled: bool,
) -> Result<(), String> {
    let mut manifest = read_installed_manifest()?;
    let installation = manifest
        .installations
        .iter_mut()
        .find(|installation| {
            installation.org == org
                && installation.name == name
                && installation.agent == agent
                && installation.scope == scope
                && installation.project_dir == project_dir
        })
        .ok_or_else(|| format!("Tracked installation not found: {name}"))?;

    installation.auto_update_enabled = Some(enabled);
    write_installed_manifest(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(org: &str, name: &str, agent: &str, scope: &str) -> TrackedInstallation {
        TrackedInstallation {
            org: org.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            agent: agent.to_string(),
            scope: scope.to_string(),
            project_dir: None,
            install_path: format!("/tmp/{name}"),
            content_hash: "abc123".to_string(),
            source_org: None,
            sha256: Some("tarball-sha".to_string()),
            auto_update_enabled: None,
            last_checked_at: None,
            last_updated_at: None,
            last_error: None,
        }
    }

    fn temp_manifest_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "skillreg-installed-manifest-{name}-{}.json",
            std::process::id()
        ))
    }

    #[test]
    fn missing_manifest_returns_empty_default() {
        let path = temp_manifest_path("missing");
        let _ = fs::remove_file(&path);

        let manifest = read_installed_manifest_from_path(&path).unwrap();

        assert_eq!(manifest.version, INSTALLED_MANIFEST_VERSION);
        assert!(manifest.installations.is_empty());
    }

    #[test]
    fn upsert_replaces_existing_installation_key() {
        let path = temp_manifest_path("upsert");
        let _ = fs::remove_file(&path);
        let entry = sample_entry("kairia", "deploy-helper", "codex", "user");
        let mut replacement = entry.clone();
        replacement.version = "1.1.0".to_string();
        replacement.content_hash = "def456".to_string();

        upsert_tracked_installation_in_path(&path, entry).unwrap();
        upsert_tracked_installation_in_path(&path, replacement).unwrap();

        let manifest = read_installed_manifest_from_path(&path).unwrap();
        assert_eq!(manifest.installations.len(), 1);
        assert_eq!(manifest.installations[0].version, "1.1.0");
        assert_eq!(manifest.installations[0].content_hash, "def456");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn remove_deletes_only_matching_installation() {
        let path = temp_manifest_path("remove");
        let _ = fs::remove_file(&path);
        let target = sample_entry("kairia", "deploy-helper", "codex", "user");
        let other = sample_entry("kairia", "review-helper", "codex", "user");

        upsert_tracked_installation_in_path(&path, target).unwrap();
        upsert_tracked_installation_in_path(&path, other).unwrap();
        remove_tracked_installation_in_path(&path, "deploy-helper", "codex", "user", None).unwrap();

        let manifest = read_installed_manifest_from_path(&path).unwrap();
        assert_eq!(manifest.installations.len(), 1);
        assert_eq!(manifest.installations[0].name, "review-helper");
        let _ = fs::remove_file(path);
    }
}
