pub use crate::commands::local::EnvVarDecl;
use crate::managed_skills::{
    agents::AgentRegistry,
    archive::{compute_tree_hash, validate_and_extract_archive},
    bindings::{BindingApplyReport, ManagedBindingService},
    errors::{ManagedError, ManagedErrorCode},
    manifest::{load_or_create_manifest, write_manifest_atomic, ManagedSkillsManifest},
    paths::{validate_org_slug, validate_skill_name, ManagedPaths},
    platform_links::PlatformLinker,
    AgentId, BindingStatus, ManagedBinding, ManagedSkill, ManagedSkillOrigin, ManagedSkillStatus,
};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    future::Future,
    io::{Read, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

const MAX_COMPRESSED_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
static MANAGED_INSTALL_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

pub async fn acquire_managed_mutation_lock() -> OwnedMutexGuard<()> {
    MANAGED_INSTALL_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
        .lock_owned()
        .await
}

pub type ManagedRegistryFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ManagedError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedRegistryRequest {
    pub consumer_org: String,
    pub source_org: String,
    pub name: String,
}

impl ManagedRegistryRequest {
    pub fn target_url(&self, base_url: &str) -> String {
        format!(
            "{}/api/v1/orgs/{}/managed-skills/{}/{}/target",
            base_url.trim_end_matches('/'),
            self.consumer_org,
            self.source_org,
            self.name
        )
    }

    pub fn download_url(&self, base_url: &str) -> String {
        format!(
            "{}/api/v1/orgs/{}/managed-skills/{}/{}/download",
            base_url.trim_end_matches('/'),
            self.consumer_org,
            self.source_org,
            self.name
        )
    }

    fn validate(&self) -> Result<(), ManagedError> {
        validate_org_slug(&self.consumer_org)?;
        validate_org_slug(&self.source_org)?;
        validate_skill_name(&self.name)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedInstallRequest {
    pub consumer_org: String,
    pub source_org: String,
    pub name: String,
}

impl From<&ManagedInstallRequest> for ManagedRegistryRequest {
    fn from(request: &ManagedInstallRequest) -> Self {
        Self {
            consumer_org: request.consumer_org.clone(),
            source_org: request.source_org.clone(),
            name: request.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedTargetPolicy {
    pub mode: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedTarget {
    pub skill_id: String,
    #[serde(rename = "sourceOrgSlug")]
    pub source_org: String,
    #[serde(rename = "consumerOrgSlug")]
    pub consumer_org: String,
    pub skill_name: String,
    pub resolved_version: String,
    pub sha256: String,
    pub validation_level: String,
    pub policy: ManagedTargetPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedDownloadMetadata {
    pub skill_id: String,
    pub source_org: String,
    pub consumer_org: String,
    pub resolved_version: String,
    pub sha256: String,
    pub policy: String,
    pub validation_level: Option<String>,
}

impl ManagedDownloadMetadata {
    pub fn from_target(target: &ManagedTarget) -> Self {
        Self {
            skill_id: target.skill_id.clone(),
            source_org: target.source_org.clone(),
            consumer_org: target.consumer_org.clone(),
            resolved_version: target.resolved_version.clone(),
            sha256: target.sha256.clone(),
            policy: target.policy.mode.clone(),
            validation_level: Some(target.validation_level.clone()),
        }
    }
}

pub trait ManagedRegistryClient: Send + Sync {
    fn resolve_target<'a>(
        &'a self,
        request: &'a ManagedRegistryRequest,
    ) -> ManagedRegistryFuture<'a, ManagedTarget>;

    fn download_to<'a>(
        &'a self,
        request: &'a ManagedRegistryRequest,
        destination: &'a Path,
    ) -> ManagedRegistryFuture<'a, ManagedDownloadMetadata>;
}

#[derive(Clone)]
pub struct ReqwestManagedRegistryClient {
    client: reqwest::Client,
    token: String,
    base_url: String,
}

impl ReqwestManagedRegistryClient {
    pub fn new(token: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            token,
            base_url,
        }
    }
}

impl ManagedRegistryClient for ReqwestManagedRegistryClient {
    fn resolve_target<'a>(
        &'a self,
        request: &'a ManagedRegistryRequest,
    ) -> ManagedRegistryFuture<'a, ManagedTarget> {
        Box::pin(async move {
            request.validate()?;
            let response = self
                .client
                .get(request.target_url(&self.base_url))
                .header("Authorization", format!("Bearer {}", self.token))
                .send()
                .await
                .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
            if !response.status().is_success() {
                return Err(ManagedError::new(ManagedErrorCode::RegistryRequestFailed)
                    .with_parameter("status", response.status().as_u16().to_string()));
            }
            let target = response
                .json::<ManagedTarget>()
                .await
                .map_err(|_| ManagedError::new(ManagedErrorCode::DownloadMetadataInvalid))?;
            validate_target(request, &target)?;
            Ok(target)
        })
    }

    fn download_to<'a>(
        &'a self,
        request: &'a ManagedRegistryRequest,
        destination: &'a Path,
    ) -> ManagedRegistryFuture<'a, ManagedDownloadMetadata> {
        Box::pin(async move {
            request.validate()?;
            let mut response = self
                .client
                .get(request.download_url(&self.base_url))
                .header("Authorization", format!("Bearer {}", self.token))
                .send()
                .await
                .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
            if !response.status().is_success() {
                return Err(ManagedError::new(ManagedErrorCode::RegistryRequestFailed)
                    .with_parameter("status", response.status().as_u16().to_string()));
            }
            let metadata = parse_download_metadata(response.headers(), request)?;
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(destination)
                .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
            let mut hasher = Sha256::new();
            let mut downloaded = 0u64;
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?
            {
                downloaded = downloaded
                    .checked_add(chunk.len() as u64)
                    .ok_or_else(|| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
                if downloaded > MAX_COMPRESSED_ARCHIVE_BYTES {
                    return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
                }
                hasher.update(&chunk);
                output
                    .write_all(&chunk)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
            }
            output
                .sync_all()
                .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
            let actual = format!("{:x}", hasher.finalize());
            if actual != metadata.sha256 {
                return Err(ManagedError::new(ManagedErrorCode::ArchiveChecksumMismatch));
            }
            Ok(metadata)
        })
    }
}

pub fn parse_download_metadata(
    headers: &HeaderMap,
    request: &ManagedRegistryRequest,
) -> Result<ManagedDownloadMetadata, ManagedError> {
    let get = |name: &'static str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::DownloadMetadataInvalid))
    };
    let metadata = ManagedDownloadMetadata {
        skill_id: get("x-skill-id")?,
        resolved_version: get("x-skill-version")?,
        sha256: get("x-skill-sha256")?,
        source_org: get("x-skill-source-org")?,
        consumer_org: get("x-skill-consumer-org")?,
        policy: get("x-skill-version-policy")?,
        validation_level: headers
            .get("x-validation-level")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    };
    validate_download_metadata(request, &metadata)?;
    Ok(metadata)
}

pub trait ManagedManifestStore: Send + Sync {
    fn load(&self, paths: &ManagedPaths) -> Result<ManagedSkillsManifest, ManagedError>;
    fn write(
        &self,
        paths: &ManagedPaths,
        manifest: &ManagedSkillsManifest,
    ) -> Result<(), ManagedError>;
}

#[derive(Debug, Clone, Copy)]
pub struct FileManifestStore;

impl ManagedManifestStore for FileManifestStore {
    fn load(&self, paths: &ManagedPaths) -> Result<ManagedSkillsManifest, ManagedError> {
        load_or_create_manifest(paths)
    }

    fn write(
        &self,
        paths: &ManagedPaths,
        manifest: &ManagedSkillsManifest,
    ) -> Result<(), ManagedError> {
        write_manifest_atomic(paths, manifest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedWarningCode {
    BindingConflict,
    BindingUnsupported,
    BindingError,
    SkillSourceNameConflict,
    CleanupDeferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedUpdateDecision {
    UpToDate,
    UpdateAvailable,
    UpdateNow,
    SkipLocal,
    SkipGlobalDisabled,
    BlockModifiedContent,
    BlockMissingChecksum,
    BlockPolicy,
    RetryLater,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUpdateSummary {
    pub checked: usize,
    pub available: usize,
    pub updated: usize,
    pub action_required: usize,
    pub failed: usize,
}

pub fn decide_managed_update(
    global_enabled: bool,
    force: bool,
    apply: bool,
    content_matches: bool,
    installation: &ManagedSkill,
    target: Result<&ManagedTarget, ManagedErrorCode>,
) -> ManagedUpdateDecision {
    if installation.origin == ManagedSkillOrigin::Local {
        return ManagedUpdateDecision::SkipLocal;
    }
    if !global_enabled && !force {
        return ManagedUpdateDecision::SkipGlobalDisabled;
    }
    if !content_matches {
        return ManagedUpdateDecision::BlockModifiedContent;
    }
    let target = match target {
        Ok(target) => target,
        Err(ManagedErrorCode::DownloadMetadataInvalid) => {
            return ManagedUpdateDecision::BlockMissingChecksum;
        }
        Err(_) => return ManagedUpdateDecision::RetryLater,
    };
    if !is_sha256(&target.sha256) {
        return ManagedUpdateDecision::BlockMissingChecksum;
    }
    let policy_is_valid = match target.policy.mode.as_str() {
        "pinned" => target.policy.version.as_deref() == Some(target.resolved_version.as_str()),
        "latest_approved" => target.policy.version.is_none(),
        _ => false,
    };
    let identity_is_valid = target.consumer_org == installation.consumer_org
        && target.source_org == installation.source_org
        && target.skill_name == installation.skill_name
        && installation
            .skill_id
            .as_ref()
            .is_none_or(|skill_id| skill_id == &target.skill_id);
    if !policy_is_valid || !identity_is_valid {
        return ManagedUpdateDecision::BlockPolicy;
    }
    if target.resolved_version == installation.active_version {
        return if target.sha256 == installation.sha256 {
            ManagedUpdateDecision::UpToDate
        } else {
            ManagedUpdateDecision::BlockPolicy
        };
    }
    if apply {
        ManagedUpdateDecision::UpdateNow
    } else {
        ManagedUpdateDecision::UpdateAvailable
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWarning {
    pub code: ManagedWarningCode,
    pub agent: Option<AgentId>,
}

#[derive(Debug, Clone)]
pub struct ManagedInstallOutcome {
    pub installation: ManagedSkill,
    pub bindings: Vec<ManagedBinding>,
    pub required_env_vars: Vec<EnvVarDecl>,
    pub warnings: Vec<ManagedWarning>,
}

pub struct ManagedSkillService<
    C: ManagedRegistryClient,
    A: AgentRegistry,
    L: PlatformLinker,
    S: ManagedManifestStore,
> {
    client: C,
    bindings: ManagedBindingService<A, L>,
    manifest_store: S,
    paths: ManagedPaths,
}

impl<C: ManagedRegistryClient, A: AgentRegistry, L: PlatformLinker, S: ManagedManifestStore>
    ManagedSkillService<C, A, L, S>
{
    pub fn new(client: C, agents: A, linker: L, manifest_store: S, paths: ManagedPaths) -> Self {
        Self {
            client,
            bindings: ManagedBindingService::new(agents, linker, paths.clone()),
            manifest_store,
            paths,
        }
    }

    pub async fn install(
        &self,
        request: ManagedInstallRequest,
    ) -> Result<ManagedInstallOutcome, ManagedError> {
        let _guard = acquire_managed_mutation_lock().await;
        self.install_serialized(request).await
    }

    pub async fn run_managed_updates(
        &self,
        global_enabled: bool,
        force: bool,
        apply: bool,
    ) -> Result<ManagedUpdateSummary, ManagedError> {
        if !global_enabled && !force {
            return Ok(ManagedUpdateSummary::default());
        }
        let _guard = acquire_managed_mutation_lock().await;
        self.run_managed_updates_serialized(global_enabled, force, apply)
            .await
    }

    async fn run_managed_updates_serialized(
        &self,
        global_enabled: bool,
        force: bool,
        apply: bool,
    ) -> Result<ManagedUpdateSummary, ManagedError> {
        let installation_ids = self
            .manifest_store
            .load(&self.paths)?
            .skills
            .into_iter()
            .map(|skill| skill.installation_id)
            .collect::<Vec<_>>();
        let mut summary = ManagedUpdateSummary::default();

        for installation_id in installation_ids {
            let mut manifest = self.manifest_store.load(&self.paths)?;
            let Some(index) = manifest
                .skills
                .iter()
                .position(|skill| skill.installation_id == installation_id)
            else {
                continue;
            };
            let installation = manifest.skills[index].clone();
            if installation.origin == ManagedSkillOrigin::Local {
                continue;
            }
            summary.checked += 1;
            let content_matches = compute_tree_hash(Path::new(&installation.content_path))
                .is_ok_and(|hash| hash == installation.content_hash);
            let request = ManagedRegistryRequest {
                consumer_org: installation.consumer_org.clone(),
                source_org: installation.source_org.clone(),
                name: installation.skill_name.clone(),
            };
            let target_result = if content_matches {
                self.client.resolve_target(&request).await
            } else {
                Err(ManagedError::new(ManagedErrorCode::ContentModified))
            };
            let target_error = target_result.as_ref().err().map(ManagedError::code);
            let decision = decide_managed_update(
                global_enabled,
                force,
                apply,
                content_matches,
                &installation,
                target_result.as_ref().map_err(|error| error.code()),
            );
            let now = current_timestamp();

            match decision {
                ManagedUpdateDecision::SkipLocal => {}
                ManagedUpdateDecision::SkipGlobalDisabled => {}
                ManagedUpdateDecision::UpToDate => {
                    let status = status_without_update(&manifest, &installation);
                    update_checked_installation(&mut manifest, index, status, now, None);
                    self.manifest_store.write(&self.paths, &manifest)?;
                }
                ManagedUpdateDecision::UpdateAvailable => {
                    summary.available += 1;
                    let status = if has_binding_action(&manifest, &installation.installation_id) {
                        ManagedSkillStatus::ActionRequired
                    } else {
                        ManagedSkillStatus::UpdateAvailable
                    };
                    update_checked_installation(&mut manifest, index, status, now, None);
                    self.manifest_store.write(&self.paths, &manifest)?;
                }
                ManagedUpdateDecision::UpdateNow => {
                    summary.available += 1;
                    match self
                        .install_serialized(ManagedInstallRequest {
                            consumer_org: installation.consumer_org.clone(),
                            source_org: installation.source_org.clone(),
                            name: installation.skill_name.clone(),
                        })
                        .await
                    {
                        Ok(outcome) => {
                            summary.updated += 1;
                            if matches!(
                                outcome.installation.status,
                                ManagedSkillStatus::ActionRequired
                                    | ManagedSkillStatus::Conflict
                                    | ManagedSkillStatus::Error
                            ) {
                                summary.action_required += 1;
                            }
                        }
                        Err(error) => {
                            summary.failed += 1;
                            let mut latest = self.manifest_store.load(&self.paths)?;
                            if let Some(latest_index) = latest
                                .skills
                                .iter()
                                .position(|skill| skill.installation_id == installation_id)
                            {
                                update_checked_installation(
                                    &mut latest,
                                    latest_index,
                                    ManagedSkillStatus::Error,
                                    now,
                                    Some(error.clone()),
                                );
                                self.manifest_store.write(&self.paths, &latest)?;
                            }
                        }
                    }
                }
                ManagedUpdateDecision::BlockModifiedContent => {
                    summary.action_required += 1;
                    update_checked_installation(
                        &mut manifest,
                        index,
                        ManagedSkillStatus::ActionRequired,
                        now,
                        Some(ManagedError::new(ManagedErrorCode::ContentModified)),
                    );
                    self.manifest_store.write(&self.paths, &manifest)?;
                }
                ManagedUpdateDecision::BlockMissingChecksum
                | ManagedUpdateDecision::BlockPolicy => {
                    summary.action_required += 1;
                    update_checked_installation(
                        &mut manifest,
                        index,
                        ManagedSkillStatus::ActionRequired,
                        now,
                        Some(ManagedError::new(
                            target_error.unwrap_or(ManagedErrorCode::DownloadMetadataInvalid),
                        )),
                    );
                    self.manifest_store.write(&self.paths, &manifest)?;
                }
                ManagedUpdateDecision::RetryLater => {
                    summary.failed += 1;
                    update_checked_installation(
                        &mut manifest,
                        index,
                        ManagedSkillStatus::Error,
                        now,
                        Some(ManagedError::new(
                            target_error.unwrap_or(ManagedErrorCode::RegistryRequestFailed),
                        )),
                    );
                    self.manifest_store.write(&self.paths, &manifest)?;
                }
            }
        }
        Ok(summary)
    }

    pub(crate) async fn install_serialized(
        &self,
        request: ManagedInstallRequest,
    ) -> Result<ManagedInstallOutcome, ManagedError> {
        let registry_request = ManagedRegistryRequest::from(&request);
        registry_request.validate()?;
        let mut manifest = self.manifest_store.load(&self.paths)?;
        match manifest.active_org.as_deref() {
            Some(active_org) if active_org != request.consumer_org => {
                return Err(ManagedError::new(
                    ManagedErrorCode::ActiveOrganizationConflict,
                ));
            }
            None => manifest.active_org = Some(request.consumer_org.clone()),
            Some(_) => {}
        }
        recover_interrupted_swap(&self.paths, &request)?;
        let target = self.client.resolve_target(&registry_request).await?;
        validate_target(&registry_request, &target)?;

        let existing = manifest
            .skills
            .iter()
            .find(|skill| {
                skill.consumer_org == request.consumer_org
                    && skill.source_org == request.source_org
                    && skill.skill_name == request.name
            })
            .cloned();
        if let Some(existing) = existing.as_ref() {
            self.reject_modified_existing(existing)?;
            if existing.active_version == target.resolved_version
                && existing.sha256 == target.sha256
            {
                return self.reconcile_idempotent(existing.clone(), manifest);
            }
        }

        let root = ensure_managed_skill_root(&self.paths, &request)?;
        let staging =
            self.paths
                .staging_dir(&request.consumer_org, &request.source_org, &request.name)?;
        let previous =
            self.paths
                .previous_dir(&request.consumer_org, &request.source_org, &request.name)?;
        cleanup_owned_directory(&staging, &root)?;
        let download = root.join(format!("download-{}.tar.gz", Uuid::new_v4()));

        let result = async {
            let metadata = self
                .client
                .download_to(&registry_request, &download)
                .await?;
            validate_download_metadata(&registry_request, &metadata)?;
            verify_download_file(&download, &metadata.sha256)?;
            validate_and_extract_archive(&download, &staging)?;
            let (content_hash, required_env_vars) =
                validate_extracted_skill(&staging, &request.name)?;
            let now = current_timestamp();
            let installation_id = existing
                .as_ref()
                .map(|skill| skill.installation_id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            let mut installation = ManagedSkill {
                installation_id: installation_id.clone(),
                consumer_org: request.consumer_org.clone(),
                source_org: metadata.source_org.clone(),
                skill_id: Some(metadata.skill_id.clone()),
                skill_name: request.name.clone(),
                origin: ManagedSkillOrigin::Registry,
                active_version: metadata.resolved_version.clone(),
                sha256: metadata.sha256.clone(),
                content_path: self
                    .paths
                    .content_dir(&request.consumer_org, &metadata.source_org, &request.name)?
                    .to_string_lossy()
                    .into_owned(),
                content_hash,
                status: ManagedSkillStatus::Ready,
                installed_at: existing
                    .as_ref()
                    .map(|skill| skill.installed_at.clone())
                    .unwrap_or_else(|| now.clone()),
                last_checked_at: Some(now.clone()),
                last_updated_at: Some(now),
                last_error: None,
                cleanup_dismissed_until: None,
            };

            let old_bindings = manifest
                .bindings
                .iter()
                .filter(|binding| binding.installation_id == installation_id)
                .cloned()
                .collect::<Vec<_>>();
            let source_name_conflict = has_active_source_name_conflict(&manifest, &installation);
            let binding_plan = if source_name_conflict {
                None
            } else {
                let detections = self.bindings.detect_all();
                Some(
                    self.bindings
                        .plan_with_bindings(&installation, &detections, &old_bindings)?,
                )
            };

            let content = PathBuf::from(&installation.content_path);
            let had_previous = activate_staging(&staging, &content, &previous, &root)?;
            let binding_report = binding_plan
                .map(|plan| self.bindings.apply(plan))
                .unwrap_or_default();

            let mut warnings = binding_warnings(&binding_report);
            if source_name_conflict {
                installation.status = ManagedSkillStatus::Conflict;
                warnings.push(ManagedWarning {
                    code: ManagedWarningCode::SkillSourceNameConflict,
                    agent: None,
                });
            } else if warnings.iter().any(|warning| {
                matches!(
                    warning.code,
                    ManagedWarningCode::BindingConflict
                        | ManagedWarningCode::BindingUnsupported
                        | ManagedWarningCode::BindingError
                )
            }) {
                installation.status = ManagedSkillStatus::ActionRequired;
            }

            upsert_installation(&mut manifest, installation.clone());
            merge_binding_results(
                &mut manifest,
                &installation.installation_id,
                &binding_report,
            );
            if let Err(error) = self.manifest_store.write(&self.paths, &manifest) {
                rollback_transaction(
                    &self.bindings,
                    &installation,
                    &binding_report,
                    &content,
                    &previous,
                    had_previous,
                    &root,
                )?;
                return Err(error);
            }

            if cleanup_owned_directory(&previous, &root).is_err() {
                warnings.push(ManagedWarning {
                    code: ManagedWarningCode::CleanupDeferred,
                    agent: None,
                });
            }
            let bindings = manifest
                .bindings
                .iter()
                .filter(|binding| binding.installation_id == installation.installation_id)
                .cloned()
                .collect();
            Ok(ManagedInstallOutcome {
                installation,
                bindings,
                required_env_vars,
                warnings,
            })
        }
        .await;

        cleanup_owned_file(&download, &root);
        if staging.exists() {
            let _ = cleanup_owned_directory(&staging, &root);
        }
        result
    }

    fn reject_modified_existing(&self, installation: &ManagedSkill) -> Result<(), ManagedError> {
        let content = Path::new(&installation.content_path);
        let actual = compute_tree_hash(content)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ContentModified))?;
        if actual != installation.content_hash {
            return Err(ManagedError::new(ManagedErrorCode::ContentModified));
        }
        Ok(())
    }

    fn reconcile_idempotent(
        &self,
        mut installation: ManagedSkill,
        mut manifest: ManagedSkillsManifest,
    ) -> Result<ManagedInstallOutcome, ManagedError> {
        let old_bindings = manifest
            .bindings
            .iter()
            .filter(|binding| binding.installation_id == installation.installation_id)
            .cloned()
            .collect::<Vec<_>>();
        let source_name_conflict = has_active_source_name_conflict(&manifest, &installation);
        let binding_report = if source_name_conflict {
            BindingApplyReport::default()
        } else {
            let detections = self.bindings.detect_all();
            let plan =
                self.bindings
                    .plan_with_bindings(&installation, &detections, &old_bindings)?;
            self.bindings.apply(plan)
        };
        let mut warnings = binding_warnings(&binding_report);
        if source_name_conflict {
            installation.status = ManagedSkillStatus::Conflict;
            warnings.push(ManagedWarning {
                code: ManagedWarningCode::SkillSourceNameConflict,
                agent: None,
            });
        } else if warnings.iter().any(|warning| {
            matches!(
                warning.code,
                ManagedWarningCode::BindingConflict
                    | ManagedWarningCode::BindingUnsupported
                    | ManagedWarningCode::BindingError
            )
        }) {
            installation.status = ManagedSkillStatus::ActionRequired;
        }
        installation.last_checked_at = Some(current_timestamp());
        upsert_installation(&mut manifest, installation.clone());
        merge_binding_results(
            &mut manifest,
            &installation.installation_id,
            &binding_report,
        );
        self.manifest_store.write(&self.paths, &manifest)?;
        let skill_md = fs::read_to_string(Path::new(&installation.content_path).join("SKILL.md"))
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        let required_env_vars = crate::commands::local::parse_env_from_frontmatter(&skill_md)
            .into_iter()
            .filter(|variable| variable.required)
            .collect();
        let bindings = manifest
            .bindings
            .into_iter()
            .filter(|binding| binding.installation_id == installation.installation_id)
            .collect();
        Ok(ManagedInstallOutcome {
            installation,
            bindings,
            required_env_vars,
            warnings,
        })
    }
}

fn validate_target(
    request: &ManagedRegistryRequest,
    target: &ManagedTarget,
) -> Result<(), ManagedError> {
    if target.consumer_org != request.consumer_org
        || target.source_org != request.source_org
        || target.skill_name != request.name
        || !is_valid_version(&target.resolved_version)
        || !is_sha256(&target.sha256)
        || !matches!(target.policy.mode.as_str(), "pinned" | "latest_approved")
    {
        return Err(ManagedError::new(ManagedErrorCode::DownloadMetadataInvalid));
    }
    Ok(())
}

fn validate_download_metadata(
    request: &ManagedRegistryRequest,
    metadata: &ManagedDownloadMetadata,
) -> Result<(), ManagedError> {
    if metadata.consumer_org != request.consumer_org
        || metadata.source_org != request.source_org
        || metadata.skill_id.is_empty()
        || metadata.skill_id.len() > 256
        || metadata.skill_id.chars().any(char::is_control)
        || !is_valid_version(&metadata.resolved_version)
        || !is_sha256(&metadata.sha256)
        || !matches!(metadata.policy.as_str(), "pinned" | "latest_approved")
    {
        return Err(ManagedError::new(ManagedErrorCode::DownloadMetadataInvalid));
    }
    Ok(())
}

fn is_valid_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 128
        && !version.eq_ignore_ascii_case("latest")
        && version
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".+-_".contains(character))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn verify_download_file(path: &Path, expected: &str) -> Result<(), ManagedError> {
    let mut file =
        File::open(path).map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if format!("{:x}", hasher.finalize()) != expected {
        return Err(ManagedError::new(ManagedErrorCode::ArchiveChecksumMismatch));
    }
    Ok(())
}

fn validate_extracted_skill(
    staging: &Path,
    expected_name: &str,
) -> Result<(String, Vec<EnvVarDecl>), ManagedError> {
    let skill_md_path = staging.join("SKILL.md");
    let metadata = fs::symlink_metadata(&skill_md_path)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
    }
    let content = fs::read_to_string(&skill_md_path)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    let frontmatter = crate::commands::local::parse_frontmatter_pub(&content);
    if frontmatter.get("name").map(String::as_str) != Some(expected_name)
        || frontmatter
            .get("description")
            .is_none_or(|description| description.trim().is_empty())
    {
        return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
    }
    let required = crate::commands::local::parse_env_from_frontmatter(&content)
        .into_iter()
        .filter(|variable| variable.required)
        .collect();
    Ok((compute_tree_hash(staging)?, required))
}

fn ensure_managed_skill_root(
    paths: &ManagedPaths,
    request: &ManagedInstallRequest,
) -> Result<PathBuf, ManagedError> {
    let root = paths.skill_root(&request.consumer_org, &request.source_org, &request.name)?;
    ensure_safe_directory_tree(paths.home(), &root)?;
    Ok(root)
}

fn ensure_safe_directory_tree(home: &Path, directory: &Path) -> Result<(), ManagedError> {
    if !directory.starts_with(home) {
        return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
    }
    let mut current = home.to_path_buf();
    for component in directory
        .strip_prefix(home)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot))?
        .components()
    {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot))?;
            }
            Err(_) => {
                return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
            }
        }
    }
    Ok(())
}

fn recover_interrupted_swap(
    paths: &ManagedPaths,
    request: &ManagedInstallRequest,
) -> Result<(), ManagedError> {
    let root = paths.skill_root(&request.consumer_org, &request.source_org, &request.name)?;
    if !root.exists() {
        return Ok(());
    }
    let content = paths.content_dir(&request.consumer_org, &request.source_org, &request.name)?;
    let previous = paths.previous_dir(&request.consumer_org, &request.source_org, &request.name)?;
    if previous.exists() && !content.exists() {
        fs::rename(&previous, &content)
            .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
    } else if previous.exists() && content.exists() {
        cleanup_owned_directory(&previous, &root)?;
    }
    let staging = paths.staging_dir(&request.consumer_org, &request.source_org, &request.name)?;
    cleanup_owned_directory(&staging, &root)
}

fn activate_staging(
    staging: &Path,
    content: &Path,
    previous: &Path,
    root: &Path,
) -> Result<bool, ManagedError> {
    cleanup_owned_directory(previous, root)?;
    let had_previous = match fs::symlink_metadata(content) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::rename(content, previous)
                .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
            true
        }
        Ok(_) => return Err(ManagedError::new(ManagedErrorCode::ContentModified)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err(ManagedError::new(ManagedErrorCode::RollbackFailed)),
    };
    if fs::rename(staging, content).is_err() {
        if had_previous {
            let _ = fs::rename(previous, content);
        }
        return Err(ManagedError::new(ManagedErrorCode::RollbackFailed));
    }
    Ok(had_previous)
}

fn rollback_transaction<A: AgentRegistry, L: PlatformLinker>(
    bindings: &ManagedBindingService<A, L>,
    installation: &ManagedSkill,
    report: &BindingApplyReport,
    content: &Path,
    previous: &Path,
    had_previous: bool,
    root: &Path,
) -> Result<(), ManagedError> {
    for result in report.results.iter().rev() {
        if result.created {
            if let Some(binding) = &result.binding {
                bindings
                    .remove_owned(installation, binding)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
            }
        }
    }
    cleanup_owned_directory(content, root)
        .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
    if had_previous {
        fs::rename(previous, content)
            .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
    }
    Ok(())
}

fn cleanup_owned_directory(path: &Path, root: &Path) -> Result<(), ManagedError> {
    if !path.starts_with(root) || path == root {
        return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
    }
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)
                .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))
        }
        Ok(_) | Err(_) => Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot)),
    }
}

fn cleanup_owned_file(path: &Path, root: &Path) {
    if path.starts_with(root) && path != root {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if metadata.is_file() && !metadata.file_type().is_symlink() {
                let _ = fs::remove_file(path);
            }
        }
    }
}

fn has_active_source_name_conflict(
    manifest: &ManagedSkillsManifest,
    installation: &ManagedSkill,
) -> bool {
    let skills = manifest
        .skills
        .iter()
        .map(|skill| (skill.installation_id.as_str(), skill))
        .collect::<HashMap<_, _>>();
    manifest.bindings.iter().any(|binding| {
        binding.status.is_active()
            && skills
                .get(binding.installation_id.as_str())
                .is_some_and(|existing| {
                    existing.consumer_org == installation.consumer_org
                        && existing.skill_name == installation.skill_name
                        && existing.source_org != installation.source_org
                })
    })
}

fn binding_warnings(report: &BindingApplyReport) -> Vec<ManagedWarning> {
    report
        .results
        .iter()
        .filter_map(|result| {
            let code = match result.status {
                BindingStatus::Conflict => ManagedWarningCode::BindingConflict,
                BindingStatus::Unsupported => ManagedWarningCode::BindingUnsupported,
                BindingStatus::Error => ManagedWarningCode::BindingError,
                _ => return None,
            };
            Some(ManagedWarning {
                code,
                agent: Some(result.agent),
            })
        })
        .collect()
}

fn upsert_installation(manifest: &mut ManagedSkillsManifest, installation: ManagedSkill) {
    if let Some(existing) = manifest
        .skills
        .iter_mut()
        .find(|skill| skill.installation_id == installation.installation_id)
    {
        *existing = installation;
    } else {
        manifest.skills.push(installation);
    }
}

fn merge_binding_results(
    manifest: &mut ManagedSkillsManifest,
    installation_id: &str,
    report: &BindingApplyReport,
) {
    for result in &report.results {
        let Some(binding) = &result.binding else {
            continue;
        };
        if let Some(existing) = manifest.bindings.iter_mut().find(|existing| {
            existing.installation_id == installation_id && existing.agent == binding.agent
        }) {
            *existing = binding.clone();
        } else {
            manifest.bindings.push(binding.clone());
        }
    }
}

fn has_binding_action(manifest: &ManagedSkillsManifest, installation_id: &str) -> bool {
    manifest
        .bindings
        .iter()
        .any(|binding| binding.installation_id == installation_id && !binding.status.is_active())
}

fn status_without_update(
    manifest: &ManagedSkillsManifest,
    installation: &ManagedSkill,
) -> ManagedSkillStatus {
    if has_binding_action(manifest, &installation.installation_id) {
        ManagedSkillStatus::ActionRequired
    } else {
        ManagedSkillStatus::Ready
    }
}

fn update_checked_installation(
    manifest: &mut ManagedSkillsManifest,
    index: usize,
    status: ManagedSkillStatus,
    checked_at: String,
    error: Option<ManagedError>,
) {
    manifest.skills[index].status = status;
    manifest.skills[index].last_checked_at = Some(checked_at.clone());
    manifest.skills[index].last_error = error.map(|error| error.into_record(Some(checked_at)));
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    fn request() -> ManagedRegistryRequest {
        ManagedRegistryRequest {
            consumer_org: "acme".to_string(),
            source_org: "publisher".to_string(),
            name: "review-helper".to_string(),
        }
    }

    fn valid_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-skill-id", HeaderValue::from_static("skill-id"));
        headers.insert("x-skill-version", HeaderValue::from_static("1.2.3"));
        headers.insert(
            "x-skill-sha256",
            HeaderValue::from_str(&"a".repeat(64)).unwrap(),
        );
        headers.insert("x-skill-source-org", HeaderValue::from_static("publisher"));
        headers.insert("x-skill-consumer-org", HeaderValue::from_static("acme"));
        headers.insert(
            "x-skill-version-policy",
            HeaderValue::from_static("latest_approved"),
        );
        headers
    }

    #[test]
    fn managed_urls_never_contain_a_version_or_query_parameter() {
        let request = request();

        assert_eq!(
            request.download_url("https://app.skillreg.dev"),
            "https://app.skillreg.dev/api/v1/orgs/acme/managed-skills/publisher/review-helper/download"
        );
        assert_eq!(
            request.target_url("https://app.skillreg.dev"),
            "https://app.skillreg.dev/api/v1/orgs/acme/managed-skills/publisher/review-helper/target"
        );
        assert!(!request
            .download_url("https://app.skillreg.dev")
            .contains('?'));
    }

    #[test]
    fn missing_version_or_checksum_headers_are_rejected() {
        for header in ["x-skill-version", "x-skill-sha256"] {
            let mut headers = valid_headers();
            headers.remove(header);

            let error = parse_download_metadata(&headers, &request()).unwrap_err();

            assert_eq!(error.code(), ManagedErrorCode::DownloadMetadataInvalid);
        }
    }

    #[test]
    fn latest_and_cross_org_metadata_are_rejected() {
        let mut latest = valid_headers();
        latest.insert("x-skill-version", HeaderValue::from_static("latest"));
        assert_eq!(
            parse_download_metadata(&latest, &request())
                .unwrap_err()
                .code(),
            ManagedErrorCode::DownloadMetadataInvalid
        );

        let mut other_org = valid_headers();
        other_org.insert("x-skill-consumer-org", HeaderValue::from_static("other"));
        assert_eq!(
            parse_download_metadata(&other_org, &request())
                .unwrap_err()
                .code(),
            ManagedErrorCode::DownloadMetadataInvalid
        );
    }

    fn managed_installation() -> ManagedSkill {
        ManagedSkill {
            installation_id: Uuid::new_v4().to_string(),
            consumer_org: "acme".to_string(),
            source_org: "publisher".to_string(),
            skill_id: Some("skill-id".to_string()),
            skill_name: "review-helper".to_string(),
            origin: ManagedSkillOrigin::Registry,
            active_version: "1.2.3".to_string(),
            sha256: "a".repeat(64),
            content_path: "/tmp/content".to_string(),
            content_hash: "b".repeat(64),
            status: ManagedSkillStatus::Ready,
            installed_at: "1".to_string(),
            last_checked_at: Some("1".to_string()),
            last_updated_at: Some("1".to_string()),
            last_error: None,
            cleanup_dismissed_until: None,
        }
    }

    #[test]
    fn managed_update_decision_uses_the_server_target_not_semver() {
        let installation = managed_installation();
        let mut target = ManagedTarget {
            skill_id: "skill-id".to_string(),
            source_org: "publisher".to_string(),
            consumer_org: "acme".to_string(),
            skill_name: "review-helper".to_string(),
            resolved_version: "1.2.3".to_string(),
            sha256: "a".repeat(64),
            validation_level: "verified".to_string(),
            policy: ManagedTargetPolicy {
                mode: "pinned".to_string(),
                version: Some("1.2.3".to_string()),
            },
        };

        assert_eq!(
            decide_managed_update(false, false, true, true, &installation, Ok(&target)),
            ManagedUpdateDecision::SkipGlobalDisabled
        );
        assert_eq!(
            decide_managed_update(false, true, false, true, &installation, Ok(&target)),
            ManagedUpdateDecision::UpToDate
        );

        target.resolved_version = "0.9.0".to_string();
        target.policy.version = Some("0.9.0".to_string());
        assert_eq!(
            decide_managed_update(true, false, false, true, &installation, Ok(&target)),
            ManagedUpdateDecision::UpdateAvailable
        );
        assert_eq!(
            decide_managed_update(true, false, true, true, &installation, Ok(&target)),
            ManagedUpdateDecision::UpdateNow
        );
    }

    #[test]
    fn local_skills_are_never_candidates_for_registry_updates() {
        let mut installation = managed_installation();
        installation.origin = ManagedSkillOrigin::Local;
        let target = ManagedTarget {
            skill_id: "skill-id".to_string(),
            source_org: "publisher".to_string(),
            consumer_org: "acme".to_string(),
            skill_name: "review-helper".to_string(),
            resolved_version: "9.9.9".to_string(),
            sha256: "c".repeat(64),
            validation_level: "verified".to_string(),
            policy: ManagedTargetPolicy {
                mode: "latest_approved".to_string(),
                version: None,
            },
        };

        assert_eq!(
            decide_managed_update(true, true, true, true, &installation, Ok(&target)),
            ManagedUpdateDecision::SkipLocal
        );
    }

    #[test]
    fn managed_update_decision_blocks_modified_content_checksum_and_policy_failures() {
        let installation = managed_installation();
        let mut target = ManagedTarget {
            skill_id: "skill-id".to_string(),
            source_org: "publisher".to_string(),
            consumer_org: "acme".to_string(),
            skill_name: "review-helper".to_string(),
            resolved_version: "2.0.0".to_string(),
            sha256: "a".repeat(64),
            validation_level: "verified".to_string(),
            policy: ManagedTargetPolicy {
                mode: "latest_approved".to_string(),
                version: None,
            },
        };

        assert_eq!(
            decide_managed_update(true, false, true, false, &installation, Ok(&target)),
            ManagedUpdateDecision::BlockModifiedContent
        );
        target.sha256.clear();
        assert_eq!(
            decide_managed_update(true, false, true, true, &installation, Ok(&target)),
            ManagedUpdateDecision::BlockMissingChecksum
        );
        target.sha256 = "a".repeat(64);
        target.policy.mode = "publisher_latest".to_string();
        assert_eq!(
            decide_managed_update(true, false, true, true, &installation, Ok(&target)),
            ManagedUpdateDecision::BlockPolicy
        );
        assert_eq!(
            decide_managed_update(
                true,
                false,
                true,
                true,
                &installation,
                Err(ManagedErrorCode::RegistryRequestFailed),
            ),
            ManagedUpdateDecision::RetryLater
        );
    }
}
