use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedErrorCode {
    ManagedPathOutsideRoot,
    ManifestInvalid,
    ManifestWriteFailed,
    ArchiveChecksumMismatch,
    ArchiveUnsafe,
    ContentModified,
    BindingConflict,
    BindingUnsupported,
    BindingCreateFailed,
    BindingVerifyFailed,
    SkillSourceNameConflict,
    RollbackFailed,
    MigrationRequiresAction,
    RegistryRequestFailed,
    DownloadMetadataInvalid,
    ManagedInstallBusy,
    ManagedInstallationNotFound,
    ActiveOrganizationUnauthorized,
    ActiveOrganizationConflict,
    AuthenticationRequired,
    LocalConfigurationInvalid,
}

impl ManagedErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ManagedPathOutsideRoot => "MANAGED_PATH_OUTSIDE_ROOT",
            Self::ManifestInvalid => "MANIFEST_INVALID",
            Self::ManifestWriteFailed => "MANIFEST_WRITE_FAILED",
            Self::ArchiveChecksumMismatch => "ARCHIVE_CHECKSUM_MISMATCH",
            Self::ArchiveUnsafe => "ARCHIVE_UNSAFE",
            Self::ContentModified => "CONTENT_MODIFIED",
            Self::BindingConflict => "BINDING_CONFLICT",
            Self::BindingUnsupported => "BINDING_UNSUPPORTED",
            Self::BindingCreateFailed => "BINDING_CREATE_FAILED",
            Self::BindingVerifyFailed => "BINDING_VERIFY_FAILED",
            Self::SkillSourceNameConflict => "SKILL_SOURCE_NAME_CONFLICT",
            Self::RollbackFailed => "ROLLBACK_FAILED",
            Self::MigrationRequiresAction => "MIGRATION_REQUIRES_ACTION",
            Self::RegistryRequestFailed => "REGISTRY_REQUEST_FAILED",
            Self::DownloadMetadataInvalid => "DOWNLOAD_METADATA_INVALID",
            Self::ManagedInstallBusy => "MANAGED_INSTALL_BUSY",
            Self::ManagedInstallationNotFound => "MANAGED_INSTALLATION_NOT_FOUND",
            Self::ActiveOrganizationUnauthorized => "ACTIVE_ORGANIZATION_UNAUTHORIZED",
            Self::ActiveOrganizationConflict => "ACTIVE_ORGANIZATION_CONFLICT",
            Self::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            Self::LocalConfigurationInvalid => "LOCAL_CONFIGURATION_INVALID",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedErrorRecord {
    pub code: ManagedErrorCode,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedError {
    code: ManagedErrorCode,
    parameters: BTreeMap<String, String>,
}

impl ManagedError {
    pub fn new(code: ManagedErrorCode) -> Self {
        Self {
            code,
            parameters: BTreeMap::new(),
        }
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    pub const fn code(&self) -> ManagedErrorCode {
        self.code
    }

    pub fn parameters(&self) -> &BTreeMap<String, String> {
        &self.parameters
    }

    pub fn into_record(self, occurred_at: Option<String>) -> ManagedErrorRecord {
        ManagedErrorRecord {
            code: self.code,
            parameters: self.parameters,
            occurred_at,
        }
    }
}

impl fmt::Display for ManagedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.as_str())
    }
}

impl std::error::Error for ManagedError {}
