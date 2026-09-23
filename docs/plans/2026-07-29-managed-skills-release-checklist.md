# Checklist de release — expérience de skills gérées

**Date :** 2026-07-29  
**Périmètre :** lot A de l’expérience collaborateur métier  
**Version desktop de travail :** 0.3.25  
**Statut actuel :** **NO-GO pour une activation générale** ; prêt pour validation locale et
dogfood macOS après passage de toutes les gates automatisées.

Ce document est la gate de release du stockage canonique, des bindings gérés, de l’installation
en un clic, des mises à jour globales, de la migration v1, de la désinstallation, de la réparation
et du changement d’organisation. Les compteurs d’usage, suggestions de nettoyage et vues
administrateur appartiennent au lot B et sont explicitement hors périmètre.

## 1. Décision de transition

### Mode de déploiement retenu

La transition est pilotée par **cohortes de distribution du binaire**, pas par un flag local
incomplet :

- le parcours desktop courant utilise le modèle `managed` pour les nouvelles installations ;
- une installation v1 n’est migrée qu’après aperçu et confirmation explicite ;
- le lecteur v1 reste présent et les installations projet, externes, modifiées ou conflictuelles
  restent intactes ;
- le CLI ne mutile jamais le manifest v2 et refuse les installations utilisateur concurrentes ;
- aucun faux mode `legacy` ne doit être ajouté tant que l’ancienne UI et un vrai chemin de
  downgrade ne peuvent pas être restaurés ensemble.

Un simple enum `legacy | shadow | managed` dans `config.json` donnerait une fausse garantie :
l’interface collaborateur legacy n’existe plus dans cette version. Le rollout se fait donc par
builds signés réservés aux cohortes. La gate « activation générale » reste fermée tant que le
scénario 18 n’a pas un playbook de downgrade vérifié.

### Invariants de transition

- [x] La lecture et l’aperçu v1 ne mutent aucun fichier.
- [x] La migration est opt-in et télécharge une copie approuvée vérifiée.
- [x] `installed.json` reste lisible et n’est jamais réécrit par le manifest v2.
- [x] Une sauvegarde `installed-v1.backup.json` est créée avant la première migration.
- [x] Les copies projet, externes, manquantes, modifiées ou divergentes restent hors migration.
- [x] Une opération interrompue restaure son ancien contenu ou signale un rollback impossible.
- [x] Migration, install, update, repair, uninstall et switch d’organisation partagent un verrou
  global.
- [ ] Un downgrade complet vers la dernière version legacy a été exécuté sur une fixture migrée.

## 2. Gates automatisées

Renseigner le commit testé au moment de la release :

```text
Commit :
Date :
Opérateur :
```

| Gate | Commande | Résultat attendu | État |
| --- | --- | --- | --- |
| App tests | `cd skillreg-app && pnpm test` | 0 échec | **PASS — 428 tests** |
| App types | `cd skillreg-app && pnpm typecheck` | 0 erreur | **PASS** |
| App format | `cd skillreg-app && pnpm format:check` | 0 erreur | **PASS** |
| App build | `cd skillreg-app && pnpm build` | build production | **PASS** |
| CLI bundle | `cd skillreg-app && pnpm --filter @skillreg/cli run build:publish` | bundle npm local | **PASS** |
| Desktop tests UI | `cd skillreg-local && pnpm test` | 0 échec | **PASS — 56 tests** |
| Desktop format | `cd skillreg-local && pnpm format:check` | 0 erreur | **PASS** |
| Desktop frontend | `cd skillreg-local && pnpm build` | build Vite | **PASS** |
| Desktop Rust | `cd skillreg-local && cargo test --manifest-path src-tauri/Cargo.toml` | 0 échec | **PASS — 130 tests** |
| Desktop Rust check | `cd skillreg-local && cargo check --manifest-path src-tauri/Cargo.toml` | 0 erreur | **PASS** |
| Desktop package | `cd skillreg-local && pnpm tauri build --no-sign` | bundle local | **PASS — app, DMG et archive arm64 non signés** |
| Desktop signature | `cd skillreg-local && pnpm tauri build` | updater signé | **BLOQUÉ — clé privée absente de la session locale** |
| Website lint | `cd skillreg-website && npm run lint` | 0 erreur | **PASS** |
| Website build | `cd skillreg-website && npm run build` | build production | **PASS** |

La CI ajoute deux gates Rust sur `macos-14` et `windows-2022`, en complément de la gate Linux.
Ces jobs vérifient le code et les contrats filesystem ; ils ne remplacent pas le test de
découverte d’un skill par le vrai agent.

Résultats locaux du 2026-07-29 :

- app/API/CLI : 50 fichiers de tests, 428 tests, typecheck et build production verts ;
- desktop : 6 tests Node, 56 tests frontend, 82 tests Rust unitaires et 48 tests Rust
  d’intégration/doc, tous verts ;
- packaging macOS arm64 rejoué après l’import local : `.app`, `.dmg` et `.app.tar.gz` générés
  avec `--no-sign` ;
- site public : lint et 38 pages/routes construites ;
- avertissements non bloquants : chunk Vite principal supérieur à 500 kB, convention Next
  `middleware` dépréciée, artefacts updater Tauri v1 dépréciés ;
- QA visuelle native : dashboard validé à 900×600, option d’import visible dans les Réglages,
  47 skills locales importées, 141 bindings créés et lien externe `decktype` laissé intact.

## 3. Couverture automatique critique

- [x] Manifest v2 strict, version future refusée, corruption préservée.
- [x] Écriture atomique et permissions privées du manifest.
- [x] Écriture atomique de `config.json`, ancien fichier préservé en cas d’échec.
- [x] Permissions Unix `0700` pour `.skillreg` et `0600` pour `config.json`.
- [x] Validation archive : traversal, chemins absolus, symlinks, hardlinks, profondeur et taille.
- [x] Checksum et métadonnées serveur obligatoires.
- [x] Installation transactionnelle et idempotente.
- [x] Une archive téléchargée pour plusieurs agents.
- [x] Swap `staging/content/previous` récupérable après interruption.
- [x] Conflits dossier, fichier, symlink et source homonyme non destructifs.
- [x] Suppression limitée aux bindings dont la propriété est prouvée.
- [x] Désinstallation idempotente, env conservé, tombstone bornée.
- [x] Migration copies identiques, copies divergentes et contenu modifié.
- [x] Import local : aperçu pur, doublons identiques, lien externe intact, rollback et idempotence.
- [x] Descriptions YAML littérales/repliées et anciens noms d’affichage acceptés.
- [x] Les skills d’origine locale sont ignorées avant toute résolution de mise à jour distante.
- [x] Migration et autres mutations concurrentes sérialisées.
- [x] Auto-update ON, OFF, manuel, checksum invalide et contenu local modifié.
- [x] Switch d’organisation : autorisation, cache réutilisé, rollback sur conflit.
- [x] Config et manifest restent couverts par le verrou pendant le switch.
- [x] Un assistant détecté après l’installation reçoit automatiquement ses bindings à
  l’ouverture du dashboard.
- [x] Le CLI scope projet reste fonctionnel.
- [x] Le CLI scope utilisateur refuse de concurrencer un manifest v2.
- [x] Le frontend ne transmet ni agent, ni scope, ni chemin, ni version.
- [x] L’onboarding ne reste pas bloqué après une erreur de préparation.

## 4. Matrice cross-platform

`Automatisé` signifie test Rust/fixture filesystem. `Runtime` signifie découverte par le vrai
Claude Code, Codex ou Cursor. Une ligne ne peut passer à `Validée` qu’avec les deux.

| OS | Arch. | Claude runtime | Codex runtime | Cursor runtime | Install | Update | Uninstall | Migration | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| macOS | arm64 | Validé 2026-07-29 | Validé 2026-07-29 | Validé 2026-07-29 | Automatisé | Automatisé | Automatisé | Validé runtime local | E2E restants requis |
| macOS | x64 | À tester | À tester | À tester | CI à exécuter | CI à exécuter | CI à exécuter | CI à exécuter | Bloqué |
| Windows | x64 | À tester | À tester | À tester | CI à exécuter | CI à exécuter | CI à exécuter | CI à exécuter | Bloqué |
| Linux | x64 | À tester | À tester | À tester | CI à exécuter | CI à exécuter | CI à exécuter | CI à exécuter | Bloqué |

Les adaptateurs Linux et Windows restent désactivés tant que leur ligne runtime n’est pas
documentée dans `docs/agent-compatibility.md`. Il n’existe aucun fallback silencieux vers une
copie physique.

## 5. Scénarios E2E obligatoires

Exécuter avec un home de test dédié, un compte de test sans secret de production et au moins une
skill sentinelle. Conserver uniquement des captures expurgées.

| # | Scénario | Automatique | Runtime manuel | Résultat |
| ---: | --- | --- | --- | --- |
| 1 | Nouvelle installation, un agent | Oui | Requis | À tester |
| 2 | Nouvelle installation, trois agents | Oui | Requis | À tester |
| 3 | Aucun agent | UI + Rust | Requis | À tester |
| 4 | Agent installé après la skill, réconciliation automatique à l’ouverture | Oui | Requis | À tester |
| 5 | Dossier homonyme non géré | Oui | Recommandé | À tester |
| 6 | Symlink homonyme vers une autre cible | Oui | Recommandé | À tester |
| 7 | Agent désinstallé | Oui | Requis | À tester |
| 8 | Réseau perdu pendant téléchargement | Oui | Requis | À tester |
| 9 | Crash avant et après swap | Oui | Requis | À tester |
| 10 | Checksum invalide | Oui | Recommandé | À tester |
| 11 | Tarball malveillant | Oui | Non nécessaire | Couvert |
| 12 | Auto-update global ON | Oui | Requis | À tester |
| 13 | Auto-update OFF puis update manuel | Oui | Requis | À tester |
| 14 | Migration de copies identiques | Oui | Requis | À tester |
| 15 | Migration de copie modifiée | Oui | Requis | À tester |
| 16 | Installation projet CLI présente | Oui | Requis | À tester |
| 17 | Changement d’organisation | Oui | Requis | À tester |
| 18 | Downgrade vers la version legacy | Non | Obligatoire | **Bloquant** |
| 19 | Logout/login autre utilisateur OS | Partiel | Obligatoire | À tester |
| 20 | Home avec espaces et caractères non ASCII | Fixtures | Obligatoire | À tester |
| 21 | Import de dossiers locaux non suivis, doublons et lien externe | Oui | Requis | **Validé macOS arm64 — 47 skills** |

## 6. Parcours collaborateur à vérifier visuellement

- [ ] L’onboarding ne demande que l’entreprise lorsqu’un choix réel existe.
- [x] La détection et l’import local utilisent du langage métier.
- [ ] Un échec de préparation affiche « Réessayer » et ne laisse pas un spinner infini.
- [x] Le dashboard rend l’état global compréhensible en moins de cinq secondes.
- [x] Le toggle de mise à jour automatique est visible sans ouvrir les réglages.
- [ ] « Mettre à jour maintenant » fonctionne lorsque le toggle est OFF.
- [ ] Le catalogue propose une seule action d’installation.
- [ ] Le détail ne montre ni sélecteur d’agent, ni scope, ni version.
- [x] « Mes skills » affiche une ligne canonique par skill et les assistants comme disponibilité.
- [ ] Les états offline, vide, chargement, conflit et action requise restent actionnables.
- [ ] Réparer et désinstaller demandent une confirmation proportionnée.
- [ ] La désinstallation explique que les accès enregistrés sont conservés.
- [ ] Un changement d’entreprise n’expose jamais simultanément les bindings des deux entreprises.
- [x] Aucun compteur d’usage, leaderboard ou suggestion de désinstallation n’est visible.
- [ ] Vérification clavier, focus, lecteur d’écran et contraste effectuée.
- [~] Fenêtre compacte 900×600 vérifiée ; tailles standard et maximisée déjà couvertes par la
  campagne QA précédente.

## 7. Vérifications sécurité et confidentialité

### Filesystem

- [x] Le stockage canonique valide tous les slugs et noms avant de construire un chemin.
- [x] Une cible hors `~/.skillreg/skills` est refusée.
- [x] L’extraction ne suit aucun lien et refuse les entrées non régulières.
- [x] Le désinstalleur retire le reparse point, jamais sa cible.
- [x] Les manifests corrompus ou futurs ne sont jamais réécrits.
- [x] Le token local est écrit atomiquement dans un fichier privé sur Unix.
- [ ] Permissions et ACL de `.skillreg` vérifiées sur Windows.
- [ ] Junctions testées sur NTFS, même volume, profil standard sans privilège admin.

### Données

- [x] Aucune valeur de variable d’environnement n’entre dans le manifest v2.
- [x] Aucune archive, URL signée, prompt ou contenu de `SKILL.md` n’est envoyé par le frontend.
- [x] Le lot A ne collecte aucun usage de skill.
- [x] Une absence de signal reste `unavailable`, jamais zéro.
- [ ] Logs Rust/Tauri inspectés sur un run complet pour token, URL signée et chemin absolu.
- [ ] Sentry inspecté avec une erreur simulée et un compte de test.

### Release

- [ ] Bundle signé sur chaque plateforme.
- [ ] Notarisation macOS validée.
- [ ] Hashes des artefacts publiés vérifiés.
- [ ] Endpoint updater testé depuis la version stable précédente.
- [ ] Aucun secret présent dans les bundles, sourcemaps ou logs CI.

## 8. Rollback

### Rollback d’une opération

Ces rollbacks sont implémentés et testés :

- install/update : restauration de `previous`, ancien manifest conservé ;
- création de bindings : retrait des seuls liens nouvellement créés ;
- migration : restauration des répertoires legacy tant que la transaction n’est pas committée ;
- switch d’organisation : restauration des bindings et de l’organisation précédente ;
- échec d’écriture config : config précédente intacte.

### Rollback d’une release

Procédure provisoire en dogfood :

1. arrêter immédiatement la distribution du build concerné ;
2. fermer SkillReg et les agents ;
3. sauvegarder intégralement `~/.skillreg`, `installed.json` et les dossiers agents sans suivre
   les liens ;
4. ne supprimer, déplacer ou recopier manuellement aucun binding ;
5. collecter uniquement les codes d’erreur et versions, jamais le token ni les valeurs env ;
6. réparer avec le build géré corrigé ou restaurer depuis une sauvegarde validée ;
7. ne réinstaller une version legacy qu’après validation du playbook sur une fixture identique.

Le downgrade binaire post-migration n’est **pas encore une opération supportée**. La copie legacy
est remplacée par un binding après commit et son backup de répertoire est supprimé ; seul le
manifest v1 est sauvegardé. Il faut donc soit :

- fournir un outil interne de démigration qui retélécharge et vérifie les versions v1 avant de
  recréer les dossiers physiques ;
- soit prouver qu’un downgrade vers le dernier binaire stable sait gérer les bindings v2 sans
  mutation destructive.

Tant que l’une de ces voies n’est pas testée, le rollout reste limité à des profils de test
sauvegardés.

## 9. Plan de rollout

1. [x] Toutes les gates automatisées locales passent.
2. [ ] Les jobs CI Linux, macOS et Windows passent sur le commit candidat.
3. [ ] Dogfood macOS arm64 sur profils sauvegardés.
4. [ ] Parcours E2E 1 à 20 exécutés et preuves consignées.
5. [ ] Runtime Claude, Codex et Cursor validé sur chaque OS activé.
6. [ ] Cohorte interne Kairia en build signé.
7. [ ] Revue des erreurs de binding, migration et rollback après 72 heures.
8. [ ] Design partners volontaires, avec procédure de support explicite.
9. [ ] Revue à J+7 : succès install/update, actions requises, migrations partielles.
10. [ ] Activation par défaut uniquement après fermeture du scénario 18.
11. [ ] Lecteur v1 conservé pendant au moins une version stable supplémentaire.
12. [ ] Retrait legacy traité dans un plan séparé.

Les métriques du dogfood proviennent des journaux locaux expurgés et d’un relevé manuel. Aucun
upload de télémétrie de contenu ou d’usage n’est activé dans le lot A.

## 10. Go / no-go

### Conditions GO dogfood macOS

- [x] Toutes les gates locales sont vertes.
- [ ] Le package Tauri est produit et signé.
- [ ] Les trois agents macOS découvrent la fixture via le binding géré.
- [ ] Les scénarios destructifs 5, 6, 9, 14, 15 et 17 sont rejoués.
- [ ] Le profil de test est sauvegardé avant migration.

### Conditions GO activation générale

- [ ] Zéro perte de données sur toutes les fixtures.
- [ ] Chaque OS/agent activé est validé en runtime.
- [ ] Le downgrade de release est fonctionnel et documenté.
- [ ] Aucun choix technique n’apparaît dans le parcours principal.
- [ ] Documentation app, desktop, site et CLI alignée.
- [ ] Installation projet CLI sans régression.
- [ ] Signature, notarisation et updater validés.

### Décision

```text
Décision : NO-GO activation générale
Motifs ouverts :
- runtime Windows/Linux non validé ;
- macOS x64 non validé ;
- scénario 18 de downgrade post-migration non prouvé ;
- campagne E2E Tauri manuelle à exécuter.
```
