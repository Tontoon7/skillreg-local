mod support;

use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use skillreg_local_lib::managed_skills::{
    errors::{ManagedError, ManagedErrorCode},
    manifest::read_manifest,
    migration::{ManagedMigrationService, TrackedInstallation},
    paths::ManagedPaths,
    platform_links::{LinkInspection, PlatformLinker, SystemPlatformLinker},
    service::{
        FileManifestStore, ManagedDownloadMetadata, ManagedInstallRequest, ManagedRegistryClient,
        ManagedRegistryFuture, ManagedRegistryRequest, ManagedSkillService, ManagedTarget,
        ManagedTargetPolicy,
    },
};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};
use tar::{Builder, Header};
use uuid::Uuid;

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone)]
struct OfflineRegistry(Arc<Mutex<(ManagedTarget, Vec<u8>)>>);

impl OfflineRegistry {
    fn new(version: &str) -> Self {
        let content = format!("---\nname: review-helper\ndescription: Release {version}\n---\n");
        let mut archive = Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
        let mut header = Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(
                &mut header,
                "review-helper/SKILL.md",
                Cursor::new(content.as_bytes()),
            )
            .unwrap();
        let bytes = archive.into_inner().unwrap().finish().unwrap();
        let target = ManagedTarget {
            skill_id: "fixture-review".to_string(),
            source_org: "publisher".to_string(),
            consumer_org: "acme".to_string(),
            skill_name: "review-helper".to_string(),
            resolved_version: version.to_string(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            validation_level: "verified".to_string(),
            policy: ManagedTargetPolicy {
                mode: "latest_approved".to_string(),
                version: None,
            },
        };
        Self(Arc::new(Mutex::new((target, bytes))))
    }
}

impl ManagedRegistryClient for OfflineRegistry {
    fn resolve_target<'a>(
        &'a self,
        _request: &'a ManagedRegistryRequest,
    ) -> ManagedRegistryFuture<'a, ManagedTarget> {
        let target = self.0.lock().unwrap().0.clone();
        Box::pin(async move { Ok(target) })
    }

    fn download_to<'a>(
        &'a self,
        _request: &'a ManagedRegistryRequest,
        destination: &'a Path,
    ) -> ManagedRegistryFuture<'a, ManagedDownloadMetadata> {
        let (target, bytes) = self.0.lock().unwrap().clone();
        Box::pin(async move {
            fs::write(destination, bytes)
                .map_err(|_| ManagedError::new(ManagedErrorCode::RegistryRequestFailed))?;
            Ok(ManagedDownloadMetadata::from_target(&target))
        })
    }
}

fn invoke_tool(command: &str, home: &Path, backup: &Path, recovery: Option<&Path>) {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/managed-skills-backup-restore.mjs");
    let mut node = Command::new("node");
    node.arg(script)
        .arg(command)
        .arg("--home")
        .arg(home)
        .arg("--backup")
        .arg(backup);
    if let Some(recovery) = recovery {
        node.arg("--apply").arg("--recovery").arg(recovery);
    }
    let output = node
        .output()
        .expect("Node is required for the rollback tool");
    assert!(
        output.status.success(),
        "rollback tool failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_skill(path: &Path, content: &str) {
    fs::create_dir_all(path).unwrap();
    fs::write(path.join("SKILL.md"), content).unwrap();
}

#[tokio::test]
async fn real_migration_and_update_restore_the_complete_legacy_profile_using_the_support_tool() {
    let fixture = Fixture(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(format!(".rollback-rust-fixture-{}", Uuid::new_v4())),
    );
    fs::create_dir(&fixture.0).unwrap();
    let root = fs::canonicalize(&fixture.0).unwrap();
    let home = root.join("profil été");
    fs::create_dir(&home).unwrap();
    let backup = root.join("before");
    let recovery = root.join("after");
    let paths = ManagedPaths::new(home.clone()).unwrap();
    let claude = home.join(".claude/skills/review-helper");
    let codex = home.join(".agents/skills/review-helper");
    let modified = home.join(".claude/skills/modified");
    let project = home.join("project/.claude/skills/project-helper");
    let external = root.join("external");
    let legacy_content = "---\nname: review-helper\ndescription: Legacy fixture\n---\n";
    write_skill(&claude, legacy_content);
    write_skill(&codex, legacy_content);
    write_skill(&modified, "preserve modified legacy content");
    write_skill(&project, "preserve project content");
    write_skill(&external, "preserve external content");
    let external_link = home.join(".agents/skills/external");
    SystemPlatformLinker::current()
        .create_dir_link(&external, &external_link)
        .unwrap();
    let external_link_state = SystemPlatformLinker::current()
        .inspect(&external_link)
        .unwrap();
    fs::create_dir_all(paths.skillreg_root().join("env")).unwrap();
    let config = br#"{"token":"fixture-access","defaultOrg":"acme"}"#;
    let env = br#"{"FIXTURE_ACCESS":"offline-memory-equivalent"}"#;
    fs::write(paths.skillreg_root().join("config.json"), config).unwrap();
    fs::write(paths.skillreg_root().join("env/acme.json"), env).unwrap();
    let hash = format!("{:x}", Sha256::digest(legacy_content.as_bytes()));
    let entry = |name: &str, agent: &str, scope: &str, path: &Path, content_hash: &str| {
        serde_json::json!({
            "org": "acme", "name": name, "version": "1.0.0", "agent": agent,
            "scope": scope, "projectDir": if scope == "project" { Some(home.join("project")) } else { None },
            "installPath": path, "contentHash": content_hash,
            "sourceOrg": "publisher", "sha256": "a".repeat(64)
        })
    };
    let legacy = serde_json::to_vec_pretty(&serde_json::json!({
        "version": 1,
        "installations": [
            entry("review-helper", "claude", "user", &claude, &hash),
            entry("review-helper", "codex", "user", &codex, &hash),
            entry("modified", "claude", "user", &modified, "old-unmodified-hash"),
            entry("project-helper", "claude", "project", &project, "project-hash")
        ]
    }))
    .unwrap();
    fs::write(paths.legacy_manifest_path(), &legacy).unwrap();
    invoke_tool("snapshot", &home, &backup, None);

    let client = OfflineRegistry::new("2.0.0");
    let service = || {
        ManagedSkillService::new(
            client.clone(),
            support::registry(),
            SystemPlatformLinker::current(),
            FileManifestStore,
            paths.clone(),
        )
    };
    let migration = ManagedMigrationService::new(
        service(),
        support::registry(),
        SystemPlatformLinker::current(),
        paths.clone(),
    );
    let migrated = migration.run(true).await.unwrap();
    assert_eq!(migrated.migrated, 1);
    assert_eq!(migrated.errors, 0);
    assert!(matches!(
        SystemPlatformLinker::current().inspect(&claude).unwrap(),
        LinkInspection::Link {
            target_exists: true,
            ..
        }
    ));
    *client.0.lock().unwrap() = OfflineRegistry::new("3.0.0").0.lock().unwrap().clone();
    let updated = service()
        .install(ManagedInstallRequest {
            consumer_org: "acme".to_string(),
            source_org: "publisher".to_string(),
            name: "review-helper".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(updated.installation.active_version, "3.0.0");
    assert_eq!(
        read_manifest(&paths).unwrap().skills[0].active_version,
        "3.0.0"
    );
    fs::write(
        paths.skillreg_root().join("config.json"),
        "later-fixture-access",
    )
    .unwrap();
    fs::write(
        paths.skillreg_root().join("later.txt"),
        "keep post-migration addition",
    )
    .unwrap();
    drop(migration);

    invoke_tool("verify", &home, &backup, None);
    invoke_tool("restore", &home, &backup, None);
    assert!(paths.manifest_path().exists());
    invoke_tool("restore", &home, &backup, Some(&recovery));
    assert_eq!(fs::read(paths.legacy_manifest_path()).unwrap(), legacy);
    assert!(!paths.manifest_path().exists());
    assert_eq!(
        fs::read(paths.skillreg_root().join("config.json")).unwrap(),
        config
    );
    assert_eq!(
        fs::read(paths.skillreg_root().join("env/acme.json")).unwrap(),
        env
    );
    for path in [&claude, &codex] {
        assert_eq!(
            SystemPlatformLinker::current().inspect(path).unwrap(),
            LinkInspection::Other
        );
        assert_eq!(
            fs::read_to_string(path.join("SKILL.md")).unwrap(),
            legacy_content
        );
    }
    for (path, content) in [
        (&modified, "preserve modified legacy content"),
        (&project, "preserve project content"),
        (&external, "preserve external content"),
    ] {
        assert_eq!(fs::read_to_string(path.join("SKILL.md")).unwrap(), content);
    }
    assert_eq!(
        SystemPlatformLinker::current()
            .inspect(&external_link)
            .unwrap(),
        external_link_state
    );
    assert_eq!(
        fs::read_to_string(recovery.join("data/.skillreg/later.txt")).unwrap(),
        "keep post-migration addition"
    );
    assert_eq!(
        fs::read_to_string(recovery.join("data/.skillreg/config.json")).unwrap(),
        "later-fixture-access"
    );
    let restored_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(paths.legacy_manifest_path()).unwrap()).unwrap();
    let restored_entries: Vec<TrackedInstallation> =
        serde_json::from_value(restored_manifest["installations"].clone()).unwrap();
    assert_eq!(restored_entries.len(), 4);
    assert_eq!(restored_entries[0].version, "1.0.0");
    let verified = root.join("verified");
    invoke_tool("snapshot", &home, &verified, None);
    let before: serde_json::Value =
        serde_json::from_slice(&fs::read(backup.join("inventory.json")).unwrap()).unwrap();
    let after: serde_json::Value =
        serde_json::from_slice(&fs::read(verified.join("inventory.json")).unwrap()).unwrap();
    assert_eq!(before["entries"], after["entries"]);
}
