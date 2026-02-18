use super::config::read_config;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const API_BASE_URL: &str = "https://app.skillreg.dev";

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
pub struct EnvVarDecl {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub name: String,
    pub version: String,
    pub path: String,
    pub files_count: usize,
    pub env_vars: Vec<EnvVarDecl>,
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
        return Err(format!("API error {}: {}", status, body));
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
        return Err(format!("API error {}: {}", status, body));
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
pub async fn search_skills(
    query: String,
    org: Option<String>,
) -> Result<SearchResponse, String> {
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
        .and_then(|v| v.sha256.clone());

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

    let tarball_bytes = dl_resp
        .bytes()
        .await
        .map_err(|e| format!("Download error: {}", e))?;

    // 3. Verify SHA-256
    if let Some(ref expected) = expected_sha {
        let mut hasher = Sha256::new();
        hasher.update(&tarball_bytes);
        let actual = format!("{:x}", hasher.finalize());
        if actual != *expected {
            return Err(format!(
                "Checksum mismatch: expected {}, got {}",
                expected, actual
            ));
        }
    }

    // 4. Determine install path
    let install_dir = get_install_path(&agent, &scope, &name, project_dir.as_deref())?;

    // Clean existing install
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;

    // 5. Extract tarball (tar.gz)
    let decoder = flate2::read::GzDecoder::new(&tarball_bytes[..]);
    let mut archive = tar::Archive::new(decoder);
    let mut file_count = 0usize;

    for entry in archive.entries().map_err(|e| format!("Tar error: {}", e))? {
        let mut entry = entry.map_err(|e| format!("Tar entry error: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Path error: {}", e))?
            .into_owned();

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

    // 6. Parse env vars from installed SKILL.md
    let env_vars = install_dir
        .join("SKILL.md")
        .exists()
        .then(|| {
            fs::read_to_string(install_dir.join("SKILL.md"))
                .ok()
                .map(|c| parse_env_from_frontmatter(&c))
                .unwrap_or_default()
        })
        .unwrap_or_default();

    Ok(InstallResult {
        name: name.clone(),
        version: target_version,
        path: install_dir.to_string_lossy().into_owned(),
        files_count: file_count,
        env_vars,
    })
}

fn parse_env_from_frontmatter(content: &str) -> Vec<EnvVarDecl> {
    let mut result = Vec::new();

    let fm_content = match content.strip_prefix("---\n") {
        Some(rest) => match rest.find("\n---") {
            Some(i) => &rest[..i],
            None => return result,
        },
        None => return result,
    };

    let mut in_env = false;
    let mut in_item = false;
    let mut name = String::new();
    let mut description = String::new();
    let mut required = false;
    let mut default_val: Option<String> = None;

    let flush = |result: &mut Vec<EnvVarDecl>,
                 name: &mut String,
                 description: &mut String,
                 required: &mut bool,
                 default_val: &mut Option<String>| {
        if !name.is_empty() {
            result.push(EnvVarDecl {
                name: name.clone(),
                description: description.clone(),
                required: *required,
                default: default_val.clone(),
            });
        }
        name.clear();
        description.clear();
        *required = false;
        *default_val = None;
    };

    for line in fm_content.lines() {
        let trimmed = line.trim();

        // Detect start of env: section
        if trimmed == "env:" {
            in_env = true;
            continue;
        }

        if !in_env {
            continue;
        }

        // Top-level key at indent 0 → end of env section
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && !trimmed.is_empty() {
            break;
        }

        // List item: "- name: FOO"
        if trimmed.starts_with("- ") {
            // Flush previous item
            flush(
                &mut result,
                &mut name,
                &mut description,
                &mut required,
                &mut default_val,
            );
            in_item = true;

            let after_dash = trimmed.strip_prefix("- ").unwrap().trim();
            if let Some(sep) = after_dash.find(':') {
                let key = after_dash[..sep].trim();
                let val = after_dash[sep + 1..].trim().trim_matches(|c| c == '"' || c == '\'');
                match key {
                    "name" => name = val.to_string(),
                    "description" => description = val.to_string(),
                    "required" => required = val == "true",
                    "default" => default_val = Some(val.to_string()),
                    _ => {}
                }
            }
            continue;
        }

        // Continuation lines (indented properties of current item)
        if in_item && indent >= 4 {
            if let Some(sep) = trimmed.find(':') {
                let key = trimmed[..sep].trim();
                let val = trimmed[sep + 1..].trim().trim_matches(|c| c == '"' || c == '\'');
                match key {
                    "name" => name = val.to_string(),
                    "description" => description = val.to_string(),
                    "required" => required = val == "true",
                    "default" => default_val = Some(val.to_string()),
                    _ => {}
                }
            }
        }
    }

    // Flush last item
    flush(
        &mut result,
        &mut name,
        &mut description,
        &mut required,
        &mut default_val,
    );

    result
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
        let url = format!(
            "{}/api/v1/orgs/{}/skills/{}",
            API_BASE_URL, org, skill.name
        );
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
    let tarball = create_tarball(dir)?;
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
        return Err(format!("Upload failed {}: {}", status, body));
    }

    Ok(PushResult {
        name: skill_name,
        version: skill_version,
        size,
        sha256: sha,
        dry_run: false,
    })
}

fn create_tarball(dir: &Path) -> Result<Vec<u8>, String> {
    let buf = Vec::new();
    let enc = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
    let mut builder = tar::Builder::new(enc);

    fn add_dir_recursive(
        builder: &mut tar::Builder<flate2::write::GzEncoder<Vec<u8>>>,
        dir: &Path,
        prefix: &Path,
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
                add_dir_recursive(builder, &path, &archive_path)?;
            } else if path.is_file() {
                let mut f = fs::File::open(&path).map_err(|e| e.to_string())?;
                builder
                    .append_file(archive_path, &mut f)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    let dir_name = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    add_dir_recursive(&mut builder, dir, Path::new(&dir_name))?;

    let enc = builder.into_inner().map_err(|e| e.to_string())?;
    enc.finish().map_err(|e| e.to_string())
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
}
