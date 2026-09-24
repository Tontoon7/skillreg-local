use skillreg_local_lib::managed_skills::{
    agents::{
        AgentAdapter, AgentError, BindingRequest, BindingState, DefaultAgentRegistry,
        DetectionResult, DetectionState, Platform, UsageCapability,
    },
    archive::compute_tree_hash,
    bindings::ManagedBindingService,
    manifest::{write_manifest_atomic, ManagedSkillsManifest},
    paths::ManagedPaths,
    platform_links::SystemPlatformLinker,
    reconcile::ManagedLifecycleService,
    service::FileManifestStore,
    AgentId, BindingStatus, ManagedBinding, ManagedSkill, ManagedSkillOrigin, ManagedSkillStatus,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub struct TestHome {
    pub path: PathBuf,
}

impl TestHome {
    pub fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "skillreg-managed-lifecycle-{name}-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub fn paths(&self) -> ManagedPaths {
        ManagedPaths::new(self.path.clone()).unwrap()
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
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

pub fn registry() -> DefaultAgentRegistry {
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

pub fn lifecycle(
    paths: &ManagedPaths,
) -> ManagedLifecycleService<DefaultAgentRegistry, SystemPlatformLinker, FileManifestStore> {
    ManagedLifecycleService::new(
        registry(),
        SystemPlatformLinker::current(),
        FileManifestStore,
        paths.clone(),
    )
}

pub fn create_skill(
    paths: &ManagedPaths,
    consumer_org: &str,
    source_org: &str,
    name: &str,
) -> ManagedSkill {
    let content = paths.content_dir(consumer_org, source_org, name).unwrap();
    fs::create_dir_all(&content).unwrap();
    fs::write(
        content.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Managed test skill\n---\n"),
    )
    .unwrap();
    ManagedSkill {
        installation_id: Uuid::new_v4().to_string(),
        consumer_org: consumer_org.to_string(),
        source_org: source_org.to_string(),
        skill_id: Some(format!("{source_org}-{name}")),
        skill_name: name.to_string(),
        origin: ManagedSkillOrigin::Registry,
        active_version: "1.0.0".to_string(),
        sha256: "a".repeat(64),
        content_path: content.to_string_lossy().into_owned(),
        content_hash: compute_tree_hash(&content).unwrap(),
        status: ManagedSkillStatus::Ready,
        installed_at: "1".to_string(),
        last_checked_at: None,
        last_updated_at: None,
        last_error: None,
        cleanup_dismissed_until: None,
    }
}

pub fn create_bindings(paths: &ManagedPaths, skill: &ManagedSkill) -> Vec<ManagedBinding> {
    let bindings =
        ManagedBindingService::new(registry(), SystemPlatformLinker::current(), paths.clone());
    bindings
        .apply(bindings.plan(skill, &bindings.detect_all()).unwrap())
        .results
        .into_iter()
        .filter_map(|result| result.binding)
        .collect()
}

pub fn write_manifest(
    paths: &ManagedPaths,
    active_org: Option<&str>,
    skills: Vec<ManagedSkill>,
    bindings: Vec<ManagedBinding>,
) {
    let mut manifest = ManagedSkillsManifest::new();
    manifest.active_org = active_org.map(ToOwned::to_owned);
    manifest.skills = skills;
    manifest.bindings = bindings;
    write_manifest_atomic(paths, &manifest).unwrap();
}
