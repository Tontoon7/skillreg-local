# Validation du candidat skills gérées

La [checklist](plans/2026-07-29-managed-skills-release-checklist.md) porte la décision de
release. Ce registre distingue les preuves automatisées locales des campagnes natives et
externes. **NO-GO dogfood et activation générale** tant que les obligations du périmètre
choisi n’ont pas une preuve applicable au candidat.

## Candidat et environnement

- Base : `793ba1b390aae6c90030a1061ca805bd3c6c32fd`, version 0.3.25 inchangée.
- Socle repris par patches ciblés : `bbfbe94df75a472221733374cfac8170e836e011`, delta depuis
  `3e024f35e721313dc88ce4496765853c800d4ec1`. Publication des commandes de la base conservée.
- Candidat : base + diff de cette PR. Les journaux et l’empreinte des sources testées sont
  conservés dans le dossier de sortie de l’étape usine avec `rapport.json`. À la livraison,
  rattacher ces preuves au SHA créé par l’orchestrateur et rejouer la CI sur ce SHA.
- Session locale reprise et revalidée : 2026-09-30, macOS 26.6.2 (25G83), Darwin arm64, Node 26.8.1, pnpm 9.15.9,
  Rust/Cargo 1.96.1. La CI choisit Node 22 et reste à exécuter.
- Aucun vrai agent lancé, aucun trousseau personnel ni donnée de production consulté.
  Les providers réseau et identifiants sont simulés ; les tests utilisent des homes injectés.
- Résultats du 2026-07-29 : historiques seulement, rapportés par la branche source sans
  empreinte de binaire. Ne pas les recopier comme PASS du candidat.

## Registre des preuves automatiques

Les sorties finales, compteurs et restrictions sont consignés après exécution dans ce tableau.
`PASS local` ne signifie jamais PASS Windows, Linux, lecteur d’écran ou agent réel.
Après correction de l’écriture des bits d’auto-héritage Windows, A1–A3 ont été rejoués sur `2055060`
avec le diff de correction : résultats ci-dessous, journaux dans la sortie
`recovery-1/ci-3-correction`. A4–A5 restent les vérifications précédentes, non réexécutées
pendant cette correction ; aucun workflow n’a changé.

| ID | Commande / inspection | Résultat courant et portée |
| --- | --- | --- |
| A1 | `pnpm format:check && pnpm build && cargo test --manifest-path src-tauri/Cargo.toml --locked && node --test --experimental-strip-types tests/*.test.ts` | **Format et build PASS ; Cargo 89 unitaires PASS / 4 échecs socket sandbox, code 101.** La chaîne s’arrête avant Node. Exécution complète `--no-fail-fast` : **144 PASS / 4 mêmes échecs**, aucun test ignoré. Node lancé séparément : **55/55 PASS** ; commande CI `pnpm test:node` également **55/55 PASS**. Voir `factory-validation.log`, `rust-all.log`, `node-factory.log`, `node-ci.log`. |
| A2 | `pnpm test:frontend` | **72/72 PASS, 18 fichiers**, DOM jsdom/IPC simulé seulement ; `frontend.log`. |
| A3 | `cargo check --manifest-path src-tauri/Cargo.toml --locked` | **PASS**, `cargo-check.log`, compilation macOS arm64 uniquement. |
| A4 | `bash scripts/check-release-notarization.sh` | **PASS**, présence des commandes, pas notarisation réelle ; sortie « Release workflow contains macOS notarization hooks. ». |
| A5 | Erreurs synthétiques, recherche Sentry, inspection de `dist/` | **PASS local limité** : tests synthétiques frontend/Rust verts ; 6 fichiers `dist/`, 0 sourcemap, 0 détection de valeur token/clé privée/URL signée/chemin personnel/fixture. Aucune intégration Sentry trouvée dans sources et dépendances. `privacy-inspection.json` ; aucune inspection de service externe. |

Les quatre échecs sont ceux de `commands::slash_commands::tests` :
`invalid_publication_inputs_send_no_requests`, `publication_http_errors_are_contextual_and_never_retried`,
`publication_invalid_response_is_distinct_from_network_failure` et
`publishes_raw_content_with_encoded_segments_and_explicit_metadata`. Tous s’arrêtent à
`src-tauri/src/commands/slash_commands.rs:900`, `TcpListener::bind("127.0.0.1:0")`,
`Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }`.
Ce module est inchangé. Aucun test n’a été désactivé ni passé en skip. Le moteur doit rejouer
la validation avec ses sockets autorisées ; le résultat n’est pas un PASS complet local.
Les tests Windows exécutent **0 cas** sur macOS et ne prouvent rien sur NTFS.

Pour reproduire, installer les lockfiles (`pnpm install --frozen-lockfile`, `cargo fetch --locked
--manifest-path src-tauri/Cargo.toml`) et construire `dist/` avant Cargo, sur **chaque** OS.
Node 22 récent est installé en CI ; les tests UI utilisent Vitest/Testing Library déjà retenus
par le socle. Le script de sauvegarde demande les outils ACL natifs : macOS `ls`/`chmod`, Linux
`getfacl`/`setfacl` (paquet `acl`), Windows PowerShell/.NET. Ne pas contourner un refus ACL.
En usine, définir `TMPDIR` vers un dossier de fixtures dans la sortie de l’étape avant les tests
Rust ; pas de profil personnel. Aucun serveur de développement ni packaging complet.

### Reproduction avant correction

- Reprise interrompue : manifeste ancien + nouveau `content` + ancien `previous` provoquait
  `ContentModified` après suppression de `previous`. L’état ambigu perdait aussi sa copie.
  Les tests passent ensuite par le vrai chemin du service, avec hash committé et reprise répétée.
- Revue de reprise : le cycle de mises à jour vérifiait le hash avant de récupérer le swap,
  classant à tort une interruption en contenu modifié. Le nouveau test échoue d’abord
  (`available = 0`, attendu 1), puis passe après récupération sous verrou avant ce contrôle,
  en automatique et en manuel avec auto-update OFF. L’état ambigu reste intact et signalé.
  Suite `managed_installation` : 17/17 PASS ; preuve `managed-recovery-red-green.txt`.
- Correction après revue : l’erreur de récupération interrompait encore le cycle complet.
  Le test à deux skills échoue d’abord sur `RollbackFailed` (`recovery-red.log`), puis
  passe avec la suite `managed_installation` : **17/17 PASS** (`recovery-green.log`).
  Sur deux cycles successifs, la skill ambiguë conserve `content`, `previous`, version et hash,
  reçoit `ActionRequired`, l’erreur structurée et l’horodatage persistés ; la seconde skill
  est vérifiée et mise à jour une seule fois. L’installation explicite de la skill ambiguë
  échoue toujours sans modifier le manifeste. Un échec de persistance reste une erreur globale.
- Dialogue : les tests échouaient sur l’absence de `showModal` et de cancel natif ; le retour de
  focus après disparition du menu révélait un second défaut. Tests corrigés avec le composant,
  sans remplacer la modalité native par un piège de focus artisanal.
- Release : les tests de configuration échouaient sur l’absence de CI et les anciens suffixes
  updater ; les tests du helper échouaient avant sa création. Le helper valide les quatre
  plateformes avant de produire la sortie et associe chaque artefact à sa propre signature.
- Sauvegarde : tests ajoutés avant le script, puis fixture migrée par le vrai moteur Rust ;
  aucun helper de test ne réimplémente la restauration.

### Cartographie des 21 scénarios

| Scénarios checklist | Suites et limites |
| --- | --- |
| 1–3, 8–10 | `managed_installation` : agents simulés, erreurs provider, checksum et interruptions autour du commit. Aucun vrai téléchargement. |
| 4–7 | `managed_bindings`, `managed_uninstall`, `Dashboard.test.tsx` : ownership et réconciliation, pas découverte runtime. |
| 11 | Tests `archive` : traversal, liens et archives malveillantes ; fixtures suffisantes pour ce contrôle. |
| 12–13 | `managed_installation`, `auto_update`, `Dashboard.test.tsx` : ON/OFF/manuel, occupé, erreur et réessai. |
| 14–16 | `managed_migration` : copies identiques/modifiées/projets/liens externes ; le CLI externe reste à tester. |
| 17 | `active_org_reconcile`, tests config/store : cache, conflit, autorisation et rollback. |
| 18 | `managed_release_rollback` + `managed-skills-backup-restore.test.ts` : protocole filesystem réel ; binaire legacy absent. |
| 19 | Tests config/env et `managed_windows_permissions` sur Windows : protection locale ; deux utilisateurs OS réels encore nécessaires. |
| 20 | Fixtures Unicode/espaces dans liens/import/rollback ; N2/N5 doivent confirmer dans les vrais agents. |
| 21 | `local_skill_import`, `LocalSkillImportPanel.test.tsx` : doublons, externe, import idempotent. |

## N1 — CI du candidat

Après livraison du code par l’orchestrateur, conserver pour chaque OS : SHA testé, URL du run,
date, architecture, versions Node/Rust, résultat format/build/Node/Vitest/Cargo. La matrice de
`.github/workflows/ci.yml` doit être entièrement verte. `--no-fail-fast` expose tous les binaires
Rust ; un échec reste un échec, aucune règle ni test n’est désactivé.

Premier échec : **Windows au formatage** dans le run `36402842337`, job `108864527362`
(journal fourni par l’usine). L’export local avec `core.autocrlf=true` reproduit les 91 erreurs
Biome dues aux CRLF. `.gitattributes` impose désormais LF aux textes dès le checkout, sans
modifier le workflow ni assouplir Biome. La matrice doit être rejouée sur le candidat corrigé ;
les résultats des autres jobs ne sont pas fournis dans cette étape.
Preuve locale avant/après dans `reprise-1/ci-1-correction` :
`windows-format-before.log` (91 erreurs), `windows-format-after.log` (91 fichiers, PASS).
L’export corrigé ne contient plus de CRLF dans `UpdateChecker.tsx` et les icônes PNG restent
identiques octet par octet. Revalidation locale : format/build PASS, Node 51/51, Vitest 72/72,
`cargo check` PASS ; Cargo `--no-fail-fast` 144 PASS et les quatre mêmes refus de sockets
décrits en A1. Revue indépendante du diff : aucun défaut bloquant relevé.

Deuxième échec : **échec Windows des helpers Node**, run `36404147565`, job `108868714172`
(43 PASS / 8 échecs). L’extrait fourni ne contient pas les erreurs individuelles.
L’inspection identifie trois lectures stdin PowerShell sans encodage explicite, alors que Node
transmet de l’UTF-8 : inventaire ACL, détection des junctions et préparation ACL du test.
Le profil `profil été` expose ce défaut ; huit tests de rollback exigent un snapshot réussi,
les deux autres s’arrêtent avant ces lectures. Les trois lectures utilisent désormais
`StreamReader(Console.OpenStandardInput(), Encoding.UTF8)` ; les assertions et le profil
accentué sont conservés. Aucun workflow ni contrôle modifié. Le diagnostic doit encore être
confirmé par le prochain run Windows : aucun runtime Windows n’est disponible localement.
Dans `reprise-1/ci-2-correction`, `pnpm test:node` passe 51/51 et Vitest 72/72 ; format/build
PASS. Cargo `--no-fail-fast` donne 144 PASS et les quatre refus de sockets décrits en A1.
Les logs Node, frontend et Rust sont conservés avec le rapport. Revue indépendante du correctif :
aucun défaut bloquant relevé. Le statut de release reste **NO-GO**.

Échec suivant : **Windows non résolu**, run `36405167007`, job `108872050735`
(46 PASS / 5 échecs). Le nouvel extrait ne contient toujours aucun bloc d’erreur individuel.
Le nombre correspond aux cinq tests exigeant une restauration réussie, mais ne prouve ni
leur identité ni la cause. L’inspection du second clonage, des ACL, des chemins, des inventaires
vides et des swaps n’a pas permis de démontrer un défaut Windows. Aucun runtime Windows local
n’est disponible. Ne pas considérer le correctif UTF-8 ni les résultats macOS comme une preuve
de résolution de ces cinq échecs.

Cette étape ajoute des diagnostics de phase (sauvegarde de récupération, staging, vérification,
inspection d’état, déplacement, activation et commit du journal), sans remonter les chemins
des erreurs filesystem ni stderr natif. Le test de profil changé après interruption exige
désormais l’exception du hook : une panne antérieure ne peut plus le faire passer par erreur.
Le test du diagnostic échoue avant modification puis passe ; les dix tests de sauvegarde
passent localement (`reprise-1/ci-3-correction/diagnostic-red.log` et `diagnostic-green.log`).
La revue indépendante n’a relevé aucun défaut dans ce changement diagnostique. Il faut récupérer
les blocs `not ok` complets du run Windows pour corriger la cause ; la gate N1 reste ouverte.
Revalidation locale : format/build et `cargo check` PASS, Node 51/51, Vitest 72/72.
La chaîne bloquante s’arrête sur les quatre refus de sockets décrits en A1 ; l’exécution
Rust `--no-fail-fast` termine avec 144 PASS et ces quatre échecs, aucun test ignoré.
Journaux Node, Vitest, Cargo et reproduction rouge/verte conservés dans la sortie de cette étape.

Arbitrage après le run `36406359358`, job `108875937930` (Windows toujours en échec, aucun
journal disponible hors GitHub) : les trois corrections n’ont reçu que la fin du journal CI.
Hors terminal, `node --test` utilise le reporter TAP : les blocs `not ok` restent au milieu du
journal et la fin ne contient que les totaux. `pnpm test:node` utilise désormais
`--test-reporter=spec`, qui répète après les totaux chaque test en échec avec son erreur et sa
pile. Les tests, assertions et le workflow sont inchangés. La création du dossier `moved` et
l’écriture initiale du journal indiquent aussi leur phase. La cause Windows reste à établir
depuis ce résumé ; la gate N1 reste ouverte. Revalidation locale : format/build et
`cargo check` PASS, Node 51/51, Vitest 72/72, Cargo `--no-fail-fast` 144 PASS et les quatre
refus de sockets décrits en A1.

Une compilation macOS arm64 locale
ne complète aucune ligne Windows/Linux/x64. Le compte administrateur du runner Windows ne
remplace pas N5.

Correction précédente après le run `36498903145`, job `109184823362` : **Windows à rejouer**.
Le reporter donne désormais deux erreurs précises : chargement impossible du module de
`Get-Acl` dans la fixture, et inventaire divergent lors de la reprise après activation.
Les scripts ACL Node et Rust importent explicitement `Microsoft.PowerShell.Security` depuis
`$PSHOME`, pour ne pas sélectionner un module incompatible via le `PSModulePath` hérité de
`pwsh`. La fixture ACL place un module homonyme qui échoue au chargement en tête de ce chemin.

La copie des ACL ne demande plus `UNPROTECTED_DACL_SECURITY_INFORMATION` pour les clones
neufs déjà non protégés : elle écrit la DACL sauvegardée seule, et conserve le flag de
protection pour les DACL protégées. La comparaison intégrale des descripteurs, contenus et
modes reste obligatoire. Le test interrompu compare aussi les inventaires activé et conservé
immédiatement après déplacement ; les assertions restent hors du hook pour conserver leur
diff en cas d’échec. Le lien causal entre la demande de réhéritage et l’inventaire divergent
reste une hypothèse à confirmer sur Windows, sans runtime Windows disponible localement.
Les tests macOS passent aussi avant correction : ce n’est pas une reproduction rouge/verte
Windows. Aucun workflow, test ou contrôle n’est supprimé ou assoupli ; N1 reste ouverte.

Validation locale de cette correction (`resumed/ci-1-correction`) : format/build et
`cargo check` PASS, Node **51/51**, Vitest **72/72**. La chaîne bloquante s’arrête sur
les quatre refus de sockets décrits en A1 ; `cargo test --no-fail-fast` termine avec
**144 PASS / 4 mêmes échecs**, aucun test ignoré. Journaux : `factory-validation.log`,
`rust-all.log`, `node-factory-final.log`, `frontend.log`, `cargo-check.log`.
La reproduction de la commande CI `pnpm test:node` passe aussi (`node-after.log`).

Correction après le run `36500167005`, job `109188868949` : **renommage Windows remplacé,
divergence toujours présente au run suivant**. Le diff fourni localise la divergence dans la seule
racine conservée `.skillreg` : DACL héritée devenue protégée (`D:P`), ACE héritées devenues
explicites (perte des marqueurs `ID`), enfants inchangés. Les clones passent leur vérification
avant ce déplacement ; la modification du clonage dans l’étape précédente ne résolvait donc
pas la divergence observée après déplacement.

Les deux swaps de racines utilisent `SetFileInformationByHandle(FileRenameInfo)`
sur Windows, avec le droit `DELETE` et les flags d’ouverture `OPEN_REPARSE_POINT` et
`BACKUP_SEMANTICS`. Ce choix vise à conserver le descripteur sans étape de réparation ACL
après déplacement. Le renommage refuse une destination existante et agit sur la junction elle-même.
Le journal garde son renommage habituel. La comparaison intégrale des inventaires et le refus
d’un état ambigu restent inchangés. Une récupération dont l’inventaire diverge
reste à inspecter ; elle n’est pas réparée automatiquement.

Les tests comparent aussi l’inventaire conservé après le premier déplacement, avant activation,
et couvrent une racine fichier, une racine junction et un mélange de DACL héritées/protégées.
La reproduction locale `pnpm test:node` passe avant correction sur macOS ; le journal Windows
fourni constitue la preuve d’échec, sans reproduction Windows locale. Aucun workflow ni
contrôle assoupli ; N1 reste ouverte et la release **NO-GO**.

Validation locale de cette correction (`resumed/ci-2-correction`) : format/build PASS,
Node **51/51** via la commande CI et la commande usine, Vitest **72/72**, `cargo check` PASS.
La chaîne bloquante s’arrête sur les quatre refus de sockets décrits en A1 ;
`cargo test --no-fail-fast` termine avec **144 PASS / 4 mêmes échecs**, aucun test ignoré.
Revue indépendante du correctif : aucun défaut bloquant relevé ; la compilation C# et les
ACL NTFS restent à confirmer en CI Windows. Journaux : `factory-validation.log`,
`node-before-fix.log`, `node-after-fix.log`, `node-factory.log`, `frontend.log`, `rust-all.log`,
`cargo-check.log`.

Correction après revue du contrat de sortie (`resumed/ci-2-review-1-correction`) : le
branchement PowerShell appelait une méthode `void`, puis tentait de sérialiser un pipeline
vide. Sous PowerShell 5.1, une stdout vide pouvait faire échouer `JSON.parse` après le
déplacement réussi. Le branchement émet maintenant `renamed` après succès ; `winMetadata`
accepte aussi une stdout vide uniquement pour un lot exclusivement de renommages. Les
échecs natifs, JSON malformés et sorties ACL absentes (y compris lots mixtes) restent refusés.

Le test portable reproduit d’abord `SyntaxError: Unexpected end of JSON input`
(`rename-output-red.log`), puis les **12/12 tests** de sauvegarde/restauration passent
(`rename-output-green.log`). Ces deux nouveaux tests simulent seulement la sortie du processus.
Un test réservé Windows vérifie en plus le tableau `["renamed"]` renvoyé par `winMetadata`
après appel au vrai `powershell.exe`, la disparition de la source et la conservation du contenu
déplacé. Les tests existants de restauration, d’activation et de reprise restent inchangés. Ce test natif
ne s’exécute pas sur macOS et doit passer en CI Windows avant de conclure sur N1/N5.

Validation locale de cette première correction : **53/53 Node**, **72/72 Vitest**, **144 PASS Rust et
quatre refus de sockets sandbox**, `cargo check` et garde notarisation PASS. Relecture
indépendante du diff : aucun défaut bloquant relevé. La release reste **NO-GO** et les
campagnes Windows/NTFS et natives demeurent nécessaires.

Correction après seconde revue du contrat de sortie (`resumed/ci-2-review-2-correction`) :
PowerShell peut sérialiser un résultat unique en scalaire JSON (`"renamed"` ou une ACL),
au lieu d’un tableau. `winMetadata` normalise désormais le résultat parsé en tableau ; les
tableaux existants conservent leur ordre. Le test Windows porte sur ce contrat normalisé,
sans imposer la forme brute de stdout. Les refus de sortie ACL vide, de JSON malformé et
d’échec natif restent inchangés.

Les mocks portables reproduisent d’abord les deux écarts chaîne/tableau pour le marqueur et
l’ACL (`metadata-output-red.log`, **2 échecs attendus**). Après correction, les **13/13 tests**
de sauvegarde/restauration passent (`metadata-output-green.log`), avec les formes scalaire,
tableau singleton et tableau mixte. Ces mocks ne prouvent pas le runtime PowerShell 5.1 :
le test natif doit encore passer en CI Windows et la release reste **NO-GO**.
La relecture indépendante du correctif n’a relevé aucun défaut bloquant. Les résultats
complets de cette seconde correction sont conservés dans `resumed/ci-2-review-2-correction`.

Correction après le run `36502655614`, job `109196862004` : **lecture brute des ACL,
confirmation Windows requise**. La racine conservée apparaît encore en `D:P` avec ses ACE
héritées devenues explicites, malgré le renommage par handle ; les enfants restent identiques.
Le journal ne prouve donc pas que le déplacement a modifié le descripteur stocké. La revue du
chemin confirme qu’aucune écriture d’ACL ne cible cette racine pendant le swap ; la lecture
`GetSecurityInfo` reste susceptible de présenter l’héritage selon le nouveau parent privé.

La lecture utilise maintenant `GetKernelObjectSecurity`, symétrique de l’écriture existante,
pour inventorier le descripteur brut de l’objet ouvert sans suivre les junctions. La requête de
taille ne tolère que `ERROR_INSUFFICIENT_BUFFER` ; les erreurs de lecture restent bloquantes.
Aucune normalisation de DACL, réparation après déplacement ou comparaison assouplie n’est ajoutée.
Le test natif Windows compare exactement les ACL héritées avant/après déplacement sous parent
privé, puis retour au profil. Les tests de clonage, DACL protégées, junctions et reprise après
interruption restent requis. Une ancienne sauvegarde dont l’inventaire diffère de la lecture
brute sera refusée par `verify` ; ne pas modifier ses ACL pour forcer son acceptation.

Le lien causal avec `GetSecurityInfo` reste à confirmer sur Windows : aucun runtime NTFS ou
PowerShell 5.1 n’est disponible dans cette session macOS. `pnpm test:node` passe avant et après
correction (**54/54**, `node-before.log`, `node-after.log`) ; ce n’est pas une reproduction
rouge/verte Windows. La revue indépendante n’a relevé aucun défaut concret du correctif.
Les résultats locaux complets figurent dans A1–A3 ; N1 reste ouverte et la release **NO-GO**.

Correction après le run `36503766066`, job `109200402364` : **copie des descripteurs et
buffer de renommage corrigés ; confirmation Windows requise**. Le journal fourni signale
une divergence pendant le clonage et un échec natif du premier renommage, avant le hook
d’interruption. Il n’indique ni le champ divergent ni le code Win32 ; le lien causal précis
avec les défauts ci-dessous ne peut pas être confirmé depuis macOS.

L’inspection a établi deux défauts du helper : il compare propriétaire, groupe et DACL
(masque `7`) mais écrivait seulement la DACL (`4`) ; le nom dans `FILE_RENAME_INFO` était
copié dans un buffer non initialisé sans terminateur UTF-16 explicite. Les copies écrivent
désormais les champs propriétaire/groupe présents, avec `WRITE_OWNER` ; les demandes
`private` restent limitées à la DACL. Un droit insuffisant provoque un refus, sans abandonner
ces champs. Le buffer réserve et écrit deux octets nuls après le nom, exclus de
`FileNameLength`. `RootDirectory` reste nul et aucune réapplication d’ACL ne suit les swaps.

Un test natif Windows impose un groupe primaire différent de celui d’un fichier nouvellement
créé, puis vérifie exactement le descripteur sauvegardé. Le test de renommage utilise un
chemin long avec accents, vérifie le refus d’une destination existante et conserve les
comparaisons d’ACL avant/après aller-retour. Ces tests ne tournent pas sur macOS.

Les diagnostics indiquent désormais le premier index et les seuls noms de champs divergents,
ainsi que le code Win32 numérique filtré. Aucun chemin, descripteur, contenu ou stderr natif
n’est affiché. Les tests portables ont d’abord reproduit l’absence de ces diagnostics, puis
passent après correction ; ils couvrent aussi le refus d’une sortie native contenant des
champs supplémentaires ou une valeur invalide. Aucun workflow ni contrôle n’est assoupli.
Résultats locaux courants : A1–A3. La revue indépendante n’a relevé aucun défaut bloquant.
Rejouer la CI Windows sur le SHA livré ; N1/N5 restent ouvertes et la release **NO-GO**.

Correction après le run `36671892076`, job `109748471193` : **fixture héritée et copie des
DACL protégées corrigées ; confirmation Windows requise**. Le test de renommage échoue
avant tout déplacement : le descripteur de la fixture ne contient aucune ACE `ID`. Le test
ACL échoue dès le snapshot, sur l’entrée 1 (`.skillreg/config.json`) à DACL protégée ; le
message identifie le champ `acl`, sans publier sa valeur. Les autres helpers donnent 55 PASS.

La fixture Windows définit maintenant sur son propre home une DACL protégée avec une ACE
`FullControl` héritée par les dossiers et fichiers enfants pour le SID courant, **avant** de
créer ces enfants. L’existence d’ACE `ID` ne dépend plus des permissions du checkout ; les
assertions d’héritage et les comparaisons intégrales restent obligatoires.

`SetKernelObjectSecurity` ne permet pas de définir la protection d’une DACL de fichier.
Les descripteurs protégés sont écrits via `SetSecurityInfo` avec
`PROTECTED_DACL_SECURITY_INFORMATION`, en conservant propriétaire et groupe présents.
Le code Win32 de retour est vérifié directement. Pour les descripteurs non protégés,
l’écriture brute reste utilisée sans demander un nouveau calcul d’héritage depuis le
parent privé de la sauvegarde. Les handles continuent d’ouvrir les junctions elles-mêmes.
Un test natif supplémentaire vérifie que `private` protège réellement le fichier, puis
que la copie du descripteur sous un autre parent reste exacte après réouverture et snapshot.

Les tests Windows ne s’exécutent pas sur macOS ; leurs échecs sont démontrés par le journal
CI fourni, pas reproduits localement. La valeur divergente du descripteur n’est pas disponible :
le prochain run doit confirmer la conservation exacte, y compris les bits d’auto-héritage.
Aucun workflow ni contrôle n’est assoupli. Résultats locaux courants : A1–A3 ; N1/N5 restent
ouvertes et la release **NO-GO**.

Correction après le run `36673438738`, job `109753191023` : **écriture des bits
d’auto-héritage ajustée ; confirmation Windows requise**. L’échec se produit au clonage,
sur le champ `acl` de l’entrée 0 (`.skillreg`), avant toute restauration ou interruption.
La fixture de cette racine est non protégée et contient des ACE héritées. Le journal ne
donne pas les descripteurs divergents : la perte de `SE_DACL_AUTO_INHERITED` reste le
diagnostic à confirmer, pas une observation native locale.

Le helper lit maintenant le contrôle du descripteur converti avec
`GetSecurityDescriptorControl`. Si `SE_DACL_AUTO_INHERITED` (`0x0400`) est présent,
il ajoute `SE_DACL_AUTO_INHERIT_REQ` (`0x0100`) à la demande d’écriture brute : le
noyau doit consommer cette demande et conserver `AI` dans le descripteur stocké.
La protection reste appliquée explicitement par `SetSecurityInfo` lorsqu’elle est demandée,
puis la copie brute restaure aussi les bits d’auto-héritage dans ce cas. Aucun recalcul depuis
le parent de sauvegarde, privilège supplémentaire ou normalisation de l’inventaire n’est ajouté.

Le nouveau test Windows crée dossier, fichier et junction sous un parent doté d’une ACE
`BU` absente des sources. Il exige `AI` et des ACE `ID` dans les sources, puis l’égalité
intégrale des ACL après écriture **et réouverture**. La cible externe de la junction reste
intacte. Le test protégé couvre désormais `P` seul et `PAI`. Les tests existants de snapshot,
restauration, groupe primaire, renommage et interruption gardent leurs assertions exactes.

`pnpm test:node` passe localement avant correction (**55/55**), mais les quatre tests
spécifiques Windows ne sont pas exécutés sur macOS. Le journal CI fourni démontre l’échec
initial ; aucune reproduction rouge/verte Windows n’est revendiquée. La revue indépendante
n’a relevé aucun défaut bloquant ; le cas `PAI` demandé par la revue a été ajouté.
Aucun workflow ni contrôle n’est modifié. Les résultats courants sont A1–A3 ; le prochain
run Windows doit confirmer la consommation de `AR`, la conservation de `AI`/`P` et des ACE.
N1/N5 restent ouvertes et la release **NO-GO**.

## N2 — Campagne native isolée et découverte des agents

Exécuter les scénarios 1–21 dans un compte OS de test dédié, avec un profil sauvegardé selon N4,
des organisations et accès factices, et le build candidat identifié par empreinte. L’application
native résout le home de ce compte ; ne pas la lancer sur un profil personnel existant.
Utiliser `src-tauri/tests/fixtures/agents/runtime-probe/SKILL.md` comme sentinelle.

1. Préparer successivement 0, 1 puis 3 agents ; relever leur commande/version exacte et créer
   la racine de skills avant un premier démarrage, puis tester sa création après démarrage.
2. Installer la sentinelle via le candidat ; constater dans chaque agent sa découverte et la
   réponse `SKILLREG_RUNTIME_PROBE_OK`. Tester apparition à chaud et après redémarrage séparément.
3. Ajouter puis retirer un agent ; rouvrir l’accueil, vérifier la réconciliation et les états.
4. Rejouer les conflits homonymes physiques/externes, checksum, archives, interruption réseau
   simulée et crash avant/après commit. Comparer inventaires avant/après et les données protégées.
5. Rejouer OFF → vérification → mise à jour manuelle, ON, migration identique/modifiée, projet CLI,
   import local, changement d’entreprise et profil avec espaces/Unicode. Vérifier l’absence
   de bindings simultanés de deux entreprises et la conservation des accès après désinstallation.
6. Conserver uniquement erreurs structurées/codes et captures expurgées. Ne pas exporter config,
   token, variables, URL signée, prompts ou contenu privé dans le rapport.

Pour chaque ligne : SHA/empreinte, OS/architecture, filesystem, version agent, chemin relatif,
type de lien, redémarrage requis, résultat, preuve expurgée, limites. Les tests Rust utilisent
des faux agents et ne remplacent pas ce relevé. macOS x64, Windows et Linux restent désactivés
jusqu’à validation propre de leurs adaptateurs.

État : **non exécutée sur le candidat**. Aucun résultat historique importé comme preuve actuelle.

## N3 — Accessibilité et affichage natifs

Dans la webview du candidat, parcourir login/setup/entreprise/catalogue/détail/accueil/Mes skills/
accès/import/réparation/désinstallation au clavier uniquement. Tab/Shift+Tab restent dans les
dialogues modaux ; le fond est inerte ; le focus initial est non destructif et revient au
déclencheur. Échap ferme hors opération ; une opération en cours ne peut pas être interrompue
par une fermeture qui masque son résultat. Vérifier erreur, réessai, état occupé et double clic.

Avec VoiceOver sur macOS (et lecteur propre aux autres plateformes activées), relever noms,
rôles, états et annonces d’erreur/résultat. Vérifier zoom, texte long et fenêtres 900×600,
standard et maximisée. Mesurer texte normal ≥4,5:1 et indicateurs/focus ≥3:1 sur les couleurs
**effectivement composées** dans la webview, notamment hover/disabled/backdrop.

Mesure statique des tokens sombres avant campagne : texte/fond 16,13:1 ; texte secondaire/carte
5,67:1 ; texte principal/bouton primaire 9,05:1 ; erreur/carte 4,91:1. Le bouton de confirmation
destructif utilise désormais blanc sur rouge 700 (~6,47:1). Ces mesures ne prouvent ni la webview,
ni la transparence des focus, ni les petits contrôles. La bordure carte seule est à 1,39:1 :
ne pas l’utiliser comme seule indication d’un contrôle interactif sans vérification native.

Le dépôt ne définit aucune palette `.light` et Settings n’offre actuellement pas de bascule.
L’ancienne mention de thème clair dans le guide était périmée. Le périmètre reste sombre ; une
exigence clair/sombre doit être résolue explicitement avant GO, sans inventer un test clair.

État : **tests DOM locaux seulement ; campagne native non exécutée**.

## N4 — Sauvegarde, restauration et downgrade de release

Outil interne unique, sans réseau ni home implicite. Employer des chemins absolus explicites,
un profil isolé, une sauvegarde **neuve hors du profil**, puis un dossier de récupération distinct.
Les exemples suivants désignent des fixtures à préparer, jamais un vrai home par défaut :

```sh
node scripts/managed-skills-backup-restore.mjs snapshot --home /fixtures/profil --backup /fixtures/avant-migration
node scripts/managed-skills-backup-restore.mjs verify --home /fixtures/profil --backup /fixtures/avant-migration
node scripts/managed-skills-backup-restore.mjs restore --home /fixtures/profil --backup /fixtures/avant-migration
node scripts/managed-skills-backup-restore.mjs restore --home /fixtures/profil --backup /fixtures/avant-migration --apply --recovery /fixtures/avant-restauration
```

En usine, placer `/fixtures` dans le dépôt ou le dossier de sortie. Fermer d’abord SkillReg,
les agents et toute écriture dans la fixture. Les racines sauvegardées sont `.skillreg`,
`.claude`, `.codex`, `.cursor`, `.agents`, intégralement, avec fichiers, modes/ACL et liens
sans déréférencement. Les projets et cibles externes ne sont jamais restaurés à travers un lien.
Une propriété non préservable ou une sauvegarde altérée provoque un refus avant mutation.
`verify` vérifie la sauvegarde ; il n’exige pas que le profil migré soit encore identique.
La récupération doit résider sur le même filesystem que le profil (renames atomiques). Les
ancêtres sont canonisés, puis les chemins réels comparés ; les liens directs vers un profil,
une sauvegarde/récupération et leurs dossiers de données sont refusés, ainsi que les racines de
volume. Les symlinks Windows et les ACL portées par des symlinks macOS sont refusés plutôt que
copiés imparfaitement ; les junctions Windows disposent de leur propre traitement sans suivre
leur cible. Une interruption avant création du journal ne modifie pas le profil : conserver la
récupération incomplète et utiliser un nouveau chemin après contrôle du profil. La reprise
après interruption de processus autour des swaps est testée ; aucune résistance à une coupure
électrique Windows n’est revendiquée.

Sous Windows, les racines sont renommées par handle et leurs descripteurs sont lus bruts par
`GetKernelObjectSecurity`, afin de comparer les marqueurs d’héritage et de protection sans
interprétation selon le parent. Le clonage restaure aussi propriétaire et groupe lorsqu’ils
sont présents dans le descripteur ; les droits nécessaires doivent être disponibles, sinon
la copie est refusée. Les DACL protégées sont appliquées via `SetSecurityInfo` par handle ;
les autres sont copiées brutes sans recalcul d’héritage. Le nom UTF-16 du renommage est terminé
explicitement, sans inclure le terminateur dans sa longueur. La conservation exacte doit encore
être confirmée en CI Windows.
Aucune réapplication des permissions ne suit le déplacement : une
interruption peut reprendre à partir des inventaires exacts. Les junctions sont ouvertes sans
suivre leur cible ; une destination apparue entre le contrôle et le renommage provoque un refus.
Une sauvegarde ou récupération dont les descripteurs diffèrent de l’inventaire est conservée
et refusée comme ambiguë, y compris avec un inventaire issu de l’ancienne lecture d’ACL.
Ne pas convertir cet inventaire ni modifier les permissions pour forcer une reprise ; inspecter
d’abord les copies et vérifier une sauvegarde complète avant toute migration.

Protocole complet obligatoire :

1. Identifier le **dernier binaire legacy réellement distribué** : version, architecture, origine
   et SHA-256. Le numéro commun 0.3.25 n’identifie pas ce binaire. Aucun artefact legacy n’est
   fourni dans ce dépôt ; le HEAD de base est une source de code, pas une preuve binaire.
2. Dans le profil isolé, créer copies legacy identiques/modifiées, copie projet, lien externe,
   accès factices et plusieurs agents. Inventorier contenus/liens/modes/ACL et données externes.
3. Exécuter snapshot puis verify **avant migration**. Migrer avec le vrai moteur, mettre à jour,
   créer un ajout postérieur à la sauvegarde et conserver un inventaire après migration.
4. Fermer tous les processus, examiner restore sans `--apply`, puis appliquer avec `--recovery`.
   L’état post-migration reste dans cette récupération ; ne pas le supprimer. Après interruption,
   relancer exactement la même commande et le même dossier de récupération. Un état non
   vérifiable est refusé, sans suppression silencieuse.
5. Comparer les fichiers protégés, manifestes, accès factices, liens et permissions/ACL avec la
   sauvegarde. Vérifier les projets et cibles externes intacts. L’outil restaure au point de
   sauvegarde et **ne fusionne pas** automatiquement les ajouts postérieurs.
6. Démarrer le binaire legacy identifié dans ce compte isolé, lire les installations puis réaliser
   une opération legacy locale sur la sentinelle. Refaire l’inventaire et contrôler les accès.
7. Rejouer sauvegarde incomplète, fichier changé, lien ancêtre, restauration interrompue ; consigner
   refus et récupération. Le test Rust prouve les étapes filesystem par vraie migration/MAJ,
   puis appelle le script Node ; il ne simule pas le binaire legacy.

Les trousseaux OS ne sont pas exportés : les accès réels y restent conservés. La fixture utilise
des valeurs factices dans les fichiers protégés et les suites env utilisent le backend mémoire.
Le maintien des accès trousseau avec le binaire legacy relève de l’étape native.

État : **protocole filesystem automatisé ; binaire legacy et exécution native manquants**.
Sans sauvegarde complète antérieure, aucun downgrade supporté ; pas d’autorisation générale.

## N5 — Windows : NTFS et profil standard

Exécuter dans un compte Windows standard dédié **sans élévation**, sur NTFS :

```sh
pnpm install --frozen-lockfile
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml --locked --test managed_windows_permissions -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --locked --test managed_migration --test managed_bindings --test local_skill_import --test managed_release_rollback
```

Consigner statut d’élévation et filesystem avec le résultat. Vérifier création/suppression de
junctions sans privilège symlink, cible conservée, nom espaces/Unicode, conflit de cible et
interdiction d’écriture. Comparer ACL effectives de `.skillreg`, config et manifeste avant/après
remplacement atomique ; tester héritage trop large et refus de durcissement, ancien fichier intact.
Une seconde session OS ne doit lire ni modifier ces fichiers. Ne changer aucun compte ni ACL
système pour faire passer ces tests : seules les fixtures reçoivent des ACL locales.

État : **non exécuté ici** ; les tests réservés Windows ne tournent pas sur macOS. Un run vert
administrateur ne coche pas cette gate.
La suite actuelle couvre héritage trop large, remplacement privé, refus d’une junction étrangère
et création de junction interdite. Le refus effectif de `Set-Acl` pendant le durcissement, avec
ancien manifeste/config intacts, reste à reproduire dans ce profil standard ; le refus d’une
junction ne constitue pas cette preuve.

## N6 / N7 — Autres surfaces, release et observation

App/CLI/site : fournir versions/révisions et résultats de tests des dépôts correspondants.
Vérifier contrat API desktop, validation serveur et checksum, onboarding
`/onboarding?source=desktop`, téléchargement cohérent, CLI utilisateur refusant de muter v2,
CLI projet préservé et publication de commandes avec scope token `write`/`admin` conservée.
Les suites desktop n’attestent pas ces surfaces externes.

Release : sans déclencher le workflow pendant cette étape, contrôler après livraison les
signatures OS et updater, notarisation/agrafage DMG, SHA-256 des artefacts livrés et vrai update
depuis la stable précédente. Le helper choisit `.app.tar.gz` macOS, NSIS `.exe` Windows et
`.AppImage` Linux ; toute plateforme/signature manquante refuse la génération, avant création
du brouillon. Ses tests factices ne vérifient pas cryptographiquement une signature réelle.

Inspecter bundles/sourcemaps/logs CI/release expurgés. Sentry n’est pas intégré au desktop :
aucun package, initialisation ou transport dans `src`, `src-tauri/src` et leurs manifestes.
Cette ligne est non applicable au desktop ; app/site restent à vérifier séparément.

Dogfood uniquement après ses propres gates, cohorte Kairia signée, puis observation à 72 h,
design partners volontaires et revue J+7 (installations/mises à jour, actions requises, erreurs,
migrations partielles). Conserver le lecteur v1 une version stable supplémentaire ; retrait
legacy séparé. Relevés manuels locaux expurgés ; aucune télémétrie de contenu/usage.

État : **preuves externes absentes**. Ces étapes ne sont ni réalisées ni remplacées par une
procédure écrite dans cette PR. Aucune action GitHub ou publication n’est demandée à l’agent.
