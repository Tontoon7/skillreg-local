use super::config::read_config;
use super::local::parse_frontmatter_pub;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const API_BASE_URL: &str = "https://app.skillreg.dev";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalActor {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalSummary {
    pub id: String,
    pub skill_id: String,
    pub title: String,
    pub intent: String,
    pub base_version: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: ProposalActor,
    pub reviewed_at: Option<String>,
    pub reviewed_by: Option<ProposalActor>,
    pub rejection_reason: Option<String>,
    pub published_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDetail {
    #[serde(flatten)]
    pub summary: ProposalSummary,
    pub skill_md_content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProposalListResponse {
    proposals: Vec<ProposalSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProposalPayload {
    title: String,
    intent: String,
    base_version: String,
    skill_md_content: String,
}

fn get_auth_client() -> Result<(reqwest::Client, String), String> {
    let config = read_config()?;
    let token = config.token.ok_or("Not authenticated")?;
    Ok((reqwest::Client::new(), token))
}

#[tauri::command]
pub async fn propose_skill_change(
    org: String,
    dir_path: String,
    title: String,
    intent: String,
) -> Result<ProposalSummary, String> {
    let (client, token) = get_auth_client()?;
    let dir = Path::new(&dir_path);

    if !dir.exists() || !dir.is_dir() {
        return Err("Directory not found".to_string());
    }

    let skill_md_path = dir.join("SKILL.md");
    if !skill_md_path.exists() {
        return Err("SKILL.md not found in directory".to_string());
    }

    let skill_md_content = fs::read_to_string(&skill_md_path).map_err(|e| e.to_string())?;
    let frontmatter = parse_frontmatter_pub(&skill_md_content);
    let skill_name = frontmatter
        .get("name")
        .cloned()
        .ok_or("Missing 'name' in SKILL.md frontmatter")?;
    let base_version = frontmatter
        .get("version")
        .or_else(|| frontmatter.get("metadata.version"))
        .cloned()
        .ok_or("SKILL.md must keep the current official version for proposals")?;

    let payload = ProposalPayload {
        title,
        intent,
        base_version,
        skill_md_content: skill_md_content,
    };

    let resp = client
        .post(format!(
            "{}/api/v1/orgs/{}/skills/{}/proposals",
            API_BASE_URL, org, skill_name
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    resp.json::<ProposalSummary>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn list_skill_proposals(org: String, name: String) -> Result<Vec<ProposalSummary>, String> {
    let (client, token) = get_auth_client()?;

    let resp = client
        .get(format!(
            "{}/api/v1/orgs/{}/skills/{}/proposals",
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

    let payload = resp
        .json::<ProposalListResponse>()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;
    Ok(payload.proposals)
}

#[tauri::command]
pub async fn get_skill_proposal(
    org: String,
    name: String,
    proposal_id: String,
) -> Result<ProposalDetail, String> {
    let (client, token) = get_auth_client()?;

    let resp = client
        .get(format!(
            "{}/api/v1/orgs/{}/skills/{}/proposals/{}",
            API_BASE_URL, org, name, proposal_id
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

    resp.json::<ProposalDetail>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}
