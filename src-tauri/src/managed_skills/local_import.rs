use crate::{
    commands::local::parse_frontmatter_pub,
    managed_skills::{
        agents::{AgentRegistry, DetectionResult},
        archive::{compute_tree_hash, ArchiveLimits},
        bindings::{BindingApplyReport, ManagedBindingService},
        errors::{ManagedError, ManagedErrorCode},
        manifest::{
            load_or_create_manifest, read_manifest, write_manifest_atomic, ManagedSkillsManifest,
        },
        paths::{validate_org_slug, validate_skill_name, ManagedPaths},
        platform_links::PlatformLinker,
        service::acquire_managed_mutation_lock,
        AgentId, ManagedSkill, ManagedSkillOrigin, ManagedSkillStatus,
    },
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalImportClassification {
    Importable,
    DuplicateIdentical,
    DuplicateDivergent,
    ExternalLinkUntouched,
    AlreadyManaged,
    InvalidUntouched,
}

impl LocalImportClassification {
    pub const fn is_importable(self) -> bool {
        matches!(self, Self::Importable | Self::DuplicateIdentical)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalImportPreviewItem {
    pub skill_name: String,
    pub classification: LocalImportClassification,
    pub agents: Vec<AgentId>,
    #[serde(skip_serializing)]
    sources: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalImportPreview {
    pub items: Vec<LocalImportPreviewItem>,
    pub importable: usize,
    pub conflicts: usize,
    pub external_links_untouched: usize,
    pub already_managed: usize,
    pub invalid_untouched: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalImportReport {
    pub confirmed: bool,
    pub imported: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub errors: usize,
    pub preview: LocalImportPreview,
}

pub struct LocalSkillImportService<A, L>
where
    A: AgentRegistry,
    L: PlatformLinker,
{
    bindings: ManagedBindingService<A, L>,
    paths: ManagedPaths,
}

impl<A, L> LocalSkillImportService<A, L>
where
    A: AgentRegistry,
    L: PlatformLinker,
{
    pub fn new(agents: A, linker: L, paths: ManagedPaths) -> Self {
        Self {
            bindings: ManagedBindingService::new(agents, linker, paths.clone()),
            paths,
        }
    }

    pub fn preview(&self, consumer_org: &str) -> Result<LocalImportPreview, ManagedError> {
        validate_org_slug(consumer_org)?;
        let manifest = if self.paths.manifest_path().exists() {
            read_manifest(&self.paths)?
        } else {
            ManagedSkillsManifest::new()
        };
        if manifest
            .active_org
            .as_deref()
            .is_some_and(|active_org| active_org != consumer_org)
        {
            return Err(ManagedError::new(
                ManagedErrorCode::ActiveOrganizationConflict,
            ));
        }
        build_preview(
            &self.paths,
            self.bindings.agent_registry(),
            &manifest,
            consumer_org,
        )
    }

    pub async fn run(
        &self,
        consumer_org: &str,
        confirm: bool,
    ) -> Result<LocalImportReport, ManagedError> {
        if !confirm {
            return Ok(report_for_preview(false, self.preview(consumer_org)?));
        }

        let _guard = acquire_managed_mutation_lock().await;
        let preview = self.preview(consumer_org)?;
        let mut report = report_for_preview(true, preview.clone());
        if preview.importable == 0 {
            return Ok(report);
        }
        let mut manifest = load_or_create_manifest(&self.paths)?;
        match manifest.active_org.as_deref() {
            Some(active_org) if active_org != consumer_org => {
                return Err(ManagedError::new(
                    ManagedErrorCode::ActiveOrganizationConflict,
                ));
            }
            None => manifest.active_org = Some(consumer_org.to_string()),
            Some(_) => {}
        }
        let detections = self.bindings.detect_all();

        for item in preview
            .items
            .iter()
            .filter(|item| item.classification.is_importable())
        {
            match self.import_item(consumer_org, item, &detections, &mut manifest) {
                Ok(()) => report.imported += 1,
                Err(_) => report.errors += 1,
            }
        }
        Ok(report)
    }

    fn import_item(
        &self,
        consumer_org: &str,
        item: &LocalImportPreviewItem,
        detections: &[DetectionResult],
        manifest: &mut ManagedSkillsManifest,
    ) -> Result<(), ManagedError> {
        if manifest
            .skills
            .iter()
            .any(|skill| skill.consumer_org == consumer_org && skill.skill_name == item.skill_name)
        {
            return Err(ManagedError::new(ManagedErrorCode::SkillSourceNameConflict));
        }
        let source = item
            .sources
            .iter()
            .find(|source| {
                fs::symlink_metadata(source)
                    .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
            })
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        validate_import_sources(
            &self.paths,
            self.bindings.agent_registry(),
            &item.skill_name,
            &item.sources,
        )?;

        let root = self
            .paths
            .skill_root(consumer_org, "local", &item.skill_name)?;
        ensure_safe_directory_tree(self.paths.home(), &root)?;
        let content = self
            .paths
            .content_dir(consumer_org, "local", &item.skill_name)?;
        let staging = self
            .paths
            .staging_dir(consumer_org, "local", &item.skill_name)?;
        if content.exists() || staging.exists() {
            return Err(ManagedError::new(ManagedErrorCode::ContentModified));
        }

        let source_hash = compute_tree_hash(source)?;
        if let Err(error) = copy_regular_tree(source, &staging, ArchiveLimits::default()) {
            cleanup_directory(&staging, &root);
            return Err(error);
        }
        let copied_hash = compute_tree_hash(&staging)?;
        if copied_hash != source_hash || compute_tree_hash(source)? != source_hash {
            cleanup_directory(&staging, &root);
            return Err(ManagedError::new(ManagedErrorCode::ContentModified));
        }

        let now = current_timestamp();
        let installation = ManagedSkill {
            installation_id: Uuid::new_v4().to_string(),
            consumer_org: consumer_org.to_string(),
            source_org: "local".to_string(),
            skill_id: None,
            skill_name: item.skill_name.clone(),
            origin: ManagedSkillOrigin::Local,
            active_version: "local".to_string(),
            sha256: copied_hash.clone(),
            content_path: content.to_string_lossy().into_owned(),
            content_hash: copied_hash,
            status: ManagedSkillStatus::Ready,
            installed_at: now.clone(),
            last_checked_at: Some(now),
            last_updated_at: None,
            last_error: None,
            cleanup_dismissed_until: None,
        };
        let plan = self.bindings.plan(&installation, detections)?;
        if plan.operations.is_empty() {
            cleanup_directory(&staging, &root);
            return Err(ManagedError::new(ManagedErrorCode::BindingUnsupported));
        }

        let backups = match backup_sources(&item.sources) {
            Ok(backups) => backups,
            Err(error) => {
                cleanup_directory(&staging, &root);
                return Err(error);
            }
        };
        if fs::rename(&staging, &content).is_err() {
            let rollback = restore_sources(&backups);
            cleanup_directory(&staging, &root);
            return Err(rollback
                .err()
                .unwrap_or_else(|| ManagedError::new(ManagedErrorCode::RollbackFailed)));
        }

        let binding_report = self.bindings.apply(plan);
        if !binding_report_is_ready(&binding_report, &item.sources) {
            rollback_import(
                &self.bindings,
                &installation,
                &binding_report,
                &content,
                &root,
                &backups,
            )?;
            return Err(ManagedError::new(ManagedErrorCode::BindingCreateFailed));
        }

        let mut next_manifest = manifest.clone();
        next_manifest.skills.push(installation.clone());
        merge_bindings(
            &mut next_manifest,
            &installation.installation_id,
            &binding_report,
        );
        if let Err(error) = write_manifest_atomic(&self.paths, &next_manifest) {
            rollback_import(
                &self.bindings,
                &installation,
                &binding_report,
                &content,
                &root,
                &backups,
            )?;
            return Err(error);
        }
        *manifest = next_manifest;
        cleanup_backups(&backups);
        Ok(())
    }
}

fn report_for_preview(confirmed: bool, preview: LocalImportPreview) -> LocalImportReport {
    let skipped = preview
        .items
        .iter()
        .filter(|item| !item.classification.is_importable())
        .count();
    LocalImportReport {
        confirmed,
        imported: 0,
        skipped,
        conflicts: preview.conflicts,
        errors: 0,
        preview,
    }
}

#[derive(Debug)]
enum ScannedEntryKind {
    Directory {
        canonical: PathBuf,
        hash: String,
    },
    Link {
        canonical_target: Option<PathBuf>,
        owned: bool,
    },
    Invalid,
}

#[derive(Debug)]
struct ScannedEntry {
    path: PathBuf,
    agents: Vec<AgentId>,
    kind: ScannedEntryKind,
}

fn build_preview<A: AgentRegistry>(
    paths: &ManagedPaths,
    agents: &A,
    manifest: &ManagedSkillsManifest,
    consumer_org: &str,
) -> Result<LocalImportPreview, ManagedError> {
    let roots = preferred_roots(paths, agents);
    let managed_links = managed_links(manifest, consumer_org);
    let mut groups: BTreeMap<String, Vec<ScannedEntry>> = BTreeMap::new();

    for (root, root_agents) in roots {
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(ManagedError::new(ManagedErrorCode::BindingVerifyFailed)),
        };
        for entry in entries {
            let entry =
                entry.map_err(|_| ManagedError::new(ManagedErrorCode::BindingVerifyFailed))?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| ManagedError::new(ManagedErrorCode::BindingVerifyFailed))?;
            let looks_like_skill = metadata.file_type().is_symlink()
                || (metadata.is_dir() && path.join("SKILL.md").exists());
            if !looks_like_skill {
                continue;
            }
            let kind = if validate_skill_name(name).is_err() {
                ScannedEntryKind::Invalid
            } else if metadata.file_type().is_symlink() {
                let canonical_target = fs::canonicalize(&path).ok();
                let owned = managed_links
                    .get(&path)
                    .is_some_and(|expected| canonical_target.as_ref() == Some(expected));
                ScannedEntryKind::Link {
                    canonical_target,
                    owned,
                }
            } else if metadata.is_dir() {
                scan_directory(&path, name)
            } else {
                ScannedEntryKind::Invalid
            };
            groups
                .entry(name.to_string())
                .or_default()
                .push(ScannedEntry {
                    path,
                    agents: root_agents.clone(),
                    kind,
                });
        }
    }

    let mut preview = LocalImportPreview::default();
    for (skill_name, entries) in groups {
        push_group(&mut preview, skill_name, entries);
    }
    Ok(preview)
}

fn preferred_roots<A: AgentRegistry>(
    paths: &ManagedPaths,
    agents: &A,
) -> Vec<(PathBuf, Vec<AgentId>)> {
    let mut roots: Vec<(PathBuf, Vec<AgentId>)> = Vec::new();
    for adapter in agents.adapters() {
        let Ok(root) = adapter.preferred_user_skill_dir(paths.home()) else {
            continue;
        };
        if let Some((_, root_agents)) = roots.iter_mut().find(|(candidate, _)| candidate == &root) {
            if !root_agents.contains(&adapter.id()) {
                root_agents.push(adapter.id());
            }
        } else {
            roots.push((root, vec![adapter.id()]));
        }
    }
    roots.sort_by(|left, right| left.0.cmp(&right.0));
    roots
}

fn managed_links(
    manifest: &ManagedSkillsManifest,
    consumer_org: &str,
) -> HashMap<PathBuf, PathBuf> {
    let skills = manifest
        .skills
        .iter()
        .filter(|skill| skill.consumer_org == consumer_org)
        .map(|skill| (skill.installation_id.as_str(), skill))
        .collect::<HashMap<_, _>>();
    manifest
        .bindings
        .iter()
        .filter_map(|binding| {
            let skill = skills.get(binding.installation_id.as_str())?;
            let canonical = fs::canonicalize(&skill.content_path).ok()?;
            Some((PathBuf::from(&binding.link_path), canonical))
        })
        .collect()
}

fn scan_directory(path: &Path, _expected_name: &str) -> ScannedEntryKind {
    let skill_md = path.join("SKILL.md");
    let metadata = match fs::symlink_metadata(&skill_md) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => metadata,
        _ => return ScannedEntryKind::Invalid,
    };
    if metadata.len() == 0 {
        return ScannedEntryKind::Invalid;
    }
    let frontmatter = match fs::read_to_string(&skill_md) {
        Ok(content) => parse_frontmatter_pub(&content),
        Err(_) => return ScannedEntryKind::Invalid,
    };
    // The folder slug is the local installation identity; legacy SKILL.md names may be labels.
    if frontmatter
        .get("name")
        .is_none_or(|name| name.trim().is_empty())
        || frontmatter
            .get("description")
            .is_none_or(|description| description.trim().is_empty())
    {
        return ScannedEntryKind::Invalid;
    }
    let canonical = match fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(_) => return ScannedEntryKind::Invalid,
    };
    match compute_tree_hash(path) {
        Ok(hash) => ScannedEntryKind::Directory { canonical, hash },
        Err(_) => ScannedEntryKind::Invalid,
    }
}

fn push_group(preview: &mut LocalImportPreview, skill_name: String, entries: Vec<ScannedEntry>) {
    let mut agents = entries
        .iter()
        .flat_map(|entry| entry.agents.iter().copied())
        .collect::<Vec<_>>();
    agents.sort_by_key(|agent| agent_sort_key(*agent));
    agents.dedup();

    let has_owned = entries
        .iter()
        .any(|entry| matches!(entry.kind, ScannedEntryKind::Link { owned: true, .. }));
    let all_owned = entries
        .iter()
        .all(|entry| matches!(entry.kind, ScannedEntryKind::Link { owned: true, .. }));
    if all_owned {
        preview.already_managed += 1;
        preview.items.push(LocalImportPreviewItem {
            skill_name,
            classification: LocalImportClassification::AlreadyManaged,
            agents,
            sources: Vec::new(),
        });
        return;
    }
    if has_owned {
        preview.conflicts += 1;
        preview.items.push(LocalImportPreviewItem {
            skill_name,
            classification: LocalImportClassification::DuplicateDivergent,
            agents,
            sources: Vec::new(),
        });
        return;
    }
    if entries
        .iter()
        .any(|entry| matches!(entry.kind, ScannedEntryKind::Invalid))
    {
        preview.invalid_untouched += 1;
        preview.items.push(LocalImportPreviewItem {
            skill_name,
            classification: LocalImportClassification::InvalidUntouched,
            agents,
            sources: Vec::new(),
        });
        return;
    }

    let directories = entries
        .iter()
        .filter_map(|entry| match &entry.kind {
            ScannedEntryKind::Directory { canonical, hash } => {
                Some((entry.path.clone(), canonical.clone(), hash.clone()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if directories.is_empty() {
        preview.external_links_untouched += 1;
        preview.items.push(LocalImportPreviewItem {
            skill_name,
            classification: LocalImportClassification::ExternalLinkUntouched,
            agents,
            sources: Vec::new(),
        });
        return;
    }

    let canonical_directories = directories
        .iter()
        .map(|(_, canonical, _)| canonical)
        .collect::<HashSet<_>>();
    let external_link_present = entries.iter().any(|entry| match &entry.kind {
        ScannedEntryKind::Link {
            canonical_target,
            owned: false,
        } => canonical_target
            .as_ref()
            .is_none_or(|target| !canonical_directories.contains(target)),
        _ => false,
    });
    let first_hash = &directories[0].2;
    if external_link_present || directories.iter().any(|(_, _, hash)| hash != first_hash) {
        preview.conflicts += 1;
        preview.items.push(LocalImportPreviewItem {
            skill_name,
            classification: LocalImportClassification::DuplicateDivergent,
            agents,
            sources: Vec::new(),
        });
        return;
    }

    let mut sources = directories
        .iter()
        .map(|(path, _, _)| path.clone())
        .collect::<Vec<_>>();
    sources.extend(entries.iter().filter_map(|entry| match &entry.kind {
        ScannedEntryKind::Link {
            canonical_target: Some(target),
            owned: false,
        } if canonical_directories.contains(target) => Some(entry.path.clone()),
        _ => None,
    }));
    sources.sort();
    let classification = if directories.len() > 1 {
        LocalImportClassification::DuplicateIdentical
    } else {
        LocalImportClassification::Importable
    };
    preview.importable += 1;
    preview.items.push(LocalImportPreviewItem {
        skill_name,
        classification,
        agents,
        sources,
    });
}

const fn agent_sort_key(agent: AgentId) -> u8 {
    match agent {
        AgentId::Claude => 0,
        AgentId::Codex => 1,
        AgentId::Cursor => 2,
    }
}

fn validate_import_sources<A: AgentRegistry>(
    paths: &ManagedPaths,
    agents: &A,
    skill_name: &str,
    sources: &[PathBuf],
) -> Result<(), ManagedError> {
    let allowed_roots = agents
        .adapters()
        .into_iter()
        .filter_map(|adapter| adapter.preferred_user_skill_dir(paths.home()).ok())
        .collect::<Vec<_>>();
    for source in sources {
        if source.file_name().and_then(|name| name.to_str()) != Some(skill_name)
            || !allowed_roots
                .iter()
                .any(|root| source.parent() == Some(root.as_path()))
        {
            return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
        }
        let metadata = fs::symlink_metadata(source)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ContentModified))?;
        if !metadata.is_dir() && !metadata.file_type().is_symlink() {
            return Err(ManagedError::new(ManagedErrorCode::ContentModified));
        }
    }
    Ok(())
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
            Ok(_) => {
                return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .map_err(|_| ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot))?;
                set_private_directory_permissions(&current)?;
            }
            Err(_) => {
                return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
            }
        }
    }
    Ok(())
}

fn copy_regular_tree(
    source: &Path,
    destination: &Path,
    limits: ArchiveLimits,
) -> Result<(), ManagedError> {
    if destination.exists() {
        return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
    }
    fs::create_dir(destination).map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    set_private_directory_permissions(destination)?;
    let mut state = CopyState { files: 0, bytes: 0 };
    if let Err(error) = copy_regular_tree_inner(source, source, destination, limits, &mut state) {
        let _ = fs::remove_dir_all(destination);
        return Err(error);
    }
    if state.files == 0 {
        let _ = fs::remove_dir_all(destination);
        return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
    }
    Ok(())
}

struct CopyState {
    files: usize,
    bytes: u64,
}

fn copy_regular_tree_inner(
    root: &Path,
    current: &Path,
    destination: &Path,
    limits: ArchiveLimits,
    state: &mut CopyState,
) -> Result<(), ManagedError> {
    let mut entries = fs::read_dir(current)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source_path = entry.path();
        let relative = source_path
            .strip_prefix(root)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        if relative.components().count() > limits.max_depth
            || relative.as_os_str().len() > limits.max_path_bytes
        {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
        let metadata = fs::symlink_metadata(&source_path)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        if metadata.file_type().is_symlink() {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
        let destination_path = destination.join(entry.file_name());
        if metadata.is_dir() {
            fs::create_dir(&destination_path)
                .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
            set_private_directory_permissions(&destination_path)?;
            copy_regular_tree_inner(root, &source_path, &destination_path, limits, state)?;
        } else if metadata.is_file() {
            state.files = state
                .files
                .checked_add(1)
                .ok_or_else(|| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
            state.bytes = state
                .bytes
                .checked_add(metadata.len())
                .ok_or_else(|| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
            if state.files > limits.max_files
                || metadata.len() > limits.max_file_bytes
                || state.bytes > limits.max_total_bytes
            {
                return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
            }
            copy_regular_file(&source_path, &destination_path, &metadata)?;
        } else {
            return Err(ManagedError::new(ManagedErrorCode::ArchiveUnsafe));
        }
    }
    Ok(())
}

fn copy_regular_file(
    source: &Path,
    destination: &Path,
    source_metadata: &fs::Metadata,
) -> Result<(), ManagedError> {
    let mut input =
        File::open(source).map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(if source_metadata.permissions().mode() & 0o111 == 0 {
        0o600
    } else {
        0o700
    });
    let mut output = options
        .open(destination)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    }
    output
        .sync_all()
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))
}

fn set_private_directory_permissions(path: &Path) -> Result<(), ManagedError> {
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| ManagedError::new(ManagedErrorCode::ArchiveUnsafe))?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[derive(Debug)]
struct SourceBackup {
    original: PathBuf,
    backup: PathBuf,
}

fn backup_sources(sources: &[PathBuf]) -> Result<Vec<SourceBackup>, ManagedError> {
    let transaction_id = Uuid::new_v4();
    let mut backups = Vec::new();
    for source in sources {
        let parent = source
            .parent()
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot))?;
        let backup = parent.join(format!(
            ".skillreg-import-backup-{transaction_id}-{}",
            backups.len()
        ));
        if backup.exists() || fs::rename(source, &backup).is_err() {
            restore_sources(&backups)?;
            return Err(ManagedError::new(ManagedErrorCode::RollbackFailed));
        }
        backups.push(SourceBackup {
            original: source.clone(),
            backup,
        });
    }
    Ok(backups)
}

fn restore_sources(backups: &[SourceBackup]) -> Result<(), ManagedError> {
    for backup in backups.iter().rev() {
        if backup.original.exists() || fs::rename(&backup.backup, &backup.original).is_err() {
            return Err(ManagedError::new(ManagedErrorCode::RollbackFailed));
        }
    }
    Ok(())
}

fn cleanup_backups(backups: &[SourceBackup]) {
    for backup in backups {
        let Ok(metadata) = fs::symlink_metadata(&backup.backup) else {
            continue;
        };
        if metadata.file_type().is_symlink() || metadata.is_file() {
            let _ = fs::remove_file(&backup.backup);
        } else if metadata.is_dir() {
            let _ = fs::remove_dir_all(&backup.backup);
        }
    }
}

fn binding_report_is_ready(report: &BindingApplyReport, sources: &[PathBuf]) -> bool {
    !report.results.is_empty()
        && report
            .results
            .iter()
            .all(|result| result.status.is_active())
        && sources.iter().all(|source| {
            report.results.iter().any(|result| {
                result
                    .binding
                    .as_ref()
                    .is_some_and(|binding| Path::new(&binding.link_path) == source)
            })
        })
}

fn merge_bindings(
    manifest: &mut ManagedSkillsManifest,
    installation_id: &str,
    report: &BindingApplyReport,
) {
    for binding in report
        .results
        .iter()
        .filter_map(|result| result.binding.clone())
    {
        if let Some(existing) = manifest.bindings.iter_mut().find(|existing| {
            existing.installation_id == installation_id && existing.agent == binding.agent
        }) {
            *existing = binding;
        } else {
            manifest.bindings.push(binding);
        }
    }
}

fn rollback_import<A: AgentRegistry, L: PlatformLinker>(
    bindings: &ManagedBindingService<A, L>,
    installation: &ManagedSkill,
    report: &BindingApplyReport,
    content: &Path,
    root: &Path,
    backups: &[SourceBackup],
) -> Result<(), ManagedError> {
    for binding in report
        .results
        .iter()
        .rev()
        .filter(|result| result.created)
        .filter_map(|result| result.binding.as_ref())
    {
        bindings
            .remove_owned(installation, binding)
            .map_err(|_| ManagedError::new(ManagedErrorCode::RollbackFailed))?;
    }
    cleanup_directory(content, root);
    restore_sources(backups)
}

fn cleanup_directory(path: &Path, root: &Path) {
    if path.starts_with(root) && path != root {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
    if let Ok(mut entries) = fs::read_dir(root) {
        if entries.next().is_none() {
            let _ = fs::remove_dir(root);
        }
    }
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
