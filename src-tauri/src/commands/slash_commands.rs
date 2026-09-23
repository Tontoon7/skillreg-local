use super::api_error::format_api_error;
use super::config::read_config;
use super::skills::API_BASE_URL;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND_MANIFEST_VERSION: u32 = 1;
const SUPPORTED_AGENTS: [&str; 3] = ["claude", "codex", "cursor"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandVersion {
    pub id: Option<String>,
    pub version: String,
    pub content: String,
    #[serde(default)]
    pub agent_compatibility: Vec<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCommandDetail {
    pub id: Option<String>,
    pub name: String,
    pub description: String,
    pub latest_version: Option<String>,
    pub total_versions: Option<u64>,
    #[serde(default)]
    pub agent_compatibility: Vec<String>,
    pub scope: String,
    #[serde(default)]
    pub versions: Vec<CommandVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCommand {
    pub id: Option<String>,
    pub name: String,
    pub description: String,
    pub latest_version: Option<String>,
    #[serde(default)]
    pub total_versions: u64,
    #[serde(default)]
    pub agent_compatibility: Vec<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledCommandRecord {
    pub org: String,
    pub name: String,
    pub version: String,
    pub agent: String,
    pub scope: String,
    pub path: String,
    pub content_sha256: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandManifest {
    pub version: u32,
    pub commands: Vec<InstalledCommandRecord>,
}

impl Default for CommandManifest {
    fn default() -> Self {
        Self {
            version: COMMAND_MANIFEST_VERSION,
            commands: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInstallResult {
    pub org: String,
    pub name: String,
    pub version: String,
    pub scope: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandUpdateSkipped {
    #[serde(flatten)]
    pub record: InstalledCommandRecord,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandUpdateResult {
    pub updated: Vec<InstalledCommandRecord>,
    pub skipped: Vec<CommandUpdateSkipped>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRemoveResult {
    pub removed: Vec<InstalledCommandRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInstallPlan {
    pub agent: String,
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ListCommandsResponse {
    commands: Vec<RegistryCommand>,
}

#[derive(Debug, Deserialize)]
struct GetCommandResponse {
    command: RegistryCommandDetail,
}

fn get_auth_client() -> Result<(reqwest::Client, String), String> {
    let config = read_config()?;
    let token = config.token.ok_or("Not authenticated")?;
    Ok((reqwest::Client::new(), token))
}

fn command_manifest_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(home.join(".skillreg").join("commands.json"))
}

fn read_command_manifest_from_path(path: &Path) -> Result<CommandManifest, String> {
    if !path.exists() {
        return Ok(CommandManifest::default());
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut manifest: CommandManifest =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;
    if manifest.version == 0 {
        manifest.version = COMMAND_MANIFEST_VERSION;
    }
    Ok(manifest)
}

fn write_command_manifest_to_path(path: &Path, manifest: &CommandManifest) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(&CommandManifest {
        version: COMMAND_MANIFEST_VERSION,
        commands: manifest.commands.clone(),
    })
    .map_err(|e| e.to_string())?;
    fs::write(path, format!("{content}\n")).map_err(|e| e.to_string())
}

fn read_command_manifest() -> Result<CommandManifest, String> {
    read_command_manifest_from_path(&command_manifest_path()?)
}

fn write_command_manifest(manifest: &CommandManifest) -> Result<(), String> {
    write_command_manifest_to_path(&command_manifest_path()?, manifest)
}

fn same_command_record(left: &InstalledCommandRecord, right: &InstalledCommandRecord) -> bool {
    left.org == right.org
        && left.name == right.name
        && left.agent == right.agent
        && left.scope == right.scope
        && left.path == right.path
}

fn command_record_matches(
    record: &InstalledCommandRecord,
    org: Option<&str>,
    name: Option<&str>,
    agent: Option<&str>,
    scope: Option<&str>,
) -> bool {
    org.is_none_or(|value| record.org == value)
        && name.is_none_or(|value| record.name == normalize_command_name(value))
        && agent.is_none_or(|value| record.agent == value)
        && scope.is_none_or(|value| record.scope == value)
}

fn upsert_command_manifest_records(
    manifest: &mut CommandManifest,
    records: Vec<InstalledCommandRecord>,
) {
    for record in records {
        if let Some(existing) = manifest
            .commands
            .iter_mut()
            .find(|candidate| same_command_record(candidate, &record))
        {
            *existing = record;
        } else {
            manifest.commands.push(record);
        }
    }
}

fn remove_command_manifest_records(
    manifest: &mut CommandManifest,
    org: Option<&str>,
    name: Option<&str>,
    agent: Option<&str>,
    scope: Option<&str>,
) -> Vec<InstalledCommandRecord> {
    let mut removed = Vec::new();
    manifest.commands.retain(|record| {
        let matches = command_record_matches(record, org, name, agent, scope);
        if matches {
            removed.push(record.clone());
            false
        } else {
            true
        }
    });
    removed
}

fn normalize_command_name(name: &str) -> String {
    name.trim().trim_start_matches('/').to_string()
}

fn urlencoded(value: &str) -> String {
    value
        .replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
        .replace('/', "%2F")
}

fn validate_agent(agent: &str) -> Result<(), String> {
    if SUPPORTED_AGENTS.contains(&agent) {
        Ok(())
    } else {
        Err(format!(
            "Unknown agent \"{}\". Supported: {}, all",
            agent,
            SUPPORTED_AGENTS.join(", ")
        ))
    }
}

fn validate_scope(scope: &str) -> Result<(), String> {
    if scope == "project" || scope == "user" {
        Ok(())
    } else {
        Err("Command install scope must be \"project\" or \"user\"".to_string())
    }
}

fn format_yaml_string(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | ',' | '_' | '/' | '-'))
    {
        value.to_string()
    } else {
        serde_json::to_string(value).unwrap_or_else(|_| format!("\"{}\"", value))
    }
}

fn ensure_trailing_newline(value: &str) -> String {
    if value.ends_with('\n') {
        value.to_string()
    } else {
        format!("{value}\n")
    }
}

fn sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn build_claude_command_content(description: &str, content: &str) -> String {
    format!(
        "---\ndescription: {}\n---\n\n{}",
        format_yaml_string(description),
        ensure_trailing_newline(content)
    )
}

fn build_codex_skill_content(name: &str, description: &str, content: &str) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n# /{}\n\n{}",
        format_yaml_string(name),
        format_yaml_string(description),
        name,
        ensure_trailing_newline(content)
    )
}

fn build_command_file_content(agent: &str, name: &str, description: &str, content: &str) -> String {
    match agent {
        "claude" => build_claude_command_content(description, content),
        "codex" => build_codex_skill_content(name, description, content),
        _ => ensure_trailing_newline(content),
    }
}

fn select_command_version(
    command: &RegistryCommandDetail,
    requested_version: Option<&str>,
) -> Result<CommandVersion, String> {
    let requested = requested_version.unwrap_or("latest");
    if requested == "latest" {
        if let Some(latest_version) = command.latest_version.as_deref() {
            if let Some(version) = command
                .versions
                .iter()
                .find(|candidate| candidate.version == latest_version)
            {
                return Ok(version.clone());
            }
        }
        return command
            .versions
            .first()
            .cloned()
            .ok_or_else(|| "Command has no published versions".to_string());
    }

    command
        .versions
        .iter()
        .find(|candidate| candidate.version == requested)
        .cloned()
        .ok_or_else(|| format!("Command version \"{requested}\" is not available"))
}

fn resolve_command_install_agents(
    agent: &str,
    compatible_agents: &[String],
    command_name: &str,
) -> Result<Vec<String>, String> {
    let compatible = compatible_agents
        .iter()
        .filter(|candidate| SUPPORTED_AGENTS.contains(&candidate.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if compatible.is_empty() {
        return Err("Command does not declare any supported install agents".to_string());
    }

    if agent == "all" {
        return Ok(compatible);
    }

    validate_agent(agent)?;
    if !compatible.iter().any(|candidate| candidate == agent) {
        return Err(format!(
            "Command /{} is not compatible with \"{}\"",
            command_name, agent
        ));
    }
    Ok(vec![agent.to_string()])
}

fn command_relative_dir(agent: &str) -> Result<&'static str, String> {
    match agent {
        "claude" => Ok(".claude/commands"),
        "codex" => Ok(".codex/skills"),
        "cursor" => Ok(".cursor/commands"),
        _ => Err(format!("Invalid agent: {agent}")),
    }
}

fn command_base_dir(
    agent: &str,
    scope: &str,
    project_dir: Option<&str>,
    output_dir: Option<&str>,
) -> Result<PathBuf, String> {
    let relative_dir = command_relative_dir(agent)?;
    if let Some(output) = output_dir {
        return Ok(PathBuf::from(output).join(relative_dir));
    }

    match scope {
        "project" => {
            let root = project_dir.ok_or("Project directory is required for project scope")?;
            Ok(PathBuf::from(root).join(relative_dir))
        }
        "user" => {
            let home = dirs::home_dir().ok_or("Cannot find home directory")?;
            Ok(home.join(relative_dir))
        }
        _ => Err(format!("Invalid scope: {scope}")),
    }
}

fn build_command_install_plans(
    name: &str,
    description: &str,
    content: &str,
    agents: &[String],
    scope: &str,
    project_dir: Option<&str>,
    output_dir: Option<&str>,
) -> Result<Vec<CommandInstallPlan>, String> {
    validate_scope(scope)?;
    let mut plans = Vec::new();
    for agent in agents {
        validate_agent(agent)?;
        let base_dir = command_base_dir(agent, scope, project_dir, output_dir)?;
        if agent == "codex" {
            plans.push(CommandInstallPlan {
                agent: agent.clone(),
                path: base_dir.join(name).join("SKILL.md"),
                content: build_codex_skill_content(name, description, content),
            });
        } else {
            plans.push(CommandInstallPlan {
                agent: agent.clone(),
                path: base_dir.join(format!("{name}.md")),
                content: build_command_file_content(agent, name, description, content),
            });
        }
    }
    Ok(plans)
}

fn build_command_manifest_records(
    org: &str,
    name: &str,
    version: &str,
    scope: &str,
    plans: &[CommandInstallPlan],
) -> Vec<InstalledCommandRecord> {
    let installed_at = current_unix_timestamp_string();
    plans
        .iter()
        .map(|plan| InstalledCommandRecord {
            org: org.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            agent: plan.agent.clone(),
            scope: scope.to_string(),
            path: plan.path.to_string_lossy().into_owned(),
            content_sha256: sha256(&plan.content),
            installed_at: installed_at.clone(),
        })
        .collect()
}

fn current_unix_timestamp_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

async fn fetch_command_detail(
    client: &reqwest::Client,
    token: &str,
    org: &str,
    name: &str,
) -> Result<RegistryCommandDetail, String> {
    let command_name = normalize_command_name(name);
    let resp = client
        .get(format!(
            "{}/api/v1/orgs/{}/commands/{}",
            API_BASE_URL,
            org,
            urlencoded(&command_name)
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format_api_error("API error", status, &body));
    }

    resp.json::<GetCommandResponse>()
        .await
        .map(|data| data.command)
        .map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn list_commands(org: String) -> Result<Vec<RegistryCommand>, String> {
    let (client, token) = get_auth_client()?;
    let resp = client
        .get(format!("{}/api/v1/orgs/{}/commands", API_BASE_URL, org))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format_api_error("API error", status, &body));
    }

    resp.json::<ListCommandsResponse>()
        .await
        .map(|data| data.commands)
        .map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn get_command(org: String, name: String) -> Result<RegistryCommandDetail, String> {
    let (client, token) = get_auth_client()?;
    fetch_command_detail(&client, &token, &org, &name).await
}

#[tauri::command]
pub async fn pull_command(
    org: String,
    name: String,
    version: Option<String>,
    agent: String,
    scope: String,
    project_dir: Option<String>,
) -> Result<CommandInstallResult, String> {
    let (client, token) = get_auth_client()?;
    let command = fetch_command_detail(&client, &token, &org, &name).await?;
    let selected_version = select_command_version(&command, version.as_deref())?;
    let compatible_agents = if selected_version.agent_compatibility.is_empty() {
        command.agent_compatibility.clone()
    } else {
        selected_version.agent_compatibility.clone()
    };
    let agents = resolve_command_install_agents(&agent, &compatible_agents, &command.name)?;
    let install_scope = if scope.is_empty() {
        selected_version
            .scope
            .as_deref()
            .filter(|value| *value == "project" || *value == "user")
            .unwrap_or("project")
            .to_string()
    } else {
        validate_scope(&scope)?;
        scope
    };

    let plans = build_command_install_plans(
        &command.name,
        &command.description,
        &selected_version.content,
        &agents,
        &install_scope,
        project_dir.as_deref(),
        None,
    )?;

    for plan in &plans {
        if let Some(parent) = plan.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&plan.path, &plan.content).map_err(|e| e.to_string())?;
    }

    let mut manifest = read_command_manifest()?;
    let records = build_command_manifest_records(
        &org,
        &command.name,
        &selected_version.version,
        &install_scope,
        &plans,
    );
    upsert_command_manifest_records(&mut manifest, records);
    write_command_manifest(&manifest)?;

    Ok(CommandInstallResult {
        org,
        name: command.name,
        version: selected_version.version,
        scope: install_scope,
        paths: plans
            .into_iter()
            .map(|plan| plan.path.to_string_lossy().into_owned())
            .collect(),
    })
}

#[tauri::command]
pub fn list_local_commands(
    org: Option<String>,
    agent: Option<String>,
    scope: Option<String>,
) -> Result<Vec<InstalledCommandRecord>, String> {
    if let Some(value) = agent.as_deref() {
        validate_agent(value)?;
    }
    if let Some(value) = scope.as_deref() {
        validate_scope(value)?;
    }

    let manifest = read_command_manifest()?;
    Ok(manifest
        .commands
        .into_iter()
        .filter(|record| {
            command_record_matches(
                record,
                org.as_deref(),
                None,
                agent.as_deref(),
                scope.as_deref(),
            )
        })
        .collect())
}

#[tauri::command]
pub fn remove_command(
    org: String,
    name: String,
    agent: Option<String>,
    scope: Option<String>,
) -> Result<CommandRemoveResult, String> {
    if let Some(value) = agent.as_deref() {
        validate_agent(value)?;
    }
    if let Some(value) = scope.as_deref() {
        validate_scope(value)?;
    }

    let mut manifest = read_command_manifest()?;
    let removed = remove_command_manifest_records(
        &mut manifest,
        Some(&org),
        Some(&name),
        agent.as_deref(),
        scope.as_deref(),
    );

    for record in &removed {
        let path = PathBuf::from(&record.path);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    write_command_manifest(&manifest)?;

    Ok(CommandRemoveResult { removed })
}

#[tauri::command]
pub async fn update_command(
    org: Option<String>,
    name: Option<String>,
    agent: Option<String>,
    scope: Option<String>,
    version: Option<String>,
    force: Option<bool>,
) -> Result<CommandUpdateResult, String> {
    if let Some(value) = agent.as_deref() {
        validate_agent(value)?;
    }
    if let Some(value) = scope.as_deref() {
        validate_scope(value)?;
    }

    let (client, token) = get_auth_client()?;
    let mut manifest = read_command_manifest()?;
    let installed = manifest
        .commands
        .iter()
        .filter(|record| {
            command_record_matches(
                record,
                org.as_deref(),
                name.as_deref(),
                agent.as_deref(),
                scope.as_deref(),
            )
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut updated = Vec::new();
    let mut skipped = Vec::new();
    let mut command_cache: HashMap<String, RegistryCommandDetail> = HashMap::new();

    for record in installed {
        let key = format!("{}/{}", record.org, record.name);
        if !command_cache.contains_key(&key) {
            let command = fetch_command_detail(&client, &token, &record.org, &record.name).await?;
            command_cache.insert(key.clone(), command);
        }
        let command = command_cache
            .get(&key)
            .ok_or_else(|| format!("Command not found: {}", record.name))?;
        let selected_version = select_command_version(command, version.as_deref())?;
        let compatible_agents = if selected_version.agent_compatibility.is_empty() {
            command.agent_compatibility.clone()
        } else {
            selected_version.agent_compatibility.clone()
        };

        if !compatible_agents
            .iter()
            .any(|candidate| candidate == &record.agent)
        {
            skipped.push(CommandUpdateSkipped {
                record,
                reason: format!("not compatible with {}", compatible_agents.join(", ")),
            });
            continue;
        }
        if force != Some(true) && selected_version.version == record.version {
            skipped.push(CommandUpdateSkipped {
                record,
                reason: "already up to date".to_string(),
            });
            continue;
        }

        let content = build_command_file_content(
            &record.agent,
            &command.name,
            &command.description,
            &selected_version.content,
        );
        let path = PathBuf::from(&record.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, &content).map_err(|e| e.to_string())?;

        updated.push(InstalledCommandRecord {
            version: selected_version.version,
            content_sha256: sha256(&content),
            installed_at: current_unix_timestamp_string(),
            ..record
        });
    }

    if !updated.is_empty() {
        upsert_command_manifest_records(&mut manifest, updated.clone());
        write_command_manifest(&manifest)?;
    }

    Ok(CommandUpdateResult { updated, skipped })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_command() -> RegistryCommandDetail {
        RegistryCommandDetail {
            name: "review-pr".to_string(),
            description: "Review a pull request".to_string(),
            latest_version: Some("1.0.1".to_string()),
            id: None,
            total_versions: Some(2),
            agent_compatibility: vec!["claude".to_string(), "codex".to_string()],
            scope: "org".to_string(),
            versions: vec![
                CommandVersion {
                    id: None,
                    version: "1.0.0".to_string(),
                    content: "Review the diff.".to_string(),
                    agent_compatibility: vec![],
                    scope: None,
                },
                CommandVersion {
                    id: None,
                    version: "1.0.1".to_string(),
                    content: "Review the diff and tests.".to_string(),
                    agent_compatibility: vec!["claude".to_string()],
                    scope: Some("project".to_string()),
                },
            ],
        }
    }

    #[test]
    fn selects_latest_or_exact_command_version() {
        let command = sample_command();

        let latest = select_command_version(&command, None).unwrap();
        let exact = select_command_version(&command, Some("1.0.0")).unwrap();

        assert_eq!(latest.version, "1.0.1");
        assert_eq!(exact.content, "Review the diff.");
    }

    #[test]
    fn rejects_unknown_command_version() {
        let command = sample_command();

        let error = select_command_version(&command, Some("2.0.0")).unwrap_err();

        assert!(error.contains("not available"));
    }

    #[test]
    fn resolves_all_or_single_compatible_agents() {
        let compatible = vec!["claude".to_string(), "cursor".to_string()];

        assert_eq!(
            resolve_command_install_agents("all", &compatible, "review-pr").unwrap(),
            vec!["claude".to_string(), "cursor".to_string()]
        );
        assert_eq!(
            resolve_command_install_agents("cursor", &compatible, "review-pr").unwrap(),
            vec!["cursor".to_string()]
        );
    }

    #[test]
    fn rejects_incompatible_install_agent() {
        let compatible = vec!["claude".to_string()];

        let error = resolve_command_install_agents("codex", &compatible, "review-pr").unwrap_err();

        assert!(error.contains("not compatible"));
    }

    #[test]
    fn builds_agent_specific_project_install_plans() {
        let agents = vec![
            "claude".to_string(),
            "codex".to_string(),
            "cursor".to_string(),
        ];

        let plans = build_command_install_plans(
            "review-pr",
            "Review a pull request",
            "Review the diff.",
            &agents,
            "project",
            Some("/tmp/project"),
            None,
        )
        .unwrap();

        assert_eq!(plans.len(), 3);
        assert_eq!(
            plans[0].path,
            PathBuf::from("/tmp/project/.claude/commands/review-pr.md")
        );
        assert!(plans[0]
            .content
            .starts_with("---\ndescription: Review a pull request\n---"));
        assert_eq!(
            plans[1].path,
            PathBuf::from("/tmp/project/.codex/skills/review-pr/SKILL.md")
        );
        assert!(plans[1].content.contains("# /review-pr"));
        assert_eq!(
            plans[2].path,
            PathBuf::from("/tmp/project/.cursor/commands/review-pr.md")
        );
        assert_eq!(plans[2].content, "Review the diff.\n");
    }
}
