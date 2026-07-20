use super::api_error::format_api_error;
use super::config::read_config;
use super::installed_manifest::{
    remove_tracked_installation, upsert_tracked_installation, TrackedInstallation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const API_BASE_URL: &str = "https://app.skillreg.dev";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkill {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub is_public: bool,
    pub latest_version: Option<String>,
    pub total_downloads: u64,
    pub total_versions: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillVersionData {
    pub version: String,
    pub tarball_size: u64,
    pub sha256: Option<String>,
    pub skill_md_content: Option<String>,
    pub files_manifest: Option<Vec<String>>,
    pub file_count: Option<u64>,
    pub downloads: u64,
    #[serde(default)]
    pub validation_level: Option<String>,
    #[serde(default)]
    pub validation: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillVersion {
    pub id: String,
    pub version: String,
    pub tarball_size: u64,
    pub sha256: Option<String>,
    pub downloads: u64,
    pub status: String,
    #[serde(default)]
    pub validation_level: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    #[serde(flatten)]
    pub base: RegistrySkill,
    pub is_deprecated: bool,
    pub deprecated_message: Option<String>,
    pub latest_version_data: Option<SkillVersionData>,
    #[serde(default)]
    pub versions: Vec<SkillVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedVersions {
    pub versions: Vec<SkillVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedSkills {
    pub skills: Vec<RegistrySkill>,
    pub pagination: Pagination,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub latest_version: Option<String>,
    pub total_downloads: u64,
    pub org_slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub name: String,
    pub version: String,
    pub path: String,
    pub files_count: usize,
    pub env_vars: Vec<super::local::EnvVarDecl>,
    pub sha256: Option<String>,
    pub content_hash: String,
}

fn get_auth_client() -> Result<(reqwest::Client, String), String> {
    let config = read_config()?;
    let token = config.token.ok_or("Not authenticated")?;
    Ok((reqwest::Client::new(), token))
}

#[tauri::command]
pub async fn list_skills(
    org: String,
    page: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
    sort: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<PaginatedSkills, String> {
    let (client, token) = get_auth_client()?;

    let mut url = format!(
        "{}/api/v1/orgs/{}/skills?page={}&limit={}",
        API_BASE_URL,
        org,
        page.unwrap_or(1),
        limit.unwrap_or(20)
    );

    if let Some(q) = search {
        if !q.is_empty() {
            url.push_str(&format!("&search={}", urlencoded(&q)));
        }
    }
    if let Some(s) = sort {
        url.push_str(&format!("&sort={}", s));
    }
    if let Some(t) = tags {
        for tag in t {
            url.push_str(&format!("&tags={}", urlencoded(&tag)));
        }
    }

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format_api_error("API error", status, &body));
    }

    resp.json::<PaginatedSkills>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn get_skill(org: String, name: String) -> Result<SkillDetail, String> {
    let (client, token) = get_auth_client()?;

    let resp = client
        .get(format!(
            "{}/api/v1/orgs/{}/skills/{}",
            API_BASE_URL, org, name
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

    let mut skill: SkillDetail = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    // Fetch versions separately if not included in the response
    if skill.versions.is_empty() {
        let versions_resp = client
            .get(format!(
                "{}/api/v1/orgs/{}/skills/{}/versions",
                API_BASE_URL, org, name
            ))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        if let Ok(resp) = versions_resp {
            if resp.status().is_success() {
                if let Ok(paginated) = resp.json::<PaginatedVersions>().await {
                    skill.versions = paginated.versions;
                }
            }
        }
    }

    Ok(skill)
}

#[tauri::command]
pub async fn search_skills(query: String, org: Option<String>) -> Result<SearchResponse, String> {
    let (client, token) = get_auth_client()?;

    let mut url = format!("{}/api/v1/search?q={}", API_BASE_URL, urlencoded(&query));
    if let Some(o) = org {
        url.push_str(&format!("&org={}", o));
    }

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Search error: {}", resp.status()));
    }

    resp.json::<SearchResponse>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn pull_skill(
    org: String,
    name: String,
    version: Option<String>,
    agent: String,
    scope: String,
    project_dir: Option<String>,
) -> Result<InstallResult, String> {
    install_skill_from_registry(org, name, version, agent, scope, project_dir).await
}

/// Reject archive entries that would escape the install directory.
///
/// Only plain relative components are allowed: this rules out absolute paths,
/// Windows drive prefixes, and any `..` traversal.
pub(crate) fn is_safe_entry_path(path: &std::path::Path) -> bool {
    use std::path::Component;

    if path.as_os_str().is_empty() {
        return false;
    }

    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

pub async fn install_skill_from_registry(
    org: String,
    name: String,
    version: Option<String>,
    agent: String,
    scope: String,
    project_dir: Option<String>,
) -> Result<InstallResult, String> {
    let (client, token) = get_auth_client()?;

    // 1. Get skill info to find download URL and checksum
    let skill_url = format!("{}/api/v1/orgs/{}/skills/{}", API_BASE_URL, org, name);
    let skill_resp = client
        .get(&skill_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !skill_resp.status().is_success() {
        return Err(format!("Skill not found: {}", skill_resp.status()));
    }

    let skill: SkillDetail = skill_resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let target_version = version
        .as_deref()
        .or(skill.base.latest_version.as_deref())
        .ok_or("No version available")?
        .to_string();

    let expected_sha = skill
        .latest_version_data
        .as_ref()
        .filter(|v| v.version == target_version)
        .and_then(|v| v.sha256.clone())
        .or_else(|| {
            skill
                .versions
                .iter()
                .find(|v| v.version == target_version)
                .and_then(|v| v.sha256.clone())
        });

    // 2. Download tarball
    let download_url = format!(
        "{}/api/v1/orgs/{}/skills/{}/versions/{}/download",
        API_BASE_URL, org, name, target_version
    );
    let dl_resp = client
        .get(&download_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Download error: {}", e))?;

    if !dl_resp.status().is_success() {
        return Err(format!("Download failed: {}", dl_resp.status()));
    }

    let header_sha = dl_resp
        .headers()
        .get("x-checksum-sha256")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let tarball_bytes = dl_resp
        .bytes()
        .await
        .map_err(|e| format!("Download error: {}", e))?;

    // 3. Verify SHA-256. Mandatory: verification used to be skipped whenever the
    // server returned no checksum, so a stripped header disabled it silently.
    let expected = header_sha.or(expected_sha).ok_or_else(|| {
        "No checksum available for this download. Refusing to install unverified content."
            .to_string()
    })?;

    let mut hasher = Sha256::new();
    hasher.update(&tarball_bytes);
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected {
        return Err(format!(
            "Checksum mismatch: expected {}, got {}",
            expected, actual
        ));
    }

    install_verified_tarball(
        &org,
        &name,
        target_version,
        expected,
        &tarball_bytes,
        &agent,
        &scope,
        project_dir,
        None,
    )
    .await
}

/// Write a verified tarball to disk and record the installation.
///
/// Shared by private-registry and public-catalog installs so the safety
/// guarantees — path checks, destructive clean, tracking — cannot drift apart
/// between the two entry points.
#[allow(clippy::too_many_arguments)]
async fn install_verified_tarball(
    org: &str,
    name: &str,
    target_version: String,
    expected: String,
    tarball_bytes: &[u8],
    agent: &str,
    scope: &str,
    project_dir: Option<String>,
    source_org: Option<String>,
) -> Result<InstallResult, String> {
    let name = name.to_string();
    let org = org.to_string();
    let agent = agent.to_string();
    let scope = scope.to_string();
    // 4. Determine install path
    let install_dir = get_install_path(&agent, &scope, &name, project_dir.as_deref())?;

    // Clean existing install
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;

    // 5. Extract tarball (tar.gz)
    let decoder = flate2::read::GzDecoder::new(tarball_bytes);
    let mut archive = tar::Archive::new(decoder);
    let mut file_count = 0usize;

    for entry in archive.entries().map_err(|e| format!("Tar error: {}", e))? {
        let mut entry = entry.map_err(|e| format!("Tar entry error: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Path error: {}", e))?
            .into_owned();

        // The install dir was just wiped with remove_dir_all, so an entry that
        // escapes it does damage rather than merely dropping a stray file.
        if !is_safe_entry_path(&path) {
            return Err(format!(
                "Refusing to extract unsafe path from archive: {}",
                path.display()
            ));
        }

        // Flatten: strip first directory component if present
        let dest = if path.components().count() > 1 {
            let mut comps = path.components();
            comps.next(); // skip root dir in tarball
            install_dir.join(comps.as_path())
        } else {
            install_dir.join(&path)
        };

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).ok();
        }

        if entry.header().entry_type().is_file() {
            let mut content = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut content)
                .map_err(|e| format!("Read error: {}", e))?;
            let mut f = fs::File::create(&dest).map_err(|e| format!("Write error: {}", e))?;
            f.write_all(&content)
                .map_err(|e| format!("Write error: {}", e))?;
            file_count += 1;
        }
    }

    // 6. Parse env vars from installed SKILL.md and return only required values missing
    // from the org-level store. Safe legacy values are migrated first; conflicting
    // legacy values are left untouched and therefore still require user input.
    let skill_md_path = install_dir.join("SKILL.md");
    let skill_md_content = fs::read_to_string(&skill_md_path)
        .map_err(|e| format!("Installed SKILL.md not found: {}", e))?;
    let content_hash = super::local::compute_content_hash(&skill_md_content);
    let parsed_env_vars = super::local::parse_env_from_frontmatter(&skill_md_content);
    let env_store = super::env::EnvStore::new(&org);
    let _ = env_store.migrate_legacy_variables();
    let env_vars = parsed_env_vars
        .into_iter()
        .filter(|variable| {
            variable.required
                && !env_store
                    .is_variable_configured(&variable.name)
                    .unwrap_or(false)
        })
        .collect();

    let install_path = install_dir.to_string_lossy().into_owned();
    let now = current_unix_timestamp_string();
    upsert_tracked_installation(TrackedInstallation {
        org,
        name: name.clone(),
        version: target_version.clone(),
        agent: agent.clone(),
        scope: scope.clone(),
        project_dir,
        install_path: install_path.clone(),
        content_hash: content_hash.clone(),
        source_org: source_org.clone(),
        // Record the checksum that was actually verified, not the advertised one.
        sha256: Some(expected.clone()),
        auto_update_enabled: None,
        last_checked_at: Some(now.clone()),
        last_updated_at: Some(now),
        last_error: None,
    })?;

    Ok(InstallResult {
        name: name.clone(),
        version: target_version,
        path: install_path,
        files_count: file_count,
        env_vars,
        sha256: Some(expected),
        content_hash,
    })
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPolicy {
    pub mode: String,
    pub minimum_validation_level: String,
    pub allow_first_party: bool,
    pub can_install_from_catalog: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogValidation {
    pub level: String,
    pub score: u32,
    pub passed: u32,
    pub warned: u32,
    pub failed: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSkill {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub org_slug: String,
    pub org_name: String,
    #[serde(default)]
    pub is_first_party: bool,
    pub latest_version: String,
    pub total_downloads: u64,
    pub install_command: String,
    pub validation: CatalogValidation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedCatalogSkills {
    pub skills: Vec<CatalogSkill>,
    pub pagination: Pagination,
}

/// The consuming org's catalog policy.
///
/// Used only to decide whether to show the catalog at all — the server
/// re-evaluates it on every install, so hiding the view is a courtesy, not a
/// control.
#[tauri::command]
pub async fn get_catalog_policy(org: String) -> Result<CatalogPolicy, String> {
    let (client, token) = get_auth_client()?;

    let resp = client
        .get(format!("{}/api/v1/orgs/{}/catalog-policy", API_BASE_URL, org))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format_api_error("API error", status, &body));
    }

    resp.json::<CatalogPolicy>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn list_catalog_skills(
    query: Option<String>,
    first_party_only: Option<bool>,
    page: Option<u32>,
    limit: Option<u32>,
) -> Result<PaginatedCatalogSkills, String> {
    let client = reqwest::Client::new();

    let mut url = format!(
        "{}/api/v1/public/skills?page={}&limit={}",
        API_BASE_URL,
        page.unwrap_or(1),
        limit.unwrap_or(30)
    );
    if let Some(q) = query {
        if !q.is_empty() {
            url.push_str(&format!("&q={}", urlencoded(&q)));
        }
    }
    if first_party_only.unwrap_or(false) {
        url.push_str("&firstParty=true");
    }

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format_api_error("API error", status, &body));
    }

    resp.json::<PaginatedCatalogSkills>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

/// Install a skill published by another organization.
///
/// Goes through the catalog route, which evaluates the consuming org's policy
/// and its version pin server-side. Version and checksum come from the response
/// headers, so no separate metadata call is needed.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn install_catalog_skill(
    source_org: String,
    name: String,
    version: Option<String>,
    consumer_org: String,
    agent: String,
    scope: String,
    project_dir: Option<String>,
    accept_version_change: Option<bool>,
) -> Result<InstallResult, String> {
    let (client, token) = get_auth_client()?;

    let requested = version.unwrap_or_else(|| "latest".to_string());
    let url = format!(
        "{}/api/v1/catalog/{}/{}/versions/{}/download?asOrg={}",
        API_BASE_URL,
        source_org,
        name,
        requested,
        urlencoded(&consumer_org)
    );

    let mut request = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("x-client-type", "desktop");

    if accept_version_change.unwrap_or(false) {
        request = request.header("x-accept-version-change", "true");
    }

    let resp = request
        .send()
        .await
        .map_err(|e| format!("Download error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format_api_error("Install refused", status, &body));
    }

    let expected = resp
        .headers()
        .get("x-checksum-sha256")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or("The registry returned no checksum. Refusing to install unverified content.")?;

    let resolved_version = resp
        .headers()
        .get("x-skill-version")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(requested);

    let tarball_bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Download error: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&tarball_bytes);
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected {
        return Err(format!(
            "Checksum mismatch: expected {}, got {}",
            expected, actual
        ));
    }

    install_verified_tarball(
        &source_org,
        &name,
        resolved_version,
        expected,
        &tarball_bytes,
        &agent,
        &scope,
        project_dir,
        Some(source_org.clone()),
    )
    .await
}

fn current_unix_timestamp_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn get_install_path(
    agent: &str,
    scope: &str,
    skill_name: &str,
    project_dir: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;

    let agent_dir = match agent {
        "claude" => ".claude/skills",
        "codex" => ".codex/skills",
        "cursor" => ".cursor/skills",
        _ => return Err(format!("Invalid agent: {}", agent)),
    };

    let base = match scope {
        "user" => home.join(agent_dir),
        "project" => {
            let root = project_dir
                .map(PathBuf::from)
                .ok_or("Project directory is required for project scope")?;
            root.join(agent_dir)
        }
        _ => return Err(format!("Invalid scope: {}", scope)),
    };

    Ok(base.join(skill_name))
}

#[tauri::command]
pub fn uninstall_skill(
    name: String,
    agent: String,
    scope: String,
    project_dir: Option<String>,
) -> Result<bool, String> {
    let path = get_install_path(&agent, &scope, &name, project_dir.as_deref())?;
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    remove_tracked_installation(&name, &agent, &scope, project_dir.as_deref())?;
    Ok(true)
}

/// Permanently delete a skill and all its versions from the registry.
/// Distinct from `uninstall_skill`, which only removes the local copy on disk.
#[tauri::command]
pub async fn delete_skill(org: String, name: String) -> Result<bool, String> {
    let (client, token) = get_auth_client()?;
    let resp = client
        .delete(format!(
            "{}/api/v1/orgs/{}/skills/{}",
            API_BASE_URL, org, name
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Delete failed {}: {}", status, body));
    }

    Ok(true)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub name: String,
    pub local_version: String,
    pub server_version: String,
    pub agent: String,
    pub scope: String,
}

#[tauri::command]
pub async fn check_updates(
    org: String,
    local_skills: Vec<super::local::LocalSkill>,
) -> Result<Vec<UpdateInfo>, String> {
    let (client, token) = get_auth_client()?;
    let mut updates = Vec::new();

    for skill in &local_skills {
        let url = format!("{}/api/v1/orgs/{}/skills/{}", API_BASE_URL, org, skill.name);
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        if let Ok(resp) = resp {
            if resp.status().is_success() {
                if let Ok(detail) = resp.json::<SkillDetail>().await {
                    if let Some(ref server_v) = detail.base.latest_version {
                        if *server_v != skill.version && skill.version != "-" {
                            updates.push(UpdateInfo {
                                name: skill.name.clone(),
                                local_version: skill.version.clone(),
                                server_version: server_v.clone(),
                                agent: skill.agent.clone(),
                                scope: skill.scope.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(updates)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushResult {
    pub name: String,
    pub version: String,
    pub size: u64,
    pub sha256: String,
    pub dry_run: bool,
}

#[tauri::command]
pub async fn push_skill(
    org: String,
    dir_path: String,
    version: Option<String>,
    tag: Option<String>,
    dry_run: bool,
) -> Result<PushResult, String> {
    let (client, token) = get_auth_client()?;
    let dir = Path::new(&dir_path);

    if !dir.exists() || !dir.is_dir() {
        return Err("Directory not found".to_string());
    }

    // Read SKILL.md
    let skill_md_path = dir.join("SKILL.md");
    if !skill_md_path.exists() {
        return Err("SKILL.md not found in directory".to_string());
    }

    let skill_content = fs::read_to_string(&skill_md_path).map_err(|e| e.to_string())?;
    let fm = super::local::parse_frontmatter_pub(&skill_content);
    let skill_name = fm
        .get("name")
        .cloned()
        .ok_or("Missing 'name' in SKILL.md frontmatter")?;

    let skill_version = version
        .or_else(|| fm.get("version").cloned())
        .or_else(|| fm.get("metadata.version").cloned())
        .ok_or("No version specified")?;

    // Create tarball in memory
    let tarball = create_tarball(dir, &skill_version)?;
    let size = tarball.len() as u64;

    // Compute SHA256
    let mut hasher = Sha256::new();
    hasher.update(&tarball);
    let sha = format!("{:x}", hasher.finalize());

    if dry_run {
        return Ok(PushResult {
            name: skill_name,
            version: skill_version,
            size,
            sha256: sha,
            dry_run: true,
        });
    }

    // Upload multipart
    let form = reqwest::multipart::Form::new()
        .text("version", skill_version.clone())
        .text("checksum", sha.clone())
        .text("tag", tag.unwrap_or_else(|| "latest".to_string()))
        .part(
            "tarball",
            reqwest::multipart::Part::bytes(tarball)
                .file_name(format!("{}-{}.tar.gz", skill_name, skill_version))
                .mime_str("application/gzip")
                .map_err(|e| e.to_string())?,
        );

    let resp = client
        .post(format!(
            "{}/api/v1/orgs/{}/skills/{}/versions",
            API_BASE_URL, org, skill_name
        ))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Upload error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format_api_error("Upload failed", status, &body));
    }

    Ok(PushResult {
        name: skill_name,
        version: skill_version,
        size,
        sha256: sha,
        dry_run: false,
    })
}

fn ensure_skill_md_version(content: &str, version: &str) -> String {
    let Some(rest) = content.strip_prefix("---\n") else {
        return content.to_string();
    };
    let Some(fm_end) = rest.find("\n---") else {
        return content.to_string();
    };

    let fm_content = &rest[..fm_end];
    let suffix = &rest[fm_end..];
    let mut lines: Vec<String> = fm_content.lines().map(ToString::to_string).collect();

    if let Some(index) = lines
        .iter()
        .position(|line| line.trim_start().starts_with("version:"))
    {
        let indent_len = lines[index].len() - lines[index].trim_start().len();
        let indent = &lines[index][..indent_len];
        lines[index] = format!("{}version: \"{}\"", indent, version);
    } else if let Some(index) = lines
        .iter()
        .position(|line| line.trim() == "metadata:" && !line.starts_with(' '))
    {
        lines.insert(index + 1, format!("  version: \"{}\"", version));
    } else {
        lines.push("metadata:".to_string());
        lines.push(format!("  version: \"{}\"", version));
    }

    format!("---\n{}{}", lines.join("\n"), suffix)
}

fn create_tarball(dir: &Path, skill_version: &str) -> Result<Vec<u8>, String> {
    let buf = Vec::new();
    let enc = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
    let mut builder = tar::Builder::new(enc);

    fn add_dir_recursive(
        builder: &mut tar::Builder<flate2::write::GzEncoder<Vec<u8>>>,
        dir: &Path,
        prefix: &Path,
        skill_version: &str,
    ) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            // Skip hidden files and node_modules
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }

            let archive_path = prefix.join(&name);

            if path.is_dir() {
                add_dir_recursive(builder, &path, &archive_path, skill_version)?;
            } else if path.is_file() {
                if name == "SKILL.md" {
                    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                    let updated = ensure_skill_md_version(&content, skill_version);
                    let bytes = updated.into_bytes();
                    let mut header = tar::Header::new_gnu();
                    header.set_size(bytes.len() as u64);
                    header.set_mode(0o644);
                    header.set_cksum();
                    builder
                        .append_data(&mut header, archive_path, bytes.as_slice())
                        .map_err(|e| e.to_string())?;
                } else {
                    let mut f = fs::File::open(&path).map_err(|e| e.to_string())?;
                    builder
                        .append_file(archive_path, &mut f)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    let dir_name = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    add_dir_recursive(&mut builder, dir, Path::new(&dir_name), skill_version)?;

    let enc = builder.into_inner().map_err(|e| e.to_string())?;
    enc.finish().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{ensure_skill_md_version, is_safe_entry_path};
    use std::path::Path;

    #[test]
    fn accepts_ordinary_relative_entry_paths() {
        assert!(is_safe_entry_path(Path::new("SKILL.md")));
        assert!(is_safe_entry_path(Path::new("my-skill/SKILL.md")));
        assert!(is_safe_entry_path(Path::new("./scripts/run.sh")));
        assert!(is_safe_entry_path(Path::new("docs/v1.2..3/notes.md")));
    }

    #[test]
    fn rejects_parent_traversal() {
        assert!(!is_safe_entry_path(Path::new("../evil.md")));
        assert!(!is_safe_entry_path(Path::new("skill/../../evil.md")));
        assert!(!is_safe_entry_path(Path::new("a/b/../../../etc/passwd")));
    }

    #[test]
    fn rejects_absolute_paths() {
        assert!(!is_safe_entry_path(Path::new("/etc/passwd")));
        assert!(!is_safe_entry_path(Path::new("/")));
    }

    #[test]
    fn rejects_empty_path() {
        assert!(!is_safe_entry_path(Path::new("")));
    }

    #[test]
    fn adds_metadata_version_when_missing() {
        let content = "---\nname: sync\ndescription: Sync helper\n---\n# Sync\n";

        let updated = ensure_skill_md_version(content, "1.4.0");

        assert!(updated.contains("metadata:\n  version: \"1.4.0\""));
    }

    #[test]
    fn updates_existing_frontmatter_version() {
        let content =
            "---\nname: sync\nmetadata:\n  author: SkillReg\n  version: \"1.3.0\"\n---\n# Sync\n";

        let updated = ensure_skill_md_version(content, "1.4.0");

        assert!(updated.contains("  version: \"1.4.0\""));
        assert!(!updated.contains("1.3.0"));
    }
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
}
