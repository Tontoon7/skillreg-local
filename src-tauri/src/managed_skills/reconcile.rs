use crate::managed_skills::{
    agents::AgentRegistry,
    archive::compute_tree_hash,
    bindings::{BindingApplyReport, ManagedBindingService},
    errors::{ManagedError, ManagedErrorCode},
    manifest::ManagedSkillsManifest,
    migration::ReconcileReport,
    paths::{validate_org_slug, ManagedPaths},
    platform_links::PlatformLinker,
    service::{acquire_managed_mutation_lock, ManagedManifestStore},
    BindingStatus, ManagedBinding, ManagedSkill, ManagedSkillStatus,
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUninstallResult {
    pub installation_id: String,
    pub removed: bool,
    pub already_removed: bool,
    pub bindings_removed: usize,
    pub conflicts: usize,
    pub env_values_preserved: bool,
    pub cleanup_deferred: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveOrgSwitchReport {
    pub previous_org: Option<String>,
    pub active_org: String,
    pub bindings_removed: usize,
    pub bindings_created: usize,
    pub cached_skills_reused: usize,
}

pub struct ManagedLifecycleService<A, L, S>
where
    A: AgentRegistry,
    L: PlatformLinker,
    S: ManagedManifestStore,
{
    bindings: ManagedBindingService<A, L>,
    manifest_store: S,
    paths: ManagedPaths,
}

impl<A, L, S> ManagedLifecycleService<A, L, S>
where
    A: AgentRegistry,
    L: PlatformLinker,
    S: ManagedManifestStore,
{
    pub fn new(agents: A, linker: L, manifest_store: S, paths: ManagedPaths) -> Self {
        Self {
            bindings: ManagedBindingService::new(agents, linker, paths.clone()),
            manifest_store,
            paths,
        }
    }

    pub async fn uninstall(
        &self,
        installation_id: &str,
    ) -> Result<ManagedUninstallResult, ManagedError> {
        let _guard = acquire_managed_mutation_lock().await;
        let mut manifest = self.manifest_store.load(&self.paths)?;
        let Some(index) = manifest
            .skills
            .iter()
            .position(|skill| skill.installation_id == installation_id)
        else {
            if manifest
                .removed_installation_ids
                .iter()
                .any(|removed| removed == installation_id)
            {
                return Ok(ManagedUninstallResult {
                    installation_id: installation_id.to_string(),
                    removed: false,
                    already_removed: true,
                    bindings_removed: 0,
                    conflicts: 0,
                    env_values_preserved: true,
                    cleanup_deferred: false,
                });
            }
            return Err(ManagedError::new(
                ManagedErrorCode::ManagedInstallationNotFound,
            ));
        };

        let installation = manifest.skills[index].clone();
        verify_content_hash(&installation)?;
        let owned_bindings = manifest
            .bindings
            .iter()
            .filter(|binding| binding.installation_id == installation_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut conflicts = 0;
        for binding in &owned_bindings {
            match self.bindings.verify_owned_removal(&installation, binding) {
                Ok(()) => {}
                Err(error) if error.code() == ManagedErrorCode::BindingConflict => {
                    conflicts += 1;
                }
                Err(error) => return Err(error),
            }
        }
        if conflicts > 0 {
            return Ok(ManagedUninstallResult {
                installation_id: installation_id.to_string(),
                removed: false,
                already_removed: false,
                bindings_removed: 0,
                conflicts,
                env_values_preserved: true,
                cleanup_deferred: false,
            });
        }

        let mut removed_bindings = Vec::new();
        for binding in &owned_bindings {
            if let Err(error) = self.bindings.remove_owned(&installation, binding) {
                restore_bindings(&self.bindings, &installation, &owned_bindings)?;
                return Err(error);
            }
            removed_bindings.push(binding.clone());
        }

        let skill_root = self.paths.skill_root(
            &installation.consumer_org,
            &installation.source_org,
            &installation.skill_name,
        )?;
        let removing_root = removing_path(&skill_root);
        if let Err(error) = rename_owned_root(&skill_root, &removing_root) {
            restore_bindings(&self.bindings, &installation, &owned_bindings)?;
            return Err(error);
        }

        manifest.skills.remove(index);
        manifest
            .bindings
            .retain(|binding| binding.installation_id != installation_id);
        record_tombstone(&mut manifest, installation_id);
        if let Err(error) = self.manifest_store.write(&self.paths, &manifest) {
            restore_owned_root(&skill_root, &removing_root)?;
            restore_bindings(&self.bindings, &installation, &owned_bindings)?;
            return Err(error);
        }

        let cleanup_deferred = remove_owned_root(&removing_root).is_err();
        Ok(ManagedUninstallResult {
            installation_id: installation_id.to_string(),
            removed: true,
            already_removed: false,
            bindings_removed: removed_bindings.len(),
            conflicts: 0,
            env_values_preserved: true,
            cleanup_deferred,
        })
    }

    pub async fn repair_one(&self, installation_id: &str) -> Result<ReconcileReport, ManagedError> {
        let _guard = acquire_managed_mutation_lock().await;
        let manifest = self.manifest_store.load(&self.paths)?;
        let installation = manifest
            .skills
            .iter()
            .find(|skill| skill.installation_id == installation_id)
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::ManagedInstallationNotFound))?;
        if manifest
            .active_org
            .as_deref()
            .is_some_and(|active_org| active_org != installation.consumer_org)
        {
            return Err(ManagedError::new(
                ManagedErrorCode::ActiveOrganizationConflict,
            ));
        }
        self.repair_serialized(manifest, &[installation_id.to_string()])
    }

    pub async fn repair_all(&self) -> Result<ReconcileReport, ManagedError> {
        let _guard = acquire_managed_mutation_lock().await;
        let mut manifest = self.manifest_store.load(&self.paths)?;
        let active_org = resolve_active_org(&manifest)?;
        if active_org.is_none()
            && manifest
                .skills
                .iter()
                .map(|skill| skill.consumer_org.as_str())
                .collect::<HashSet<_>>()
                .len()
                > 1
        {
            return Err(ManagedError::new(
                ManagedErrorCode::ActiveOrganizationConflict,
            ));
        }
        if manifest.active_org.is_none() {
            manifest.active_org = active_org.clone();
        }
        let installation_ids = manifest
            .skills
            .iter()
            .filter(|skill| {
                active_org
                    .as_deref()
                    .is_none_or(|active| skill.consumer_org == active)
            })
            .map(|skill| skill.installation_id.clone())
            .collect::<Vec<_>>();
        self.repair_serialized(manifest, &installation_ids)
    }

    pub async fn switch_active_org(
        &self,
        target_org: &str,
        authorized_orgs: &[String],
    ) -> Result<ActiveOrgSwitchReport, ManagedError> {
        let _guard = acquire_managed_mutation_lock().await;
        self.switch_active_org_serialized(target_org, authorized_orgs)
    }

    pub(crate) fn switch_active_org_serialized(
        &self,
        target_org: &str,
        authorized_orgs: &[String],
    ) -> Result<ActiveOrgSwitchReport, ManagedError> {
        validate_org_slug(target_org)?;
        if !authorized_orgs.iter().any(|org| org == target_org) {
            return Err(ManagedError::new(
                ManagedErrorCode::ActiveOrganizationUnauthorized,
            ));
        }

        let mut manifest = self.manifest_store.load(&self.paths)?;
        let previous_org = resolve_active_org(&manifest)?;
        if previous_org.as_deref() == Some(target_org) {
            return Ok(ActiveOrgSwitchReport {
                previous_org,
                active_org: target_org.to_string(),
                bindings_removed: 0,
                bindings_created: 0,
                cached_skills_reused: manifest
                    .skills
                    .iter()
                    .filter(|skill| skill.consumer_org == target_org)
                    .count(),
            });
        }

        let skills_by_id = manifest
            .skills
            .iter()
            .map(|skill| (skill.installation_id.as_str(), skill))
            .collect::<HashMap<_, _>>();
        let old_bindings = manifest
            .bindings
            .iter()
            .filter(|binding| {
                binding.status.is_active()
                    && skills_by_id
                        .get(binding.installation_id.as_str())
                        .is_some_and(|skill| skill.consumer_org != target_org)
            })
            .cloned()
            .collect::<Vec<_>>();
        for binding in &old_bindings {
            let installation = skills_by_id
                .get(binding.installation_id.as_str())
                .ok_or_else(|| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
            self.bindings.verify_owned_removal(installation, binding)?;
        }

        let target_skills = manifest
            .skills
            .iter()
            .filter(|skill| skill.consumer_org == target_org)
            .cloned()
            .collect::<Vec<_>>();
        validate_target_skill_set(&target_skills)?;
        let detections = self.bindings.detect_all();
        let target_plans = target_skills
            .iter()
            .map(|skill| {
                verify_content_hash(skill)?;
                let existing = manifest
                    .bindings
                    .iter()
                    .filter(|binding| binding.installation_id == skill.installation_id)
                    .cloned()
                    .collect::<Vec<_>>();
                self.bindings
                    .plan_with_bindings(skill, &detections, &existing)
            })
            .collect::<Result<Vec<_>, _>>()?;

        for binding in &old_bindings {
            let installation = skills_by_id
                .get(binding.installation_id.as_str())
                .ok_or_else(|| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
            if let Err(error) = self.bindings.remove_owned(installation, binding) {
                restore_old_bindings(&self.bindings, &manifest, &old_bindings)?;
                return Err(error);
            }
        }

        let mut target_reports = Vec::new();
        for plan in target_plans {
            let installation = plan.installation.clone();
            target_reports.push((installation, self.bindings.apply(plan)));
        }
        if let Some(error_code) = first_binding_failure(&target_reports) {
            rollback_target_bindings(&self.bindings, &target_reports)?;
            restore_old_bindings(&self.bindings, &manifest, &old_bindings)?;
            return Err(ManagedError::new(error_code));
        }

        for binding in &old_bindings {
            if let Some(record) = manifest.bindings.iter_mut().find(|record| {
                record.installation_id == binding.installation_id && record.agent == binding.agent
            }) {
                record.status = BindingStatus::Missing;
                record.last_error = None;
            }
        }
        for (installation, report) in &target_reports {
            merge_binding_report(&mut manifest, report);
            set_installation_status(&mut manifest, installation, report);
        }
        manifest.active_org = Some(target_org.to_string());
        if let Err(error) = self.manifest_store.write(&self.paths, &manifest) {
            rollback_target_bindings(&self.bindings, &target_reports)?;
            let original = self.manifest_store.load(&self.paths)?;
            restore_old_bindings(&self.bindings, &original, &old_bindings)?;
            return Err(error);
        }

        Ok(ActiveOrgSwitchReport {
            previous_org,
            active_org: target_org.to_string(),
            bindings_removed: old_bindings.len(),
            bindings_created: target_reports
                .iter()
                .flat_map(|(_, report)| &report.results)
                .filter(|result| result.created && result.status.is_active())
                .count(),
            cached_skills_reused: target_skills.len(),
        })
    }

    fn repair_serialized(
        &self,
        mut manifest: ManagedSkillsManifest,
        installation_ids: &[String],
    ) -> Result<ReconcileReport, ManagedError> {
        let detections = self.bindings.detect_all();
        let mut summary = ReconcileReport::default();
        let mut created_bindings = Vec::new();
        let now = current_timestamp();

        for installation_id in installation_ids {
            let Some(index) = manifest
                .skills
                .iter()
                .position(|skill| &skill.installation_id == installation_id)
            else {
                continue;
            };
            summary.checked += 1;
            let installation = manifest.skills[index].clone();
            if verify_content_hash(&installation).is_err() {
                summary.modified_content += 1;
                manifest.skills[index].status = ManagedSkillStatus::ActionRequired;
                manifest.skills[index].last_checked_at = Some(now.clone());
                manifest.skills[index].last_error = Some(
                    ManagedError::new(ManagedErrorCode::ContentModified)
                        .into_record(Some(now.clone())),
                );
                continue;
            }

            let existing = manifest
                .bindings
                .iter()
                .filter(|binding| binding.installation_id == installation.installation_id)
                .cloned()
                .collect::<Vec<_>>();
            let report = self
                .bindings
                .reconcile(&installation, &existing, &detections);
            summary.repaired += report
                .results
                .iter()
                .filter(|result| result.created && result.status.is_active())
                .count();
            summary.conflicts += report
                .results
                .iter()
                .filter(|result| {
                    matches!(
                        result.status,
                        BindingStatus::Conflict | BindingStatus::Unsupported
                    )
                })
                .count();
            summary.failed += report
                .results
                .iter()
                .filter(|result| result.status == BindingStatus::Error)
                .count();
            created_bindings.extend(report.results.iter().filter_map(|result| {
                (result.created)
                    .then(|| result.binding.clone())
                    .flatten()
                    .map(|binding| (installation.clone(), binding))
            }));
            merge_binding_report(&mut manifest, &report);
            set_installation_status(&mut manifest, &installation, &report);
            if let Some(record) = manifest.skills.get_mut(index) {
                record.last_checked_at = Some(now.clone());
            }
        }

        if let Err(error) = self.manifest_store.write(&self.paths, &manifest) {
            for (installation, binding) in created_bindings.iter().rev() {
                self.bindings
                    .remove_owned(installation, binding)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
            }
            return Err(error);
        }
        Ok(summary)
    }
}

fn verify_content_hash(installation: &ManagedSkill) -> Result<(), ManagedError> {
    let content = Path::new(&installation.content_path);
    let actual = compute_tree_hash(content)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ContentModified))?;
    if actual != installation.content_hash {
        return Err(ManagedError::new(ManagedErrorCode::ContentModified));
    }
    Ok(())
}

fn validate_target_skill_set(skills: &[ManagedSkill]) -> Result<(), ManagedError> {
    let mut names = HashSet::new();
    if skills
        .iter()
        .any(|skill| !names.insert(skill.skill_name.as_str()))
    {
        return Err(ManagedError::new(ManagedErrorCode::SkillSourceNameConflict));
    }
    Ok(())
}

fn resolve_active_org(manifest: &ManagedSkillsManifest) -> Result<Option<String>, ManagedError> {
    if manifest.active_org.is_some() {
        return Ok(manifest.active_org.clone());
    }
    let skills_by_id = manifest
        .skills
        .iter()
        .map(|skill| (skill.installation_id.as_str(), skill))
        .collect::<HashMap<_, _>>();
    let mut active_orgs = manifest
        .bindings
        .iter()
        .filter(|binding| binding.status.is_active())
        .filter_map(|binding| {
            skills_by_id
                .get(binding.installation_id.as_str())
                .map(|skill| skill.consumer_org.as_str())
        })
        .collect::<HashSet<_>>();
    if active_orgs.len() > 1 {
        return Err(ManagedError::new(
            ManagedErrorCode::ActiveOrganizationConflict,
        ));
    }
    if let Some(active) = active_orgs.drain().next() {
        return Ok(Some(active.to_string()));
    }

    let orgs = manifest
        .skills
        .iter()
        .map(|skill| skill.consumer_org.as_str())
        .collect::<HashSet<_>>();
    if orgs.len() > 1 {
        return Ok(None);
    }
    Ok(orgs.into_iter().next().map(ToOwned::to_owned))
}

fn merge_binding_report(manifest: &mut ManagedSkillsManifest, report: &BindingApplyReport) {
    for result in &report.results {
        if let Some(binding) = &result.binding {
            if let Some(existing) = manifest.bindings.iter_mut().find(|existing| {
                existing.installation_id == binding.installation_id
                    && existing.agent == binding.agent
            }) {
                *existing = binding.clone();
            } else {
                manifest.bindings.push(binding.clone());
            }
        }
    }
}

fn set_installation_status(
    manifest: &mut ManagedSkillsManifest,
    installation: &ManagedSkill,
    report: &BindingApplyReport,
) {
    let Some(record) = manifest
        .skills
        .iter_mut()
        .find(|record| record.installation_id == installation.installation_id)
    else {
        return;
    };
    let action_required = report
        .results
        .iter()
        .any(|result| !result.status.is_active());
    record.status = if action_required {
        ManagedSkillStatus::ActionRequired
    } else {
        ManagedSkillStatus::Ready
    };
    record.last_error = report
        .results
        .iter()
        .find_map(|result| result.error.clone());
}

fn restore_bindings<A: AgentRegistry, L: PlatformLinker>(
    service: &ManagedBindingService<A, L>,
    installation: &ManagedSkill,
    bindings: &[ManagedBinding],
) -> Result<(), ManagedError> {
    let detections = service.detect_all();
    let report = service.apply(service.plan_with_bindings(installation, &detections, bindings)?);
    if report
        .results
        .iter()
        .all(|result| result.status.is_active())
    {
        Ok(())
    } else {
        Err(ManagedError::new(ManagedErrorCode::RollbackFailed))
    }
}

fn restore_old_bindings<A: AgentRegistry, L: PlatformLinker>(
    service: &ManagedBindingService<A, L>,
    manifest: &ManagedSkillsManifest,
    bindings: &[ManagedBinding],
) -> Result<(), ManagedError> {
    let skills = manifest
        .skills
        .iter()
        .map(|skill| (skill.installation_id.as_str(), skill))
        .collect::<HashMap<_, _>>();
    let installation_ids = bindings
        .iter()
        .map(|binding| binding.installation_id.as_str())
        .collect::<HashSet<_>>();
    for installation_id in installation_ids {
        let installation = skills
            .get(installation_id)
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
        let owned = bindings
            .iter()
            .filter(|binding| binding.installation_id == installation_id)
            .cloned()
            .collect::<Vec<_>>();
        restore_bindings(service, installation, &owned)?;
    }
    Ok(())
}

fn rollback_target_bindings<A: AgentRegistry, L: PlatformLinker>(
    service: &ManagedBindingService<A, L>,
    reports: &[(ManagedSkill, BindingApplyReport)],
) -> Result<(), ManagedError> {
    for (installation, report) in reports.iter().rev() {
        for result in report.results.iter().rev() {
            if result.created {
                if let Some(binding) = &result.binding {
                    service
                        .remove_owned(installation, binding)
                        .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
                }
            }
        }
    }
    Ok(())
}

fn first_binding_failure(
    reports: &[(ManagedSkill, BindingApplyReport)],
) -> Option<ManagedErrorCode> {
    reports
        .iter()
        .flat_map(|(_, report)| &report.results)
        .find_map(|result| match result.status {
            BindingStatus::Conflict => Some(ManagedErrorCode::BindingConflict),
            BindingStatus::Unsupported => Some(ManagedErrorCode::BindingUnsupported),
            BindingStatus::Error => Some(ManagedErrorCode::BindingVerifyFailed),
            BindingStatus::Ready | BindingStatus::NeedsRestart | BindingStatus::Missing => None,
        })
}

fn removing_path(skill_root: &Path) -> PathBuf {
    let name = skill_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill");
    skill_root.with_file_name(format!("{name}.removing-{}", Uuid::new_v4()))
}

fn rename_owned_root(source: &Path, destination: &Path) -> Result<(), ManagedError> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ContentModified))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || destination.exists() {
        return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
    }
    fs::rename(source, destination).map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))
}

fn restore_owned_root(source: &Path, removing: &Path) -> Result<(), ManagedError> {
    if source.exists() || !removing.exists() {
        return Err(ManagedError::new(ManagedErrorCode::RollbackFailed));
    }
    fs::rename(removing, source).map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))
}

fn remove_owned_root(path: &Path) -> Result<(), ManagedError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
    }
    fs::remove_dir_all(path).map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))
}

fn record_tombstone(manifest: &mut ManagedSkillsManifest, installation_id: &str) {
    manifest
        .removed_installation_ids
        .retain(|removed| removed != installation_id);
    manifest
        .removed_installation_ids
        .push(installation_id.to_string());
    if manifest.removed_installation_ids.len() > 256 {
        let overflow = manifest.removed_installation_ids.len() - 256;
        manifest.removed_installation_ids.drain(0..overflow);
    }
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
