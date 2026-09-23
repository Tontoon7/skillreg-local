use crate::managed_skills::{
    errors::{ManagedError, ManagedErrorCode},
    paths::{validate_org_slug, validate_skill_name, ManagedPaths},
    ManagedBinding, ManagedSkill,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

pub const MANAGED_MANIFEST_VERSION: u32 = 2;
static MANIFEST_WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSkillsManifest {
    pub version: u32,
    pub active_org: Option<String>,
    pub client_instance_id: String,
    pub migrated_from_v1_at: Option<String>,
    #[serde(default)]
    pub removed_installation_ids: Vec<String>,
    pub skills: Vec<ManagedSkill>,
    pub bindings: Vec<ManagedBinding>,
}

impl ManagedSkillsManifest {
    pub fn new() -> Self {
        Self {
            version: MANAGED_MANIFEST_VERSION,
            active_org: None,
            client_instance_id: Uuid::new_v4().to_string(),
            migrated_from_v1_at: None,
            removed_installation_ids: Vec::new(),
            skills: Vec::new(),
            bindings: Vec::new(),
        }
    }
}

impl Default for ManagedSkillsManifest {
    fn default() -> Self {
        Self::new()
    }
}

pub fn load_or_create_manifest(
    paths: &ManagedPaths,
) -> Result<ManagedSkillsManifest, ManagedError> {
    if paths.manifest_path().exists() {
        return read_manifest(paths);
    }
    let manifest = ManagedSkillsManifest::new();
    write_manifest_atomic(paths, &manifest)?;
    Ok(manifest)
}

pub fn read_manifest(paths: &ManagedPaths) -> Result<ManagedSkillsManifest, ManagedError> {
    let content = fs::read(paths.manifest_path())
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
    let manifest: ManagedSkillsManifest = serde_json::from_slice(&content)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
    validate_manifest(paths, &manifest)?;
    Ok(manifest)
}

pub fn write_manifest_atomic(
    paths: &ManagedPaths,
    manifest: &ManagedSkillsManifest,
) -> Result<(), ManagedError> {
    validate_manifest(paths, manifest)?;
    let _guard = MANIFEST_WRITE_LOCK
        .lock()
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;

    create_private_directory(paths.skillreg_root())?;
    let manifest_path = paths.manifest_path();
    let temp_path = manifest_temp_path(&manifest_path);
    let content = serde_json::to_vec_pretty(manifest)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;

    let write_result = (|| {
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut temporary = options
            .open(&temp_path)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
        temporary
            .write_all(&content)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
        temporary
            .sync_all()
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
        drop(temporary);

        replace_manifest_file(&temp_path, &manifest_path)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
        sync_parent_directory(paths.skillreg_root())?;
        Ok(())
    })();

    if write_result.is_err() && temp_path.is_file() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

pub fn manifest_temp_path(manifest_path: impl AsRef<Path>) -> PathBuf {
    let manifest_path = manifest_path.as_ref();
    let filename = manifest_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("managed-skills.json");
    manifest_path.with_file_name(format!("{filename}.tmp"))
}

pub fn validate_manifest(
    paths: &ManagedPaths,
    manifest: &ManagedSkillsManifest,
) -> Result<(), ManagedError> {
    if manifest.version != MANAGED_MANIFEST_VERSION
        || Uuid::parse_str(&manifest.client_instance_id).is_err()
    {
        return Err(ManagedError::new(ManagedErrorCode::ManifestInvalid));
    }
    if let Some(active_org) = &manifest.active_org {
        validate_org_slug(active_org)
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
    }

    let mut installation_ids = HashSet::new();
    for skill in &manifest.skills {
        if !installation_ids.insert(skill.installation_id.as_str())
            || Uuid::parse_str(&skill.installation_id).is_err()
            || skill.active_version.eq_ignore_ascii_case("latest")
            || !is_sha256(&skill.sha256)
            || !is_sha256(&skill.content_hash)
        {
            return Err(ManagedError::new(ManagedErrorCode::ManifestInvalid));
        }
        validate_org_slug(&skill.consumer_org)
            .and_then(|_| validate_org_slug(&skill.source_org))
            .and_then(|_| validate_skill_name(&skill.skill_name))
            .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
        let expected_content =
            paths.content_dir(&skill.consumer_org, &skill.source_org, &skill.skill_name)?;
        let stored_content = PathBuf::from(&skill.content_path);
        paths.validate_managed_content_path(&stored_content)?;
        if stored_content != expected_content {
            return Err(ManagedError::new(ManagedErrorCode::ManifestInvalid));
        }
    }

    let mut removed_ids = HashSet::new();
    if manifest.removed_installation_ids.len() > 256
        || manifest
            .removed_installation_ids
            .iter()
            .any(|installation_id| {
                Uuid::parse_str(installation_id).is_err()
                    || !removed_ids.insert(installation_id.as_str())
                    || installation_ids.contains(installation_id.as_str())
            })
    {
        return Err(ManagedError::new(ManagedErrorCode::ManifestInvalid));
    }

    let skills_by_id: HashMap<&str, &ManagedSkill> = manifest
        .skills
        .iter()
        .map(|skill| (skill.installation_id.as_str(), skill))
        .collect();
    let mut active_names: HashMap<(&str, &str), (&str, &str)> = HashMap::new();
    for binding in &manifest.bindings {
        let skill = skills_by_id
            .get(binding.installation_id.as_str())
            .ok_or_else(|| ManagedError::new(ManagedErrorCode::ManifestInvalid))?;
        if !binding.status.is_active() {
            continue;
        }
        if let Some(active_org) = &manifest.active_org {
            if &skill.consumer_org != active_org {
                return Err(ManagedError::new(ManagedErrorCode::ManifestInvalid));
            }
        }
        let key = (skill.consumer_org.as_str(), skill.skill_name.as_str());
        if let Some((installation_id, source_org)) = active_names.get(&key) {
            if *installation_id != skill.installation_id && *source_org != skill.source_org {
                return Err(ManagedError::new(ManagedErrorCode::SkillSourceNameConflict));
            }
        } else {
            active_names.insert(
                key,
                (skill.installation_id.as_str(), skill.source_org.as_str()),
            );
        }
    }

    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn create_private_directory(path: &Path) -> Result<(), ManagedError> {
    fs::create_dir_all(path)
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_manifest_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(target_os = "windows")]
fn replace_manifest_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), ManagedError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| ManagedError::new(ManagedErrorCode::ManifestWriteFailed))
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), ManagedError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_skills::{
        errors::ManagedErrorCode, AgentId, BindingStatus, LinkKind, ManagedBinding, ManagedSkill,
        ManagedSkillOrigin, ManagedSkillStatus, UsageObservability,
    };
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    struct TestHome {
        path: PathBuf,
    }

    impl TestHome {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "skillreg-managed-manifest-{name}-{}",
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

    fn sample_skill(
        paths: &ManagedPaths,
        installation_id: &str,
        source_org: &str,
        skill_name: &str,
    ) -> ManagedSkill {
        ManagedSkill {
            installation_id: installation_id.to_string(),
            consumer_org: "acme".to_string(),
            source_org: source_org.to_string(),
            skill_id: Some(format!("skill-{installation_id}")),
            skill_name: skill_name.to_string(),
            origin: ManagedSkillOrigin::Registry,
            active_version: "1.4.2".to_string(),
            sha256: "a".repeat(64),
            content_path: paths
                .content_dir("acme", source_org, skill_name)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            content_hash: "b".repeat(64),
            status: ManagedSkillStatus::Ready,
            installed_at: "2026-07-29T00:00:00Z".to_string(),
            last_checked_at: None,
            last_updated_at: None,
            last_error: None,
            cleanup_dismissed_until: None,
        }
    }

    #[test]
    fn missing_manifest_creates_v2_with_stable_client_uuid() {
        let home = TestHome::new("missing");
        let paths = ManagedPaths::new(home.path.clone()).unwrap();

        let first = load_or_create_manifest(&paths).unwrap();
        let second = load_or_create_manifest(&paths).unwrap();

        assert_eq!(first.version, MANAGED_MANIFEST_VERSION);
        assert_eq!(first.client_instance_id, second.client_instance_id);
        Uuid::parse_str(&first.client_instance_id).unwrap();
        assert!(paths.manifest_path().is_file());
    }

    #[test]
    fn registry_origin_is_explicit_but_defaults_for_existing_v2_entries() {
        let home = TestHome::new("origin-default");
        let paths = ManagedPaths::new(home.path.clone()).unwrap();
        let skill = sample_skill(
            &paths,
            &Uuid::new_v4().to_string(),
            "publisher",
            "review-helper",
        );
        let mut serialized = serde_json::to_value(&skill).unwrap();

        assert_eq!(serialized["origin"], "registry");

        serialized.as_object_mut().unwrap().remove("origin");
        let restored: ManagedSkill = serde_json::from_value(serialized).unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap()["origin"],
            "registry"
        );
    }

    #[test]
    fn future_manifest_version_is_refused_without_rewrite() {
        let home = TestHome::new("future");
        let paths = ManagedPaths::new(home.path.clone()).unwrap();
        fs::create_dir_all(paths.skillreg_root()).unwrap();
        let original = r#"{"version":3,"activeOrg":null,"clientInstanceId":"future","migratedFromV1At":null,"skills":[],"bindings":[]}"#;
        fs::write(paths.manifest_path(), original).unwrap();

        let error = load_or_create_manifest(&paths).unwrap_err();

        assert_eq!(error.code(), ManagedErrorCode::ManifestInvalid);
        assert_eq!(fs::read_to_string(paths.manifest_path()).unwrap(), original);
    }

    #[test]
    fn corrupt_manifest_is_preserved_and_reported() {
        let home = TestHome::new("corrupt");
        let paths = ManagedPaths::new(home.path.clone()).unwrap();
        fs::create_dir_all(paths.skillreg_root()).unwrap();
        let original = b"{not-valid-json";
        fs::write(paths.manifest_path(), original).unwrap();

        let error = load_or_create_manifest(&paths).unwrap_err();

        assert_eq!(error.code(), ManagedErrorCode::ManifestInvalid);
        assert_eq!(fs::read(paths.manifest_path()).unwrap(), original);
    }

    #[test]
    fn atomic_write_uses_neighbor_temporary_file() {
        let home = TestHome::new("temporary");
        let paths = ManagedPaths::new(home.path.clone()).unwrap();
        let manifest = ManagedSkillsManifest::new();

        write_manifest_atomic(&paths, &manifest).unwrap();

        assert_eq!(
            manifest_temp_path(paths.manifest_path()),
            paths.skillreg_root().join("managed-skills.json.tmp")
        );
        assert!(!manifest_temp_path(paths.manifest_path()).exists());
    }

    #[test]
    fn failure_before_rename_preserves_previous_manifest() {
        let home = TestHome::new("rollback");
        let paths = ManagedPaths::new(home.path.clone()).unwrap();
        let original = ManagedSkillsManifest::new();
        write_manifest_atomic(&paths, &original).unwrap();
        fs::create_dir(manifest_temp_path(paths.manifest_path())).unwrap();
        let mut replacement = original.clone();
        replacement.active_org = Some("acme".to_string());

        let error = write_manifest_atomic(&paths, &replacement).unwrap_err();

        assert_eq!(error.code(), ManagedErrorCode::ManifestWriteFailed);
        let reloaded = read_manifest(&paths).unwrap();
        assert_eq!(reloaded.active_org, None);
        assert_eq!(reloaded.client_instance_id, original.client_instance_id);
    }

    #[test]
    fn two_sources_with_same_name_cannot_have_active_bindings() {
        let home = TestHome::new("collision");
        let paths = ManagedPaths::new(home.path.clone()).unwrap();
        let first_id = Uuid::new_v4().to_string();
        let second_id = Uuid::new_v4().to_string();
        let mut manifest = ManagedSkillsManifest::new();
        manifest.active_org = Some("acme".to_string());
        manifest.skills = vec![
            sample_skill(&paths, &first_id, "publisher-one", "review-helper"),
            sample_skill(&paths, &second_id, "publisher-two", "review-helper"),
        ];
        manifest.bindings = vec![
            ManagedBinding {
                installation_id: first_id,
                agent: AgentId::Claude,
                link_path: home
                    .path
                    .join(".claude/skills/review-helper")
                    .to_string_lossy()
                    .into_owned(),
                link_kind: LinkKind::Symlink,
                status: BindingStatus::Ready,
                last_checked_at: None,
                last_error: None,
                usage_observability: UsageObservability::Unavailable,
            },
            ManagedBinding {
                installation_id: second_id,
                agent: AgentId::Codex,
                link_path: home
                    .path
                    .join(".agents/skills/review-helper")
                    .to_string_lossy()
                    .into_owned(),
                link_kind: LinkKind::Symlink,
                status: BindingStatus::Ready,
                last_checked_at: None,
                last_error: None,
                usage_observability: UsageObservability::Unavailable,
            },
        ];

        let error = validate_manifest(&paths, &manifest).unwrap_err();

        assert_eq!(error.code(), ManagedErrorCode::SkillSourceNameConflict);
    }

    #[test]
    fn reading_v2_never_changes_legacy_installed_manifest() {
        let home = TestHome::new("legacy");
        let paths = ManagedPaths::new(home.path.clone()).unwrap();
        fs::create_dir_all(paths.skillreg_root()).unwrap();
        let legacy = br#"{"version":1,"installations":[{"name":"legacy"}]}"#;
        fs::write(paths.legacy_manifest_path(), legacy).unwrap();

        load_or_create_manifest(&paths).unwrap();

        assert_eq!(fs::read(paths.legacy_manifest_path()).unwrap(), legacy);
    }
}
