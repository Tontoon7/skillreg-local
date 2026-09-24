mod support;

use skillreg_local_lib::managed_skills::{errors::ManagedErrorCode, manifest::read_manifest};
use std::{fs, path::Path};
use support::{create_bindings, create_skill, lifecycle, write_manifest, TestHome};
use uuid::Uuid;

#[tokio::test]
async fn uninstall_removes_owned_bindings_and_content_but_preserves_environment_values() {
    let home = TestHome::new("uninstall");
    let paths = home.paths();
    let skill = create_skill(&paths, "acme", "publisher", "review-helper");
    let bindings = create_bindings(&paths, &skill);
    let env_file = paths.skillreg_root().join("env/acme/review-helper.env");
    fs::create_dir_all(env_file.parent().unwrap()).unwrap();
    fs::write(&env_file, "TOKEN=secret").unwrap();
    write_manifest(&paths, Some("acme"), vec![skill.clone()], bindings.clone());

    let result = lifecycle(&paths)
        .uninstall(&skill.installation_id)
        .await
        .unwrap();

    assert!(result.removed);
    assert!(!result.already_removed);
    assert_eq!(result.bindings_removed, bindings.len());
    assert!(result.env_values_preserved);
    assert!(!Path::new(&skill.content_path).exists());
    assert!(bindings
        .iter()
        .all(|binding| fs::symlink_metadata(&binding.link_path).is_err()));
    assert_eq!(fs::read_to_string(env_file).unwrap(), "TOKEN=secret");
    let manifest = read_manifest(&paths).unwrap();
    assert!(manifest.skills.is_empty());
    assert!(manifest.bindings.is_empty());
}

#[tokio::test]
async fn uninstall_is_idempotent_for_a_known_tombstone_but_refuses_an_unknown_id() {
    let home = TestHome::new("idempotent");
    let paths = home.paths();
    let skill = create_skill(&paths, "acme", "publisher", "review-helper");
    let bindings = create_bindings(&paths, &skill);
    write_manifest(&paths, Some("acme"), vec![skill.clone()], bindings);
    let service = lifecycle(&paths);

    service.uninstall(&skill.installation_id).await.unwrap();
    let second = service.uninstall(&skill.installation_id).await.unwrap();
    let unknown = service
        .uninstall(&Uuid::new_v4().to_string())
        .await
        .unwrap_err();

    assert!(second.already_removed);
    assert!(!second.removed);
    assert_eq!(
        unknown.code(),
        ManagedErrorCode::ManagedInstallationNotFound
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_binding_that_no_longer_points_to_owned_content_blocks_uninstall_without_data_loss() {
    let home = TestHome::new("binding-conflict");
    let paths = home.paths();
    let skill = create_skill(&paths, "acme", "publisher", "review-helper");
    let bindings = create_bindings(&paths, &skill);
    let binding = &bindings[0];
    fs::remove_file(&binding.link_path).unwrap();
    let unmanaged = home.path.join("unmanaged");
    fs::create_dir_all(&unmanaged).unwrap();
    std::os::unix::fs::symlink(&unmanaged, &binding.link_path).unwrap();
    write_manifest(&paths, Some("acme"), vec![skill.clone()], bindings.clone());

    let result = lifecycle(&paths)
        .uninstall(&skill.installation_id)
        .await
        .unwrap();

    assert!(!result.removed);
    assert_eq!(result.conflicts, 1);
    assert!(Path::new(&skill.content_path).join("SKILL.md").is_file());
    assert_eq!(
        fs::canonicalize(&binding.link_path).unwrap(),
        fs::canonicalize(unmanaged).unwrap()
    );
    assert_eq!(read_manifest(&paths).unwrap().skills.len(), 1);
}

#[tokio::test]
async fn repair_recreates_a_missing_owned_binding_without_redownloading_content() {
    let home = TestHome::new("repair-missing");
    let paths = home.paths();
    let skill = create_skill(&paths, "acme", "publisher", "review-helper");
    let bindings = create_bindings(&paths, &skill);
    for binding in &bindings {
        #[cfg(unix)]
        fs::remove_file(&binding.link_path).unwrap();
        #[cfg(target_os = "windows")]
        std::fs::remove_dir(&binding.link_path).unwrap();
    }
    write_manifest(&paths, Some("acme"), vec![skill.clone()], bindings.clone());
    let before = fs::read(Path::new(&skill.content_path).join("SKILL.md")).unwrap();

    let report = lifecycle(&paths)
        .repair_one(&skill.installation_id)
        .await
        .unwrap();

    assert_eq!(report.checked, 1);
    assert_eq!(report.repaired, bindings.len());
    assert_eq!(
        fs::read(Path::new(&skill.content_path).join("SKILL.md")).unwrap(),
        before
    );
    assert!(bindings
        .iter()
        .all(|binding| fs::canonicalize(&binding.link_path).is_ok()));
}

#[tokio::test]
async fn repair_connects_an_agent_detected_after_the_skill_was_installed() {
    let home = TestHome::new("repair-new-agent");
    let paths = home.paths();
    let skill = create_skill(&paths, "acme", "publisher", "review-helper");
    write_manifest(&paths, Some("acme"), vec![skill.clone()], vec![]);

    let report = lifecycle(&paths).repair_all().await.unwrap();
    let manifest = read_manifest(&paths).unwrap();

    assert_eq!(report.checked, 1);
    assert_eq!(report.repaired, 2);
    assert_eq!(manifest.bindings.len(), 2);
    assert!(manifest.bindings.iter().all(|binding| {
        binding.installation_id == skill.installation_id
            && fs::canonicalize(&binding.link_path).is_ok()
    }));
}

#[tokio::test]
async fn repair_blocks_modified_canonical_content() {
    let home = TestHome::new("repair-modified");
    let paths = home.paths();
    let skill = create_skill(&paths, "acme", "publisher", "review-helper");
    write_manifest(&paths, Some("acme"), vec![skill.clone()], vec![]);
    fs::write(
        Path::new(&skill.content_path).join("SKILL.md"),
        "locally modified",
    )
    .unwrap();

    let report = lifecycle(&paths)
        .repair_one(&skill.installation_id)
        .await
        .unwrap();

    assert_eq!(report.modified_content, 1);
    assert!(!home.path.join(".claude/skills/review-helper").exists());
    assert_eq!(
        fs::read_to_string(Path::new(&skill.content_path).join("SKILL.md")).unwrap(),
        "locally modified"
    );
}
