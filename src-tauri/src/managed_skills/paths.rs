use crate::managed_skills::errors::{ManagedError, ManagedErrorCode};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedPaths {
    home: PathBuf,
    skillreg_root: PathBuf,
    skills_root: PathBuf,
}

impl ManagedPaths {
    pub fn from_home() -> Result<Self, ManagedError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot))?;
        Self::new(home)
    }

    pub fn new(home: PathBuf) -> Result<Self, ManagedError> {
        if !home.is_absolute() || has_unsafe_components(&home) {
            return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
        }
        let skillreg_root = home.join(".skillreg");
        let skills_root = skillreg_root.join("skills");
        Ok(Self {
            home,
            skillreg_root,
            skills_root,
        })
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn skillreg_root(&self) -> &Path {
        &self.skillreg_root
    }

    pub fn skills_root(&self) -> &Path {
        &self.skills_root
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.skillreg_root.join("managed-skills.json")
    }

    pub fn legacy_manifest_path(&self) -> PathBuf {
        self.skillreg_root.join("installed.json")
    }

    pub fn skill_root(
        &self,
        consumer_org: &str,
        source_org: &str,
        skill_name: &str,
    ) -> Result<PathBuf, ManagedError> {
        validate_org_slug(consumer_org)?;
        validate_org_slug(source_org)?;
        validate_skill_name(skill_name)?;
        let path = self
            .skills_root
            .join(consumer_org)
            .join(source_org)
            .join(skill_name);
        self.validate_descendant(&path)?;
        Ok(path)
    }

    pub fn content_dir(
        &self,
        consumer_org: &str,
        source_org: &str,
        skill_name: &str,
    ) -> Result<PathBuf, ManagedError> {
        Ok(self
            .skill_root(consumer_org, source_org, skill_name)?
            .join("content"))
    }

    pub fn previous_dir(
        &self,
        consumer_org: &str,
        source_org: &str,
        skill_name: &str,
    ) -> Result<PathBuf, ManagedError> {
        Ok(self
            .skill_root(consumer_org, source_org, skill_name)?
            .join("previous"))
    }

    pub fn staging_dir(
        &self,
        consumer_org: &str,
        source_org: &str,
        skill_name: &str,
    ) -> Result<PathBuf, ManagedError> {
        Ok(self
            .skill_root(consumer_org, source_org, skill_name)?
            .join("staging"))
    }

    pub fn validate_managed_content_path(&self, path: &Path) -> Result<PathBuf, ManagedError> {
        self.validate_descendant(path)?;
        if path.file_name().and_then(|name| name.to_str()) != Some("content") {
            return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
        }
        Ok(path.to_path_buf())
    }

    fn validate_descendant(&self, path: &Path) -> Result<(), ManagedError> {
        if !path.is_absolute()
            || has_unsafe_components(path)
            || path == self.skills_root
            || !path.starts_with(&self.skills_root)
        {
            return Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot));
        }
        Ok(())
    }
}

pub fn validate_org_slug(value: &str) -> Result<(), ManagedError> {
    if is_valid_identifier(value, 2, 48, |character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    }) {
        Ok(())
    } else {
        Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot))
    }
}

pub fn validate_skill_name(value: &str) -> Result<(), ManagedError> {
    if is_valid_identifier(value, 2, 64, |character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | '-')
    }) {
        Ok(())
    } else {
        Err(ManagedError::new(ManagedErrorCode::ManagedPathOutsideRoot))
    }
}

fn is_valid_identifier(
    value: &str,
    minimum_length: usize,
    maximum_length: usize,
    allowed: impl Fn(char) -> bool,
) -> bool {
    let length = value.len();
    if !(minimum_length..=maximum_length).contains(&length) || value.contains(['/', '\\']) {
        return false;
    }
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    let last = value.chars().last().unwrap_or(first);
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && (last.is_ascii_lowercase() || last.is_ascii_digit())
        && value.chars().all(allowed)
}

fn has_unsafe_components(path: &Path) -> bool {
    let has_traversal = path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir));
    #[cfg(not(target_os = "windows"))]
    {
        has_traversal || path.to_string_lossy().contains('\\')
    }
    #[cfg(target_os = "windows")]
    {
        has_traversal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_skills::errors::ManagedErrorCode;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn test_home() -> PathBuf {
        std::env::temp_dir().join(format!("skillreg-managed-paths-{}", Uuid::new_v4()))
    }

    #[test]
    fn source_orgs_with_same_skill_name_have_distinct_roots() {
        let paths = ManagedPaths::new(test_home()).unwrap();

        let first = paths
            .skill_root("acme", "publisher-one", "review-helper")
            .unwrap();
        let second = paths
            .skill_root("acme", "publisher-two", "review-helper")
            .unwrap();

        assert_ne!(first, second);
        assert!(first.ends_with("acme/publisher-one/review-helper"));
        assert!(second.ends_with("acme/publisher-two/review-helper"));
    }

    #[test]
    fn skill_names_follow_the_server_validation_contract() {
        let paths = ManagedPaths::new(test_home()).unwrap();

        let root = paths
            .skill_root("acme", "publisher", "review..helper")
            .unwrap();

        assert!(root.ends_with("acme/publisher/review..helper"));
    }

    #[test]
    fn invalid_or_injected_identifiers_are_rejected() {
        let paths = ManagedPaths::new(test_home()).unwrap();

        for invalid in [
            "..",
            "../acme",
            "/tmp/acme",
            r"acme\other",
            "Uppercase",
            "trailing-",
        ] {
            let error = paths
                .skill_root(invalid, "publisher", "review-helper")
                .unwrap_err();
            assert_eq!(error.code(), ManagedErrorCode::ManagedPathOutsideRoot);
        }
    }

    #[test]
    fn canonical_content_path_is_always_below_managed_skills_root() {
        let paths = ManagedPaths::new(test_home()).unwrap();

        let content = paths
            .content_dir("acme", "publisher", "review-helper")
            .unwrap();

        assert!(content.starts_with(paths.skills_root()));
        assert_eq!(
            paths.validate_managed_content_path(&content).unwrap(),
            content
        );
    }

    #[test]
    fn absolute_parent_and_windows_separator_injections_are_refused() {
        let paths = ManagedPaths::new(test_home()).unwrap();
        let mut candidates = vec![
            PathBuf::from("/tmp/outside/content"),
            paths.skills_root().join("../outside/content"),
        ];
        // A backslash is the native separator on Windows; elsewhere it smuggles extra segments.
        if !cfg!(target_os = "windows") {
            candidates.push(paths.skills_root().join(r"acme\publisher\skill\content"));
        }
        for candidate in candidates {
            let error = paths.validate_managed_content_path(&candidate).unwrap_err();
            assert_eq!(error.code(), ManagedErrorCode::ManagedPathOutsideRoot);
        }
    }
}
