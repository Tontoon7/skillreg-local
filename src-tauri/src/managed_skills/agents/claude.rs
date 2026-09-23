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

const MINIMUM_MANAGED_LINK_VERSION: (u64, u64, u64) = (2, 1, 203);

pub struct ClaudeAdapter {
    probe: Arc<dyn AgentProbe>,
}

impl ClaudeAdapter {
    pub fn with_probe(probe: Arc<dyn AgentProbe>) -> Self {
        Self { probe }
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::with_probe(Arc::new(ProcessAgentProbe::new("claude", &["--version"])))
    }
}

impl AgentAdapter for ClaudeAdapter {
    fn id(&self) -> AgentId {
        AgentId::Claude
    }

    fn detect(&self, home: &Path) -> DetectionResult {
        let preferred = self.preferred_user_skill_dir(home);
        let requires_restart = preferred.as_ref().is_ok_and(|path| !path.exists());
        let mut result = detection_from_probe(
            self.id(),
            preferred,
            self.probe.as_ref(),
            &[],
            requires_restart,
        );

        if result.state == DetectionState::Detected {
            let compatible = result.detected_version.as_deref().is_some_and(|version| {
                super::version_at_least(version, MINIMUM_MANAGED_LINK_VERSION)
            });
            if !compatible {
                result.state = DetectionState::Unsupported;
                result.detail_code = Some("managed_links_require_claude_2_1_203".to_string());
            } else if !self.supports_managed_links(Platform::current()) {
                result.state = DetectionState::Unsupported;
                result.detail_code = Some("managed_links_not_validated_on_platform".to_string());
            }
        }
        result
    }

    fn candidate_user_skill_dirs(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".claude/skills")]
    }

    fn preferred_user_skill_dir(&self, home: &Path) -> Result<PathBuf, AgentError> {
        preferred_path(home, ".claude/skills")
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
