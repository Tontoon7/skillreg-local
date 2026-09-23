use sha2::{Digest, Sha256};
use skillreg_local_lib::managed_skills::migration::{
    build_migration_preview, classify_legacy_entries, repair_managed_skills, LegacyEvidence,
    LegacyPathState, ManagedMigrationInstaller, ManagedMigrationService, MigrationClassification,
    MigrationInstallFuture, TrackedInstallation,
};
use skillreg_local_lib::managed_skills::{
    agents::{
        AgentAdapter, AgentError, BindingRequest, BindingState, DefaultAgentRegistry,
        DetectionResult, DetectionState, Platform, UsageCapability,
    },
    errors::ManagedErrorCode,
    paths::ManagedPaths,
    platform_links::{LinkInspection, PlatformLinker, SystemPlatformLinker},
    service::{acquire_managed_mutation_lock, ManagedInstallOutcome, ManagedInstallRequest},
    AgentId, BindingStatus, LinkKind, ManagedSkill, ManagedSkillOrigin, ManagedSkillStatus,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use uuid::Uuid;

fn tracked(name: &str, agent: &str, scope: &str, path: &str, hash: &str) -> TrackedInstallation {
    TrackedInstallation {
        org: "acme".to_string(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        agent: agent.to_string(),
        scope: scope.to_string(),
        project_dir: (scope == "project").then(|| "/workspace".to_string()),
        install_path: path.to_string(),
        content_hash: hash.to_string(),
        source_org: Some("publisher".to_string()),
        sha256: Some("a".repeat(64)),
        auto_update_enabled: None,
        last_checked_at: None,
        last_updated_at: None,
        last_error: None,
    }
}

fn evidence(
    entry: TrackedInstallation,
    path_state: LegacyPathState,
    expected_user_path: bool,
    agent_supported: bool,
    already_migrated: bool,
) -> LegacyEvidence {
    LegacyEvidence {
        entry,
        path_state,
        expected_user_path,
        agent_supported,
        already_migrated,
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

fn registry() -> DefaultAgentRegistry {
    DefaultAgentRegistry::from_adapters(vec![
        Box::new(TestAdapter {
            agent: AgentId::Claude,
            root: ".claude/skills",
        }),
        Box::new(TestAdapter {
            agent: AgentId::Codex,
            root: ".agents/skills",
        }),
    ])
}

fn hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Clone)]
struct DownloadingInstaller {
    paths: ManagedPaths,
    calls: Arc<AtomicUsize>,
}

impl DownloadingInstaller {
    fn new(paths: ManagedPaths) -> Self {
        Self {
            paths,
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl ManagedMigrationInstaller for DownloadingInstaller {
    fn install_for_migration<'a>(
        &'a self,
        request: ManagedInstallRequest,
    ) -> MigrationInstallFuture<'a> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let content = self.paths.content_dir(
                &request.consumer_org,
                &request.source_org,
                &request.name,
            )?;
            fs::create_dir_all(&content).map_err(|_| {
                skillreg_local_lib::managed_skills::errors::ManagedError::new(
                    skillreg_local_lib::managed_skills::errors::ManagedErrorCode::RollbackFailed,
                )
            })?;
            fs::write(
                content.join("SKILL.md"),
                format!(
                    "---\nname: {}\ndescription: Downloaded and verified\n---\n",
                    request.name
                ),
            )
            .map_err(|_| {
                skillreg_local_lib::managed_skills::errors::ManagedError::new(
                    skillreg_local_lib::managed_skills::errors::ManagedErrorCode::RollbackFailed,
                )
            })?;
            let content_hash =
                skillreg_local_lib::managed_skills::archive::compute_tree_hash(&content)?;
            Ok(ManagedInstallOutcome {
                installation: ManagedSkill {
                    installation_id: Uuid::new_v4().to_string(),
                    consumer_org: request.consumer_org,
                    source_org: request.source_org,
                    skill_id: Some("downloaded-id".to_string()),
                    skill_name: request.name,
                    origin: ManagedSkillOrigin::Registry,
                    active_version: "2.0.0".to_string(),
                    sha256: "a".repeat(64),
                    content_path: content.to_string_lossy().into_owned(),
                    content_hash,
                    status: ManagedSkillStatus::Ready,
                    installed_at: "1".to_string(),
                    last_checked_at: Some("1".to_string()),
                    last_updated_at: Some("1".to_string()),
                    last_error: None,
                    cleanup_dismissed_until: None,
                },
                bindings: vec![],
                required_env_vars: vec![],
                warnings: vec![],
            })
        })
    }
}

#[derive(Clone)]
struct BlockingInstaller {
    inner: DownloadingInstaller,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

impl ManagedMigrationInstaller for BlockingInstaller {
    fn install_for_migration<'a>(
        &'a self,
        request: ManagedInstallRequest,
    ) -> MigrationInstallFuture<'a> {
        let future = self.inner.install_for_migration(request);
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            future.await
        })
    }
}

fn write_legacy_manifest(paths: &ManagedPaths, entries: &[TrackedInstallation]) -> Vec<u8> {
    fs::create_dir_all(paths.skillreg_root()).unwrap();
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "version": 1,
        "installations": entries,
    }))
    .unwrap();
    fs::write(paths.legacy_manifest_path(), &bytes).unwrap();
    bytes
}

fn create_legacy_skill(path: &Path, name: &str) -> String {
    fs::create_dir_all(path).unwrap();
    let content = format!("---\nname: {name}\ndescription: Legacy copy\n---\n");
    fs::write(path.join("SKILL.md"), &content).unwrap();
    hash(&content)
}

#[test]
fn classification_is_pure_and_never_chooses_divergent_copies() {
    let entries = vec![
        evidence(
            tracked(
                "identical",
                "claude",
                "user",
                "/home/.claude/skills/identical",
                "same",
            ),
            LegacyPathState::Regular {
                content_hash: "same".to_string(),
            },
            true,
            true,
            false,
        ),
        evidence(
            tracked(
                "identical",
                "codex",
                "user",
                "/home/.agents/skills/identical",
                "same",
            ),
            LegacyPathState::Regular {
                content_hash: "same".to_string(),
            },
            true,
            true,
            false,
        ),
        evidence(
            tracked(
                "project-only",
                "claude",
                "project",
                "/workspace/.claude/skills/project-only",
                "project",
            ),
            LegacyPathState::Regular {
                content_hash: "project".to_string(),
            },
            false,
            true,
            false,
        ),
        evidence(
            tracked(
                "modified",
                "claude",
                "user",
                "/home/.claude/skills/modified",
                "expected",
            ),
            LegacyPathState::Regular {
                content_hash: "actual".to_string(),
            },
            true,
            true,
            false,
        ),
        evidence(
            tracked(
                "missing",
                "claude",
                "user",
                "/home/.claude/skills/missing",
                "missing",
            ),
            LegacyPathState::Missing,
            true,
            true,
            false,
        ),
        evidence(
            tracked("unmanaged", "claude", "user", "/other/unmanaged", "same"),
            LegacyPathState::Regular {
                content_hash: "same".to_string(),
            },
            false,
            true,
            false,
        ),
        evidence(
            tracked(
                "unsupported",
                "other",
                "user",
                "/home/.other/skills/unsupported",
                "same",
            ),
            LegacyPathState::Regular {
                content_hash: "same".to_string(),
            },
            true,
            false,
            false,
        ),
        evidence(
            tracked(
                "done",
                "claude",
                "user",
                "/home/.claude/skills/done",
                "same",
            ),
            LegacyPathState::ManagedBinding,
            true,
            true,
            true,
        ),
        evidence(
            tracked(
                "divergent",
                "claude",
                "user",
                "/home/.claude/skills/divergent",
                "one",
            ),
            LegacyPathState::Regular {
                content_hash: "one".to_string(),
            },
            true,
            true,
            false,
        ),
        evidence(
            tracked(
                "divergent",
                "codex",
                "user",
                "/home/.agents/skills/divergent",
                "two",
            ),
            LegacyPathState::Regular {
                content_hash: "two".to_string(),
            },
            true,
            true,
            false,
        ),
    ];

    let preview = classify_legacy_entries(&entries);
    let classification = |name: &str| {
        preview
            .items
            .iter()
            .find(|item| item.skill_name == name)
            .unwrap()
            .classification
    };

    assert_eq!(
        classification("identical"),
        MigrationClassification::DuplicateIdentical
    );
    assert_eq!(
        classification("project-only"),
        MigrationClassification::ProjectScopeLeaveUntouched
    );
    assert_eq!(
        classification("modified"),
        MigrationClassification::ModifiedLeaveUntouched
    );
    assert_eq!(
        classification("missing"),
        MigrationClassification::MissingLeaveRecord
    );
    assert_eq!(
        classification("unmanaged"),
        MigrationClassification::UnmanagedConflict
    );
    assert_eq!(
        classification("unsupported"),
        MigrationClassification::UnsupportedAgent
    );
    assert_eq!(
        classification("done"),
        MigrationClassification::AlreadyMigrated
    );
    assert_eq!(
        classification("divergent"),
        MigrationClassification::DuplicateDivergent
    );
    assert_eq!(preview.managed_candidates, 1);
    assert_eq!(preview.project_scope_untouched, 1);
    assert_eq!(preview.modified_untouched, 1);
    assert_eq!(preview.conflicts, 2);
}

#[test]
fn preview_reads_v1_without_creating_or_rewriting_any_manifest() {
    let home = std::env::temp_dir().join(format!("skillreg-migration-preview-{}", Uuid::new_v4()));
    fs::create_dir_all(&home).unwrap();
    let paths = ManagedPaths::new(home.clone()).unwrap();
    let skill_dir = home.join(".claude/skills/review-helper");
    fs::create_dir_all(&skill_dir).unwrap();
    let content = "---\nname: review-helper\ndescription: Review\n---\n";
    fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    fs::create_dir_all(paths.skillreg_root()).unwrap();
    let legacy = serde_json::json!({
        "version": 1,
        "installations": [{
            "org": "acme",
            "name": "review-helper",
            "version": "1.0.0",
            "agent": "claude",
            "scope": "user",
            "projectDir": null,
            "installPath": skill_dir,
            "contentHash": hash(content),
            "sourceOrg": "publisher",
            "sha256": "a".repeat(64),
            "autoUpdateEnabled": null,
            "lastCheckedAt": null,
            "lastUpdatedAt": null,
            "lastError": null
        }]
    });
    let original = serde_json::to_vec_pretty(&legacy).unwrap();
    fs::write(paths.legacy_manifest_path(), &original).unwrap();

    let preview = build_migration_preview(&paths, &registry()).unwrap();

    assert_eq!(preview.managed_candidates, 1);
    assert_eq!(
        preview.items[0].classification,
        MigrationClassification::ManagedCandidate
    );
    assert_eq!(fs::read(paths.legacy_manifest_path()).unwrap(), original);
    assert!(!paths.manifest_path().exists());
    assert!(!paths
        .skillreg_root()
        .join("installed-v1.backup.json")
        .exists());
    fs::remove_dir_all(home).unwrap();
}

#[tokio::test]
async fn confirmed_migration_redownloads_once_backs_up_v1_and_is_idempotent() {
    let home = std::env::temp_dir().join(format!("skillreg-migration-run-{}", Uuid::new_v4()));
    fs::create_dir_all(&home).unwrap();
    let paths = ManagedPaths::new(home.clone()).unwrap();
    let claude_path = home.join(".claude/skills/review-helper");
    let codex_path = home.join(".agents/skills/review-helper");
    let legacy_hash = create_legacy_skill(&claude_path, "review-helper");
    create_legacy_skill(&codex_path, "review-helper");
    let entries = vec![
        tracked(
            "review-helper",
            "claude",
            "user",
            claude_path.to_str().unwrap(),
            &legacy_hash,
        ),
        tracked(
            "review-helper",
            "codex",
            "user",
            codex_path.to_str().unwrap(),
            &legacy_hash,
        ),
    ];
    let original_manifest = write_legacy_manifest(&paths, &entries);
    let installer = DownloadingInstaller::new(paths.clone());
    let migration = ManagedMigrationService::new(
        installer.clone(),
        registry(),
        SystemPlatformLinker::current(),
        paths.clone(),
    );

    let declined = migration.run(false).await.unwrap();
    assert!(!declined.confirmed);
    assert_eq!(installer.calls(), 0);
    assert!(claude_path.is_dir());
    assert!(!paths
        .skillreg_root()
        .join("installed-v1.backup.json")
        .exists());

    let report = migration.run(true).await.unwrap();
    assert!(report.confirmed);
    assert_eq!(report.migrated, 1);
    assert_eq!(report.errors, 0);
    assert_eq!(installer.calls(), 1);
    assert_eq!(
        fs::read(paths.skillreg_root().join("installed-v1.backup.json")).unwrap(),
        original_manifest
    );
    assert_eq!(
        fs::canonicalize(&claude_path).unwrap(),
        fs::canonicalize(
            paths
                .content_dir("acme", "publisher", "review-helper")
                .unwrap()
        )
        .unwrap()
    );
    assert_eq!(
        fs::canonicalize(&codex_path).unwrap(),
        fs::canonicalize(
            paths
                .content_dir("acme", "publisher", "review-helper")
                .unwrap()
        )
        .unwrap()
    );
    assert!(fs::read_to_string(claude_path.join("SKILL.md"))
        .unwrap()
        .contains("Downloaded and verified"));
    assert_eq!(
        fs::read(paths.legacy_manifest_path()).unwrap(),
        original_manifest
    );

    let repeated = migration.run(true).await.unwrap();
    assert_eq!(repeated.migrated, 0);
    assert_eq!(installer.calls(), 1);
    fs::remove_dir_all(home).unwrap();
}

#[tokio::test]
async fn confirmed_migration_holds_global_lock_for_the_entire_mutation() {
    let home = std::env::temp_dir().join(format!("skillreg-migration-lock-{}", Uuid::new_v4()));
    fs::create_dir_all(&home).unwrap();
    let paths = ManagedPaths::new(home.clone()).unwrap();
    let legacy_path = home.join(".claude/skills/review-helper");
    let legacy_hash = create_legacy_skill(&legacy_path, "review-helper");
    write_legacy_manifest(
        &paths,
        &[tracked(
            "review-helper",
            "claude",
            "user",
            legacy_path.to_str().unwrap(),
            &legacy_hash,
        )],
    );
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let migration = ManagedMigrationService::new(
        BlockingInstaller {
            inner: DownloadingInstaller::new(paths.clone()),
            entered: entered.clone(),
            release: release.clone(),
        },
        registry(),
        SystemPlatformLinker::current(),
        paths,
    );

    let task = tokio::spawn(async move { migration.run(true).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
        .await
        .expect("migration should reach the installer");

    let competing_lock = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        acquire_managed_mutation_lock(),
    )
    .await;
    assert!(
        competing_lock.is_err(),
        "another managed mutation must wait until migration completes"
    );

    release.notify_one();
    let report = task.await.unwrap().unwrap();
    assert_eq!(report.migrated, 1);
    fs::remove_dir_all(home).unwrap();
}

#[derive(Clone, Copy)]
struct FailingCreateLinker;

impl PlatformLinker for FailingCreateLinker {
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
        SystemPlatformLinker::current().inspect(link)
    }

    fn create_dir_link(&self, _target: &Path, _link: &Path) -> Result<(), ManagedErrorCode> {
        Err(ManagedErrorCode::BindingCreateFailed)
    }

    fn rename_link(&self, source: &Path, destination: &Path) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().rename_link(source, destination)
    }

    fn remove_link(&self, link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().remove_link(link, kind)
    }
}

#[tokio::test]
async fn binding_failure_keeps_the_verified_central_copy_and_restores_legacy_access() {
    let home = std::env::temp_dir().join(format!("skillreg-migration-failure-{}", Uuid::new_v4()));
    fs::create_dir_all(&home).unwrap();
    let paths = ManagedPaths::new(home.clone()).unwrap();
    let legacy_path = home.join(".claude/skills/review-helper");
    let legacy_hash = create_legacy_skill(&legacy_path, "review-helper");
    write_legacy_manifest(
        &paths,
        &[tracked(
            "review-helper",
            "claude",
            "user",
            legacy_path.to_str().unwrap(),
            &legacy_hash,
        )],
    );
    let installer = DownloadingInstaller::new(paths.clone());
    let migration =
        ManagedMigrationService::new(installer, registry(), FailingCreateLinker, paths.clone());

    let report = migration.run(true).await.unwrap();

    assert_eq!(report.migrated, 0);
    assert_eq!(report.errors, 1);
    assert!(legacy_path.is_dir());
    assert!(!fs::symlink_metadata(&legacy_path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(fs::read_to_string(legacy_path.join("SKILL.md"))
        .unwrap()
        .contains("Legacy copy"));
    assert!(paths
        .content_dir("acme", "publisher", "review-helper")
        .unwrap()
        .is_dir());
    assert!(!legacy_path
        .with_file_name(".review-helper.skillreg-v1-backup")
        .exists());
    fs::remove_dir_all(home).unwrap();
}

#[tokio::test]
async fn untracked_collision_and_project_or_missing_records_are_never_deleted_or_recreated() {
    let home = std::env::temp_dir().join(format!("skillreg-migration-preserve-{}", Uuid::new_v4()));
    fs::create_dir_all(&home).unwrap();
    let paths = ManagedPaths::new(home.clone()).unwrap();
    let tracked_path = home.join(".claude/skills/review-helper");
    let tracked_hash = create_legacy_skill(&tracked_path, "review-helper");
    let untracked_path = home.join(".agents/skills/review-helper");
    fs::create_dir_all(&untracked_path).unwrap();
    fs::write(untracked_path.join("keep.txt"), "do not touch").unwrap();
    let project_path = home.join("workspace/.claude/skills/project-helper");
    let project_hash = create_legacy_skill(&project_path, "project-helper");
    let missing_path = home.join(".claude/skills/missing-helper");
    write_legacy_manifest(
        &paths,
        &[
            tracked(
                "review-helper",
                "claude",
                "user",
                tracked_path.to_str().unwrap(),
                &tracked_hash,
            ),
            tracked(
                "project-helper",
                "claude",
                "project",
                project_path.to_str().unwrap(),
                &project_hash,
            ),
            tracked(
                "missing-helper",
                "claude",
                "user",
                missing_path.to_str().unwrap(),
                "missing",
            ),
        ],
    );
    let installer = DownloadingInstaller::new(paths.clone());
    let migration = ManagedMigrationService::new(
        installer.clone(),
        registry(),
        SystemPlatformLinker::current(),
        paths,
    );

    let report = migration.run(true).await.unwrap();

    assert_eq!(report.migrated, 1);
    assert_eq!(installer.calls(), 1);
    assert_eq!(
        fs::read_to_string(untracked_path.join("keep.txt")).unwrap(),
        "do not touch"
    );
    assert!(project_path.is_dir());
    assert!(!missing_path.exists());
    fs::remove_dir_all(home).unwrap();
}

#[tokio::test]
async fn repair_recreates_missing_owned_bindings_but_blocks_modified_canonical_content() {
    let home = std::env::temp_dir().join(format!("skillreg-migration-repair-{}", Uuid::new_v4()));
    fs::create_dir_all(&home).unwrap();
    let paths = ManagedPaths::new(home.clone()).unwrap();
    let legacy_path = home.join(".claude/skills/review-helper");
    let legacy_hash = create_legacy_skill(&legacy_path, "review-helper");
    write_legacy_manifest(
        &paths,
        &[tracked(
            "review-helper",
            "claude",
            "user",
            legacy_path.to_str().unwrap(),
            &legacy_hash,
        )],
    );
    let migration = ManagedMigrationService::new(
        DownloadingInstaller::new(paths.clone()),
        registry(),
        SystemPlatformLinker::current(),
        paths.clone(),
    );
    migration.run(true).await.unwrap();
    fs::remove_file(&legacy_path).unwrap();

    let repaired = repair_managed_skills(&paths, registry(), SystemPlatformLinker::current())
        .await
        .unwrap();

    assert_eq!(repaired.repaired, 1);
    assert!(fs::canonicalize(&legacy_path).is_ok());

    fs::write(
        paths
            .content_dir("acme", "publisher", "review-helper")
            .unwrap()
            .join("SKILL.md"),
        "locally modified",
    )
    .unwrap();
    fs::remove_file(&legacy_path).unwrap();
    let blocked = repair_managed_skills(&paths, registry(), SystemPlatformLinker::current())
        .await
        .unwrap();

    assert_eq!(blocked.modified_content, 1);
    assert!(!legacy_path.exists());
    fs::remove_dir_all(home).unwrap();
}
