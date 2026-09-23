use skillreg_local_lib::managed_skills::{
    agents::{
        AgentAdapter, AgentError, AgentErrorCode, BindingRequest, BindingState,
        DefaultAgentRegistry, DetectionResult, DetectionState, Platform, UsageCapability,
    },
    bindings::ManagedBindingService,
    errors::ManagedErrorCode,
    paths::ManagedPaths,
    platform_links::{LinkInspection, PlatformLinker, SystemPlatformLinker},
    AgentId, BindingStatus, LinkKind, ManagedBinding, ManagedSkill, ManagedSkillOrigin,
    ManagedSkillStatus,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use uuid::Uuid;

struct TestHome {
    path: PathBuf,
}

impl TestHome {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "skillreg-managed-bindings-{name}-{}",
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

struct TestAdapter {
    agent: AgentId,
    relative_root: &'static str,
    allow_directory_creation: bool,
}

impl TestAdapter {
    fn new(agent: AgentId, relative_root: &'static str) -> Self {
        Self {
            agent,
            relative_root,
            allow_directory_creation: true,
        }
    }

    fn without_directory_creation(mut self) -> Self {
        self.allow_directory_creation = false;
        self
    }
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
            preferred_path: Some(home.join(self.relative_root)),
            legacy_skill_dirs: vec![],
            requires_restart_after_binding: false,
            detail_code: None,
        }
    }

    fn candidate_user_skill_dirs(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(self.relative_root)]
    }

    fn preferred_user_skill_dir(&self, home: &Path) -> Result<PathBuf, AgentError> {
        if !home.is_absolute() {
            return Err(AgentError {
                code: AgentErrorCode::InvalidHome,
            });
        }
        Ok(home.join(self.relative_root))
    }

    fn supports_managed_links(&self, _platform: Platform) -> bool {
        true
    }

    fn allows_user_skill_dir_creation(&self) -> bool {
        self.allow_directory_creation
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

fn registry(adapters: Vec<(AgentId, &'static str)>) -> DefaultAgentRegistry {
    DefaultAgentRegistry::from_adapters(
        adapters
            .into_iter()
            .map(|(agent, root)| Box::new(TestAdapter::new(agent, root)) as Box<dyn AgentAdapter>)
            .collect(),
    )
}

fn sample_skill(paths: &ManagedPaths) -> ManagedSkill {
    let content = paths
        .content_dir("acme", "publisher", "review-helper")
        .unwrap();
    fs::create_dir_all(&content).unwrap();
    fs::write(
        content.join("SKILL.md"),
        "---\nname: review-helper\ndescription: Review changes\n---\n",
    )
    .unwrap();
    ManagedSkill {
        installation_id: Uuid::new_v4().to_string(),
        consumer_org: "acme".to_string(),
        source_org: "publisher".to_string(),
        skill_id: Some("skill-id".to_string()),
        skill_name: "review-helper".to_string(),
        origin: ManagedSkillOrigin::Registry,
        active_version: "1.2.3".to_string(),
        sha256: "a".repeat(64),
        content_path: content.to_string_lossy().into_owned(),
        content_hash: "b".repeat(64),
        status: ManagedSkillStatus::Ready,
        installed_at: "2026-07-29T00:00:00Z".to_string(),
        last_checked_at: None,
        last_updated_at: None,
        last_error: None,
        cleanup_dismissed_until: None,
    }
}

fn detection(agent: AgentId, path: PathBuf) -> DetectionResult {
    DetectionResult {
        agent,
        state: DetectionState::Detected,
        detected_version: Some("test".to_string()),
        preferred_path: Some(path),
        legacy_skill_dirs: vec![],
        requires_restart_after_binding: false,
        detail_code: None,
    }
}

fn service(home: &TestHome) -> ManagedBindingService<DefaultAgentRegistry, SystemPlatformLinker> {
    ManagedBindingService::new(
        registry(vec![(AgentId::Claude, ".claude/skills")]),
        SystemPlatformLinker::current(),
        ManagedPaths::new(home.path.clone()).unwrap(),
    )
}

fn first_binding(
    report: &skillreg_local_lib::managed_skills::bindings::BindingApplyReport,
) -> ManagedBinding {
    report.results[0].binding.clone().unwrap()
}

#[test]
fn creates_a_ready_link_in_an_empty_agent_directory() {
    let home = TestHome::new("create");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let service = service(&home);
    let plan = service
        .plan(
            &skill,
            &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
        )
        .unwrap();

    let report = service.apply(plan);

    assert_eq!(report.results[0].status, BindingStatus::Ready);
    let binding = first_binding(&report);
    assert_eq!(
        PathBuf::from(&binding.link_path),
        home.path.join(".claude/skills/review-helper")
    );
    assert_eq!(
        fs::canonicalize(&binding.link_path).unwrap(),
        fs::canonicalize(&skill.content_path).unwrap()
    );
}

#[test]
fn applying_an_owned_binding_twice_is_idempotent() {
    let home = TestHome::new("idempotent");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let service = service(&home);
    let detections = [detection(AgentId::Claude, home.path.join(".claude/skills"))];
    let first = service.apply(service.plan(&skill, &detections).unwrap());
    let binding = first_binding(&first);
    let before = fs::symlink_metadata(&binding.link_path).unwrap();

    let second = service.apply(
        service
            .plan_with_bindings(&skill, &detections, std::slice::from_ref(&binding))
            .unwrap(),
    );

    assert_eq!(second.results[0].status, BindingStatus::Ready);
    assert_eq!(
        fs::symlink_metadata(&binding.link_path)
            .unwrap()
            .file_type(),
        before.file_type()
    );
}

#[test]
fn a_link_to_another_target_is_a_conflict_and_remains_intact() {
    let home = TestHome::new("wrong-target");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let wrong_target = home.path.join("unmanaged");
    let link = home.path.join(".claude/skills/review-helper");
    fs::create_dir_all(&wrong_target).unwrap();
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    create_test_link(&wrong_target, &link);
    let service = service(&home);

    let report = service.apply(
        service
            .plan(
                &skill,
                &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
            )
            .unwrap(),
    );

    assert_eq!(report.results[0].status, BindingStatus::Conflict);
    assert_eq!(
        fs::canonicalize(link).unwrap(),
        fs::canonicalize(wrong_target).unwrap()
    );
}

#[test]
fn a_physical_directory_is_a_conflict_and_remains_intact() {
    assert_physical_conflict(true);
}

#[test]
fn a_physical_file_is_a_conflict_and_remains_intact() {
    assert_physical_conflict(false);
}

fn assert_physical_conflict(directory: bool) {
    let home = TestHome::new(if directory { "directory" } else { "file" });
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let link = home.path.join(".claude/skills/review-helper");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    if directory {
        fs::create_dir(&link).unwrap();
        fs::write(link.join("keep.txt"), "keep").unwrap();
    } else {
        fs::write(&link, "keep").unwrap();
    }
    let service = service(&home);

    let report = service.apply(
        service
            .plan(
                &skill,
                &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
            )
            .unwrap(),
    );

    assert_eq!(report.results[0].status, BindingStatus::Conflict);
    assert!(link.exists());
    if directory {
        assert_eq!(fs::read_to_string(link.join("keep.txt")).unwrap(), "keep");
    } else {
        assert_eq!(fs::read_to_string(link).unwrap(), "keep");
    }
}

#[test]
fn removing_an_owned_binding_never_removes_its_target() {
    let home = TestHome::new("remove");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let service = service(&home);
    let report = service.apply(
        service
            .plan(
                &skill,
                &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
            )
            .unwrap(),
    );
    let binding = first_binding(&report);

    service.remove_owned(&skill, &binding).unwrap();

    assert!(fs::symlink_metadata(&binding.link_path).is_err());
    assert!(Path::new(&skill.content_path).join("SKILL.md").is_file());
}

#[test]
fn a_target_outside_the_managed_root_is_refused_before_mutation() {
    let home = TestHome::new("outside");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let mut skill = sample_skill(&paths);
    skill.content_path = home
        .path
        .join("outside/content")
        .to_string_lossy()
        .into_owned();
    let service = service(&home);

    let error = service
        .plan(
            &skill,
            &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
        )
        .unwrap_err();

    assert_eq!(error.code(), ManagedErrorCode::ManagedPathOutsideRoot);
    assert!(!home.path.join(".claude").exists());
}

#[test]
fn an_adapter_must_explicitly_allow_agent_directory_creation() {
    let home = TestHome::new("parent-policy");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let adapters = vec![Box::new(
        TestAdapter::new(AgentId::Claude, ".claude/skills").without_directory_creation(),
    ) as Box<dyn AgentAdapter>];
    let service = ManagedBindingService::new(
        DefaultAgentRegistry::from_adapters(adapters),
        SystemPlatformLinker::current(),
        paths,
    );

    let report = service.apply(
        service
            .plan(
                &skill,
                &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
            )
            .unwrap(),
    );

    assert_eq!(report.results[0].status, BindingStatus::Unsupported);
    assert!(!home.path.join(".claude").exists());
}

#[cfg(unix)]
#[test]
fn a_symlinked_agent_parent_is_refused_without_writing_through_it() {
    let home = TestHome::new("linked-agent-parent");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let outside = home.path.join("outside-agent-root");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, home.path.join(".claude")).unwrap();
    let service = service(&home);

    let report = service.apply(
        service
            .plan(
                &skill,
                &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
            )
            .unwrap(),
    );

    assert_eq!(report.results[0].status, BindingStatus::Conflict);
    assert!(!outside.join("skills/review-helper").exists());
}

#[cfg(unix)]
#[test]
fn a_managed_content_symlink_that_escapes_the_store_is_refused() {
    let home = TestHome::new("linked-content");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let outside = home.path.join("outside-content");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("SKILL.md"), "outside").unwrap();
    fs::remove_dir_all(&skill.content_path).unwrap();
    std::os::unix::fs::symlink(&outside, &skill.content_path).unwrap();
    let service = service(&home);

    let report = service.apply(
        service
            .plan(
                &skill,
                &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
            )
            .unwrap(),
    );

    assert_eq!(report.results[0].status, BindingStatus::Error);
    assert!(!home.path.join(".claude").exists());
    assert_eq!(
        fs::read_to_string(outside.join("SKILL.md")).unwrap(),
        "outside"
    );
}

#[derive(Clone)]
struct VerifyFailureLinker {
    inner: SystemPlatformLinker,
    failed: Arc<AtomicBool>,
}

impl PlatformLinker for VerifyFailureLinker {
    fn platform(&self) -> Platform {
        self.inner.platform()
    }

    fn link_kind(&self) -> LinkKind {
        self.inner.link_kind()
    }

    fn validate_paths(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        self.inner.validate_paths(target, link)
    }

    fn inspect(&self, link: &Path) -> Result<LinkInspection, ManagedErrorCode> {
        if link
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(".skillreg-tmp"))
            && !self.failed.swap(true, Ordering::SeqCst)
        {
            return Err(ManagedErrorCode::BindingVerifyFailed);
        }
        self.inner.inspect(link)
    }

    fn create_dir_link(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        self.inner.create_dir_link(target, link)
    }

    fn rename_link(&self, source: &Path, destination: &Path) -> Result<(), ManagedErrorCode> {
        self.inner.rename_link(source, destination)
    }

    fn remove_link(&self, link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode> {
        self.inner.remove_link(link, kind)
    }
}

#[test]
fn failure_after_temporary_link_creation_cleans_only_the_temporary_link() {
    let home = TestHome::new("temp-cleanup");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let inner = SystemPlatformLinker::current();
    let service = ManagedBindingService::new(
        registry(vec![(AgentId::Claude, ".claude/skills")]),
        VerifyFailureLinker {
            inner,
            failed: Arc::new(AtomicBool::new(false)),
        },
        paths,
    );

    let report = service.apply(
        service
            .plan(
                &skill,
                &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
            )
            .unwrap(),
    );

    assert_eq!(report.results[0].status, BindingStatus::Error);
    let entries = fs::read_dir(home.path.join(".claude/skills"))
        .unwrap()
        .flatten()
        .collect::<Vec<_>>();
    assert!(entries.is_empty());
    assert!(Path::new(&skill.content_path).join("SKILL.md").is_file());
}

#[derive(Clone)]
struct SelectiveFailureLinker {
    inner: SystemPlatformLinker,
}

impl PlatformLinker for SelectiveFailureLinker {
    fn platform(&self) -> Platform {
        self.inner.platform()
    }

    fn link_kind(&self) -> LinkKind {
        self.inner.link_kind()
    }

    fn validate_paths(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        self.inner.validate_paths(target, link)
    }

    fn inspect(&self, link: &Path) -> Result<LinkInspection, ManagedErrorCode> {
        self.inner.inspect(link)
    }

    fn create_dir_link(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        if link.to_string_lossy().contains(".claude") {
            return Err(ManagedErrorCode::BindingCreateFailed);
        }
        self.inner.create_dir_link(target, link)
    }

    fn rename_link(&self, source: &Path, destination: &Path) -> Result<(), ManagedErrorCode> {
        self.inner.rename_link(source, destination)
    }

    fn remove_link(&self, link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode> {
        self.inner.remove_link(link, kind)
    }
}

#[test]
fn reconcile_continues_with_other_agents_after_one_failure() {
    let home = TestHome::new("continue");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let service = ManagedBindingService::new(
        registry(vec![
            (AgentId::Claude, ".claude/skills"),
            (AgentId::Codex, ".agents/skills"),
        ]),
        SelectiveFailureLinker {
            inner: SystemPlatformLinker::current(),
        },
        paths,
    );
    let detections = [
        detection(AgentId::Claude, home.path.join(".claude/skills")),
        detection(AgentId::Codex, home.path.join(".agents/skills")),
    ];

    let report = service.reconcile(&skill, &[], &detections);

    assert_eq!(report.results.len(), 2);
    assert_eq!(report.results[0].status, BindingStatus::Error);
    assert_eq!(report.results[1].status, BindingStatus::Ready);
    assert!(home
        .path
        .join(".agents/skills/review-helper/SKILL.md")
        .is_file());
}

#[derive(Clone)]
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

#[test]
fn a_link_kind_incompatible_with_the_platform_is_refused() {
    let home = TestHome::new("wrong-kind");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let service = ManagedBindingService::new(
        registry(vec![(AgentId::Claude, ".claude/skills")]),
        WrongKindLinker,
        paths,
    );

    let error = service
        .plan(
            &skill,
            &[detection(AgentId::Claude, home.path.join(".claude/skills"))],
        )
        .unwrap_err();

    assert_eq!(error.code(), ManagedErrorCode::BindingUnsupported);
    assert!(!home.path.join(".claude").exists());
}

#[test]
fn missing_owned_link_is_recreated() {
    let home = TestHome::new("broken-owned");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let service = service(&home);
    let detections = [detection(AgentId::Claude, home.path.join(".claude/skills"))];
    let first = service.apply(service.plan(&skill, &detections).unwrap());
    let binding = first_binding(&first);
    remove_test_link(Path::new(&binding.link_path));

    let repaired = service.reconcile(&skill, &[binding], &detections);

    assert_eq!(repaired.results[0].status, BindingStatus::Ready);
    assert!(Path::new(&repaired.results[0].binding.as_ref().unwrap().link_path).is_dir());
}

#[test]
fn broken_owned_link_recovers_after_canonical_content_is_restored() {
    let home = TestHome::new("broken-target");
    let paths = ManagedPaths::new(home.path.clone()).unwrap();
    let skill = sample_skill(&paths);
    let service = service(&home);
    let detections = [detection(AgentId::Claude, home.path.join(".claude/skills"))];
    let first = service.apply(service.plan(&skill, &detections).unwrap());
    let binding = first_binding(&first);
    fs::remove_dir_all(&skill.content_path).unwrap();
    assert!(fs::symlink_metadata(&binding.link_path).is_ok());
    assert!(!Path::new(&binding.link_path).exists());
    fs::create_dir_all(&skill.content_path).unwrap();
    fs::write(
        Path::new(&skill.content_path).join("SKILL.md"),
        "---\nname: review-helper\ndescription: Restored\n---\n",
    )
    .unwrap();

    let repaired = service.reconcile(&skill, &[binding], &detections);

    assert_eq!(repaired.results[0].status, BindingStatus::Ready);
    assert!(
        Path::new(&repaired.results[0].binding.as_ref().unwrap().link_path)
            .join("SKILL.md")
            .is_file()
    );
}

#[cfg(unix)]
fn create_test_link(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[cfg(windows)]
fn create_test_link(target: &Path, link: &Path) {
    junction::create(target, link).unwrap();
}

#[cfg(unix)]
fn remove_test_link(link: &Path) {
    fs::remove_file(link).unwrap();
}

#[cfg(windows)]
fn remove_test_link(link: &Path) {
    junction::delete(link).unwrap();
}

#[cfg(target_os = "windows")]
#[test]
fn windows_rejects_unc_and_cross_volume_junctions() {
    let linker = SystemPlatformLinker::current();

    assert_eq!(
        linker
            .validate_paths(
                Path::new(r"\\server\share\skill"),
                Path::new(r"C:\Users\employee\.claude\skills\skill")
            )
            .unwrap_err(),
        ManagedErrorCode::BindingUnsupported
    );
    assert_eq!(
        linker
            .validate_paths(
                Path::new(r"D:\SkillReg\skill"),
                Path::new(r"C:\Users\employee\.claude\skills\skill")
            )
            .unwrap_err(),
        ManagedErrorCode::BindingUnsupported
    );
}
