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
pub struct PublishCommandVersionInput {
    pub version: String,
    pub content: String,
    pub agent_compatibility: Vec<String>,
    pub scope: String,
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

#[derive(Debug, Deserialize)]
struct PublishCommandVersionResponse {
    version: CommandVersion,
}

fn get_auth_client(builder: reqwest::ClientBuilder) -> Result<(reqwest::Client, String), String> {
    let config = read_config()?;
    let token = config.token.ok_or("Not authenticated")?;
    let client = builder
        .build()
        .map_err(|e| format!("Cannot create API client: {e}"))?;
    Ok((client, token))
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

fn validate_command_publication(
    org: &str,
    name: &str,
    input: &PublishCommandVersionInput,
) -> Result<(), String> {
    let error = |message: &str| format!("Command version publish failed: {message}");
    if org.trim().is_empty() || matches!(org, "." | "..") {
        return Err(error("A valid organization is required"));
    }
    let name = normalize_command_name(name);
    if name.is_empty() || matches!(name.as_str(), "." | "..") {
        return Err(error("A valid command name is required"));
    }
    if trim_command_publication_text(&input.version).is_empty() {
        return Err(error("Version is required"));
    }
    let content_length = trim_command_publication_text(&input.content)
        .encode_utf16()
        .count();
    if content_length == 0 || content_length > 20_000 {
        return Err(error(
            "Content must contain between 1 and 20,000 UTF-16 code units",
        ));
    }
    if input.agent_compatibility.is_empty() || input.agent_compatibility.len() > 3 {
        return Err(error("Select between one and three compatible agents"));
    }
    for (index, agent) in input.agent_compatibility.iter().enumerate() {
        if !SUPPORTED_AGENTS.contains(&agent.as_str()) {
            return Err(error("Compatible agents must be claude, codex or cursor"));
        }
        if input.agent_compatibility[..index].contains(agent) {
            return Err(error("Compatible agents must not contain duplicates"));
        }
    }
    if !matches!(input.scope.as_str(), "org" | "project" | "user") {
        return Err(error("Command scope must be org, project or user"));
    }
    Ok(())
}

fn trim_command_publication_text(value: &str) -> &str {
    // ECMAScript trim includes BOM but excludes NEXT LINE, unlike Rust's trim.
    value.trim_matches(|character: char| {
        (character.is_whitespace() && character != '\u{0085}') || character == '\u{feff}'
    })
}

fn command_publication_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .retry(reqwest::retry::never())
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
}

async fn publish_command_version_with_client(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    org: &str,
    name: &str,
    mut input: PublishCommandVersionInput,
) -> Result<CommandVersion, String> {
    validate_command_publication(org, name, &input)?;
    input.version = trim_command_publication_text(&input.version).to_string();
    let name = normalize_command_name(name);
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|e| format!("Command version publish failed: Invalid API URL: {e}"))?;
    url.path_segments_mut()
        .map_err(|_| "Command version publish failed: Invalid API URL".to_string())?
        .pop_if_empty()
        .extend(["api", "v1", "orgs", org, "commands", &name, "versions"]);
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(&input)
        .send()
        .await
        .map_err(|e| format!("Command version publish failed: Network error: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format_api_error(
            "Command version publish failed",
            status,
            &body,
        ));
    }
    response
        .json::<PublishCommandVersionResponse>()
        .await
        .map(|data| data.version)
        .map_err(|e| format!("Command version publish failed: Invalid response: {e}"))
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
    let (client, token) = get_auth_client(reqwest::Client::builder())?;
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
    let (client, token) = get_auth_client(reqwest::Client::builder())?;
    fetch_command_detail(&client, &token, &org, &name).await
}

#[tauri::command]
pub async fn publish_command_version(
    org: String,
    name: String,
    input: PublishCommandVersionInput,
) -> Result<CommandVersion, String> {
    let (client, token) = get_auth_client(command_publication_client_builder())?;
    publish_command_version_with_client(&client, API_BASE_URL, &token, &org, &name, input).await
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
    let (client, token) = get_auth_client(reqwest::Client::builder())?;
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

    let (client, token) = get_auth_client(reqwest::Client::builder())?;
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
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    struct PublishServer {
        base_url: String,
        stop: oneshot::Sender<()>,
        task: tokio::task::JoinHandle<Vec<String>>,
    }

    impl PublishServer {
        async fn start(response: Option<(u16, &str)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let response = response.map(|(status, body)| {
                format!(
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nLocation: /redirected\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
            });
            let (stop, mut stopped) = oneshot::channel();
            let task = tokio::spawn(async move {
                let mut requests = Vec::new();
                loop {
                    let (mut stream, _) = tokio::select! {
                        _ = &mut stopped => break,
                        connection = listener.accept() => connection.unwrap(),
                    };
                    let request = tokio::time::timeout(Duration::from_secs(2), async {
                        let mut bytes = Vec::new();
                        loop {
                            let mut buffer = [0_u8; 4096];
                            let read = stream.read(&mut buffer).await.unwrap();
                            assert_ne!(read, 0, "Connection closed before request was complete");
                            bytes.extend_from_slice(&buffer[..read]);
                            assert!(bytes.len() < 200_000);
                            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n")
                            {
                                let headers = String::from_utf8_lossy(&bytes[..end]);
                                let length = headers
                                    .lines()
                                    .find_map(|line| {
                                        let (name, value) = line.split_once(':')?;
                                        name.eq_ignore_ascii_case("content-length")
                                            .then(|| value.trim().parse::<usize>().unwrap())
                                    })
                                    .unwrap();
                                if bytes.len() >= end + 4 + length {
                                    break String::from_utf8(bytes).unwrap();
                                }
                            }
                        }
                    })
                    .await
                    .expect("Timed out reading mock request");
                    requests.push(request);
                    if let Some(response) = &response {
                        tokio::time::timeout(
                            Duration::from_secs(2),
                            stream.write_all(response.as_bytes()),
                        )
                        .await
                        .unwrap()
                        .unwrap();
                    }
                }
                requests
            });
            Self {
                base_url,
                stop,
                task,
            }
        }

        async fn finish(self) -> Vec<String> {
            self.stop.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(2), self.task)
                .await
                .unwrap()
                .unwrap()
        }
    }

    fn publish_input() -> PublishCommandVersionInput {
        PublishCommandVersionInput {
            version: " 1.2.3-beta.1+build.2 ".to_string(),
            content: "  Réviser le diff 🦀.\n\n  Preserve indentation.\n".to_string(),
            agent_compatibility: vec!["claude".to_string(), "codex".to_string()],
            scope: "org".to_string(),
        }
    }

    fn publish_client() -> reqwest::Client {
        command_publication_client_builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap()
    }

    #[test]
    fn validates_publication_content_in_utf16_after_trimming() {
        let mut input = publish_input();
        for content in ["é".repeat(20_000), "🦀".repeat(10_000)] {
            input.content = format!("\n {content} \n");
            assert!(validate_command_publication("acme", "review", &input).is_ok());
            input.content = format!("{content}x");
            assert!(validate_command_publication("acme", "review", &input)
                .unwrap_err()
                .contains("20,000"));
        }
        for scope in ["org", "project", "user"] {
            input.content = "Content".to_string();
            input.scope = scope.to_string();
            assert!(validate_command_publication("acme", "review", &input).is_ok());
        }
    }

    #[tokio::test]
    async fn invalid_publication_inputs_send_no_requests() {
        let server = PublishServer::start(None).await;
        let client = publish_client();
        let mut cases = vec![
            ("", "review", publish_input()),
            (" \n", "review", publish_input()),
            ("acme", " / ", publish_input()),
            ("..", "review", publish_input()),
            ("acme", " /.. ", publish_input()),
        ];
        let mut input = publish_input();
        input.version = " \n".to_string();
        cases.push(("acme", "review", input));
        for content in [" \n\t".to_string(), "🦀".repeat(10_001)] {
            let mut input = publish_input();
            input.content = content;
            cases.push(("acme", "review", input));
        }
        for agents in [vec![], vec!["all"], vec!["claude"; 4], vec!["codex"; 2]] {
            let mut input = publish_input();
            input.agent_compatibility = agents.into_iter().map(str::to_string).collect();
            cases.push(("acme", "review", input));
        }
        for scope in ["", "all"] {
            let mut input = publish_input();
            input.scope = scope.to_string();
            cases.push(("acme", "review", input));
        }
        for (org, name, input) in cases {
            let error = publish_command_version_with_client(
                &client,
                &server.base_url,
                "fake-token",
                org,
                name,
                input,
            )
            .await
            .unwrap_err();
            assert!(
                error.starts_with("Command version publish failed:"),
                "{error}"
            );
            assert!(!error.contains("Network error"), "{error}");
        }
        assert!(server.finish().await.is_empty());
    }

    #[test]
    fn publication_content_trimming_matches_javascript() {
        let mut input = publish_input();
        for content in [
            format!("\u{feff}{}\u{feff}", "é".repeat(20_000)),
            "\u{0085}".to_string(),
        ] {
            input.content = content;
            assert!(validate_command_publication("acme", "review", &input).is_ok());
        }
        input.content = "\u{feff}".to_string();
        assert!(validate_command_publication("acme", "review", &input).is_err());
    }

    #[tokio::test]
    async fn publishes_raw_content_with_encoded_segments_and_explicit_metadata() {
        let body = r#"{"version":{"id":"version-id","version":"1.2.3-beta.1+build.2","content":"Réviser 🦀.\nSecond line.","agentCompatibility":["claude","codex"],"scope":"org"}}"#;
        let server = PublishServer::start(Some((201, body))).await;
        let input = publish_input();
        let version = publish_command_version_with_client(
            &publish_client(),
            &server.base_url,
            "fake-token",
            "team /é?#",
            " /review/é?# ",
            input.clone(),
        )
        .await
        .unwrap();
        assert_eq!(version.version, "1.2.3-beta.1+build.2");
        assert_eq!(version.content, "Réviser 🦀.\nSecond line.");
        assert_eq!(version.agent_compatibility, input.agent_compatibility);
        assert_eq!(version.scope.as_deref(), Some("org"));
        let requests = server.finish().await;
        assert_eq!(requests.len(), 1);
        let (headers, body) = requests[0].split_once("\r\n\r\n").unwrap();
        assert_eq!(
            headers.lines().next().unwrap(),
            "POST /api/v1/orgs/team%20%2F%C3%A9%3F%23/commands/review%2F%C3%A9%3F%23/versions HTTP/1.1"
        );
        let headers = headers.to_ascii_lowercase();
        assert!(headers.contains("authorization: bearer fake-token\r\n"));
        assert!(headers.contains("content-type: application/json\r\n"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(body).unwrap(),
            serde_json::json!({
                "version": "1.2.3-beta.1+build.2",
                "content": input.content,
                "agentCompatibility": ["claude", "codex"],
                "scope": "org"
            })
        );
    }

    #[tokio::test]
    async fn publication_http_errors_are_contextual_and_never_retried() {
        for (status, message) in [
            (307, "Temporary redirect"),
            (308, "Permanent redirect"),
            (400, "Invalid version"),
            (401, "Not authenticated"),
            (403, "Write scope required"),
            (404, "Command not found"),
            (409, "Version already exists"),
            (500, "Server error"),
        ] {
            let body = serde_json::json!({"error": message}).to_string();
            let server = PublishServer::start(Some((status, &body))).await;
            let error = publish_command_version_with_client(
                &publish_client(),
                &server.base_url,
                "fake-token",
                "acme",
                "review",
                publish_input(),
            )
            .await
            .unwrap_err();
            assert_eq!(
                error,
                format!("Command version publish failed {status}: {message}")
            );
            assert_eq!(server.finish().await.len(), 1);
        }
    }

    #[tokio::test]
    async fn publication_invalid_response_is_distinct_from_network_failure() {
        for body in ["not json", r#"{"version":{"version":"1.0.0"}}"#, "{}"] {
            let server = PublishServer::start(Some((201, body))).await;
            let error = publish_command_version_with_client(
                &publish_client(),
                &server.base_url,
                "fake-token",
                "acme",
                "review",
                publish_input(),
            )
            .await
            .unwrap_err();
            assert!(
                error.starts_with("Command version publish failed: Invalid response:"),
                "{error}"
            );
            assert_eq!(server.finish().await.len(), 1);
        }
        let server = PublishServer::start(None).await;
        let error = publish_command_version_with_client(
            &publish_client(),
            &server.base_url,
            "fake-token",
            "acme",
            "review",
            publish_input(),
        )
        .await
        .unwrap_err();
        assert!(
            error.starts_with("Command version publish failed: Network error:"),
            "{error}"
        );
        assert_eq!(server.finish().await.len(), 1);
    }

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
