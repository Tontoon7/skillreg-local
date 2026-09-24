use crate::managed_skills::{
    agents::{AgentRegistry, DetectionResult, DetectionState},
    errors::{ManagedError, ManagedErrorCode, ManagedErrorRecord},
    paths::ManagedPaths,
    platform_links::{expected_link_kind, LinkInspection, PlatformLinker},
    AgentId, BindingStatus, ManagedBinding, ManagedSkill, UsageObservability,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BindingOperation {
    pub installation_id: String,
    pub agent: AgentId,
    pub link_path: PathBuf,
    pub target_path: PathBuf,
    pub link_kind: crate::managed_skills::LinkKind,
    pub requires_restart: bool,
    pub create_parent: bool,
    pub owned_binding: Option<ManagedBinding>,
}

#[derive(Debug, Clone)]
pub struct BindingPlan {
    pub installation: ManagedSkill,
    pub operations: Vec<BindingOperation>,
}

#[derive(Debug, Clone)]
pub struct BindingApplyResult {
    pub agent: AgentId,
    pub status: BindingStatus,
    pub binding: Option<ManagedBinding>,
    pub error: Option<ManagedErrorRecord>,
    pub created: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BindingApplyReport {
    pub results: Vec<BindingApplyResult>,
}

pub type BindingReconcileReport = BindingApplyReport;

pub struct ManagedBindingService<A: AgentRegistry, L: PlatformLinker> {
    agents: A,
    linker: L,
    paths: ManagedPaths,
}

impl<A: AgentRegistry, L: PlatformLinker> ManagedBindingService<A, L> {
    pub fn new(agents: A, linker: L, paths: ManagedPaths) -> Self {
        Self {
            agents,
            linker,
            paths,
        }
    }

    pub fn plan(
        &self,
        installation: &ManagedSkill,
        detections: &[DetectionResult],
    ) -> Result<BindingPlan, ManagedError> {
        self.plan_with_bindings(installation, detections, &[])
    }

    pub fn detect_all(&self) -> Vec<DetectionResult> {
        self.agents.detect_all(self.paths.home())
    }

    pub fn agent_registry(&self) -> &A {
        &self.agents
    }

    pub fn plan_with_bindings(
        &self,
        installation: &ManagedSkill,
        detections: &[DetectionResult],
        bindings: &[ManagedBinding],
    ) -> Result<BindingPlan, ManagedError> {
        let target_path = self.validate_installation_target(installation)?;
        let expected_kind =
            expected_link_kind(self.linker.platform()).map_err(ManagedError::new)?;
        if self.linker.link_kind() != expected_kind {
            return Err(ManagedError::new(ManagedErrorCode::BindingUnsupported));
        }

        let mut operations = Vec::new();
        for detection in detections {
            if detection.state != DetectionState::Detected {
                continue;
            }
            let Some(adapter) = self.agents.adapter(detection.agent) else {
                continue;
            };
            if !adapter.supports_managed_links(self.linker.platform()) {
                continue;
            }

            let preferred = adapter
                .preferred_user_skill_dir(self.paths.home())
                .map_err(|_| ManagedError::new(ManagedErrorCode::BindingUnsupported))?;
            if detection.preferred_path.as_ref() != Some(&preferred) {
                return Err(ManagedError::new(ManagedErrorCode::BindingVerifyFailed));
            }
            let link_path = preferred.join(&installation.skill_name);
            self.linker
                .validate_paths(&target_path, &link_path)
                .map_err(ManagedError::new)?;

            let owned_binding = bindings
                .iter()
                .find(|binding| {
                    binding.installation_id == installation.installation_id
                        && binding.agent == detection.agent
                })
                .cloned();
            if let Some(binding) = &owned_binding {
                if Path::new(&binding.link_path) != link_path || binding.link_kind != expected_kind
                {
                    return Err(ManagedError::new(ManagedErrorCode::BindingConflict));
                }
            }

            operations.push(BindingOperation {
                installation_id: installation.installation_id.clone(),
                agent: detection.agent,
                link_path,
                target_path: target_path.clone(),
                link_kind: expected_kind,
                requires_restart: detection.requires_restart_after_binding,
                create_parent: adapter.allows_user_skill_dir_creation(),
                owned_binding,
            });
        }

        Ok(BindingPlan {
            installation: installation.clone(),
            operations,
        })
    }

    pub fn apply(&self, plan: BindingPlan) -> BindingApplyReport {
        let mut results = Vec::with_capacity(plan.operations.len());
        for operation in plan.operations {
            results.push(self.apply_operation(&operation));
        }
        BindingApplyReport { results }
    }

    pub fn remove_owned(
        &self,
        installation: &ManagedSkill,
        binding: &ManagedBinding,
    ) -> Result<(), ManagedError> {
        self.verify_owned_removal(installation, binding)?;
        let expected_link = PathBuf::from(&binding.link_path);
        if matches!(
            self.linker
                .inspect(&expected_link)
                .map_err(ManagedError::new)?,
            LinkInspection::Missing
        ) {
            return Ok(());
        }
        self.linker
            .remove_link(&expected_link, binding.link_kind)
            .map_err(ManagedError::new)
    }

    pub fn verify_owned_removal(
        &self,
        installation: &ManagedSkill,
        binding: &ManagedBinding,
    ) -> Result<(), ManagedError> {
        let target = self.validate_installation_target(installation)?;
        if binding.installation_id != installation.installation_id {
            return Err(ManagedError::new(ManagedErrorCode::BindingConflict));
        }
        let adapter = self
            .agents
            .adapter(binding.agent)
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::BindingUnsupported))?;
        let expected_link = adapter
            .preferred_user_skill_dir(self.paths.home())
            .map_err(|_| ManagedError::new(ManagedErrorCode::BindingUnsupported))?
            .join(&installation.skill_name);
        if Path::new(&binding.link_path) != expected_link
            || binding.link_kind != self.linker.link_kind()
        {
            return Err(ManagedError::new(ManagedErrorCode::BindingConflict));
        }

        match self
            .linker
            .inspect(&expected_link)
            .map_err(ManagedError::new)?
        {
            LinkInspection::Missing => Ok(()),
            LinkInspection::Link {
                kind,
                target: actual_target,
                ..
            } if kind == binding.link_kind && actual_target == target => Ok(()),
            LinkInspection::Link { .. } | LinkInspection::Other => {
                Err(ManagedError::new(ManagedErrorCode::BindingConflict))
            }
        }
    }

    pub fn reconcile(
        &self,
        installation: &ManagedSkill,
        bindings: &[ManagedBinding],
        detections: &[DetectionResult],
    ) -> BindingReconcileReport {
        match self.plan_with_bindings(installation, detections, bindings) {
            Ok(plan) => self.apply(plan),
            Err(error) => BindingApplyReport {
                results: detections
                    .iter()
                    .filter(|detection| detection.state == DetectionState::Detected)
                    .map(|detection| BindingApplyResult {
                        agent: detection.agent,
                        status: BindingStatus::Error,
                        binding: None,
                        error: Some(error.clone().into_record(None)),
                        created: false,
                    })
                    .collect(),
            },
        }
    }

    fn validate_installation_target(
        &self,
        installation: &ManagedSkill,
    ) -> Result<PathBuf, ManagedError> {
        let target = PathBuf::from(&installation.content_path);
        self.paths.validate_managed_content_path(&target)?;
        let expected = self.paths.content_dir(
            &installation.consumer_org,
            &installation.source_org,
            &installation.skill_name,
        )?;
        if target != expected {
            return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
        }
        Ok(target)
    }

    fn apply_operation(&self, operation: &BindingOperation) -> BindingApplyResult {
        match self.try_apply_operation(operation) {
            Ok((binding, created)) => BindingApplyResult {
                agent: operation.agent,
                status: binding.status,
                binding: Some(binding),
                error: None,
                created,
            },
            Err(error) => {
                let status = if error.code() == ManagedErrorCode::BindingConflict {
                    BindingStatus::Conflict
                } else if error.code() == ManagedErrorCode::BindingUnsupported {
                    BindingStatus::Unsupported
                } else {
                    BindingStatus::Error
                };
                let binding = operation.owned_binding.clone().map(|mut binding| {
                    binding.status = status;
                    binding.last_error = Some(error.clone().into_record(None));
                    binding
                });
                BindingApplyResult {
                    agent: operation.agent,
                    status,
                    binding,
                    error: Some(error.into_record(None)),
                    created: false,
                }
            }
        }
    }

    fn try_apply_operation(
        &self,
        operation: &BindingOperation,
    ) -> Result<(ManagedBinding, bool), ManagedError> {
        self.paths
            .validate_managed_content_path(&operation.target_path)?;
        self.validate_existing_target(&operation.target_path)?;
        self.linker
            .validate_paths(&operation.target_path, &operation.link_path)
            .map_err(ManagedError::new)?;
        let parent = operation
            .link_path
            .parent()
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::BindingCreateFailed))?;
        ensure_safe_directory_tree(self.paths.home(), parent, operation.create_parent)?;

        match self
            .linker
            .inspect(&operation.link_path)
            .map_err(ManagedError::new)?
        {
            LinkInspection::Missing => self
                .create_and_verify(operation)
                .map(|binding| (binding, true)),
            LinkInspection::Link {
                kind,
                target,
                target_exists,
            } => {
                let is_owned = operation.owned_binding.as_ref().is_some_and(|binding| {
                    binding.installation_id == operation.installation_id
                        && binding.agent == operation.agent
                        && Path::new(&binding.link_path) == operation.link_path
                        && binding.link_kind == operation.link_kind
                        && kind == operation.link_kind
                        && target == operation.target_path
                });
                if !is_owned {
                    return Err(ManagedError::new(ManagedErrorCode::BindingConflict));
                }
                if target_exists {
                    Ok((self.binding_record(operation), false))
                } else {
                    self.linker
                        .remove_link(&operation.link_path, operation.link_kind)
                        .map_err(ManagedError::new)?;
                    self.create_and_verify(operation)
                        .map(|binding| (binding, true))
                }
            }
            LinkInspection::Other => Err(ManagedError::new(ManagedErrorCode::BindingConflict)),
        }
    }

    fn create_and_verify(
        &self,
        operation: &BindingOperation,
    ) -> Result<ManagedBinding, ManagedError> {
        let temporary = temporary_link_path(&operation.link_path);
        self.linker
            .create_dir_link(&operation.target_path, &temporary)
            .map_err(ManagedError::new)?;

        let temporary_verified = self
            .linker
            .inspect(&temporary)
            .map(|inspection| link_matches(&inspection, operation))
            .unwrap_or(false);
        if !temporary_verified {
            let _ = self.linker.remove_link(&temporary, operation.link_kind);
            return Err(ManagedError::new(ManagedErrorCode::BindingVerifyFailed));
        }

        if let Err(code) = self.linker.rename_link(&temporary, &operation.link_path) {
            let _ = self.linker.remove_link(&temporary, operation.link_kind);
            return Err(ManagedError::new(code));
        }

        let final_inspection = self
            .linker
            .inspect(&operation.link_path)
            .map_err(ManagedError::new)?;
        if !link_matches(&final_inspection, operation) {
            return Err(ManagedError::new(ManagedErrorCode::BindingVerifyFailed));
        }
        Ok(self.binding_record(operation))
    }

    fn binding_record(&self, operation: &BindingOperation) -> ManagedBinding {
        ManagedBinding {
            installation_id: operation.installation_id.clone(),
            agent: operation.agent,
            link_path: operation.link_path.to_string_lossy().into_owned(),
            link_kind: operation.link_kind,
            status: if operation.requires_restart {
                BindingStatus::NeedsRestart
            } else {
                BindingStatus::Ready
            },
            last_checked_at: None,
            last_error: None,
            usage_observability: UsageObservability::Unavailable,
        }
    }

    fn validate_existing_target(&self, target: &Path) -> Result<(), ManagedError> {
        let metadata = fs::symlink_metadata(target)
            .map_err(|_| ManagedError::new(ManagedErrorCode::BindingVerifyFailed))?;
        if metadata_is_link_like(&metadata) || !metadata.is_dir() {
            return Err(ManagedError::new(ManagedErrorCode::BindingVerifyFailed));
        }
        let canonical_root = fs::canonicalize(self.paths.skills_root())
            .map_err(|_| ManagedError::new(ManagedErrorCode::BindingVerifyFailed))?;
        let canonical_target = fs::canonicalize(target)
            .map_err(|_| ManagedError::new(ManagedErrorCode::BindingVerifyFailed))?;
        if canonical_target == canonical_root || !canonical_target.starts_with(&canonical_root) {
            return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
        }
        Ok(())
    }
}

fn temporary_link_path(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill");
    destination.with_file_name(format!("{name}.{}.skillreg-tmp", Uuid::new_v4()))
}

fn link_matches(inspection: &LinkInspection, operation: &BindingOperation) -> bool {
    matches!(
        inspection,
        LinkInspection::Link {
            kind,
            target,
            target_exists: true,
        } if *kind == operation.link_kind && *target == operation.target_path
    )
}

fn ensure_safe_directory_tree(
    home: &Path,
    directory: &Path,
    allow_creation: bool,
) -> Result<(), ManagedError> {
    if !home.is_absolute() || !directory.starts_with(home) {
        return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
    }
    let home_metadata = fs::symlink_metadata(home)
        .map_err(|_| ManagedError::new(ManagedErrorCode::BindingCreateFailed))?;
    if metadata_is_link_like(&home_metadata) || !home_metadata.is_dir() {
        return Err(ManagedError::new(ManagedErrorCode::BindingConflict));
    }

    let relative = directory
        .strip_prefix(home)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot))?;
    let mut current = home.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata_is_link_like(&metadata) || !metadata.is_dir() {
                    return Err(ManagedError::new(ManagedErrorCode::BindingConflict));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if !allow_creation {
                    return Err(ManagedError::new(ManagedErrorCode::BindingUnsupported));
                }
                fs::create_dir(&current)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::BindingCreateFailed))?;
                let metadata = fs::symlink_metadata(&current)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::BindingCreateFailed))?;
                if metadata_is_link_like(&metadata) || !metadata.is_dir() {
                    return Err(ManagedError::new(ManagedErrorCode::BindingConflict));
                }
            }
            Err(_) => {
                return Err(ManagedError::new(ManagedErrorCode::BindingVerifyFailed));
            }
        }
    }
    Ok(())
}

fn metadata_is_link_like(metadata: &fs::Metadata) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        metadata.file_type().is_symlink()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_skills::LinkKind;

    fn operation() -> BindingOperation {
        BindingOperation {
            installation_id: "installation-id".to_string(),
            agent: AgentId::Claude,
            link_path: PathBuf::from("/tmp/.claude/skills/review-helper"),
            target_path: PathBuf::from(
                "/tmp/.skillreg/skills/acme/publisher/review-helper/content",
            ),
            link_kind: LinkKind::Symlink,
            requires_restart: false,
            create_parent: true,
            owned_binding: None,
        }
    }

    #[test]
    fn temporary_link_is_a_unique_neighbor_of_the_destination() {
        let operation = operation();

        let first = temporary_link_path(&operation.link_path);
        let second = temporary_link_path(&operation.link_path);

        assert_eq!(first.parent(), operation.link_path.parent());
        assert_eq!(second.parent(), operation.link_path.parent());
        assert_ne!(first, second);
        assert!(first
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap()
            .ends_with(".skillreg-tmp"));
    }

    #[test]
    fn verification_requires_the_exact_kind_target_and_existing_target() {
        let operation = operation();

        assert!(link_matches(
            &LinkInspection::Link {
                kind: LinkKind::Symlink,
                target: operation.target_path.clone(),
                target_exists: true,
            },
            &operation,
        ));
        assert!(!link_matches(
            &LinkInspection::Link {
                kind: LinkKind::Junction,
                target: operation.target_path.clone(),
                target_exists: true,
            },
            &operation,
        ));
        assert!(!link_matches(
            &LinkInspection::Link {
                kind: LinkKind::Symlink,
                target: operation.target_path.clone(),
                target_exists: false,
            },
            &operation,
        ));
    }
}
