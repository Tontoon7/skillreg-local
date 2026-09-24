mod claude;
mod codex;
mod cursor;

pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use cursor::CursorAdapter;

use crate::managed_skills::{AgentId, BindingStatus, LinkKind};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    fs, io,
    path::{Component, Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Macos,
    Linux,
    Windows,
    Unsupported,
}

impl Platform {
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Unsupported
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionState {
    Detected,
    NotDetected,
    Unsupported,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub agent: AgentId,
    pub state: DetectionState,
    pub detected_version: Option<String>,
    pub preferred_path: Option<PathBuf>,
    pub legacy_skill_dirs: Vec<PathBuf>,
    pub requires_restart_after_binding: bool,
    pub detail_code: Option<String>,
}

impl DetectionResult {
    fn new(agent: AgentId, state: DetectionState, preferred_path: Option<PathBuf>) -> Self {
        Self {
            agent,
            state,
            detected_version: None,
            preferred_path,
            legacy_skill_dirs: Vec::new(),
            requires_restart_after_binding: false,
            detail_code: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageCapability {
    Exact,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentErrorCode {
    DetectionFailed,
    InvalidHome,
    BindingInvalid,
    BindingInspectFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentError {
    pub code: AgentErrorCode,
}

impl AgentError {
    pub const fn detection_failed() -> Self {
        Self {
            code: AgentErrorCode::DetectionFailed,
        }
    }

    const fn invalid_home() -> Self {
        Self {
            code: AgentErrorCode::InvalidHome,
        }
    }

    const fn binding_invalid() -> Self {
        Self {
            code: AgentErrorCode::BindingInvalid,
        }
    }

    const fn binding_inspect_failed() -> Self {
        Self {
            code: AgentErrorCode::BindingInspectFailed,
        }
    }
}

pub trait AgentProbe: Debug + Send + Sync {
    fn version(&self) -> Result<Option<String>, AgentError>;
}

#[derive(Debug)]
pub struct ProcessAgentProbe {
    executable: &'static str,
    arguments: &'static [&'static str],
}

impl ProcessAgentProbe {
    pub const fn new(executable: &'static str, arguments: &'static [&'static str]) -> Self {
        Self {
            executable,
            arguments,
        }
    }
}

impl AgentProbe for ProcessAgentProbe {
    fn version(&self) -> Result<Option<String>, AgentError> {
        let output = match Command::new(self.executable).args(self.arguments).output() {
            Ok(output) => output,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(AgentError::detection_failed()),
        };

        if !output.status.success() {
            return Err(AgentError::detection_failed());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let version = stdout
            .lines()
            .chain(stderr.lines())
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToOwned::to_owned);
        Ok(version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingRequest {
    pub agent: AgentId,
    pub home: PathBuf,
    pub link_path: PathBuf,
    pub target_path: PathBuf,
    pub link_kind: LinkKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingState {
    pub status: BindingStatus,
    pub actual_target: Option<PathBuf>,
}

pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> AgentId;
    fn detect(&self, home: &Path) -> DetectionResult;
    fn candidate_user_skill_dirs(&self, home: &Path) -> Vec<PathBuf>;
    fn preferred_user_skill_dir(&self, home: &Path) -> Result<PathBuf, AgentError>;
    fn supports_managed_links(&self, platform: Platform) -> bool;
    fn allows_user_skill_dir_creation(&self) -> bool {
        false
    }
    fn verify_binding(&self, request: &BindingRequest) -> Result<BindingState, AgentError>;
    fn usage_capability(&self) -> UsageCapability {
        UsageCapability::Unavailable
    }
}

pub trait AgentRegistry: Send + Sync {
    fn adapters(&self) -> Vec<&dyn AgentAdapter>;

    fn adapter(&self, agent: AgentId) -> Option<&dyn AgentAdapter> {
        self.adapters()
            .into_iter()
            .find(|adapter| adapter.id() == agent)
    }

    fn detect_all(&self, home: &Path) -> Vec<DetectionResult> {
        self.adapters()
            .into_iter()
            .map(|adapter| adapter.detect(home))
            .collect()
    }
}

pub struct DefaultAgentRegistry {
    adapters: Vec<Box<dyn AgentAdapter>>,
}

impl DefaultAgentRegistry {
    pub fn from_adapters(adapters: Vec<Box<dyn AgentAdapter>>) -> Self {
        Self { adapters }
    }
}

impl Default for DefaultAgentRegistry {
    fn default() -> Self {
        Self::from_adapters(vec![
            Box::new(ClaudeAdapter::default()),
            Box::new(CodexAdapter::default()),
            Box::new(CursorAdapter::default()),
        ])
    }
}

impl AgentRegistry for DefaultAgentRegistry {
    fn adapters(&self) -> Vec<&dyn AgentAdapter> {
        self.adapters
            .iter()
            .map(|adapter| adapter.as_ref())
            .collect()
    }
}

pub(crate) fn preferred_path(home: &Path, relative: &str) -> Result<PathBuf, AgentError> {
    validate_home(home)?;
    Ok(home.join(relative))
}

fn validate_home(home: &Path) -> Result<(), AgentError> {
    if !home.is_absolute()
        || home
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(AgentError::invalid_home());
    }
    Ok(())
}

pub(crate) fn discover_skill_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut discovered = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("SKILL.md").is_file() {
                discovered.push(path);
            }
        }
    }
    discovered.sort();
    discovered
}

pub(crate) fn normalize_version(raw: &str) -> Option<String> {
    raw.split_whitespace()
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        })
        .map(|part| {
            part.trim_matches(|character: char| {
                !character.is_ascii_alphanumeric() && character != '.' && character != '-'
            })
            .to_string()
        })
        .filter(|version| !version.is_empty())
}

pub(crate) fn version_at_least(raw: &str, minimum: (u64, u64, u64)) -> bool {
    let Some(version) = normalize_version(raw) else {
        return false;
    };
    let mut parts = version.split('.').map(|part| {
        part.split(|character: char| !character.is_ascii_digit())
            .next()
            .and_then(|part| part.parse::<u64>().ok())
    });
    let parsed = (
        parts.next().flatten().unwrap_or(0),
        parts.next().flatten().unwrap_or(0),
        parts.next().flatten().unwrap_or(0),
    );
    parsed >= minimum
}

pub(crate) fn detection_from_probe(
    agent: AgentId,
    preferred: Result<PathBuf, AgentError>,
    probe: &dyn AgentProbe,
    legacy_roots: &[PathBuf],
    requires_restart_after_binding: bool,
) -> DetectionResult {
    let preferred = match preferred {
        Ok(preferred) => preferred,
        Err(_) => {
            let mut result = DetectionResult::new(agent, DetectionState::Error, None);
            result.detail_code = Some("invalid_home".to_string());
            return result;
        }
    };
    let mut result = DetectionResult::new(agent, DetectionState::NotDetected, Some(preferred));
    result.legacy_skill_dirs = discover_skill_dirs(legacy_roots);
    result.requires_restart_after_binding = requires_restart_after_binding;

    match probe.version() {
        Ok(Some(raw_version)) => {
            result.detected_version =
                normalize_version(&raw_version).or_else(|| Some(raw_version.trim().to_string()));
            result.state = DetectionState::Detected;
        }
        Ok(None) => {}
        Err(_) => {
            result.state = DetectionState::Error;
            result.detail_code = Some("agent_detection_failed".to_string());
        }
    }
    result
}

pub(crate) fn verify_symlink_binding(
    adapter: &dyn AgentAdapter,
    request: &BindingRequest,
) -> Result<BindingState, AgentError> {
    if request.agent != adapter.id()
        || request.link_kind != LinkKind::Symlink
        || !adapter.supports_managed_links(Platform::current())
    {
        return Ok(BindingState {
            status: BindingStatus::Unsupported,
            actual_target: None,
        });
    }
    validate_home(&request.home)?;
    if !request.link_path.is_absolute() || !request.target_path.is_absolute() {
        return Err(AgentError::binding_invalid());
    }
    let allowed_parent = adapter
        .candidate_user_skill_dirs(&request.home)
        .into_iter()
        .any(|candidate| request.link_path.parent() == Some(candidate.as_path()));
    if !allowed_parent {
        return Err(AgentError::binding_invalid());
    }

    let metadata = match fs::symlink_metadata(&request.link_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(BindingState {
                status: BindingStatus::Missing,
                actual_target: None,
            });
        }
        Err(_) => return Err(AgentError::binding_inspect_failed()),
    };
    if !metadata.file_type().is_symlink() {
        return Ok(BindingState {
            status: BindingStatus::Conflict,
            actual_target: None,
        });
    }

    let raw_target =
        fs::read_link(&request.link_path).map_err(|_| AgentError::binding_inspect_failed())?;
    let actual_target = if raw_target.is_absolute() {
        raw_target
    } else {
        request
            .link_path
            .parent()
            .ok_or_else(AgentError::binding_invalid)?
            .join(raw_target)
    };
    let status = if actual_target == request.target_path {
        BindingStatus::Ready
    } else {
        BindingStatus::Conflict
    };
    Ok(BindingState {
        status,
        actual_target: Some(actual_target),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_skills::AgentId;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::Arc,
    };
    use uuid::Uuid;

    #[derive(Debug)]
    struct FixedProbe {
        result: ProbeResult,
    }

    impl AgentProbe for FixedProbe {
        fn version(&self) -> Result<Option<String>, AgentError> {
            match &self.result {
                ProbeResult::Missing => Ok(None),
                ProbeResult::Version(version) => Ok(Some(version.clone())),
                ProbeResult::Error => Err(AgentError::detection_failed()),
            }
        }
    }

    #[derive(Debug)]
    enum ProbeResult {
        Missing,
        Version(String),
        Error,
    }

    fn probe(result: ProbeResult) -> Arc<dyn AgentProbe> {
        Arc::new(FixedProbe { result })
    }

    fn test_home() -> PathBuf {
        std::env::temp_dir().join(format!("skillreg-agent-adapters-{}", Uuid::new_v4()))
    }

    fn assert_missing(path: &Path) {
        assert!(
            !path.exists(),
            "read-only detection unexpectedly created {}",
            path.display()
        );
    }

    #[test]
    fn absent_agent_is_not_detected_and_detection_creates_nothing() {
        let home = test_home();
        let preferred = home.join(".claude/skills");
        let adapter = ClaudeAdapter::with_probe(probe(ProbeResult::Missing));

        let result = adapter.detect(&home);

        assert_eq!(result.agent, AgentId::Claude);
        assert_eq!(result.state, DetectionState::NotDetected);
        assert_eq!(result.preferred_path, Some(preferred.clone()));
        assert_missing(&home);
        assert_missing(&preferred);
    }

    #[test]
    fn incompatible_binary_is_reported_as_unsupported() {
        let home = test_home();
        let adapter = ClaudeAdapter::with_probe(probe(ProbeResult::Version("2.1.100".to_string())));

        let result = adapter.detect(&home);

        assert_eq!(result.state, DetectionState::Unsupported);
        assert_eq!(result.detected_version.as_deref(), Some("2.1.100"));
        assert_eq!(
            result.detail_code.as_deref(),
            Some("managed_links_require_claude_2_1_203")
        );
    }

    #[test]
    fn claude_requires_restart_when_the_top_level_skills_directory_was_missing() {
        let home = test_home();
        fs::create_dir_all(home.join(".claude")).unwrap();
        let adapter = ClaudeAdapter::with_probe(probe(ProbeResult::Version("2.1.220".to_string())));

        let result = adapter.detect(&home);

        assert_validated_platform_state(&result);
        assert!(result.requires_restart_after_binding);
        assert_missing(&home.join(".claude/skills"));

        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn preferred_paths_only_depend_on_the_explicit_home() {
        let home = test_home();
        let adapter = ClaudeAdapter::with_probe(probe(ProbeResult::Missing));

        assert_eq!(
            adapter.preferred_user_skill_dir(&home).unwrap(),
            home.join(".claude/skills")
        );
    }

    #[test]
    fn codex_prefers_the_current_shared_agents_location() {
        let home = test_home();
        let adapter = CodexAdapter::with_probe(probe(ProbeResult::Version("0.145.0".to_string())));

        assert_eq!(
            adapter.preferred_user_skill_dir(&home).unwrap(),
            home.join(".agents/skills")
        );
        assert_eq!(
            adapter.candidate_user_skill_dirs(&home),
            vec![home.join(".agents/skills"), home.join(".codex/skills")]
        );
    }

    #[test]
    fn legacy_installation_is_reported_but_never_claimed() {
        let home = test_home();
        let legacy_skill = home.join(".codex/skills/review-helper");
        fs::create_dir_all(&legacy_skill).unwrap();
        fs::write(
            legacy_skill.join("SKILL.md"),
            "---\nname: review-helper\ndescription: Review changes\n---\n",
        )
        .unwrap();
        let adapter = CodexAdapter::with_probe(probe(ProbeResult::Missing));

        let result = adapter.detect(&home);

        assert_eq!(result.state, DetectionState::NotDetected);
        assert_eq!(result.legacy_skill_dirs, vec![legacy_skill]);
        assert_missing(&home.join(".agents"));

        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn detection_errors_are_structured_and_do_not_stop_the_registry() {
        let home = test_home();
        let registry = DefaultAgentRegistry::from_adapters(vec![
            Box::new(ClaudeAdapter::with_probe(probe(ProbeResult::Error))),
            Box::new(CodexAdapter::with_probe(probe(ProbeResult::Version(
                "0.145.0".to_string(),
            )))),
        ]);

        let results = registry.detect_all(&home);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].state, DetectionState::Error);
        assert_validated_platform_state(&results[1]);
    }

    // Managed links are only enabled on platforms validated by hand (macOS today).
    fn assert_validated_platform_state(result: &DetectionResult) {
        if Platform::current() == Platform::Macos {
            assert_eq!(result.state, DetectionState::Detected);
        } else {
            assert_eq!(result.state, DetectionState::Unsupported);
            assert_eq!(
                result.detail_code.as_deref(),
                Some("managed_links_not_validated_on_platform")
            );
        }
    }

    #[test]
    fn cursor_links_are_enabled_only_on_the_manually_validated_platform() {
        let home = test_home();
        let adapter =
            CursorAdapter::with_probe(probe(ProbeResult::Version("2026.07.23".to_string())));

        let result = adapter.detect(&home);

        if Platform::current() == Platform::Macos {
            assert_eq!(result.state, DetectionState::Detected);
            assert_eq!(result.detail_code, None);
        }
        assert!(adapter.supports_managed_links(Platform::Macos));
        assert!(!adapter.supports_managed_links(Platform::Linux));
        assert!(!adapter.supports_managed_links(Platform::Windows));
    }

    #[test]
    fn usage_is_unavailable_and_no_default_adapter_claims_exact_usage() {
        let registry = DefaultAgentRegistry::default();

        for adapter in registry.adapters() {
            assert_eq!(adapter.usage_capability(), UsageCapability::Unavailable);
            assert_ne!(adapter.usage_capability(), UsageCapability::Exact);
        }
    }
}
