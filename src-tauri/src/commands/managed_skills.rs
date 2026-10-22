use super::{config::read_config, local::EnvVarDecl, skills::API_BASE_URL};
use crate::managed_skills::{
    agents::{AgentRegistry, DefaultAgentRegistry, DetectionState},
    errors::{ManagedError, ManagedErrorCode, ManagedErrorRecord},
    manifest::read_manifest,
    migration::ReconcileReport,
    paths::ManagedPaths,
    platform_links::SystemPlatformLinker,
    reconcile::{ActiveOrgSwitchReport, ManagedLifecycleService, ManagedUninstallResult},
    service::{
        acquire_managed_mutation_lock, FileManifestStore, ManagedInstallOutcome,
        ManagedInstallRequest, ReqwestManagedRegistryClient,
    },
    AgentId, BindingStatus, ManagedBinding, ManagedSkill, ManagedSkillOrigin, ManagedSkillStatus,
    UsageObservability,
};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedErrorDto {
    pub code: ManagedErrorCode,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
}

impl From<ManagedError> for ManagedErrorDto {
    fn from(error: ManagedError) -> Self {
        Self {
            code: error.code(),
            parameters: error.parameters().clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSkillDto {
    pub installation_id: String,
    pub consumer_org: String,
    pub source_org: String,
    pub skill_id: Option<String>,
    pub skill_name: String,
    pub origin: ManagedSkillOrigin,
    pub active_version: String,
    pub status: ManagedSkillStatus,
    pub installed_at: String,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<ManagedErrorRecord>,
}

impl From<ManagedSkill> for ManagedSkillDto {
    fn from(skill: ManagedSkill) -> Self {
        Self {
            installation_id: skill.installation_id,
            consumer_org: skill.consumer_org,
            source_org: skill.source_org,
            skill_id: skill.skill_id,
            skill_name: skill.skill_name,
            origin: skill.origin,
            active_version: skill.active_version,
            status: skill.status,
            installed_at: skill.installed_at,
            last_checked_at: skill.last_checked_at,
            last_updated_at: skill.last_updated_at,
            last_error: skill.last_error,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedBindingDto {
    pub agent: AgentId,
    pub status: BindingStatus,
    pub last_checked_at: Option<String>,
    pub last_error: Option<ManagedErrorRecord>,
    pub usage_observability: UsageObservability,
}

impl From<ManagedBinding> for ManagedBindingDto {
    fn from(binding: ManagedBinding) -> Self {
        Self {
            agent: binding.agent,
            status: binding.status,
            last_checked_at: binding.last_checked_at,
            last_error: binding.last_error,
            usage_observability: binding.usage_observability,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWarningDto {
    pub code: crate::managed_skills::service::ManagedWarningCode,
    pub agent: Option<AgentId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedInstallResult {
    pub installation: ManagedSkillDto,
    pub bindings: Vec<ManagedBindingDto>,
    pub required_env_vars: Vec<EnvVarDecl>,
    pub warnings: Vec<ManagedWarningDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAgentDto {
    pub agent: AgentId,
    pub state: DetectionState,
    pub detected_version: Option<String>,
    pub requires_restart_after_binding: bool,
    pub detail_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedOverviewInstallationDto {
    pub installation: ManagedSkillDto,
    pub bindings: Vec<ManagedBindingDto>,
    pub missing_env_vars: Vec<EnvVarDecl>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedOverview {
    pub installations: Vec<ManagedOverviewInstallationDto>,
    pub agents: Vec<ManagedAgentDto>,
    pub auto_update_enabled: bool,
}

impl From<ManagedInstallOutcome> for ManagedInstallResult {
    fn from(outcome: ManagedInstallOutcome) -> Self {
        Self {
            installation: outcome.installation.into(),
            bindings: outcome.bindings.into_iter().map(Into::into).collect(),
            required_env_vars: outcome.required_env_vars,
            warnings: outcome
                .warnings
                .into_iter()
                .map(|warning| ManagedWarningDto {
                    code: warning.code,
                    agent: warning.agent,
                })
                .collect(),
        }
    }
}

#[tauri::command]
pub async fn install_managed_skill(
    consumer_org: String,
    source_org: Option<String>,
    name: String,
) -> Result<ManagedInstallResult, ManagedErrorDto> {
    let config = read_config().map_err(|_| {
        ManagedErrorDto::from(ManagedError::new(
            ManagedErrorCode::LocalConfigurationInvalid,
        ))
    })?;
    let token = config.token.ok_or_else(|| {
        ManagedErrorDto::from(ManagedError::new(ManagedErrorCode::AuthenticationRequired))
    })?;
    let source_org = source_org.unwrap_or_else(|| consumer_org.clone());
    let paths = ManagedPaths::from_home().map_err(ManagedErrorDto::from)?;
    let client = ReqwestManagedRegistryClient::new(token, normalize_api_base(config.api_url));
    let service = crate::managed_skills::service::ManagedSkillService::new(
        client,
        DefaultAgentRegistry::default(),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths,
    );

    service
        .install(ManagedInstallRequest {
            consumer_org,
            source_org,
            name,
        })
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn detect_managed_agents() -> Result<Vec<ManagedAgentDto>, ManagedErrorDto> {
    let paths = ManagedPaths::from_home().map_err(ManagedErrorDto::from)?;
    Ok(DefaultAgentRegistry::default()
        .detect_all(paths.home())
        .into_iter()
        .map(|detection| ManagedAgentDto {
            agent: detection.agent,
            state: detection.state,
            detected_version: detection.detected_version,
            requires_restart_after_binding: detection.requires_restart_after_binding,
            detail_code: detection.detail_code,
        })
        .collect())
}

#[tauri::command]
pub fn get_managed_overview() -> Result<ManagedOverview, ManagedErrorDto> {
    let paths = ManagedPaths::from_home().map_err(ManagedErrorDto::from)?;
    let config = read_config().map_err(|_| {
        ManagedErrorDto::from(ManagedError::new(
            ManagedErrorCode::LocalConfigurationInvalid,
        ))
    })?;
    let agents = detect_managed_agents()?;
    if !paths.manifest_path().exists() {
        return Ok(ManagedOverview {
            installations: vec![],
            agents,
            auto_update_enabled: config.auto_update_enabled_value(),
        });
    }
    let manifest = read_manifest(&paths).map_err(ManagedErrorDto::from)?;
    let active_org = manifest.active_org.clone().or(config.org.clone());
    let installations = manifest
        .skills
        .into_iter()
        .filter(|installation| {
            active_org
                .as_deref()
                .is_none_or(|active_org| installation.consumer_org == active_org)
        })
        .map(|installation| {
            let bindings = manifest
                .bindings
                .iter()
                .filter(|binding| binding.installation_id == installation.installation_id)
                .cloned()
                .map(Into::into)
                .collect();
            let missing_env_vars = missing_required_env_vars(&installation);
            ManagedOverviewInstallationDto {
                installation: installation.into(),
                bindings,
                missing_env_vars,
            }
        })
        .collect();
    Ok(ManagedOverview {
        installations,
        agents,
        auto_update_enabled: config.auto_update_enabled_value(),
    })
}

#[tauri::command]
pub async fn uninstall_managed_skill(
    installation_id: String,
) -> Result<ManagedUninstallResult, ManagedErrorDto> {
    managed_lifecycle_service()?
        .uninstall(&installation_id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn repair_managed_skill(
    installation_id: String,
) -> Result<ReconcileReport, ManagedErrorDto> {
    managed_lifecycle_service()?
        .repair_one(&installation_id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn repair_all_managed_skills() -> Result<ReconcileReport, ManagedErrorDto> {
    managed_lifecycle_service()?
        .repair_all()
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn switch_active_org(org: String) -> Result<ActiveOrgSwitchReport, ManagedErrorDto> {
    let whoami = super::auth::whoami().await.map_err(|_| {
        ManagedErrorDto::from(ManagedError::new(ManagedErrorCode::AuthenticationRequired))
    })?;
    let authorized_orgs = whoami
        .orgs
        .into_iter()
        .map(|organization| organization.slug)
        .collect::<Vec<_>>();
    let _guard = acquire_managed_mutation_lock().await;
    let service = managed_lifecycle_service()?;
    let report = service
        .switch_active_org_serialized(&org, &authorized_orgs)
        .map_err(ManagedErrorDto::from)?;

    let mut config = read_config().map_err(|_| {
        ManagedErrorDto::from(ManagedError::new(
            ManagedErrorCode::LocalConfigurationInvalid,
        ))
    })?;
    config.org = Some(org);
    if super::config::write_config(config).is_err() {
        if let Some(previous_org) = report.previous_org.as_deref() {
            let _ = service.switch_active_org_serialized(previous_org, &authorized_orgs);
        }
        return Err(ManagedErrorDto::from(ManagedError::new(
            ManagedErrorCode::LocalConfigurationInvalid,
        )));
    }
    Ok(report)
}

fn managed_lifecycle_service() -> Result<
    ManagedLifecycleService<DefaultAgentRegistry, SystemPlatformLinker, FileManifestStore>,
    ManagedErrorDto,
> {
    let paths = ManagedPaths::from_home().map_err(ManagedErrorDto::from)?;
    Ok(ManagedLifecycleService::new(
        DefaultAgentRegistry::default(),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths,
    ))
}

fn missing_required_env_vars(installation: &ManagedSkill) -> Vec<EnvVarDecl> {
    let skill_md = match fs::read_to_string(Path::new(&installation.content_path).join("SKILL.md"))
    {
        Ok(content) => content,
        Err(_) => return vec![],
    };
    let env_store = super::env::EnvStore::new(&installation.consumer_org);
    crate::commands::local::parse_env_from_frontmatter(&skill_md)
        .into_iter()
        .filter(|variable| {
            variable.required
                && !env_store
                    .is_variable_configured(&variable.name)
                    .unwrap_or(false)
        })
        .collect()
}

pub(super) fn normalize_api_base(configured: Option<String>) -> String {
    let configured = configured
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(API_BASE_URL)
        .trim_end_matches('/');
    configured
        .strip_suffix("/api/v1")
        .unwrap_or(configured)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_skills::{
        service::{ManagedWarning, ManagedWarningCode},
        LinkKind,
    };

    #[test]
    fn configured_api_v1_url_is_normalized_for_managed_route_builders() {
        assert_eq!(
            normalize_api_base(Some("https://registry.example/api/v1/".to_string())),
            "https://registry.example"
        );
        assert_eq!(normalize_api_base(None), API_BASE_URL);
    }

    #[test]
    fn employee_result_never_serializes_canonical_or_binding_paths() {
        let outcome = ManagedInstallOutcome {
            installation: ManagedSkill {
                installation_id: "installation-id".to_string(),
                consumer_org: "acme".to_string(),
                source_org: "skillreg".to_string(),
                skill_id: Some("skill-id".to_string()),
                skill_name: "meeting-summary".to_string(),
                origin: ManagedSkillOrigin::Registry,
                active_version: "1.4.2".to_string(),
                sha256: "a".repeat(64),
                content_path: "/private/canonical/content".to_string(),
                content_hash: "b".repeat(64),
                status: ManagedSkillStatus::Ready,
                installed_at: "1".to_string(),
                last_checked_at: Some("1".to_string()),
                last_updated_at: Some("1".to_string()),
                last_error: None,
                cleanup_dismissed_until: None,
            },
            bindings: vec![ManagedBinding {
                installation_id: "installation-id".to_string(),
                agent: AgentId::Claude,
                link_path: "/private/.claude/skills/meeting-summary".to_string(),
                link_kind: LinkKind::Symlink,
                status: BindingStatus::Ready,
                last_checked_at: Some("1".to_string()),
                last_error: None,
                usage_observability: UsageObservability::Unavailable,
            }],
            required_env_vars: vec![],
            warnings: vec![ManagedWarning {
                code: ManagedWarningCode::BindingUnsupported,
                agent: Some(AgentId::Cursor),
            }],
        };

        let json = serde_json::to_value(ManagedInstallResult::from(outcome)).unwrap();
        let serialized = serde_json::to_string(&json).unwrap();

        assert!(!serialized.contains("contentPath"));
        assert!(!serialized.contains("contentHash"));
        assert!(!serialized.contains("linkPath"));
        assert!(!serialized.contains("/private/"));
        assert_eq!(json["installation"]["status"], "ready");
        assert_eq!(json["bindings"][0]["agent"], "claude");
    }
}
