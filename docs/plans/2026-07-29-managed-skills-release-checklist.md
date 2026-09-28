# Checklist de release — expérience de skills gérées

**État du candidat :** 2026-09-28
**Périmètre :** lot A de l’expérience collaborateur métier
**Version desktop de travail :** 0.3.25
**Statut actuel : NO-GO dogfood macOS et NO-GO activation générale.**
Aucun périmètre de distribution autorisé. Une PR prête à relire ne remplit pas les critères
« checklist entièrement cochée » et « autorisé à livrer ».

Les preuves courantes, commandes, environnements et limites sont dans le
[registre de validation](../managed-skills-release-validation.md).
Les résultats de juillet repris depuis `bbfbe94df75a472221733374cfac8170e836e011` sont
historiques, sans empreinte du binaire ; ils ne valident pas ce candidat.

Ce document est la gate de release du stockage canonique, des bindings gérés, de l’installation
en un clic, des mises à jour globales, de la migration v1, de la désinstallation, de la réparation
et du changement d’organisation. Les compteurs d’usage, suggestions de nettoyage et vues
administrateur appartiennent au lot B et sont explicitement hors périmètre.

## 1. Décision de transition

Preuves automatiques : A1/A2. Downgrade du binaire : N4.

### Mode de déploiement retenu

La transition est pilotée par **cohortes de distribution du binaire**, pas par un flag local
incomplet :

- le parcours desktop courant utilise le modèle `managed` pour les nouvelles installations ;
- une installation v1 n’est migrée qu’après aperçu et confirmation explicite ;
- le lecteur v1 reste présent et les installations projet, externes, modifiées ou conflictuelles
  restent intactes ;
- le contrat CLI attendu refuse les mutations utilisateur d’un manifeste v2 ; sa validation
  externe reste obligatoire (N6) ;
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
- [ ] Un downgrade complet avec le binaire identifié de la dernière version legacy a été exécuté sur une fixture migrée.

## 2. Gates automatisées et provenance

| Gate | Référence de preuve courante | État |
| --- | --- | --- |
| Desktop format, types, build, Rust et Node | A1 | Voir registre |
| Desktop tests React et IPC mock strict | A2 | Voir registre |
| Cargo check verrouillé | A3 | Voir registre |
| Garde notarisation | A4 | Voir registre |
| Confidentialité des erreurs et bundle local | A5 | Voir registre |
| CI Linux/macOS/Windows sur le SHA candidat | N1 | Manquante |
| App tests/types/format/build et bundle CLI | N6 | Preuves externes manquantes |
| Website lint/build, compatibilité téléchargement | N6 | Preuves externes manquantes |
| Packaging, signatures, notarisation, updater réel | N7 | Non exécutés pendant cette étape |

La CI construit `dist/` sur chacun des runners `ubuntu-22.04`, `macos-14`, `windows-2022`
avant Cargo, avec pnpm 9, Node 22, Rust stable, installation figée et
`cargo test --locked --no-fail-fast`. Aucun secret, signature ou publication en CI.
Une compilation ne prouve pas la découverte par un agent ; un runner Windows administrateur
ne prouve pas les droits d’un compte standard ; macOS arm64 ne prouve pas macOS x64.

Archive historique du 2026-07-29 (non réexécutée sur ces autres dépôts, non applicable au
candidat) : app 428 tests, desktop 6 Node / 56 UI / 130 Rust, site 38 routes, packaging macOS
arm64 non signé et import de 47 skills. Les anciennes alertes de chunks et de formats updater
ne décrivent plus ce candidat : le workflow utilise désormais les formats natifs Tauri 2.

## 3. Couverture automatique critique

Les cases locales renvoient à A1/A2 ; les deux contrats CLI renvoient à N6.
Une case locale cochée ne remplace jamais la preuve native demandée dans les sections suivantes.

- [x] Manifest v2 strict, version future refusée, corruption préservée.
- [x] Écriture atomique et permissions privées Unix du manifest ; Windows reste N5.
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
- [ ] Le CLI scope projet reste fonctionnel.
- [ ] Le CLI scope utilisateur refuse de concurrencer un manifest v2.
- [x] Le frontend ne transmet ni agent, ni scope, ni chemin, ni version.
- [x] L’onboarding ne reste pas bloqué après une erreur de préparation.

## 4. Matrice cross-platform

`Automatisé` signifie test Rust/fixture filesystem. `Runtime` signifie découverte par le vrai
Claude Code, Codex ou Cursor. Une ligne ne peut passer à `Validée` qu’avec les deux.

| OS | Arch. | Claude runtime | Codex runtime | Cursor runtime | Install | Update | Uninstall | Migration | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| macOS | arm64 | Historique, à rejouer | Historique, à rejouer | Historique, à rejouer | A1 | A1 | A1 | A1, runtime à rejouer | NO-GO |
| macOS | x64 | À tester | À tester | À tester | CI à exécuter | CI à exécuter | CI à exécuter | CI à exécuter | Bloqué |
| Windows | x64 | À tester | À tester | À tester | CI à exécuter | CI à exécuter | CI à exécuter | CI à exécuter | Bloqué |
| Linux | x64 | À tester | À tester | À tester | CI à exécuter | CI à exécuter | CI à exécuter | CI à exécuter | Bloqué |

Les adaptateurs macOS x64, Linux et Windows restent désactivés tant que leur ligne runtime n’est pas
documentée dans `docs/agent-compatibility.md`. Il n’existe aucun fallback silencieux vers une
copie physique.

## 5. Scénarios E2E obligatoires

Exécuter avec un home de test dédié, un compte de test sans secret de production et au moins une
skill sentinelle. Conserver uniquement des captures expurgées. Protocoles N2–N5 dans le registre ; ne jamais
utiliser le vrai profil personnel ni un compte de production.

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
| 11 | Tarball malveillant | Oui | Non nécessaire | A1 |
| 12 | Auto-update global ON | Oui | Requis | À tester |
| 13 | Auto-update OFF puis update manuel | Oui | Requis | À tester |
| 14 | Migration de copies identiques | Oui | Requis | À tester |
| 15 | Migration de copie modifiée | Oui | Requis | À tester |
| 16 | Installation projet CLI présente | Oui | Requis | À tester |
| 17 | Changement d’organisation | Oui | Requis | À tester |
| 18 | Downgrade vers la version legacy | Fixture migrée + script réel (A1) | Binaire réel obligatoire (N4) | **Bloquant** |
| 19 | Logout/login autre utilisateur OS | Partiel | Obligatoire | À tester |
| 20 | Home avec espaces et caractères non ASCII | Fixtures | Obligatoire | À tester |
| 21 | Import de dossiers locaux non suivis, doublons et lien externe | Oui | Requis | Historique 47 skills ; candidat à rejouer |

## 6. Parcours collaborateur à vérifier visuellement

Preuve native N3 requise ; jsdom ne prouve ni VoiceOver ni le fond inerte réel.

- [ ] L’onboarding ne demande que l’entreprise lorsqu’un choix réel existe.
- [ ] La détection et l’import local utilisent du langage métier.
- [ ] Un échec de préparation affiche « Réessayer » et ne laisse pas un spinner infini.
- [ ] Le dashboard rend l’état global compréhensible en moins de cinq secondes.
- [ ] Le toggle de mise à jour automatique est visible sans ouvrir les réglages.
- [ ] « Mettre à jour maintenant » fonctionne lorsque le toggle est OFF.
- [ ] Le catalogue propose une seule action d’installation.
- [ ] Le détail ne montre ni sélecteur d’agent, ni scope, ni version.
- [ ] « Mes skills » affiche une ligne canonique par skill et les assistants comme disponibilité.
- [ ] Les états offline, vide, chargement, conflit et action requise restent actionnables.
- [ ] Réparer et désinstaller demandent une confirmation proportionnée.
- [ ] La désinstallation explique que les accès enregistrés sont conservés.
- [ ] Un changement d’entreprise n’expose jamais simultanément les bindings des deux entreprises.
- [ ] Aucun compteur d’usage, leaderboard ou suggestion de désinstallation n’est visible.
- [ ] Vérification clavier, focus, lecteur d’écran et contraste effectuée.
- [ ] Fenêtres compacte 900×600, standard et maximisée vérifiées sur le candidat.
- [ ] Mode clair fourni et vérifié, ou explicitement exclu du périmètre approuvé. Le candidat
  propose uniquement une palette sombre ; aucun résultat clair n’est revendiqué.

## 7. Vérifications sécurité et confidentialité

### Filesystem

A1 pour les fixtures locales ; N5 pour NTFS et ACL en compte Windows standard.

- [x] Le stockage canonique valide tous les slugs et noms avant de construire un chemin.
- [x] Une cible hors `~/.skillreg/skills` est refusée.
- [x] L’extraction ne suit aucun lien et refuse les entrées non régulières.
- [x] Le désinstalleur retire le seul lien dans les fixtures Unix, jamais sa cible ; N5 reste
  nécessaire pour les junctions Windows.
- [x] Les manifests corrompus ou futurs ne sont jamais réécrits.
- [x] Le token local est écrit atomiquement dans un fichier privé sur Unix.
- [ ] Permissions et ACL de `.skillreg` vérifiées sur Windows.
- [ ] Junctions testées sur NTFS, même volume, profil standard sans privilège admin.

### Données

Inspection locale A5 ; parcours natif N2 ; app/site N6.

- [x] Aucune valeur de variable d’environnement n’entre dans le manifest v2.
- [x] Aucune archive, URL signée, prompt ou contenu de `SKILL.md` n’est envoyé par le frontend.
- [x] Le lot A ne collecte aucun usage de skill.
- [x] Une absence de signal reste `unavailable`, jamais zéro.
- [ ] Logs Rust/Tauri inspectés sur un run complet pour token, URL signée et chemin absolu.
- Sentry desktop : **non applicable**, aucune intégration trouvée dans le code et les
  manifestes (A5). La vérification app/site reste séparée, sans accès à la production.

### Release

- [ ] Bundle signé sur chaque plateforme.
- [ ] Notarisation macOS validée.
- [ ] Hashes des artefacts publiés vérifiés.
- [ ] Endpoint updater testé depuis la version stable précédente.
- [ ] Aucun secret présent dans les bundles, sourcemaps ou logs CI.

## 8. Rollback

### Rollback d’une opération

Couverture automatique A1 (distincte du downgrade de release) :

- install/update : comparer les hashes de `content` et `previous` au hash du manifeste
  committé ; restaurer l’ancienne copie ou nettoyer après commit prouvé ; conserver les deux
  et signaler une erreur si l’état est ambigu ;
- création de bindings : retrait des seuls liens nouvellement créés ;
- migration : restauration des répertoires legacy tant que la transaction n’est pas committée ;
- switch d’organisation : restauration des bindings et de l’organisation précédente ;
- échec d’écriture config : config précédente intacte.

### Rollback d’une release

Méthode retenue : **sauvegarde intégrale avant migration**, puis restauration au point de
sauvegarde. `installed-v1.backup.json` seul est insuffisant : les dossiers physiques legacy
ont été remplacés par des liens après migration.

Le protocole N4 utilise `scripts/managed-skills-backup-restore.mjs`, également appelé par les
tests Node et le test Rust `managed_release_rollback`. Home et sauvegarde sont explicites ;
`restore` montre un aperçu, `--apply --recovery <autre dossier>` conserve l’état remplacé à part.
Fermer SkillReg et tous les agents avant snapshot et restore. Le journal de récupération permet
de reprendre une restauration interrompue. Les ajouts ultérieurs ne sont pas fusionnés ni perdus
silencieusement ; le trousseau système reste en place, sans export de secrets.

Le test Rust migre réellement une fixture, puis vérifie la restauration de ses fichiers,
manifests, liens et accès factices. **Il ne remplace pas l’exécution du dernier binaire legacy**,
dont version et empreinte doivent être identifiées dans N4. Sans sauvegarde complète antérieure,
le downgrade n’est pas supporté. Le scénario 18 reste ouvert.

## 9. Plan de rollout

La validation locale A1 conserve quatre échecs de socket dus au sandbox, à rejouer par le
moteur ; aucune condition native ou distante n’est cochée à partir de cette limitation.

1. [ ] Toutes les gates automatisées locales passent.
2. [ ] Les jobs CI Linux, macOS et Windows passent sur le commit candidat.
3. [ ] Dogfood macOS arm64 sur profils sauvegardés.
4. [ ] Parcours E2E 1 à 21 exécutés et preuves consignées.
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

### Conditions GO dogfood macOS arm64 sur profils sauvegardés

- [ ] Toutes les gates locales sont vertes.
- [ ] CI trois OS sur le SHA candidat.
- [ ] Le package Tauri est produit, signé et notarisé ; updater/intégrité vérifiés.
- [ ] Accessibilité native N3 validée.
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
- [ ] ACL et junctions Windows validées en compte standard.
- [ ] Observation à 72 h et revue J+7 consignées.

### Décision

**NO-GO dogfood macOS et NO-GO activation générale ; aucun périmètre de distribution autorisé.**
Les campagnes natives, CI distante, droits Windows standard, binaire legacy, release signée,
compatibilité app/CLI/site et observations à 72 h/J+7 restent sans preuves applicables.
La préparation du code et des protocoles ne coche pas les critères finaux du ticket.
