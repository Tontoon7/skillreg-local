pub mod agents;
pub mod archive;
pub mod bindings;
pub mod errors;
pub mod local_import;
pub mod manifest;
pub mod migration;
pub mod paths;
pub mod platform_links;
pub mod reconcile;
pub mod service;

use errors::ManagedErrorRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentId {
    Claude,
    Codex,
    Cursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Symlink,
    Junction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedSkillStatus {
    Installing,
    Ready,
    UpdateAvailable,
    Updating,
    ActionRequired,
    Conflict,
    Error,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedSkillOrigin {
    #[default]
    Registry,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingStatus {
    Ready,
    Missing,
    Conflict,
    Unsupported,
    NeedsRestart,
    Error,
}

impl BindingStatus {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Ready | Self::NeedsRestart)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageObservability {
    Exact,
    Partial,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSkill {
    pub installation_id: String,
    pub consumer_org: String,
    pub source_org: String,
    pub skill_id: Option<String>,
    pub skill_name: String,
    #[serde(default)]
    pub origin: ManagedSkillOrigin,
    pub active_version: String,
    pub sha256: String,
    pub content_path: String,
    pub content_hash: String,
    pub status: ManagedSkillStatus,
    pub installed_at: String,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<ManagedErrorRecord>,
    pub cleanup_dismissed_until: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedBinding {
    pub installation_id: String,
    pub agent: AgentId,
    pub link_path: String,
    pub link_kind: LinkKind,
    pub status: BindingStatus,
    pub last_checked_at: Option<String>,
    pub last_error: Option<ManagedErrorRecord>,
    pub usage_observability: UsageObservability,
}
