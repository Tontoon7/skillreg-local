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
        let key = key.into();
        if key == "status" {
            if let Ok(status) = value.into().parse::<u16>() {
                if (100..=599).contains(&status) {
                    self.parameters.insert(key, status.to_string());
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_registry_errors_do_not_persist_sensitive_diagnostics() {
        let diagnostics = [
            "fixture-access-token",
            "https://example.invalid/archive?signature=fixture-secret",
            "/Users/fixture/private/config.json",
        ];
        let mut error = ManagedError::new(ManagedErrorCode::RegistryRequestFailed)
            .with_parameter("status", "403");
        for (index, diagnostic) in diagnostics.iter().enumerate() {
            error = error.with_parameter(format!("diagnostic{index}"), *diagnostic);
        }
        assert_eq!(error.to_string(), "REGISTRY_REQUEST_FAILED");
        let record = serde_json::to_string(&error.into_record(Some("1".into()))).unwrap();
        for diagnostic in diagnostics {
            assert!(!record.contains(diagnostic));
        }
        assert!(record.contains("403"));
    }
}
