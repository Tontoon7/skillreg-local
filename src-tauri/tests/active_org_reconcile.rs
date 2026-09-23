mod support;

use skillreg_local_lib::managed_skills::{errors::ManagedErrorCode, manifest::read_manifest};
use std::{fs, path::Path};
use support::{create_bindings, create_skill, lifecycle, write_manifest, TestHome};

#[tokio::test]
async fn switching_organizations_unlinks_the_old_org_and_reuses_verified_cached_content() {
    let home = TestHome::new("org-switch");
    let paths = home.paths();
    let skill_a = create_skill(&paths, "acme", "publisher", "review-helper");
    let skill_b = create_skill(&paths, "globex", "publisher", "review-helper");
    let bindings_a = create_bindings(&paths, &skill_a);
    write_manifest(
        &paths,
        Some("acme"),
        vec![skill_a.clone(), skill_b.clone()],
        bindings_a,
    );
    let service = lifecycle(&paths);

    let switched = service
        .switch_active_org("globex", &["acme".to_string(), "globex".to_string()])
        .await
        .unwrap();

    assert_eq!(switched.previous_org.as_deref(), Some("acme"));
    assert_eq!(switched.active_org, "globex");
    assert_eq!(
        fs::canonicalize(home.path.join(".claude/skills/review-helper")).unwrap(),
        fs::canonicalize(&skill_b.content_path).unwrap()
    );
    assert!(Path::new(&skill_a.content_path).join("SKILL.md").is_file());
    assert_eq!(
        read_manifest(&paths).unwrap().active_org.as_deref(),
        Some("globex")
    );

    service
        .switch_active_org("acme", &["acme".to_string(), "globex".to_string()])
        .await
        .unwrap();
    assert_eq!(
        fs::canonicalize(home.path.join(".claude/skills/review-helper")).unwrap(),
        fs::canonicalize(&skill_a.content_path).unwrap()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_target_org_conflict_rolls_back_without_leaving_mixed_bindings() {
    let home = TestHome::new("org-conflict");
    let paths = home.paths();
    let skill_a = create_skill(&paths, "acme", "publisher", "review-helper");
    let skill_b = create_skill(&paths, "globex", "publisher", "globex-helper");
    let bindings_a = create_bindings(&paths, &skill_a);
    write_manifest(
        &paths,
        Some("acme"),
        vec![skill_a.clone(), skill_b],
        bindings_a,
    );
    let conflict = home.path.join(".claude/skills/globex-helper");
    fs::create_dir_all(&conflict).unwrap();
    fs::write(conflict.join("keep.txt"), "keep").unwrap();

    let error = lifecycle(&paths)
        .switch_active_org("globex", &["acme".to_string(), "globex".to_string()])
        .await
        .unwrap_err();

    assert_eq!(error.code(), ManagedErrorCode::BindingConflict);
    assert_eq!(
        fs::canonicalize(home.path.join(".claude/skills/review-helper")).unwrap(),
        fs::canonicalize(&skill_a.content_path).unwrap()
    );
    assert_eq!(
        fs::read_to_string(conflict.join("keep.txt")).unwrap(),
        "keep"
    );
    assert_eq!(
        read_manifest(&paths).unwrap().active_org.as_deref(),
        Some("acme")
    );
}

#[tokio::test]
async fn an_unauthorized_org_is_rejected_before_mutation() {
    let home = TestHome::new("unauthorized");
    let paths = home.paths();
    let skill = create_skill(&paths, "acme", "publisher", "review-helper");
    let bindings = create_bindings(&paths, &skill);
    write_manifest(&paths, Some("acme"), vec![skill.clone()], bindings);

    let error = lifecycle(&paths)
        .switch_active_org("globex", &["acme".to_string()])
        .await
        .unwrap_err();

    assert_eq!(
        error.code(),
        ManagedErrorCode::ActiveOrganizationUnauthorized
    );
    assert_eq!(
        fs::canonicalize(home.path.join(".claude/skills/review-helper")).unwrap(),
        fs::canonicalize(&skill.content_path).unwrap()
    );
}
