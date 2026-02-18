use super::config::{read_config, write_config};
use serde::{Deserialize, Serialize};

const API_BASE_URL: &str = "https://app.skillreg.dev";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFlowResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PollResponse {
    pub status: String,
    pub token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhoamiResponse {
    pub user: WhoamiUser,
    #[serde(default)]
    pub orgs: Vec<WhoamiOrg>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhoamiUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhoamiOrg {
    pub slug: String,
    pub name: String,
    pub role: String,
}

#[tauri::command]
pub async fn login_initiate() -> Result<DeviceFlowResponse, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/v1/auth/cli/initiate", API_BASE_URL))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Server error: {}", resp.status()));
    }

    resp.json::<DeviceFlowResponse>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn login_poll(device_code: String) -> Result<PollResponse, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/api/v1/auth/cli/poll?device_code={}",
            API_BASE_URL, device_code
        ))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if resp.status().as_u16() == 410 {
        return Err("Code expired".to_string());
    }

    if !resp.status().is_success() {
        return Err(format!("Server error: {}", resp.status()));
    }

    let poll: PollResponse = resp.json().await.map_err(|e| format!("Parse error: {}", e))?;

    // If complete, save token to config
    if poll.status == "complete" {
        if let Some(ref token) = poll.token {
            let mut config = read_config().unwrap_or_default();
            config.token = Some(token.clone());
            write_config(config).ok();
        }
    }

    Ok(poll)
}

#[tauri::command]
pub async fn whoami() -> Result<WhoamiResponse, String> {
    let config = read_config()?;
    let token = config.token.ok_or("Not authenticated")?;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/v1/auth/whoami", API_BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Auth error: {}", resp.status()));
    }

    resp.json::<WhoamiResponse>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn login_with_token(token: String) -> Result<bool, String> {
    let trimmed = token.trim().to_string();

    // Validate token format
    if !(trimmed.starts_with("sr_live_")
        || trimmed.starts_with("sr_test_")
        || trimmed.starts_with("sk_"))
    {
        return Err("Invalid token format. Expected sr_live_*, sr_test_* or sk_*".to_string());
    }

    // Verify token works by calling whoami
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/v1/auth/whoami", API_BASE_URL))
        .header("Authorization", format!("Bearer {}", trimmed))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err("Token is invalid or expired".to_string());
    }

    // Save token
    let mut config = read_config().unwrap_or_default();
    config.token = Some(trimmed);
    write_config(config)?;

    Ok(true)
}

#[tauri::command]
pub async fn logout() -> Result<(), String> {
    let mut config = read_config().unwrap_or_default();
    config.token = None;
    write_config(config)
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}
