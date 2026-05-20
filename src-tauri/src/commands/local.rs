use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPaths {
    pub project: String,
    pub user: String,
}

pub fn get_agent_paths() -> HashMap<String, AgentPaths> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut map = HashMap::new();
    map.insert(
        "claude".into(),
        AgentPaths {
            project: ".claude/skills".into(),
            user: home.join(".claude/skills").to_string_lossy().into_owned(),
        },
    );
    map.insert(
        "codex".into(),
        AgentPaths {
            project: ".codex/skills".into(),
            user: home.join(".codex/skills").to_string_lossy().into_owned(),
        },
    );
    map.insert(
        "cursor".into(),
        AgentPaths {
            project: ".cursor/skills".into(),
            user: home.join(".cursor/skills").to_string_lossy().into_owned(),
        },
    );
    map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarDecl {
    pub name: String,
    pub description: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSkill {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
    pub path: String,
    pub agent: String,
    pub scope: String,
    pub content_hash: String,
    pub modified_at: Option<String>,
    pub env_vars: Vec<EnvVarDecl>,
}

fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn parse_frontmatter_pub(content: &str) -> HashMap<String, String> {
    parse_frontmatter(content)
}

fn parse_frontmatter(content: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    let fm_content = match content.strip_prefix("---\n") {
        Some(rest) => match rest.find("\n---") {
            Some(i) => &rest[..i],
            None => return result,
        },
        None => return result,
    };

    let mut current_parent = String::new();
    for line in fm_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(sep) = trimmed.find(':') {
            let key = trimmed[..sep].trim();
            let raw_val = trimmed[sep + 1..].trim();
            let indent = line.len() - line.trim_start().len();

            if raw_val.is_empty() || raw_val == "|" || raw_val == ">" {
                if indent == 0 {
                    current_parent = format!("{}.", key);
                }
                continue;
            }

            let val = raw_val.trim_matches(|c| c == '"' || c == '\'');

            if indent > 0 && !current_parent.is_empty() {
                result.insert(format!("{}{}", current_parent, key), val.to_string());
            } else {
                current_parent.clear();
                result.insert(key.to_string(), val.to_string());
            }
        }
    }

    result
}

fn parse_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();

    let fm_content = match content.strip_prefix("---\n") {
        Some(rest) => match rest.find("\n---") {
            Some(i) => &rest[..i],
            None => return tags,
        },
        None => return tags,
    };

    let mut in_tags = false;
    for line in fm_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("tags:") {
            let inline = trimmed.strip_prefix("tags:").unwrap().trim();
            if inline.starts_with('[') {
                let inner = inline.trim_start_matches('[').trim_end_matches(']');
                for tag in inner.split(',') {
                    let t = tag.trim().trim_matches(|c| c == '"' || c == '\'');
                    if !t.is_empty() {
                        tags.push(t.to_string());
                    }
                }
                return tags;
            }
            if inline.is_empty() {
                in_tags = true;
            }
            continue;
        }
        if in_tags {
            if trimmed.starts_with("- ") {
                let tag = trimmed
                    .strip_prefix("- ")
                    .unwrap()
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'');
                if !tag.is_empty() {
                    tags.push(tag.to_string());
                }
            } else if !trimmed.is_empty() {
                break;
            }
        }
    }

    tags
}

pub fn parse_env_from_frontmatter(content: &str) -> Vec<EnvVarDecl> {
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
    let mut required = true;
    let mut secret: Option<bool> = None;
    let mut default_val: Option<String> = None;

    let flush = |result: &mut Vec<EnvVarDecl>,
                 name: &mut String,
                 description: &mut String,
                 required: &mut bool,
                 secret: &mut Option<bool>,
                 default_val: &mut Option<String>| {
        if !name.is_empty() {
            result.push(EnvVarDecl {
                name: name.clone(),
                description: description.clone(),
                required: *required,
                secret: *secret,
                default: default_val.clone(),
            });
        }
        name.clear();
        description.clear();
        *required = true;
        *secret = None;
        *default_val = None;
    };

    for line in fm_content.lines() {
        let trimmed = line.trim();

        if trimmed == "env:" {
            in_env = true;
            continue;
        }

        if !in_env {
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        if indent == 0 && !trimmed.is_empty() {
            break;
        }

        if trimmed.starts_with("- ") {
            flush(
                &mut result,
                &mut name,
                &mut description,
                &mut required,
                &mut secret,
                &mut default_val,
            );
            in_item = true;

            let after_dash = trimmed.strip_prefix("- ").unwrap().trim();
            apply_env_property(
                after_dash,
                &mut name,
                &mut description,
                &mut required,
                &mut secret,
                &mut default_val,
            );
            continue;
        }

        if in_item && indent >= 4 {
            apply_env_property(
                trimmed,
                &mut name,
                &mut description,
                &mut required,
                &mut secret,
                &mut default_val,
            );
        }
    }

    flush(
        &mut result,
        &mut name,
        &mut description,
        &mut required,
        &mut secret,
        &mut default_val,
    );

    result
}

fn apply_env_property(
    line: &str,
    name: &mut String,
    description: &mut String,
    required: &mut bool,
    secret: &mut Option<bool>,
    default_val: &mut Option<String>,
) {
    let Some(sep) = line.find(':') else {
        return;
    };

    let key = line[..sep].trim();
    let val = line[sep + 1..]
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');

    match key {
        "name" => *name = val.to_string(),
        "description" => *description = val.to_string(),
        "required" => *required = val.eq_ignore_ascii_case("true"),
        "secret" => *secret = Some(val.eq_ignore_ascii_case("true")),
        "default" => *default_val = Some(val.to_string()),
        _ => {}
    }
}

#[tauri::command]
pub fn scan_local_skills(
    agent: Option<String>,
    scope: Option<String>,
) -> Result<Vec<LocalSkill>, String> {
    let agent_paths = get_agent_paths();
    let agents: Vec<String> = match agent {
        Some(a) => vec![a],
        None => vec!["claude".into(), "codex".into(), "cursor".into()],
    };
    let scopes: Vec<String> = match scope {
        Some(s) => vec![s],
        None => vec!["project".into(), "user".into()],
    };

    let mut results = Vec::new();

    for ag in &agents {
        let paths = agent_paths
            .get(ag)
            .ok_or(format!("Unknown agent: {}", ag))?;

        for sc in &scopes {
            let dir = match sc.as_str() {
                "project" => &paths.project,
                "user" => &paths.user,
                _ => continue,
            };

            let dir_path = Path::new(dir);
            if !dir_path.exists() {
                continue;
            }

            let entries = fs::read_dir(dir_path).map_err(|e| e.to_string())?;
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().into_owned();
                if fname.starts_with('.') {
                    continue;
                }

                let full_path = entry.path();
                if !full_path.is_dir() {
                    continue;
                }

                let skill_md = full_path.join("SKILL.md");
                let md_path = if skill_md.exists() {
                    Some(skill_md)
                } else {
                    // Fallback: find any .md file
                    fs::read_dir(&full_path).ok().and_then(|entries| {
                        entries.flatten().find_map(|e| {
                            let name = e.file_name().to_string_lossy().into_owned();
                            if name.ends_with(".md") {
                                Some(e.path())
                            } else {
                                None
                            }
                        })
                    })
                };

                if let Some(md) = md_path {
                    let content = fs::read_to_string(&md).unwrap_or_default();
                    let hash = compute_sha256(&content);
                    let fm = parse_frontmatter(&content);
                    let tags = parse_tags(&content);
                    let env_vars = parse_env_from_frontmatter(&content);

                    let modified_at = fs::metadata(&md)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs().to_string());

                    results.push(LocalSkill {
                        name: fm.get("name").cloned().unwrap_or_else(|| fname.clone()),
                        version: fm
                            .get("version")
                            .or_else(|| fm.get("metadata.version"))
                            .cloned()
                            .unwrap_or_else(|| "-".into()),
                        description: fm.get("description").cloned().unwrap_or_default(),
                        tags,
                        path: full_path.to_string_lossy().into_owned(),
                        agent: ag.clone(),
                        scope: sc.clone(),
                        content_hash: hash,
                        modified_at,
                        env_vars,
                    });
                }
            }
        }
    }

    Ok(results)
}
