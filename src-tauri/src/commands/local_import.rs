use super::{config::read_config, managed_skills::ManagedErrorDto};
use crate::managed_skills::{
    agents::DefaultAgentRegistry,
    errors::{ManagedError, ManagedErrorCode},
    local_import::{LocalImportPreview, LocalImportReport, LocalSkillImportService},
    paths::ManagedPaths,
    platform_links::SystemPlatformLinker,
};

#[tauri::command]
pub fn preview_local_skills_import() -> Result<LocalImportPreview, ManagedErrorDto> {
    let org = active_org()?;
    let paths = ManagedPaths::from_home().map_err(ManagedErrorDto::from)?;
    LocalSkillImportService::new(
        DefaultAgentRegistry::default(),
        SystemPlatformLinker::current(),
        paths,
    )
    .preview(&org)
    .map_err(Into::into)
}

#[tauri::command]
pub async fn run_local_skills_import(confirm: bool) -> Result<LocalImportReport, ManagedErrorDto> {
    let org = active_org()?;
    let paths = ManagedPaths::from_home().map_err(ManagedErrorDto::from)?;
    LocalSkillImportService::new(
        DefaultAgentRegistry::default(),
        SystemPlatformLinker::current(),
        paths,
    )
    .run(&org, confirm)
    .await
    .map_err(Into::into)
}

fn active_org() -> Result<String, ManagedErrorDto> {
    read_config()
        .ok()
        .and_then(|config| config.org)
        .filter(|org| !org.trim().is_empty())
        .ok_or_else(|| {
            ManagedErrorDto::from(ManagedError::new(
                ManagedErrorCode::LocalConfigurationInvalid,
            ))
        })
}
