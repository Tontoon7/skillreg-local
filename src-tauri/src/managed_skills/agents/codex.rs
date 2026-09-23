use super::{
    detection_from_probe, preferred_path, verify_symlink_binding, AgentAdapter, AgentError,
    AgentProbe, BindingRequest, BindingState, DetectionResult, DetectionState, Platform,
    ProcessAgentProbe, UsageCapability,
};
use crate::managed_skills::AgentId;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct CodexAdapter {
    probe: Arc<dyn AgentProbe>,
}

impl CodexAdapter {
    pub fn with_probe(probe: Arc<dyn AgentProbe>) -> Self {
        Self { probe }
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::with_probe(Arc::new(ProcessAgentProbe::new("codex", &["--version"])))
    }
}

impl AgentAdapter for CodexAdapter {
    fn id(&self) -> AgentId {
        AgentId::Codex
    }

    fn detect(&self, home: &Path) -> DetectionResult {
        let legacy_roots = [home.join(".codex/skills")];
        let mut result = detection_from_probe(
            self.id(),
            self.preferred_user_skill_dir(home),
            self.probe.as_ref(),
            &legacy_roots,
            true,
        );
        if result.state == DetectionState::Detected
            && !self.supports_managed_links(Platform::current())
        {
            result.state = DetectionState::Unsupported;
            result.detail_code = Some("managed_links_not_validated_on_platform".to_string());
        }
        result
    }

    fn candidate_user_skill_dirs(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".agents/skills"), home.join(".codex/skills")]
    }

    fn preferred_user_skill_dir(&self, home: &Path) -> Result<PathBuf, AgentError> {
        preferred_path(home, ".agents/skills")
    }

    fn supports_managed_links(&self, platform: Platform) -> bool {
        matches!(platform, Platform::Macos)
    }

    fn allows_user_skill_dir_creation(&self) -> bool {
        true
    }

    fn verify_binding(&self, request: &BindingRequest) -> Result<BindingState, AgentError> {
        verify_symlink_binding(self, request)
    }

    fn usage_capability(&self) -> UsageCapability {
        UsageCapability::Unavailable
    }
}
