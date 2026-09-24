use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use skillreg_local_lib::managed_skills::{
    agents::{
        AgentAdapter, AgentError, BindingRequest, BindingState, DefaultAgentRegistry,
        DetectionResult, DetectionState, Platform, UsageCapability,
    },
    errors::{ManagedError, ManagedErrorCode},
    manifest::{read_manifest, write_manifest_atomic, ManagedSkillsManifest},
    paths::ManagedPaths,
    platform_links::{LinkInspection, PlatformLinker, SystemPlatformLinker},
    service::{
        FileManifestStore, ManagedDownloadMetadata, ManagedInstallRequest, ManagedManifestStore,
        ManagedRegistryClient, ManagedRegistryFuture, ManagedRegistryRequest, ManagedSkillService,
        ManagedTarget, ManagedTargetPolicy, ManagedWarningCode,
    },
    AgentId, BindingStatus, LinkKind,
};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tar::{Builder, Header};
use uuid::Uuid;

struct TestHome {
    path: PathBuf,
}

impl TestHome {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "skillreg-managed-installation-{name}-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone)]
struct FakeRegistryClient {
    release: Arc<Mutex<FakeRelease>>,
    downloads: Arc<AtomicUsize>,
    target_checks: Arc<AtomicUsize>,
}

#[derive(Clone)]
struct FakeRelease {
    target: ManagedTarget,
    archive: Vec<u8>,
    download_metadata: ManagedDownloadMetadata,
}

impl FakeRegistryClient {
    fn new(consumer: &str, source: &str, name: &str, version: &str, skill_md: &str) -> Self {
        let archive = skill_archive(name, skill_md);
        let sha256 = sha256(&archive);
        let target = ManagedTarget {
            skill_id: format!("{source}-{name}-id"),
            source_org: source.to_string(),
            consumer_org: consumer.to_string(),
            skill_name: name.to_string(),
            resolved_version: version.to_string(),
            sha256: sha256.clone(),
            validation_level: "verified".to_string(),
            policy: ManagedTargetPolicy {
                mode: "latest_approved".to_string(),
                version: None,
            },
        };
        let download_metadata = ManagedDownloadMetadata::from_target(&target);
        Self {
            release: Arc::new(Mutex::new(FakeRelease {
                target,
                archive,
                download_metadata,
            })),
            downloads: Arc::new(AtomicUsize::new(0)),
            target_checks: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn set_release(&self, source: &str, version: &str, skill_md: &str) {
        let mut release = self.release.lock().unwrap();
        let consumer = release.target.consumer_org.clone();
        let name = release.target.skill_name.clone();
        let archive = skill_archive(&name, skill_md);
        let sha256 = sha256(&archive);
        release.target = ManagedTarget {
            skill_id: format!("{source}-{name}-id"),
            source_org: source.to_string(),
            consumer_org: consumer,
            skill_name: name,
            resolved_version: version.to_string(),
            sha256,
            validation_level: "verified".to_string(),
            policy: ManagedTargetPolicy {
                mode: "latest_approved".to_string(),
                version: None,
            },
        };
        release.archive = archive;
        release.download_metadata = ManagedDownloadMetadata::from_target(&release.target);
    }

    fn downloads(&self) -> usize {
        self.downloads.load(Ordering::SeqCst)
    }

    fn target_checks(&self) -> usize {
        self.target_checks.load(Ordering::SeqCst)
    }
}

impl ManagedRegistryClient for FakeRegistryClient {
    fn resolve_target<'a>(
        &'a self,
        _request: &'a ManagedRegistryRequest,
    ) -> ManagedRegistryFuture<'a, ManagedTarget> {
        self.target_checks.fetch_add(1, Ordering::SeqCst);
        let target = self.release.lock().unwrap().target.clone();
        Box::pin(async move { Ok(target) })
    }

    fn download_to<'a>(
        &'a self,
        _request: &'a ManagedRegistryRequest,
        destination: &'a Path,
    ) -> ManagedRegistryFuture<'a, ManagedDownloadMetadata> {
        self.downloads.fetch_add(1, Ordering::SeqCst);
        let release = self.release.lock().unwrap().clone();
        Box::pin(async move {
            fs::write(destination, &release.archive)
                .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
            Ok(release.download_metadata)
        })
    }
}

struct TestAdapter {
    agent: AgentId,
    root: &'static str,
}

impl AgentAdapter for TestAdapter {
    fn id(&self) -> AgentId {
        self.agent
    }

    fn detect(&self, home: &Path) -> DetectionResult {
        DetectionResult {
            agent: self.agent,
            state: DetectionState::Detected,
            detected_version: Some("test".to_string()),
            preferred_path: Some(home.join(self.root)),
            legacy_skill_dirs: vec![],
            requires_restart_after_binding: false,
            detail_code: None,
        }
    }

    fn candidate_user_skill_dirs(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(self.root)]
    }

    fn preferred_user_skill_dir(&self, home: &Path) -> Result<PathBuf, AgentError> {
        Ok(home.join(self.root))
    }

    fn supports_managed_links(&self, _platform: Platform) -> bool {
        true
    }

    fn allows_user_skill_dir_creation(&self) -> bool {
        true
    }

    fn verify_binding(&self, _request: &BindingRequest) -> Result<BindingState, AgentError> {
        Ok(BindingState {
            status: BindingStatus::Missing,
            actual_target: None,
        })
    }

    fn usage_capability(&self) -> UsageCapability {
        UsageCapability::Unavailable
    }
}

fn registry(adapters: &[(AgentId, &'static str)]) -> DefaultAgentRegistry {
    DefaultAgentRegistry::from_adapters(
        adapters
            .iter()
            .map(|(agent, root)| {
                Box::new(TestAdapter {
                    agent: *agent,
                    root,
                }) as Box<dyn AgentAdapter>
            })
            .collect(),
    )
}

fn skill_archive(name: &str, skill_md: &str) -> Vec<u8> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(encoder);
    let path = format!("{name}/SKILL.md");
    let mut header = Header::new_gnu();
    header.set_size(skill_md.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, path, Cursor::new(skill_md.as_bytes()))
        .unwrap();
    builder.into_inner().unwrap().finish().unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn request(source: &str) -> ManagedInstallRequest {
    ManagedInstallRequest {
        consumer_org: "acme".to_string(),
        source_org: source.to_string(),
        name: "review-helper".to_string(),
    }
}

fn skill_md(label: &str) -> String {
    format!(
        "---\nname: review-helper\ndescription: {label}\nenv:\n  - name: REVIEW_API_TOKEN\n    description: API token\n    required: true\n    secret: true\n---\n\n# {label}\n"
    )
}

#[tokio::test]
async fn installs_one_canonical_copy_and_two_bindings_from_one_download() {
    let home = TestHome::new("canonical");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.2.3",
        &skill_md("First"),
    );
    let service = ManagedSkillService::new(
        client.clone(),
        registry(&[
            (AgentId::Claude, ".claude/skills"),
            (AgentId::Codex, ".agents/skills"),
        ]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    );

    let result = service.install(request("publisher")).await.unwrap();

    assert_eq!(client.downloads(), 1);
    assert_eq!(result.installation.source_org, "publisher");
    assert_eq!(result.installation.active_version, "1.2.3");
    assert_eq!(result.bindings.len(), 2);
    let canonical = fs::canonicalize(&result.installation.content_path).unwrap();
    for binding in &result.bindings {
        assert_eq!(fs::canonicalize(&binding.link_path).unwrap(), canonical);
    }
    assert_eq!(result.required_env_vars.len(), 1);
    assert_eq!(result.required_env_vars[0].name, "REVIEW_API_TOKEN");
    let manifest_text = fs::read_to_string(paths.manifest_path()).unwrap();
    assert!(!manifest_text.contains("API_TOKEN_VALUE"));
    let skill_root = paths
        .skill_root("acme", "publisher", "review-helper")
        .unwrap();
    let leftovers = fs::read_dir(skill_root)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(leftovers, vec!["content"]);
}

#[tokio::test]
async fn reinstalling_the_same_resolved_release_is_idempotent_without_redownload() {
    let home = TestHome::new("idempotent");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.2.3",
        &skill_md("Same"),
    );
    let service = ManagedSkillService::new(
        client.clone(),
        registry(&[(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths,
    );

    let first = service.install(request("publisher")).await.unwrap();
    let second = service.install(request("publisher")).await.unwrap();

    assert_eq!(client.downloads(), 1);
    assert_eq!(
        first.installation.installation_id,
        second.installation.installation_id
    );
    assert_eq!(second.bindings.len(), 1);
}

#[derive(Clone, Copy)]
struct FailWritesStore;

impl ManagedManifestStore for FailWritesStore {
    fn load(&self, paths: &ManagedPaths) -> Result<ManagedSkillsManifest, ManagedError> {
        read_manifest(paths)
    }

    fn write(
        &self,
        _paths: &ManagedPaths,
        _manifest: &ManagedSkillsManifest,
    ) -> Result<(), ManagedError> {
        Err(ManagedError::new(ManagedErrorCode::ManifestWriteFailed))
    }
}

#[tokio::test]
async fn manifest_write_failure_restores_previous_content_and_manifest() {
    let home = TestHome::new("manifest-rollback");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.0.0",
        &skill_md("Old"),
    );
    let initial = ManagedSkillService::new(
        client.clone(),
        registry(&[(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    );
    initial.install(request("publisher")).await.unwrap();
    let old_manifest = fs::read(paths.manifest_path()).unwrap();
    client.set_release("publisher", "2.0.0", &skill_md("New"));
    let failing = ManagedSkillService::new(
        client,
        registry(&[(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        FailWritesStore,
        paths.clone(),
    );

    let error = failing.install(request("publisher")).await.unwrap_err();

    assert_eq!(error.code(), ManagedErrorCode::ManifestWriteFailed);
    assert_eq!(fs::read(paths.manifest_path()).unwrap(), old_manifest);
    assert!(fs::read_to_string(
        paths
            .content_dir("acme", "publisher", "review-helper")
            .unwrap()
            .join("SKILL.md")
    )
    .unwrap()
    .contains("# Old"));
}

#[tokio::test]
async fn one_agent_conflict_does_not_block_free_agents() {
    let home = TestHome::new("partial-conflict");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let conflict = home.path.join(".claude/skills/review-helper");
    fs::create_dir_all(&conflict).unwrap();
    fs::write(conflict.join("keep.txt"), "keep").unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.0.0",
        &skill_md("Conflict"),
    );
    let service = ManagedSkillService::new(
        client,
        registry(&[
            (AgentId::Claude, ".claude/skills"),
            (AgentId::Codex, ".agents/skills"),
        ]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths,
    );

    let result = service.install(request("publisher")).await.unwrap();

    assert_eq!(result.bindings.len(), 1);
    assert_eq!(result.bindings[0].agent, AgentId::Codex);
    assert!(result.warnings.iter().any(|warning| {
        warning.code == ManagedWarningCode::BindingConflict
            && warning.agent == Some(AgentId::Claude)
    }));
    assert_eq!(
        fs::read_to_string(conflict.join("keep.txt")).unwrap(),
        "keep"
    );
}

#[tokio::test]
async fn a_second_source_uses_distinct_storage_and_never_replaces_active_bindings() {
    let home = TestHome::new("source-conflict");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher-one",
        "review-helper",
        "1.0.0",
        &skill_md("One"),
    );
    let service = ManagedSkillService::new(
        client.clone(),
        registry(&[(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    );
    service.install(request("publisher-one")).await.unwrap();
    let original_target = fs::canonicalize(home.path.join(".claude/skills/review-helper")).unwrap();
    client.set_release("publisher-two", "1.0.0", &skill_md("Two"));

    let second = service.install(request("publisher-two")).await.unwrap();

    assert!(paths
        .content_dir("acme", "publisher-one", "review-helper")
        .unwrap()
        .is_dir());
    assert!(paths
        .content_dir("acme", "publisher-two", "review-helper")
        .unwrap()
        .is_dir());
    assert!(second.bindings.is_empty());
    assert!(second
        .warnings
        .iter()
        .any(|warning| warning.code == ManagedWarningCode::SkillSourceNameConflict));
    assert_eq!(
        fs::canonicalize(home.path.join(".claude/skills/review-helper")).unwrap(),
        original_target
    );
    assert_eq!(read_manifest(&paths).unwrap().skills.len(), 2);
}

#[tokio::test]
async fn checksum_mismatch_leaves_no_active_content_or_staging() {
    let home = TestHome::new("checksum");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.0.0",
        &skill_md("Checksum"),
    );
    client.release.lock().unwrap().download_metadata.sha256 = "f".repeat(64);
    let service = ManagedSkillService::new(
        client,
        registry(&[(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    );

    let error = service.install(request("publisher")).await.unwrap_err();

    assert_eq!(error.code(), ManagedErrorCode::ArchiveChecksumMismatch);
    let root = paths
        .skill_root("acme", "publisher", "review-helper")
        .unwrap();
    assert!(!root.join("content").exists());
    assert!(!root.join("staging").exists());
    assert!(fs::read_dir(root).unwrap().next().is_none());
}

#[derive(Clone, Copy)]
struct WrongKindLinker;

impl PlatformLinker for WrongKindLinker {
    fn platform(&self) -> Platform {
        Platform::Macos
    }

    fn link_kind(&self) -> LinkKind {
        LinkKind::Junction
    }

    fn validate_paths(&self, _target: &Path, _link: &Path) -> Result<(), ManagedErrorCode> {
        Ok(())
    }

    fn inspect(&self, _link: &Path) -> Result<LinkInspection, ManagedErrorCode> {
        Ok(LinkInspection::Missing)
    }

    fn create_dir_link(&self, _target: &Path, _link: &Path) -> Result<(), ManagedErrorCode> {
        Ok(())
    }

    fn rename_link(&self, _source: &Path, _destination: &Path) -> Result<(), ManagedErrorCode> {
        Ok(())
    }

    fn remove_link(&self, _link: &Path, _kind: LinkKind) -> Result<(), ManagedErrorCode> {
        Ok(())
    }
}

#[tokio::test]
async fn binding_plan_failure_before_activation_preserves_previous_installation() {
    let home = TestHome::new("preactivation");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.0.0",
        &skill_md("Old"),
    );
    let initial = ManagedSkillService::new(
        client.clone(),
        registry(&[(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    );
    initial.install(request("publisher")).await.unwrap();
    client.set_release("publisher", "2.0.0", &skill_md("New"));
    let failing = ManagedSkillService::new(
        client,
        registry(&[(AgentId::Claude, ".claude/skills")]),
        WrongKindLinker,
        FileManifestStore,
        paths.clone(),
    );

    let error = failing.install(request("publisher")).await.unwrap_err();

    assert_eq!(error.code(), ManagedErrorCode::BindingUnsupported);
    assert!(fs::read_to_string(
        paths
            .content_dir("acme", "publisher", "review-helper")
            .unwrap()
            .join("SKILL.md")
    )
    .unwrap()
    .contains("# Old"));
    assert_eq!(
        read_manifest(&paths).unwrap().skills[0].active_version,
        "1.0.0"
    );
}

#[tokio::test]
async fn managed_update_cycle_respects_global_off_manual_check_and_single_download() {
    let home = TestHome::new("managed-update-cycle");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.0.0",
        &skill_md("Old"),
    );
    let service = ManagedSkillService::new(
        client.clone(),
        registry(&[
            (AgentId::Claude, ".claude/skills"),
            (AgentId::Codex, ".agents/skills"),
            (AgentId::Cursor, ".cursor/skills"),
        ]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    );
    service.install(request("publisher")).await.unwrap();
    let mut manifest = read_manifest(&paths).unwrap();
    manifest.skills[0].last_updated_at = Some("previous-update".to_string());
    write_manifest_atomic(&paths, &manifest).unwrap();
    client.set_release("publisher", "0.9.0", &skill_md("Admin pin moved"));
    let before_disabled = fs::read(paths.manifest_path()).unwrap();
    let targets_before_disabled = client.target_checks();

    let disabled = service
        .run_managed_updates(false, false, true)
        .await
        .unwrap();

    assert_eq!(disabled.checked, 0);
    assert_eq!(client.target_checks(), targets_before_disabled);
    assert_eq!(client.downloads(), 1);
    assert_eq!(fs::read(paths.manifest_path()).unwrap(), before_disabled);

    let checked = service
        .run_managed_updates(false, true, false)
        .await
        .unwrap();
    assert_eq!(checked.checked, 1);
    assert_eq!(checked.available, 1);
    assert_eq!(checked.updated, 0);
    assert_eq!(client.downloads(), 1);
    let checked_manifest = read_manifest(&paths).unwrap();
    assert_eq!(
        checked_manifest.skills[0].status,
        skillreg_local_lib::managed_skills::ManagedSkillStatus::UpdateAvailable
    );
    assert_eq!(
        checked_manifest.skills[0].last_updated_at.as_deref(),
        Some("previous-update")
    );
    assert_ne!(
        checked_manifest.skills[0].last_checked_at,
        checked_manifest.skills[0].last_updated_at
    );

    let applied = service
        .run_managed_updates(false, true, true)
        .await
        .unwrap();
    assert_eq!(applied.available, 1);
    assert_eq!(applied.updated, 1);
    assert_eq!(client.downloads(), 2);
    let updated_manifest = read_manifest(&paths).unwrap();
    assert_eq!(updated_manifest.skills[0].active_version, "0.9.0");
    assert_eq!(
        updated_manifest
            .bindings
            .iter()
            .filter(|binding| binding.status.is_active())
            .count(),
        3
    );
}

#[tokio::test]
async fn managed_update_cycle_blocks_modified_canonical_content() {
    let home = TestHome::new("managed-update-modified");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.0.0",
        &skill_md("Old"),
    );
    let service = ManagedSkillService::new(
        client.clone(),
        registry(&[(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    );
    service.install(request("publisher")).await.unwrap();
    client.set_release("publisher", "2.0.0", &skill_md("New"));
    fs::write(
        paths
            .content_dir("acme", "publisher", "review-helper")
            .unwrap()
            .join("SKILL.md"),
        "modified locally",
    )
    .unwrap();

    let summary = service
        .run_managed_updates(true, false, true)
        .await
        .unwrap();

    assert_eq!(summary.action_required, 1);
    assert_eq!(summary.updated, 0);
    assert_eq!(client.downloads(), 1);
    let manifest = read_manifest(&paths).unwrap();
    assert_eq!(manifest.skills[0].active_version, "1.0.0");
    assert_eq!(
        manifest.skills[0].last_error.as_ref().unwrap().code,
        ManagedErrorCode::ContentModified
    );
}

#[tokio::test]
async fn concurrent_managed_update_runs_are_serialized_to_one_download() {
    let home = TestHome::new("managed-update-concurrent");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.0.0",
        &skill_md("Old"),
    );
    let service = ManagedSkillService::new(
        client.clone(),
        registry(&[(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths,
    );
    service.install(request("publisher")).await.unwrap();
    client.set_release("publisher", "2.0.0", &skill_md("New"));

    let (first, second) = tokio::join!(
        service.run_managed_updates(true, false, true),
        service.run_managed_updates(true, false, true)
    );

    let first = first.unwrap();
    let second = second.unwrap();
    assert_eq!(first.updated + second.updated, 1);
    assert_eq!(client.downloads(), 2);
}

#[derive(Clone)]
struct ToggleInspectLinker {
    fail_cursor: Arc<AtomicBool>,
    cursor_root: PathBuf,
}

impl PlatformLinker for ToggleInspectLinker {
    fn platform(&self) -> Platform {
        Platform::Macos
    }

    fn link_kind(&self) -> LinkKind {
        LinkKind::Symlink
    }

    fn validate_paths(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().validate_paths(target, link)
    }

    fn inspect(&self, link: &Path) -> Result<LinkInspection, ManagedErrorCode> {
        if self.fail_cursor.load(Ordering::SeqCst) && link.starts_with(&self.cursor_root) {
            return Err(ManagedErrorCode::BindingVerifyFailed);
        }
        SystemPlatformLinker::current().inspect(link)
    }

    fn create_dir_link(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().create_dir_link(target, link)
    }

    fn rename_link(&self, source: &Path, destination: &Path) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().rename_link(source, destination)
    }

    fn remove_link(&self, link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().remove_link(link, kind)
    }
}

#[tokio::test]
async fn binding_error_after_update_keeps_new_content_active_and_marks_action_required() {
    let home = TestHome::new("managed-update-binding-error");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let client = FakeRegistryClient::new(
        "acme",
        "publisher",
        "review-helper",
        "1.0.0",
        &skill_md("Old"),
    );
    let fail_cursor = Arc::new(AtomicBool::new(false));
    let linker = ToggleInspectLinker {
        fail_cursor: fail_cursor.clone(),
        cursor_root: home.path.join(".cursor/skills"),
    };
    let service = ManagedSkillService::new(
        client.clone(),
        registry(&[
            (AgentId::Claude, ".claude/skills"),
            (AgentId::Cursor, ".cursor/skills"),
        ]),
        linker,
        FileManifestStore,
        paths.clone(),
    );
    service.install(request("publisher")).await.unwrap();
    client.set_release("publisher", "2.0.0", &skill_md("New"));
    fail_cursor.store(true, Ordering::SeqCst);

    let summary = service
        .run_managed_updates(true, false, true)
        .await
        .unwrap();

    assert_eq!(summary.updated, 1);
    assert_eq!(summary.action_required, 1);
    let manifest = read_manifest(&paths).unwrap();
    assert_eq!(manifest.skills[0].active_version, "2.0.0");
    assert_eq!(
        manifest.skills[0].status,
        skillreg_local_lib::managed_skills::ManagedSkillStatus::ActionRequired
    );
    assert!(fs::read_to_string(
        paths
            .content_dir("acme", "publisher", "review-helper")
            .unwrap()
            .join("SKILL.md")
    )
    .unwrap()
    .contains("# New"));
    assert!(manifest.bindings.iter().any(|binding| {
        binding.agent == AgentId::Cursor && binding.status == BindingStatus::Error
    }));
}
