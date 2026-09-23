#![cfg(unix)]

mod support;

use skillreg_local_lib::managed_skills::{
    errors::ManagedErrorCode,
    local_import::{LocalImportClassification, LocalSkillImportService},
    manifest::read_manifest,
    platform_links::{LinkInspection, PlatformLinker, SystemPlatformLinker},
    LinkKind, ManagedSkillOrigin,
};
use std::{fs, path::Path};
use support::{create_bindings, create_skill, registry, write_manifest, TestHome};

fn write_skill(path: &Path, name: &str, body: &str) {
    fs::create_dir_all(path).unwrap();
    fs::write(
        path.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Local test skill\n---\n\n{body}\n"),
    )
    .unwrap();
}

#[test]
fn preview_accepts_skills_with_multiline_descriptions() {
    let home = TestHome::new("local-import-multiline-description");
    let paths = home.paths();
    let skill = home.path.join(".claude/skills/review-helper");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: review-helper\ndescription: >\n  Review changes and explain\n  the result in plain language.\n---\n",
    )
    .unwrap();
    let service = LocalSkillImportService::new(registry(), SystemPlatformLinker::current(), paths);

    let preview = service.preview("acme").unwrap();

    assert_eq!(preview.importable, 1);
    assert_eq!(preview.invalid_untouched, 0);
}

#[test]
fn preview_accepts_legacy_display_names_without_rewriting_the_skill() {
    let home = TestHome::new("local-import-legacy-display-name");
    let paths = home.paths();
    let skill = home.path.join(".claude/skills/kairia-finance");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: Kairia Finance Manager\ndescription: Manage Kairia finances\n---\n",
    )
    .unwrap();
    let service = LocalSkillImportService::new(registry(), SystemPlatformLinker::current(), paths);

    let preview = service.preview("acme").unwrap();

    assert_eq!(preview.importable, 1);
    assert_eq!(preview.invalid_untouched, 0);
    assert_eq!(preview.items[0].skill_name, "kairia-finance");
}

#[test]
fn preview_groups_local_copies_without_mutating_the_filesystem() {
    let home = TestHome::new("local-import-preview");
    let paths = home.paths();
    let claude = home.path.join(".claude/skills");
    let agents = home.path.join(".agents/skills");

    let review = claude.join("review-helper");
    write_skill(&review, "review-helper", "Review");
    fs::create_dir_all(&agents).unwrap();
    std::os::unix::fs::symlink(&review, agents.join("review-helper")).unwrap();

    write_skill(&claude.join("shared-skill"), "shared-skill", "Shared");
    write_skill(&agents.join("shared-skill"), "shared-skill", "Shared");

    write_skill(
        &claude.join("conflict-skill"),
        "conflict-skill",
        "Claude copy",
    );
    write_skill(
        &agents.join("conflict-skill"),
        "conflict-skill",
        "Codex copy",
    );

    let external = home.path.join("external/decktype");
    write_skill(&external, "decktype", "External");
    std::os::unix::fs::symlink(&external, claude.join("decktype")).unwrap();

    let managed = create_skill(&paths, "acme", "publisher", "managed-skill");
    let managed_bindings = create_bindings(&paths, &managed);
    write_manifest(&paths, Some("acme"), vec![managed], managed_bindings);

    let review_before = fs::read(review.join("SKILL.md")).unwrap();
    let alias_before = fs::read_link(agents.join("review-helper")).unwrap();
    let external_before = fs::read_link(claude.join("decktype")).unwrap();
    let service = LocalSkillImportService::new(registry(), SystemPlatformLinker::current(), paths);

    let preview = service.preview("acme").unwrap();

    assert_eq!(preview.importable, 2);
    assert_eq!(preview.conflicts, 1);
    assert_eq!(preview.external_links_untouched, 1);
    assert_eq!(preview.already_managed, 1);
    assert!(preview.items.iter().any(|item| {
        item.skill_name == "review-helper"
            && item.classification == LocalImportClassification::Importable
    }));
    assert!(preview.items.iter().any(|item| {
        item.skill_name == "shared-skill"
            && item.classification == LocalImportClassification::DuplicateIdentical
    }));
    assert!(preview.items.iter().any(|item| {
        item.skill_name == "conflict-skill"
            && item.classification == LocalImportClassification::DuplicateDivergent
    }));
    assert_eq!(fs::read(review.join("SKILL.md")).unwrap(), review_before);
    assert_eq!(
        fs::read_link(agents.join("review-helper")).unwrap(),
        alias_before
    );
    assert_eq!(
        fs::read_link(claude.join("decktype")).unwrap(),
        external_before
    );
}

#[tokio::test]
async fn confirmed_import_creates_one_canonical_copy_and_managed_links_per_skill() {
    let home = TestHome::new("local-import-success");
    let paths = home.paths();
    let claude = home.path.join(".claude/skills");
    let agents = home.path.join(".agents/skills");
    write_skill(&claude.join("review-helper"), "review-helper", "Review");
    write_skill(&claude.join("shared-skill"), "shared-skill", "Shared");
    write_skill(&agents.join("shared-skill"), "shared-skill", "Shared");
    let external = home.path.join("external/decktype");
    write_skill(&external, "decktype", "External");
    fs::create_dir_all(&claude).unwrap();
    std::os::unix::fs::symlink(&external, claude.join("decktype")).unwrap();
    write_skill(&claude.join("conflict-skill"), "conflict-skill", "Claude");
    write_skill(&agents.join("conflict-skill"), "conflict-skill", "Codex");
    let external_target_before = fs::read_link(claude.join("decktype")).unwrap();
    let conflict_before = fs::read(claude.join("conflict-skill/SKILL.md")).unwrap();
    let service =
        LocalSkillImportService::new(registry(), SystemPlatformLinker::current(), paths.clone());

    let report = service.run("acme", true).await.unwrap();

    assert!(report.confirmed);
    assert_eq!(report.imported, 2);
    assert_eq!(report.conflicts, 1);
    assert_eq!(report.errors, 0);
    let manifest = read_manifest(&paths).unwrap();
    let local_skills = manifest
        .skills
        .iter()
        .filter(|skill| skill.origin == ManagedSkillOrigin::Local)
        .collect::<Vec<_>>();
    assert_eq!(local_skills.len(), 2);
    for name in ["review-helper", "shared-skill"] {
        let installation = local_skills
            .iter()
            .find(|skill| skill.skill_name == name)
            .unwrap();
        let canonical = fs::canonicalize(&installation.content_path).unwrap();
        assert_eq!(fs::canonicalize(claude.join(name)).unwrap(), canonical);
        assert_eq!(fs::canonicalize(agents.join(name)).unwrap(), canonical);
    }
    assert_eq!(
        fs::read_link(claude.join("decktype")).unwrap(),
        external_target_before
    );
    assert_eq!(
        fs::read(claude.join("conflict-skill/SKILL.md")).unwrap(),
        conflict_before
    );

    let repeated = service.run("acme", true).await.unwrap();
    assert_eq!(repeated.imported, 0);
    assert_eq!(read_manifest(&paths).unwrap().skills.len(), 2);
}

#[derive(Clone, Copy)]
struct FailingCreateLinker;

impl PlatformLinker for FailingCreateLinker {
    fn platform(&self) -> skillreg_local_lib::managed_skills::agents::Platform {
        SystemPlatformLinker::current().platform()
    }

    fn link_kind(&self) -> LinkKind {
        SystemPlatformLinker::current().link_kind()
    }

    fn validate_paths(&self, target: &Path, link: &Path) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().validate_paths(target, link)
    }

    fn inspect(&self, link: &Path) -> Result<LinkInspection, ManagedErrorCode> {
        SystemPlatformLinker::current().inspect(link)
    }

    fn create_dir_link(&self, _target: &Path, _link: &Path) -> Result<(), ManagedErrorCode> {
        Err(ManagedErrorCode::BindingCreateFailed)
    }

    fn rename_link(&self, source: &Path, destination: &Path) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().rename_link(source, destination)
    }

    fn remove_link(&self, link: &Path, kind: LinkKind) -> Result<(), ManagedErrorCode> {
        SystemPlatformLinker::current().remove_link(link, kind)
    }
}

#[tokio::test]
async fn failed_binding_creation_restores_the_original_directory() {
    let home = TestHome::new("local-import-rollback");
    let paths = home.paths();
    let original = home.path.join(".claude/skills/review-helper");
    write_skill(&original, "review-helper", "Original");
    let original_content = fs::read(original.join("SKILL.md")).unwrap();
    let service = LocalSkillImportService::new(registry(), FailingCreateLinker, paths.clone());

    let report = service.run("acme", true).await.unwrap();

    assert_eq!(report.imported, 0);
    assert_eq!(report.errors, 1);
    assert!(!fs::symlink_metadata(&original)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::read(original.join("SKILL.md")).unwrap(),
        original_content
    );
    assert!(!paths
        .content_dir("acme", "local", "review-helper")
        .unwrap()
        .exists());
    assert!(read_manifest(&paths).unwrap().skills.is_empty());
}
