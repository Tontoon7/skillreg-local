# Réconciliation de la PR #7 avec le candidat #12

**Date :** 2026-10-03
**PR #7** (brouillon, Phase 7 lot A) : head `6d18e9e01ef293695029c336281e7f47cc75cacc`, branche
`feat/phase-7-lot-a-managed-skills` (`bbfbe94`), sauvegarde `refs/kairia-review/20261001/pr-7-6d18e9e0`.
**PR #12** (candidat de livraison) : head `595e849286e64e1d20698a8decf083946d4db38d`, branche
`factory/2-20260924T115436-e1d880`, sauvegarde `refs/kairia-review/20261001/pr-12-595e8492`.

Ce document donne une décision par commit, par fichier et par assertion modifiée de la PR #7 :
**intégré** dans #12 (commit cité), **porté** par ce changement (fichier cité), ou
**remplacé/refusé** (raison technique). Une ressemblance de titre ne vaut pas intégration.
Il ne coche aucune case de release et ne lève pas le NO-GO de la
[checklist](plans/2026-07-29-managed-skills-release-checklist.md). La branche qui porte ce
document descend de `595e849` (fusion sans conflit de #12, aucun squash ni rebase) : la checklist,
le registre de validation et le code cités ici s'y trouvent.

## 1. Topologie

- `6d18e9e` = merge de `bbfbe94` (sept commits du lot A au-dessus de `3e024f3`) et de main
  `793ba1b`. Le côté main n'apporte que la PR #10 (publication de commandes) : les fichiers que
  #7 ne touche pas y sont identiques à `793ba1b`.
- `595e849` est linéaire au-dessus de `793ba1b` (24 commits) : ni `6d18e9e` ni `bbfbe94` ne sont
  ses ancêtres (`git merge-base --is-ancestor` négatif). #12 a repris le lot A par patchs
  ciblés (`ca96678`, provenance `bbfbe94` citée dans la checklist), puis l'a durci.
- Comparaison de référence : `git diff 6d18e9e 595e849`. Tous les fichiers de code de #7 existent
  dans #12 ; seuls `PRODUCT.md`, `DESIGN.md` et trois plans du 2026-07-29 en sont absents.

## 2. Décision par commit

| Commit #7 | Contenu | Décision |
| --- | --- | --- |
| `798d558` | Moteur Rust des skills gérées, 36 fichiers | **Intégré** par `ca96678`, durci par `c630e3a` (junctions), `ba78f87` (cibles Windows canoniques), `af35290`/`d54416a` (CI, revue). Zones : section 3. |
| `baaae04` | Parcours collaborateur React et tests Vitest | **Intégré** par `ca96678`, avec dialogues natifs et accessibilité (section 4, assertion 5). 33 de ses 47 fichiers sont identiques entre `6d18e9e` et `595e849`. |
| `714f5e1` | CI desktop | **Remplacé** par `.github/workflows/ci.yml` de #12 (`ca96678`, `ba78f87`) : matrice `ubuntu-22.04`/`macos-14`/`windows-2022` qui lance aussi format, build, Node et Vitest sur chaque OS, sur-ensemble des jobs #7. |
| `e97091b` | Documentation Phase 7 | **Partiellement intégré** : `docs/agent-compatibility.md`, `docs/managed-skills-user-guide.md`, la checklist et `DEV-PLAN.md` réécrits par #12. **Porté ici** : `PRODUCT.md`, `DESIGN.md`, les trois plans, la Phase 7 de `ROADMAP.md`, quatre consignes de rollback (section 5). |
| `9d29e9a` | Guide des agents | **Remplacé** par `AGENTS.md` de #12 ; **porté ici** : références à `PRODUCT.md`/`DESIGN.md` et rôle réel de `main.tsx`. |
| `991f8e9` | Tests unitaires selon la plateforme | **Intégré** par `ca96678` (`paths.rs` identique), avec restriction arm64 (assertion 1). |
| `bbfbe94` | `cargo test --no-fail-fast` en CI | **Intégré**, avec `--locked` en plus (`ca96678`, `af35290`). |
| `6d18e9e` | Merge de main | **Sans delta propre** : PR #10 déjà dans `793ba1b`, base de #12. |

## 3. Décision par zone fonctionnelle

Les seules lignes de #7 absentes de #12 sont des remplacements par un durcissement. Aucune n'est
réintroduite : revenir à la version #7 ferait régresser une protection.

| Zone | Fichiers #7 | Remplacement dans #12 |
| --- | --- | --- |
| Migrations | `managed_skills/migration.rs`, `commands/managed_migration.rs` (identique), `tests/managed_migration.rs` | `is_symlink()` → `metadata_is_link_like` (reparse points Windows) ; suppression de test via `linker.remove_link` (`ca96678`). |
| Import local | `managed_skills/local_import.rs`, `commands/local_import.rs` (identique), `tests/local_skill_import.rs` | Liens détectés par `metadata_is_link_like`, sauvegarde liée retirée par le linker et non par `remove_file` ; tests multiplateformes (`ca96678`). |
| Liens et junctions | `platform_links.rs`, `bindings.rs`, `reconcile.rs`, `archive.rs` | Classification des reparse points, `fs::remove_dir` après `junction::delete` (`c630e3a`), `same_target_path` pour les cibles canoniques (`ba78f87`). |
| Installation transactionnelle | `service.rs`, `manifest.rs`, `tests/managed_installation.rs` | Reprise d'un swap interrompu prouvée par le hash du manifeste committé, rollback seulement si l'ancien état est prouvé, temporaire `create_new` préservé, racine liée refusée, verrou global partagé (`acquire_managed_mutation_lock`) et `ensure_legacy_mutation_allowed` dans `commands/skills.rs` (`ca96678`). |
| Config et droits | `commands/config.rs`, `manifest.rs` | ACL Windows privées (`permissions.rs`, `restrict_to_current_user`), temporaire possédé seulement (`ca96678`). |
| Erreurs | `errors.rs` | `with_parameter` ne conserve qu'un `status` HTTP 100–599 ; #7 n'utilisait que `status`, donc aucun paramètre utile n'est perdu (`ca96678`). |
| Adaptateurs | `agents/claude.rs`, `codex.rs`, `cursor.rs`, `mod.rs` | Liens gérés limités à macOS arm64 (`cfg!(target_arch = "aarch64")`) faute de preuve runtime macOS x64 (`docs/agent-compatibility.md`). |
| UI | `EnvVarSetupDialog.tsx`, `UninstallManagedSkillDialog.tsx`, `Dashboard.tsx`, `Installed.tsx`, `AutoUpdateControl.tsx`, `RequiredActions.tsx`, `Setup.tsx`, `main.tsx` | `<dialog>` modal natif (`showModal`), état `busy` partagé, `aria-busy`, `role="alert"`, `aria-controls`, journalisation globale expurgée dans `main.tsx` (`ca96678`). |
| Tests frontend | `src/**/__tests__/`, `src/test/`, `vitest.config.ts` | Intégrés ; tests de dialogue et de journalisation ajoutés. `package.json` : `test:node` garde le reporter `spec`. |
| CI | `.github/workflows/ci.yml` | Matrice trois OS (section 2). |

## 4. Assertions modifiées et couverture de remplacement

1. `agents/mod.rs` : `assert!(supports_managed_links(Macos))` devient une égalité avec
   `cfg!(target_arch = "aarch64")`. L'assertion reste stricte sur arm64 et vérifie en plus le refus
   sur x64, non qualifié en runtime.
2. `tests/managed_bindings.rs` et `tests/local_skill_import.rs` : les blocs `#[cfg(unix)]` avec
   `std::os::unix::fs::symlink` deviennent `create_test_link`. Mêmes assertions, étendues aux
   junctions Windows.
3. `tests/managed_installation.rs` (`ToggleInspectLinker`) : `Platform::Macos`/`LinkKind::Symlink`
   codés en dur deviennent `SystemPlatformLinker::current()`. Le test s'exécute avec le linker réel
   de chaque OS de la matrice.
4. `tests/managed_migration.rs` : `fs::remove_file` devient `linker.remove_link(…)`. Une junction
   n'est pas un fichier ; le test emprunte le chemin du produit.
5. `EnvVarSetupDialog` et `UninstallManagedSkillDialog` : la fermeture au clic sur le fond et
   l'écouteur Échap global sont remplacés par le `<dialog>` modal natif, annulation bloquée pendant
   `saving`/`busy` et focus restauré. Couverture : `EnvVarSetupDialog.test.tsx` et
   `UninstallManagedSkillDialog.test.tsx`. La fermeture au clic sur le fond est **refusée** :
   sémantique modale et pas de perte de saisie.
6. `src-tauri/tests/fixtures/agents/README.md` : la preuve runtime se consigne désormais dans
   `docs/managed-skills-release-validation.md` (N2) au lieu de `docs/agent-compatibility.md`.

Aucune assertion frontend de #7 n'est supprimée : la seule ligne retirée des tests frontend est
l'import de `Installed.test.tsx`, étendu à `fireEvent` pour un nouveau test de restauration du
focus.

## 5. Delta porté par ce changement

| Élément #7 | Destination | Ajustement |
| --- | --- | --- |
| `PRODUCT.md` | `PRODUCT.md` | Les documents liés `../skillreg-app/...` sont signalés externes. |
| `DESIGN.md` | `DESIGN.md` | Section « Light mode » corrigée : ni sélecteur de thème ni préférence persistée, mode clair non implémenté. |
| Shape brief | `docs/plans/2026-07-29-non-technical-employee-experience-shape-brief.md` | Aucun (document daté). |
| Design import local | `docs/plans/2026-07-29-dashboard-local-skill-import-design.md` | Sauts de ligne de l'en-tête écrits `\` au lieu de deux espaces finales (même rendu). |
| Plan import local | `docs/plans/2026-07-29-dashboard-local-skill-import-implementation-plan.md` | Aucun ; les fichiers `src/` cités existent dans #12. |
| Phase 7, lot B, backlog | `ROADMAP.md` | `[x]` limité à ce que code et tests du candidat prouvent ; chemins agents, 900×600, dogfood, CLI, packaging et campagnes natives en `[~]` avec renvoi à N2–N7. |
| Libellés Phases 2 et 3 | `ROADMAP.md`, `DEV-PLAN.md` | « livré, puis remplacé/retiré en Phase 7 ». |
| Ligne Phase 7 du §11 | `DEV-PLAN.md` | « implémentée, NO-GO de release : voir §14 », au lieu de « matrice cross-platform à terminer ». |
| Rollback dogfood, étapes 1, 4, 5, 6 | Checklist, « Rollback d'une release » | Consignes d'incident ajoutées sans toucher aux cases. Les étapes 2, 3 et 7 sont déjà couvertes par N4 (fermeture, sauvegarde intégrale, binaire legacy identifié). |

`DEV-PLAN.md` et la checklist n'existent sous leur forme Phase 7 que dans #12 : leurs retouches
sont appliquées ici, au-dessus de `595e849`. Elles se limitent aux lignes citées ci-dessus, plus
les rôles réels de `main.tsx` et `Settings.tsx` dans l'arborescence et un lien vers ce document.

## 6. Preuves réutilisées et à rejouer

- **N1** : CI PASS sur `579ed8088eb5264fd0a9fa6916375ac7c80d95aa`, run `36735884312`
  (`macos-14`, `ubuntu-22.04`, `windows-2022`, Node Windows 61/61). `git diff 579ed80 595e849`
  ne touche que la checklist et le registre : la preuve vaut pour le code de `595e849`.
- Ce changement n'ajoute que de la documentation Markdown. La CI sur le head livré reste à
  exécuter.
- Détail des preuves A1–A5 et N1–N7 : [registre de validation](managed-skills-release-validation.md).

## 7. Points ouverts

- N2 runtime des agents, N3 accessibilité et 900×600, N4 downgrade avec binaire legacy identifié,
  N5 ACL Windows en compte standard, N6 app/CLI/site, N7 signatures, notarisation, updater et
  distribution : **en attente**, aucune case cochée.
- Décision **NO-GO dogfood et NO-GO activation générale** maintenue ; aucune version, aucun tag,
  aucune activation d'adaptateur non qualifié (macOS x64, Windows, Linux).
- Les incidents #9 et #11 sont hors périmètre : ce ne sont pas des fonctionnalités de #7.
- PR #7 et PR #12 restent ouvertes. Une fermeture de #7 pour remplacement revient à Axel et doit
  citer le commit livré qui contient à la fois `595e849` et ce document ; tant qu'il n'existe pas,
  #7 reste ouverte.
