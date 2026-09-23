use super::{
    api_error::format_api_error,
    config::read_config,
    installed_manifest::{
        read_installed_manifest, upsert_tracked_installation, TrackedInstallation,
    },
    local::compute_content_hash,
    skills::{install_skill_from_registry, PaginatedSkills, API_BASE_URL},
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Mutex;

const REGISTRY_PAGE_SIZE: u32 = 200;
const INITIAL_WORKER_DELAY_SECONDS: u64 = 2 * 60;
const MAX_JITTER_SECONDS: u64 = 5 * 60;

static AUTO_UPDATE_RUN_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateRunSummary {
    pub checked: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub updated_skills: Vec<AutoUpdatedSkill>,
    pub skipped_skills: Vec<AutoUpdateSkippedSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdatedSkill {
    pub name: String,
    pub agent: String,
    pub scope: String,
    pub old_version: String,
    pub new_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateSkippedSkill {
    pub name: String,
    pub agent: String,
    pub scope: String,
    pub version: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoUpdateDecision {
    Update,
    Skip(&'static str),
}

pub fn should_update_installation(
    global_enabled: bool,
    installation: &TrackedInstallation,
    local_content_hash: Option<&str>,
    server_version: Option<&str>,
) -> AutoUpdateDecision {
    if !global_enabled {
        return AutoUpdateDecision::Skip("auto updates disabled");
    }

    if installation.auto_update_enabled == Some(false) {
        return AutoUpdateDecision::Skip("auto update disabled for skill");
    }

    let Some(local_hash) = local_content_hash else {
        return AutoUpdateDecision::Skip("local skill missing");
    };

    if local_hash != installation.content_hash {
        return AutoUpdateDecision::Skip("local skill modified");
    }

    let Some(server_version) = server_version else {
        return AutoUpdateDecision::Skip("not found in registry");
    };

    if !is_newer_version(server_version, &installation.version) {
        return AutoUpdateDecision::Skip("already up to date");
    }

    AutoUpdateDecision::Update
}

pub async fn run_auto_update_once() -> Result<AutoUpdateRunSummary, String> {
    let lock = AUTO_UPDATE_RUN_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone();
    let _guard = lock.lock().await;
    run_auto_update_once_inner().await
}

#[tauri::command]
pub async fn run_auto_update_now() -> Result<AutoUpdateRunSummary, String> {
    run_auto_update_once().await
}

pub fn spawn_auto_update_worker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let jitter = deterministic_jitter();
        tokio::time::sleep(Duration::from_secs(INITIAL_WORKER_DELAY_SECONDS) + jitter).await;

        loop {
            let interval = read_config()
                .map(|config| config.auto_update_interval_minutes_value())
                .unwrap_or(super::config::DEFAULT_AUTO_UPDATE_INTERVAL_MINUTES);

            if read_config()
                .map(|config| config.auto_update_enabled_value())
                .unwrap_or(super::config::DEFAULT_AUTO_UPDATE_ENABLED)
            {
                if let Ok(summary) = run_auto_update_once().await {
                    let _ = app.emit("auto-update:completed", summary.clone());
                    notify_auto_update_completed(&app, &summary);
                }
            }

            tokio::time::sleep(Duration::from_secs(interval * 60) + deterministic_jitter()).await;
        }
    });
}

async fn run_auto_update_once_inner() -> Result<AutoUpdateRunSummary, String> {
    let config = read_config()?;
    if !config.auto_update_enabled_value() {
        return Ok(AutoUpdateRunSummary::default());
    }

    let Some(token) = config.token.as_deref() else {
        return Ok(AutoUpdateRunSummary::default());
    };

    let manifest = read_installed_manifest()?;
    if manifest.installations.is_empty() {
        return Ok(AutoUpdateRunSummary::default());
    }

    let client = reqwest::Client::new();
    let mut summary = AutoUpdateRunSummary::default();
    let orgs: HashSet<String> = manifest
        .installations
        .iter()
        .map(|installation| installation.org.clone())
        .collect();
    let mut registry_versions_by_org: HashMap<String, Result<HashMap<String, String>, String>> =
        HashMap::new();

    for org in orgs {
        let result = fetch_latest_versions_for_org(&client, token, &org).await;
        registry_versions_by_org.insert(org, result);
    }

    for installation in manifest.installations {
        summary.checked += 1;

        let Some(registry_result) = registry_versions_by_org.get(&installation.org) else {
            record_installation_check(installation.clone(), Some("Registry metadata unavailable"));
            record_failed(&mut summary, &installation, "Registry metadata unavailable");
            continue;
        };

        let registry_versions = match registry_result {
            Ok(versions) => versions,
            Err(error) => {
                record_installation_check(installation.clone(), Some(error));
                record_failed(&mut summary, &installation, error);
                continue;
            }
        };

        let local_hash = read_local_skill_hash(&installation.install_path).ok();
        let server_version = registry_versions
            .get(&installation.name)
            .map(String::as_str);

        match should_update_installation(
            config.auto_update_enabled_value(),
            &installation,
            local_hash.as_deref(),
            server_version,
        ) {
            AutoUpdateDecision::Skip(reason) => {
                record_installation_check(installation.clone(), None);
                summary.skipped += 1;
                summary.skipped_skills.push(AutoUpdateSkippedSkill {
                    name: installation.name.clone(),
                    agent: installation.agent.clone(),
                    scope: installation.scope.clone(),
                    version: installation.version.clone(),
                    reason: reason.to_string(),
                });
            }
            AutoUpdateDecision::Update => {
                let new_version = server_version.unwrap_or_default().to_string();
                let old_version = installation.version.clone();
                match install_skill_from_registry(
                    installation.org.clone(),
                    installation.name.clone(),
                    Some(new_version.clone()),
                    installation.agent.clone(),
                    installation.scope.clone(),
                    installation.project_dir.clone(),
                )
                .await
                {
                    Ok(_) => {
                        summary.updated += 1;
                        summary.updated_skills.push(AutoUpdatedSkill {
                            name: installation.name.clone(),
                            agent: installation.agent.clone(),
                            scope: installation.scope.clone(),
                            old_version,
                            new_version,
                        });
                    }
                    Err(error) => {
                        record_installation_check(installation.clone(), Some(&error));
                        record_failed(&mut summary, &installation, &error);
                    }
                }
            }
        }
    }

    Ok(summary)
}

async fn fetch_latest_versions_for_org(
    client: &reqwest::Client,
    token: &str,
    org: &str,
) -> Result<HashMap<String, String>, String> {
    let mut versions = HashMap::new();
    let mut page = 1;

    loop {
        let response = client
            .get(format!(
                "{}/api/v1/orgs/{}/skills?page={}&limit={}",
                API_BASE_URL, org, page, REGISTRY_PAGE_SIZE
            ))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format_api_error("API error", status, &body));
        }

        let page_data = response
            .json::<PaginatedSkills>()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        for skill in page_data.skills {
            if let Some(version) = skill.latest_version {
                versions.insert(skill.name, version);
            }
        }

        if page >= page_data.pagination.total_pages {
            break;
        }
        page += 1;
    }

    Ok(versions)
}

fn read_local_skill_hash(install_path: &str) -> Result<String, String> {
    let skill_md_path = Path::new(install_path).join("SKILL.md");
    if !skill_md_path.exists() {
        return Err("local SKILL.md missing".to_string());
    }
    let content = fs::read_to_string(skill_md_path).map_err(|e| e.to_string())?;
    Ok(compute_content_hash(&content))
}

fn record_installation_check(mut installation: TrackedInstallation, error: Option<&str>) {
    installation.last_checked_at = Some(current_unix_timestamp_string());
    installation.last_error = error.map(ToString::to_string);
    let _ = upsert_tracked_installation(installation);
}

fn record_failed(
    summary: &mut AutoUpdateRunSummary,
    installation: &TrackedInstallation,
    error: &str,
) {
    summary.failed += 1;
    summary.skipped_skills.push(AutoUpdateSkippedSkill {
        name: installation.name.clone(),
        agent: installation.agent.clone(),
        scope: installation.scope.clone(),
        version: installation.version.clone(),
        reason: error.to_string(),
    });
}

fn notify_auto_update_completed(app: &AppHandle, summary: &AutoUpdateRunSummary) {
    if summary.updated == 0 {
        return;
    }

    let title = if summary.updated == 1 {
        "Skill updated".to_string()
    } else {
        format!("{} skills updated", summary.updated)
    };
    let body = if summary.updated == 1 {
        summary
            .updated_skills
            .first()
            .map(|skill| format!("{} updated to v{}", skill.name, skill.new_version))
            .unwrap_or_else(|| "SkillReg updated a skill".to_string())
    } else {
        let names = summary
            .updated_skills
            .iter()
            .take(3)
            .map(|skill| skill.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        if summary.updated > 3 {
            format!("{names} and {} more", summary.updated - 3)
        } else {
            names
        }
    };

    let _ = app.notification().builder().title(title).body(body).show();
}

fn is_newer_version(server_version: &str, current_version: &str) -> bool {
    match compare_versions(server_version, current_version) {
        Some(Ordering::Greater) => true,
        Some(_) => false,
        None => server_version != current_version,
    }
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    let left_parts = parse_version_parts(left)?;
    let right_parts = parse_version_parts(right)?;
    let max_len = left_parts.len().max(right_parts.len());

    for index in 0..max_len {
        let left_part = left_parts.get(index).copied().unwrap_or(0);
        let right_part = right_parts.get(index).copied().unwrap_or(0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => {}
            ordering => return Some(ordering),
        }
    }

    Some(Ordering::Equal)
}

fn parse_version_parts(version: &str) -> Option<Vec<u64>> {
    let normalized = version.trim().trim_start_matches('v');
    let core = normalized
        .split(['-', '+'])
        .next()
        .unwrap_or(normalized)
        .trim();
    if core.is_empty() {
        return None;
    }

    core.split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

fn deterministic_jitter() -> Duration {
    Duration::from_secs(current_unix_timestamp() % MAX_JITTER_SECONDS)
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn current_unix_timestamp_string() -> String {
    current_unix_timestamp().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked(version: &str) -> TrackedInstallation {
        TrackedInstallation {
            org: "kairia".to_string(),
            name: "deploy-helper".to_string(),
            version: version.to_string(),
            agent: "codex".to_string(),
            scope: "user".to_string(),
            project_dir: None,
            install_path: "/tmp/deploy-helper".to_string(),
            content_hash: "abc123".to_string(),
            source_org: None,
            sha256: None,
            auto_update_enabled: None,
            last_checked_at: None,
            last_updated_at: None,
            last_error: None,
        }
    }

    #[test]
    fn global_disabled_skips_installation() {
        assert_eq!(
            should_update_installation(false, &tracked("1.0.0"), Some("abc123"), Some("1.1.0")),
            AutoUpdateDecision::Skip("auto updates disabled")
        );
    }

    #[test]
    fn per_skill_disabled_skips_installation() {
        let mut installation = tracked("1.0.0");
        installation.auto_update_enabled = Some(false);

        assert_eq!(
            should_update_installation(true, &installation, Some("abc123"), Some("1.1.0")),
            AutoUpdateDecision::Skip("auto update disabled for skill")
        );
    }

    #[test]
    fn local_hash_mismatch_skips_installation() {
        assert_eq!(
            should_update_installation(true, &tracked("1.0.0"), Some("modified"), Some("1.1.0")),
            AutoUpdateDecision::Skip("local skill modified")
        );
    }

    #[test]
    fn same_version_skips_installation() {
        assert_eq!(
            should_update_installation(true, &tracked("1.0.0"), Some("abc123"), Some("1.0.0")),
            AutoUpdateDecision::Skip("already up to date")
        );
    }

    #[test]
    fn newer_server_version_updates_installation() {
        assert_eq!(
            should_update_installation(true, &tracked("1.0.0"), Some("abc123"), Some("1.1.0")),
            AutoUpdateDecision::Update
        );
    }
}
