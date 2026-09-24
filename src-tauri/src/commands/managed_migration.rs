use super::{
    config::read_config,
    managed_skills::{normalize_api_base, ManagedErrorDto},
};
use crate::managed_skills::{
    agents::DefaultAgentRegistry,
    errors::{ManagedError, ManagedErrorCode},
    migration::{
        build_migration_preview, ManagedMigrationService, MigrationPreview, MigrationReport,
        ReconcileReport,
    },
    paths::ManagedPaths,
    platform_links::SystemPlatformLinker,
    service::{FileManifestStore, ManagedSkillService, ReqwestManagedRegistryClient},
};

#[tauri::command]
pub fn preview_managed_skills_migration() -> Result<MigrationPreview, ManagedErrorDto> {
    let paths = ManagedPaths::from_home().map_err(ManagedErrorDto::from)?;
    build_migration_preview(&paths, &DefaultAgentRegistry::default()).map_err(Into::into)
}

#[tauri::command]
pub async fn run_managed_skills_migration(
    confirm: bool,
) -> Result<MigrationReport, ManagedErrorDto> {
    let paths = ManagedPaths::from_home().map_err(ManagedErrorDto::from)?;
    let preview = build_migration_preview(&paths, &DefaultAgentRegistry::default())
        .map_err(ManagedErrorDto::from)?;
    let config = read_config().map_err(|_| {
        ManagedErrorDto::from(ManagedError::new(
            ManagedErrorCode::LocalConfigurationInvalid,
        ))
    })?;
    let token = if confirm && preview.managed_candidates > 0 {
        config.token.ok_or_else(|| {
            ManagedErrorDto::from(ManagedError::new(ManagedErrorCode::AuthenticationRequired))
        })?
    } else {
        config.token.unwrap_or_default()
    };
    let installer = ManagedSkillService::new(
        ReqwestManagedRegistryClient::new(token, normalize_api_base(config.api_url)),
        DefaultAgentRegistry::default(),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    );
    ManagedMigrationService::new(
        installer,
        DefaultAgentRegistry::default(),
        SystemPlatformLinker::current(),
        paths,
    )
    .run(confirm)
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn repair_managed_skills() -> Result<ReconcileReport, ManagedErrorDto> {
    super::managed_skills::repair_all_managed_skills().await
}
