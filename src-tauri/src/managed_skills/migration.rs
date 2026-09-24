pub use crate::commands::installed_manifest::TrackedInstallation;
use crate::{
    commands::{installed_manifest::InstalledManifest, local::compute_content_hash},
    managed_skills::{
        agents::{AgentRegistry, Platform},
        archive::compute_tree_hash,
        bindings::{BindingApplyReport, ManagedBindingService},
        errors::{ManagedError, ManagedErrorCode},
        manifest::{read_manifest, write_manifest_atomic, ManagedSkillsManifest},
        paths::ManagedPaths,
        platform_links::PlatformLinker,
        service::{
            acquire_managed_mutation_lock, ManagedInstallOutcome, ManagedInstallRequest,
            ManagedManifestStore, ManagedRegistryClient, ManagedSkillService,
        },
        AgentId, BindingStatus, ManagedBinding, ManagedSkill, ManagedSkillStatus,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    pin::Pin,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyPathState {
    Missing,
    Regular { content_hash: String },
    ManagedBinding,
    Other,
}

#[derive(Debug, Clone)]
pub struct LegacyEvidence {
    pub entry: TrackedInstallation,
    pub path_state: LegacyPathState,
    pub expected_user_path: bool,
    pub agent_supported: bool,
    pub already_migrated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationClassification {
    ManagedCandidate,
    ProjectScopeLeaveUntouched,
    ModifiedLeaveUntouched,
    MissingLeaveRecord,
    UnmanagedConflict,
    DuplicateIdentical,
    DuplicateDivergent,
    UnsupportedAgent,
    AlreadyMigrated,
}

impl MigrationClassification {
    pub const fn is_migratable(self) -> bool {
        matches!(self, Self::ManagedCandidate | Self::DuplicateIdentical)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPreviewItem {
    pub consumer_org: String,
    pub source_org: String,
    pub skill_name: String,
    pub classification: MigrationClassification,
    pub agents: Vec<String>,
    #[serde(skip_serializing)]
    pub entries: Vec<TrackedInstallation>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPreview {
    pub items: Vec<MigrationPreviewItem>,
    pub managed_candidates: usize,
    pub project_scope_untouched: usize,
    pub modified_untouched: usize,
    pub missing_records: usize,
    pub conflicts: usize,
    pub unsupported_agents: usize,
    pub already_migrated: usize,
}

pub type MigrationInstallFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ManagedInstallOutcome, ManagedError>> + Send + 'a>>;

pub trait ManagedMigrationInstaller: Send + Sync {
    /// Runs inside the global managed-mutation critical section owned by the migration service.
    fn install_for_migration<'a>(
        &'a self,
        request: ManagedInstallRequest,
    ) -> MigrationInstallFuture<'a>;
}

impl<C, A, L, S> ManagedMigrationInstaller for ManagedSkillService<C, A, L, S>
where
    C: ManagedRegistryClient,
    A: AgentRegistry,
    L: PlatformLinker,
    S: ManagedManifestStore,
{
    fn install_for_migration<'a>(
        &'a self,
        request: ManagedInstallRequest,
    ) -> MigrationInstallFuture<'a> {
        Box::pin(async move { ManagedSkillService::install_serialized(self, request).await })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub confirmed: bool,
    pub migrated: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub errors: usize,
    pub preview: MigrationPreview,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileReport {
    pub checked: usize,
    pub repaired: usize,
    pub conflicts: usize,
    pub failed: usize,
    pub modified_content: usize,
}

pub struct ManagedMigrationService<I, A, L>
where
    I: ManagedMigrationInstaller,
    A: AgentRegistry,
    L: PlatformLinker,
{
    installer: I,
    bindings: ManagedBindingService<A, L>,
    paths: ManagedPaths,
}

impl<I, A, L> ManagedMigrationService<I, A, L>
where
    I: ManagedMigrationInstaller,
    A: AgentRegistry,
    L: PlatformLinker,
{
    pub fn new(installer: I, agents: A, linker: L, paths: ManagedPaths) -> Self {
        Self {
            installer,
            bindings: ManagedBindingService::new(agents, linker, paths.clone()),
            paths,
        }
    }

    pub fn preview(&self) -> Result<MigrationPreview, ManagedError> {
        build_migration_preview(&self.paths, self.bindings.agent_registry())
    }

    pub async fn run(&self, confirm: bool) -> Result<MigrationReport, ManagedError> {
        if !confirm {
            return Ok(report_for_preview(false, self.preview()?));
        }

        let _guard = acquire_managed_mutation_lock().await;
        let initial_preview = self.preview()?;
        recover_pending_legacy_directories(&initial_preview)?;
        let preview = self.preview()?;
        if preview.managed_candidates > 0 {
            backup_legacy_manifest(&self.paths)?;
        }

        let mut report = report_for_preview(true, preview.clone());
        for item in preview
            .items
            .iter()
            .filter(|item| item.classification.is_migratable())
        {
            match self.migrate_item(item).await {
                Ok(()) => report.migrated += 1,
                Err(_) => report.errors += 1,
            }
        }
        Ok(report)
    }

    async fn migrate_item(&self, item: &MigrationPreviewItem) -> Result<(), ManagedError> {
        let outcome = self
            .installer
            .install_for_migration(ManagedInstallRequest {
                consumer_org: item.consumer_org.clone(),
                source_org: item.source_org.clone(),
                name: item.skill_name.clone(),
            })
            .await?;
        validate_migration_installation(item, &outcome.installation, &self.paths)?;

        let mut renamed = Vec::new();
        for entry in &item.entries {
            if let Err(error) =
                rename_verified_legacy_directory(entry, self.bindings.agent_registry(), &self.paths)
                    .map(|backup| renamed.push(backup))
            {
                restore_legacy_directories(&renamed)?;
                return Err(error);
            }
        }

        let mut manifest = if self.paths.manifest_path().exists() {
            match read_manifest(&self.paths) {
                Ok(manifest) => manifest,
                Err(error) => {
                    restore_legacy_directories(&renamed)?;
                    return Err(error);
                }
            }
        } else {
            ManagedSkillsManifest::new()
        };
        upsert_managed_skill(&mut manifest, outcome.installation.clone());
        merge_bindings(&mut manifest, &outcome.bindings);
        let existing_bindings = manifest
            .bindings
            .iter()
            .filter(|binding| binding.installation_id == outcome.installation.installation_id)
            .cloned()
            .collect::<Vec<_>>();
        let detections = self.bindings.detect_all();
        let plan = match self.bindings.plan_with_bindings(
            &outcome.installation,
            &detections,
            &existing_bindings,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                restore_legacy_directories(&renamed)?;
                return Err(error);
            }
        };
        let binding_report = self.bindings.apply(plan);
        if !all_migrated_bindings_are_active(item, &binding_report) {
            rollback_migration_bindings(&self.bindings, &outcome.installation, &binding_report)?;
            restore_legacy_directories(&renamed)?;
            return Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction));
        }

        merge_binding_report(&mut manifest, &binding_report);
        if let Some(installation) = manifest
            .skills
            .iter_mut()
            .find(|skill| skill.installation_id == outcome.installation.installation_id)
        {
            installation.status = if binding_report
                .results
                .iter()
                .any(|result| result.status == BindingStatus::Conflict)
            {
                ManagedSkillStatus::ActionRequired
            } else {
                ManagedSkillStatus::Ready
            };
        }
        manifest.migrated_from_v1_at = Some(current_timestamp());
        if let Err(error) = write_manifest_atomic(&self.paths, &manifest) {
            rollback_migration_bindings(&self.bindings, &outcome.installation, &binding_report)?;
            restore_legacy_directories(&renamed)?;
            return Err(error);
        }

        for backup in renamed {
            remove_verified_backup(&backup)?;
        }
        Ok(())
    }
}

pub fn build_migration_preview<A: AgentRegistry>(
    paths: &ManagedPaths,
    agents: &A,
) -> Result<MigrationPreview, ManagedError> {
    let legacy = read_legacy_manifest(paths)?;
    let managed = if paths.manifest_path().exists() {
        Some(read_manifest(paths)?)
    } else {
        None
    };
    let evidence = legacy
        .installations
        .into_iter()
        .map(|entry| inspect_legacy_entry(paths, agents, managed.as_ref(), entry))
        .collect::<Vec<_>>();
    Ok(classify_legacy_entries(&evidence))
}

pub async fn repair_managed_skills<A: AgentRegistry, L: PlatformLinker>(
    paths: &ManagedPaths,
    agents: A,
    linker: L,
) -> Result<ReconcileReport, ManagedError> {
    let _guard = acquire_managed_mutation_lock().await;
    if !paths.manifest_path().exists() {
        return Ok(ReconcileReport::default());
    }
    let mut manifest = read_manifest(paths)?;
    let bindings = ManagedBindingService::new(agents, linker, paths.clone());
    let detections = bindings.detect_all();
    let now = current_timestamp();
    let mut summary = ReconcileReport::default();

    for index in 0..manifest.skills.len() {
        summary.checked += 1;
        let installation = manifest.skills[index].clone();
        let content_hash = compute_tree_hash(Path::new(&installation.content_path));
        if content_hash.as_deref() != Ok(installation.content_hash.as_str()) {
            summary.modified_content += 1;
            manifest.skills[index].status = ManagedSkillStatus::ActionRequired;
            manifest.skills[index].last_checked_at = Some(now.clone());
            manifest.skills[index].last_error = Some(
                ManagedError::new(ManagedErrorCode::ContentModified).into_record(Some(now.clone())),
            );
            continue;
        }

        let existing = manifest
            .bindings
            .iter()
            .filter(|binding| binding.installation_id == installation.installation_id)
            .cloned()
            .collect::<Vec<_>>();
        let report = bindings.reconcile(&installation, &existing, &detections);
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
        merge_binding_report(&mut manifest, &report);

        let has_action = report.results.iter().any(|result| {
            matches!(
                result.status,
                BindingStatus::Conflict | BindingStatus::Unsupported | BindingStatus::Error
            )
        });
        manifest.skills[index].status = if has_action {
            ManagedSkillStatus::ActionRequired
        } else {
            ManagedSkillStatus::Ready
        };
        manifest.skills[index].last_checked_at = Some(now.clone());
        manifest.skills[index].last_error = report
            .results
            .iter()
            .find_map(|result| result.error.clone());
    }
    write_manifest_atomic(paths, &manifest)?;

    let preview = build_migration_preview(paths, bindings.agent_registry())?;
    cleanup_completed_legacy_backups(&preview)?;
    Ok(summary)
}

fn report_for_preview(confirmed: bool, preview: MigrationPreview) -> MigrationReport {
    let skipped = preview
        .items
        .iter()
        .filter(|item| !item.classification.is_migratable())
        .count();
    MigrationReport {
        confirmed,
        migrated: 0,
        skipped,
        conflicts: preview.conflicts,
        errors: 0,
        preview,
    }
}

pub fn classify_legacy_entries(entries: &[LegacyEvidence]) -> MigrationPreview {
    let mut preview = MigrationPreview::default();
    let mut candidates: BTreeMap<(String, String, String), Vec<TrackedInstallation>> =
        BTreeMap::new();

    for evidence in entries {
        let source_org = source_org(&evidence.entry);
        let classification = classify_terminal_evidence(evidence);
        if let Some(classification) = classification {
            push_item(
                &mut preview,
                classification,
                evidence.entry.org.clone(),
                source_org,
                evidence.entry.name.clone(),
                vec![evidence.entry.clone()],
            );
            continue;
        }

        candidates
            .entry((
                evidence.entry.org.clone(),
                source_org,
                evidence.entry.name.clone(),
            ))
            .or_default()
            .push(evidence.entry.clone());
    }

    for ((consumer_org, source_org, skill_name), mut group) in candidates {
        group.sort_by(|left, right| {
            left.agent
                .cmp(&right.agent)
                .then_with(|| left.install_path.cmp(&right.install_path))
        });
        let classification = if group.len() == 1 {
            MigrationClassification::ManagedCandidate
        } else {
            let first_hash = &group[0].content_hash;
            let first_version = &group[0].version;
            if group
                .iter()
                .all(|entry| entry.content_hash == *first_hash && entry.version == *first_version)
            {
                MigrationClassification::DuplicateIdentical
            } else {
                MigrationClassification::DuplicateDivergent
            }
        };
        push_item(
            &mut preview,
            classification,
            consumer_org,
            source_org,
            skill_name,
            group,
        );
    }

    preview.items.sort_by(|left, right| {
        left.skill_name
            .cmp(&right.skill_name)
            .then_with(|| left.consumer_org.cmp(&right.consumer_org))
            .then_with(|| left.source_org.cmp(&right.source_org))
            .then_with(|| {
                format!("{:?}", left.classification).cmp(&format!("{:?}", right.classification))
            })
    });
    preview
}

fn classify_terminal_evidence(evidence: &LegacyEvidence) -> Option<MigrationClassification> {
    if evidence.already_migrated {
        return Some(MigrationClassification::AlreadyMigrated);
    }
    if evidence.entry.scope != "user" {
        return Some(MigrationClassification::ProjectScopeLeaveUntouched);
    }
    if !evidence.agent_supported {
        return Some(MigrationClassification::UnsupportedAgent);
    }
    if !evidence.expected_user_path {
        return Some(MigrationClassification::UnmanagedConflict);
    }
    match &evidence.path_state {
        LegacyPathState::Missing => Some(MigrationClassification::MissingLeaveRecord),
        LegacyPathState::ManagedBinding => Some(MigrationClassification::AlreadyMigrated),
        LegacyPathState::Other => Some(MigrationClassification::UnmanagedConflict),
        LegacyPathState::Regular { content_hash }
            if *content_hash != evidence.entry.content_hash =>
        {
            Some(MigrationClassification::ModifiedLeaveUntouched)
        }
        LegacyPathState::Regular { .. } => None,
    }
}

fn push_item(
    preview: &mut MigrationPreview,
    classification: MigrationClassification,
    consumer_org: String,
    source_org: String,
    skill_name: String,
    entries: Vec<TrackedInstallation>,
) {
    match classification {
        MigrationClassification::ManagedCandidate | MigrationClassification::DuplicateIdentical => {
            preview.managed_candidates += 1
        }
        MigrationClassification::ProjectScopeLeaveUntouched => {
            preview.project_scope_untouched += entries.len()
        }
        MigrationClassification::ModifiedLeaveUntouched => {
            preview.modified_untouched += entries.len()
        }
        MigrationClassification::MissingLeaveRecord => preview.missing_records += entries.len(),
        MigrationClassification::UnmanagedConflict
        | MigrationClassification::DuplicateDivergent => preview.conflicts += 1,
        MigrationClassification::UnsupportedAgent => preview.unsupported_agents += entries.len(),
        MigrationClassification::AlreadyMigrated => preview.already_migrated += entries.len(),
    }
    let agents = entries.iter().map(|entry| entry.agent.clone()).collect();
    preview.items.push(MigrationPreviewItem {
        consumer_org,
        source_org,
        skill_name,
        classification,
        agents,
        entries,
    });
}

fn source_org(entry: &TrackedInstallation) -> String {
    entry
        .source_org
        .clone()
        .unwrap_or_else(|| entry.org.clone())
}

fn read_legacy_manifest(paths: &ManagedPaths) -> Result<InstalledManifest, ManagedError> {
    match fs::read(paths.legacy_manifest_path()) {
        Ok(content) => {
            let manifest: InstalledManifest = serde_json::from_slice(&content)
                .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
            if manifest.version > crate::commands::installed_manifest::INSTALLED_MANIFEST_VERSION {
                return Err(ManagedError::new(ManagedErrorCode::ManifestInvalid));
            }
            Ok(manifest)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(InstalledManifest::default())
        }
        Err(_) => Err(ManagedError::new(ManagedErrorCode::ManifestInvalid)),
    }
}

fn inspect_legacy_entry<A: AgentRegistry>(
    paths: &ManagedPaths,
    agents: &A,
    managed: Option<&crate::managed_skills::manifest::ManagedSkillsManifest>,
    entry: TrackedInstallation,
) -> LegacyEvidence {
    let agent = parse_agent_id(&entry.agent);
    let adapter = agent.and_then(|agent| agents.adapter(agent));
    let agent_supported =
        adapter.is_some_and(|adapter| adapter.supports_managed_links(Platform::current()));
    let install_path = Path::new(&entry.install_path);
    let expected_user_path = adapter.is_some_and(|adapter| {
        adapter
            .candidate_user_skill_dirs(paths.home())
            .into_iter()
            .any(|root| root.join(&entry.name) == install_path)
    });
    let source = source_org(&entry);
    let managed_installation = managed.and_then(|manifest| {
        manifest.skills.iter().find(|skill| {
            skill.consumer_org == entry.org
                && skill.source_org == source
                && skill.skill_name == entry.name
        })
    });
    let owned_binding = managed_installation.and_then(|installation| {
        managed.and_then(|manifest| {
            manifest.bindings.iter().find(|binding| {
                Some(binding.agent) == agent
                    && binding.installation_id == installation.installation_id
                    && binding.status.is_active()
                    && Path::new(&binding.link_path) == install_path
            })
        })
    });
    let already_migrated = owned_binding.is_some()
        && managed_installation.is_some_and(|installation| {
            paths
                .validate_managed_content_path(Path::new(&installation.content_path))
                .is_ok()
                && fs::canonicalize(install_path)
                    .ok()
                    .zip(fs::canonicalize(&installation.content_path).ok())
                    .is_some_and(|(actual, expected)| actual == expected)
        });

    let path_state = if already_migrated {
        LegacyPathState::ManagedBinding
    } else {
        inspect_legacy_path(install_path)
    };
    LegacyEvidence {
        entry,
        path_state,
        expected_user_path,
        agent_supported,
        already_migrated,
    }
}

fn inspect_legacy_path(path: &Path) -> LegacyPathState {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => LegacyPathState::Missing,
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            match fs::read_to_string(path.join("SKILL.md")) {
                Ok(content) => LegacyPathState::Regular {
                    content_hash: compute_content_hash(&content),
                },
                Err(_) => LegacyPathState::Other,
            }
        }
        Ok(_) | Err(_) => LegacyPathState::Other,
    }
}

#[derive(Debug)]
struct RenamedLegacyDirectory {
    original: PathBuf,
    backup: PathBuf,
}

fn validate_migration_installation(
    item: &MigrationPreviewItem,
    installation: &ManagedSkill,
    paths: &ManagedPaths,
) -> Result<(), ManagedError> {
    if installation.consumer_org != item.consumer_org
        || installation.source_org != item.source_org
        || installation.skill_name != item.skill_name
    {
        return Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction));
    }
    let content = Path::new(&installation.content_path);
    paths.validate_managed_content_path(content)?;
    if compute_tree_hash(content)? != installation.content_hash {
        return Err(ManagedError::new(ManagedErrorCode::ContentModified));
    }
    Ok(())
}

fn rename_verified_legacy_directory<A: AgentRegistry>(
    entry: &TrackedInstallation,
    agents: &A,
    paths: &ManagedPaths,
) -> Result<RenamedLegacyDirectory, ManagedError> {
    if entry.scope != "user" {
        return Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction));
    }
    let agent = parse_agent_id(&entry.agent)
        .ok_or_else(|| ManagedError::new(ManagedErrorCode::MigrationRequiresAction))?;
    let adapter = agents
        .adapter(agent)
        .ok_or_else(|| ManagedError::new(ManagedErrorCode::MigrationRequiresAction))?;
    let original = PathBuf::from(&entry.install_path);
    let expected = adapter
        .candidate_user_skill_dirs(paths.home())
        .into_iter()
        .any(|root| root.join(&entry.name) == original);
    if !expected {
        return Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction));
    }
    match inspect_legacy_path(&original) {
        LegacyPathState::Regular { content_hash } if content_hash == entry.content_hash => {}
        LegacyPathState::Regular { .. } => {
            return Err(ManagedError::new(ManagedErrorCode::ContentModified));
        }
        _ => {
            return Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction));
        }
    }

    let backup = legacy_directory_backup_path(&original)?;
    if fs::symlink_metadata(&backup).is_ok() {
        return Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction));
    }
    fs::rename(&original, &backup)
        .map_err(|_| ManagedError::new(ManagedErrorCode::MigrationRequiresAction))?;
    Ok(RenamedLegacyDirectory { original, backup })
}

fn legacy_directory_backup_path(original: &Path) -> Result<PathBuf, ManagedError> {
    let name = original
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ManagedError::new(ManagedErrorCode::MigrationRequiresAction))?;
    Ok(original.with_file_name(format!(".{name}.skillreg-v1-backup")))
}

fn recover_pending_legacy_directories(preview: &MigrationPreview) -> Result<(), ManagedError> {
    for entry in preview.items.iter().flat_map(|item| item.entries.iter()) {
        if entry.scope != "user" {
            continue;
        }
        let original = PathBuf::from(&entry.install_path);
        let backup = legacy_directory_backup_path(&original)?;
        let original_missing = fs::symlink_metadata(&original)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound);
        let backup_is_directory = fs::symlink_metadata(&backup)
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink());
        if original_missing && backup_is_directory {
            fs::rename(&backup, &original)
                .map_err(|_| ManagedError::new(ManagedErrorCode::MigrationRequiresAction))?;
        }
    }
    Ok(())
}

fn cleanup_completed_legacy_backups(preview: &MigrationPreview) -> Result<(), ManagedError> {
    for entry in preview
        .items
        .iter()
        .filter(|item| item.classification == MigrationClassification::AlreadyMigrated)
        .flat_map(|item| item.entries.iter())
    {
        let original = PathBuf::from(&entry.install_path);
        let backup = legacy_directory_backup_path(&original)?;
        match inspect_legacy_path(&backup) {
            LegacyPathState::Missing => {}
            LegacyPathState::Regular { content_hash } if content_hash == entry.content_hash => {
                remove_verified_backup(&RenamedLegacyDirectory { original, backup })?;
            }
            LegacyPathState::Regular { .. }
            | LegacyPathState::ManagedBinding
            | LegacyPathState::Other => {
                return Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction));
            }
        }
    }
    Ok(())
}

fn restore_legacy_directories(directories: &[RenamedLegacyDirectory]) -> Result<(), ManagedError> {
    for directory in directories.iter().rev() {
        match fs::symlink_metadata(&directory.original) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) | Err(_) => {
                return Err(ManagedError::new(ManagedErrorCode::RollbackFailed));
            }
        }
        fs::rename(&directory.backup, &directory.original)
            .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
    }
    Ok(())
}

fn remove_verified_backup(directory: &RenamedLegacyDirectory) -> Result<(), ManagedError> {
    if directory.backup.parent() != directory.original.parent() {
        return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
    }
    match fs::symlink_metadata(&directory.backup) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(&directory.backup)
                .map_err(|_| ManagedError::new(ManagedErrorCode::MigrationRequiresAction))
        }
        _ => Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction)),
    }
}

fn all_migrated_bindings_are_active(
    item: &MigrationPreviewItem,
    report: &BindingApplyReport,
) -> bool {
    item.entries.iter().all(|entry| {
        parse_agent_id(&entry.agent).is_some_and(|agent| {
            report.results.iter().any(|result| {
                result.agent == agent
                    && result.status.is_active()
                    && result.binding.as_ref().is_some_and(|binding| {
                        Path::new(&binding.link_path) == Path::new(&entry.install_path)
                    })
            })
        })
    })
}

fn rollback_migration_bindings<A: AgentRegistry, L: PlatformLinker>(
    bindings: &ManagedBindingService<A, L>,
    installation: &ManagedSkill,
    report: &BindingApplyReport,
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
    Ok(())
}

fn upsert_managed_skill(manifest: &mut ManagedSkillsManifest, installation: ManagedSkill) {
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

fn merge_bindings(manifest: &mut ManagedSkillsManifest, bindings: &[ManagedBinding]) {
    for binding in bindings {
        if let Some(existing) = manifest.bindings.iter_mut().find(|existing| {
            existing.installation_id == binding.installation_id && existing.agent == binding.agent
        }) {
            *existing = binding.clone();
        } else {
            manifest.bindings.push(binding.clone());
        }
    }
}

fn merge_binding_report(manifest: &mut ManagedSkillsManifest, report: &BindingApplyReport) {
    let bindings = report
        .results
        .iter()
        .filter_map(|result| result.binding.clone())
        .collect::<Vec<_>>();
    merge_bindings(manifest, &bindings);
}

fn backup_legacy_manifest(paths: &ManagedPaths) -> Result<(), ManagedError> {
    let source = paths.legacy_manifest_path();
    let content =
        fs::read(&source).map_err(|_| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
    let destination = paths.skillreg_root().join("installed-v1.backup.json");
    if destination.exists() {
        return if fs::read(&destination).ok().as_deref() == Some(content.as_slice()) {
            Ok(())
        } else {
            Err(ManagedError::new(ManagedErrorCode::MigrationRequiresAction))
        };
    }

    let temporary = paths
        .skillreg_root()
        .join(format!("installed-v1.backup.{}.tmp", Uuid::new_v4()));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
        file.write_all(&content)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
        file.sync_all()
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
        fs::rename(&temporary, &destination)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    write_result
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn parse_agent_id(value: &str) -> Option<AgentId> {
    match value {
        "claude" => Some(AgentId::Claude),
        "codex" => Some(AgentId::Codex),
        "cursor" => Some(AgentId::Cursor),
        _ => None,
    }
}
