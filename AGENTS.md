# skillreg-local : guide des agents

Application de bureau SkillReg (Tauri 2, Rust et React) qui permet d'installer, publier, mettre à jour et configurer des skills d'agents IA sur le poste de l'utilisateur, sans passer par la CLI.

## Produit

- **SkillReg** est un registre privé de skills d'agents IA (fichiers `SKILL.md`) pour Claude Code, Codex et Cursor : publier, versionner, gouverner, découvrir, installer. Produit de Kairia, porté par Axel. Direction : couche d'entreprise neutre entre agents, privée par défaut, avec une distribution sécurisée.
- **Ce dépôt** est l'application de bureau : interface locale pour les utilisateurs qui n'utilisent pas la CLI et pour tout ce qui touche au système de fichiers (installations, mises à jour, variables d'environnement, slash commands). C'est la **priorité produit n° 1** de SkillReg, devant `Tontoon7/skillreg-app` (web, API, CLI) et `Tontoon7/skillreg-website` (site public).
- Elle consomme la même API REST que la CLI (`https://app.skillreg.dev/api/v1`, contrat dans `skillreg-app/API-REFERENCE.md`) et partage ses fichiers de configuration locaux.
- **Distribution** : releases GitHub de `Tontoon7/skillreg-local` pour macOS (arm64 et x64, signées et notarisées), Linux (`.deb`, `.rpm`, AppImage) et Windows ; mise à jour automatique par le plugin updater de Tauri ; page de téléchargement sur https://skillreg.dev/download.
- **Docs de référence** : `DEV-PLAN.md` (architecture, commandes Rust, écrans, API), `ROADMAP.md` (avancement par phase), `docs/plans/2026-03-10-macos-notarization.md`, `docs/plans/2026-05-20-skillreg-env-engine-spec.md` (moteur de variables d'environnement par organisation).
- **État** : phases 1 à 6 terminées (squelette, auth et dashboard, catalogue et installation, skills locales et publication, variables d'environnement et réglages, packaging signé et notarisé), puis moteur de variables par organisation, slash commands, mises à jour automatiques des skills, badges de validation et catalogue public. Le candidat inclut désormais le lot A des skills gérées (stockage canonique, bindings, migration opt-in et parcours collaborateur), repris depuis la branche locale dédiée. La release reste NO-GO selon `docs/plans/2026-07-29-managed-skills-release-checklist.md` ; preuves et protocoles dans `docs/managed-skills-release-validation.md`. Le reste à faire est dans `ROADMAP.md`.

## Carte du code

Stack : Tauri v2, Rust (reqwest, serde, sha2, tar, flate2, dirs, open, keyring-core, tokio), React 19, TypeScript, Vite 6, Tailwind CSS v4, composants de type shadcn/ui copiés localement (sans Radix), Zustand 5, React Router v7, react-markdown + rehype-sanitize, lucide-react, plugins Tauri dialog, notification, updater, process et autostart. pnpm, Biome, Cargo.

| Chemin | Rôle |
| --- | --- |
| `src-tauri/src/lib.rs` | Setup Tauri (tray, menu, plugins) et enregistrement des commandes dans `generate_handler!` |
| `src-tauri/src/commands/auth.rs` | Device flow, login par token, `whoami`, logout, `open_url` |
| `src-tauri/src/commands/skills.rs` | Registre : liste, détail, recherche, pull/installation (checksum, extraction), push, désinstallation, mises à jour |
| `src-tauri/src/commands/collaboration.rs` | Propositions de changement de skill |
| `src-tauri/src/commands/local.rs` | Scan des skills installées et parsing du frontmatter |
| `src-tauri/src/commands/env.rs` | Variables d'environnement par organisation, trousseau du système, fichier de repli, migration de l'ancien format |
| `src-tauri/src/commands/config.rs` | Lecture et écriture de `~/.skillreg/config.json` |
| `src-tauri/src/managed_skills/` | Moteur géré : manifeste v2, stockage canonique, transactions, archive, détection, bindings, migration, import, permissions |
| `src-tauri/src/commands/managed_skills.rs`, `managed_migration.rs`, `local_import.rs` | IPC du cycle géré, migration opt-in et import local |
| `src-tauri/src/commands/auto_update.rs`, `installed_manifest.rs` | Worker managed et lecture/protection legacy `~/.skillreg/installed.json` |
| `src-tauri/src/commands/slash_commands.rs` | Slash commands du registre : installation, mise à jour, suppression locale, publication de versions ; manifeste `~/.skillreg/commands.json` |
| `src-tauri/src/commands/api_error.rs` | Mise en forme des erreurs d'API (limites de plan, paiement) |
| `src-tauri/tauri.conf.json`, `capabilities/`, `icons/` | Configuration Tauri (identifiant `com.skillreg.local`, updater), permissions, icônes |
| `src-tauri/gen/schemas/` | Schémas générés par Tauri : ne pas éditer à la main |
| `src-tauri/tests/tray_config.rs` | Test d'intégration Rust sur `tauri.conf.json` |
| `src/App.tsx`, `src/main.tsx` | Routes, garde d'authentification, route de setup ; point d'entrée et restauration du thème |
| `src/pages/` | Login, Setup, Dashboard, Catalog, PublicCatalog, SkillDetail, Commands, Installed, EnvVars, Settings |
| `src/components/` | Dialogues (publication, proposition, suppression, variables), `UpdateChecker`, `ValidationBadge`, `layout/` (AppShell, Sidebar, Titlebar), `ui/` |
| `src/components/PublishCommandDialog.tsx` | Publication d'une version de commande existante depuis Commands : contenu brut, version, agents et portée |
| `src/lib/api.ts` | Wrappers `invoke()` typés, seule porte vers le backend Rust |
| `src/lib/store.ts`, `types.ts`, `employee-model.ts`, `constants.ts` | Stores auth/config/managed, modèle collaborateur et contrats Rust/TypeScript |
| `src/lib/*.ts` (autres) | Logique sans IPC : inventaire des variables (`env-inventory.ts`), regroupement des skills installées (`installed-skill-groups.ts`), préremplissage et validation de publication de commandes (`command-publishing.ts`), testés dans `tests/` ; actions locales, notifications, couleurs de tags, `cn()` |
| `src/styles/globals.css` | Base Tailwind, palette sombre, prose |
| `tests/*.test.ts` | Tests Node : logique pure, release/updater et outil de sauvegarde/restauration |
| `src/**/__tests__/`, `src/test/`, `vitest.config.ts` | Tests React/Testing Library et mock IPC strict (commande non préparée = erreur) |
| `src/components/dashboard/`, `skills/`, `settings/` | Parcours collaborateur, confirmations natives et import local |
| `src-tauri/tests/managed_*.rs`, `local_skill_import.rs`, `active_org_reconcile.rs` | Intégration filesystem isolée, recovery, migration, bindings, droits Windows, rollback de release |
| `scripts/managed-skills-backup-restore.mjs` | Outil support interne snapshot/verify/restore d’un profil explicite, pas de home implicite |
| `scripts/generate-updater-manifest.mjs` | Prépare les quatre artefacts updater et signatures avant génération de latest.json |
| `.github/workflows/ci.yml` | CI de vérification Linux/macOS/Windows sans secret ni publication |
| `scripts/check-release-notarization.sh` | Garde-fou : vérifie que `release.yml` notarise toujours les DMG |

## Commandes

```bash
pnpm install --frozen-lockfile   # installation (identique à l'usine), pnpm 9
pnpm format:check                # Biome sur le frontend (ignore src-tauri/)
pnpm format                      # Biome avec corrections (--write)
pnpm build                       # tsc (typage de src/) puis vite build vers dist/
pnpm dev                         # Vite seul sur localhost:1420 ; ne s'arrête jamais
pnpm tauri dev                   # application complète ; ne s'arrête jamais
pnpm tauri build                 # packaging complet de l'application, lourd
pnpm tauri:build:local           # packaging local non signé, sans artefacts d'updater
cargo check --manifest-path src-tauri/Cargo.toml --locked             # compilation Rust sans lancer l'app
cargo test --manifest-path src-tauri/Cargo.toml --locked              # tests Rust (unitaires et src-tauri/tests)
cargo test --manifest-path src-tauri/Cargo.toml --locked <nom>        # tests Rust dont le nom contient <nom>
pnpm test                       # Node puis Vitest (React/jsdom)
pnpm test:node                  # node --test sur tests/*.test.ts
pnpm test:frontend              # Vitest run ; peut recevoir un chemin de test
node --test --experimental-strip-types tests/env-inventory.test.ts   # un fichier
bash scripts/check-release-notarization.sh                   # après une modification de release.yml (requiert rg)
```

**Validation de l'usine pour ce candidat** : `pnpm format:check && pnpm build && cargo test --manifest-path src-tauri/Cargo.toml --locked && node --test --experimental-strip-types tests/*.test.ts`.

Elle couvre Rust et Node mais **pas Vitest** : lancer aussi `pnpm test:frontend`, `cargo check --manifest-path src-tauri/Cargo.toml --locked` et la garde notarisation après modification du workflow. `pnpm build` doit précéder Cargo sur tous les OS, car Tauri référence `dist/`. Signaler les résultats et limites sandbox ; ne jamais assimiler une fixture à une preuve runtime d'agent.

Dans l'usine, ne lance jamais `pnpm tauri dev`, `pnpm dev` ou `pnpm tauri build` : les deux premiers ne s'arrêtent pas, le troisième est un packaging lourd qui n'est pas demandé.

## Conventions

- Réponses à Axel en français, commentaires de code en anglais, seulement pour une logique non évidente.
- TypeScript strict, pas de `any`. Pas de sur-ingénierie.
- Biome pour le frontend : tabulations, 100 colonnes, imports triés ; `src-tauri/` est exclu. Rust : conventions rustfmt. `.gitattributes` impose LF aux fichiers texte sur tous les OS et laisse les binaires inchangés.
- **HTTP uniquement par Rust** : toute requête vers l'API passe par une commande Rust (`reqwest`) appelée via `invoke()` depuis `src/lib/api.ts`. Le frontend ne fait jamais de `fetch` : cela évite les problèmes CORS du webview et garde le réseau et le système de fichiers côté natif. Schéma : `invoke("commande")` → Rust `reqwest` → API → résultat Rust → frontend.
- Commandes Rust dans `src-tauri/src/commands/`, déclarées dans `commands/mod.rs` et enregistrées dans `lib.rs` ; les commandes legacy renvoient `Result<T, String>` et les nouvelles commandes managed `Result<T, ManagedErrorDto>` (code stable, paramètres filtrés). Les structures échangées avec TypeScript dérivent `Serialize`/`Deserialize` en `camelCase` et restent alignées avec `src/lib/types.ts`.
- **Publication des commandes** : `publishCommandVersion()` → `publish_command_version` → POST `/api/v1/orgs/{org}/commands/{name}/versions`, avec `version`, `content`, `agentCompatibility` et `scope` explicites, réponse `{ version }`. L'API exige le scope de jeton `write` ou `admin` ; ne pas déduire cette autorisation du rôle d'organisation. Le dialogue reste lié à l'organisation et au nom d'origine, sans retry automatique ni modification des installations locales ; créer une nouvelle commande reste hors de ce parcours.
- Pages dans `src/pages/`, composants réutilisables dans `src/components/`, stores dans `src/lib/store.ts`, wrappers IPC dans `src/lib/api.ts`. Les pages lisent directement les stores (`useAuthStore` : authentification, utilisateur, organisations ; `useConfigStore` : organisation, agent, portée, `setupDone`), sans prop drilling.
- **Auth** : device flow recommandé (`login_initiate` → POST `/api/v1/auth/cli/initiate`, `open_url`, affichage du `userCode`, `login_poll` toutes les 3 s jusqu'à `status: "complete"`) ou collage d'un token `sr_live_*`, `sr_test_*` ou `sk_*` (`login_with_token` vérifie le format puis appelle `whoami`). Le token est enregistré dans `~/.skillreg/config.json`.
- **Setup** : après connexion, `/setup` sélectionne l’entreprise (automatique si unique), détecte les assistants et propose la migration après aperçu ; aucun agent/scope/version à choisir dans le parcours principal. Les réglages avancés des commandes restent distincts.
- **Thème** : palette sombre uniquement dans `src/styles/globals.css` ; aucune palette `.light` ni bascule Settings dans ce candidat. Ne pas revendiquer une validation du mode clair.
- **Interopérabilité legacy avec la CLI** : mêmes fichiers (`~/.skillreg/config.json`, `.skillregrc`, `~/.skillreg/env/`), mêmes chemins d'installation (projet : `.claude/skills/`, `.codex/skills/`, `.cursor/skills/` ; utilisateur : les mêmes sous `~/`). Un changement de contrat d'API se coordonne avec `skillreg-app` (routes, CLI, `API-REFERENCE.md`).
- **Skills gérées** : `~/.skillreg/managed-skills.json` version 2 distinct de `installed.json`, contenu `~/.skillreg/skills/<consumer-org>/<source-org>/<name>/content`, verrou de mutation commun et swaps vérifiés par hash committé. Le frontend appelle seulement les wrappers IPC managed ; Rust porte réseau et filesystem.
- **Bindings** : Claude `.claude/skills`, Codex `.agents/skills` (ancien `.codex/skills` seulement reconnu), Cursor `.cursor/skills`. macOS arm64 seul activé dans le code ; x64, Windows et Linux attendent leurs preuves runtime. Aucune copie physique de repli. La CLI externe doit prouver refus de mutation utilisateur v2 et conservation du scope projet avant release.
- **Rollback de release** : sauvegarde intégrale antérieure via l’outil interne snapshot/verify/restore, home et chemins absolus explicites. Restore est un aperçu sans `--apply --recovery <dossier distinct>`. Le journal permet une reprise ; l’état remplacé reste conservé. Les trousseaux ne sont jamais exportés. Nécessite Node et outils ACL natifs (Linux : paquet `acl`). Le test filesystem ne valide pas un binaire legacy réel.
- **Confidentialité** : erreurs managed exposées par code constant ; seul le statut HTTP validé peut être persisté comme paramètre. Les handlers globaux frontend ne journalisent pas les payloads d’erreur. Aucune intégration Sentry ni télémétrie d’usage dans le desktop.
- **TDD** pour la logique (installation, archives, variables, parsing, packaging, commandes Rust) : test Rust dans un module `#[cfg(test)]` ou `src-tauri/tests/`, test TypeScript dans `tests/`. Un test ne touche ni le vrai `~/.skillreg` ni le trousseau du système : dossier temporaire et `MemoryCredentialBackend`, comme les tests existants de `env.rs` et `installed_manifest.rs`.
- UI : garder l'esthétique sombre d'outil développeur et les composants existants.
- **Secrets** : ne jamais copier de valeur de `.env`, de `.claude/settings.local.json`, de clé de signature Tauri ou d'identifiant Apple dans le code, les tests ou la doc.
- `ROADMAP.md` : ne le modifie que si le ticket le demande (voir « Sessions avec Axel »).

## Livraison

- Branche cible : `main`. `.github/workflows/ci.yml` vérifie les PR et `main` sur Linux/macOS/Windows (format, build, Node, Vitest, Cargo verrouillé). Cette CI ne signe ni ne publie ; ses résultats doivent être rattachés au candidat avant GO.
- Release (`.github/workflows/release.yml`) : sur un tag `v*`, build Tauri pour macOS arm64 et x64 (signature Developer ID, notarisation et agrafage des DMG), Linux et Windows, puis création d'une release GitHub **en brouillon** avec les installeurs et `latest.json` pour l'updater. Axel publie ensuite le brouillon. Un build complet peut durer plus de deux heures.
- Une release commence par un commit `chore: release x.y.z` qui change la version dans `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` et `src-tauri/tauri.conf.json`, puis le tag `vx.y.z`. Ni tag ni bump de version sans demande d'Axel.
- Politique de l'usine pour ce dépôt : livraison `review`. L'usine ouvre une PR et la laisse en « À valider » ; Axel fusionne et publie une release manuellement.

## Travail dans l'usine

Les tickets de ce dépôt sont exécutés par l'usine de développement d'Axel (Codex, Claude Code ou Cursor sur son Mac mini). Dans ce cadre :
- ne commite pas, ne pousse pas, n'ouvre pas de PR et ne déploie pas : le moteur s'en charge après validation et revue. Cette règle prime sur toute consigne contraire de ce dépôt ;
- la validation bloquante est celle de la section Commandes ; la compléter par Vitest avant de conclure ;
- mets ce fichier à jour quand ton travail change une commande, une convention ou l'architecture, et complète « Pièges connus ».

## Sessions avec Axel (hors usine)

- Avant de coder : relire `DEV-PLAN.md` et `ROADMAP.md`. Après avoir codé : mettre à jour `ROADMAP.md` (cocher les items terminés) et `DEV-PLAN.md` si l'architecture change.
- Vérifier le comportement réel dans l'application avec `pnpm tauri dev` (système de fichiers, IPC, tray) quand le changement touche l'utilisateur ; le serveur Vite seul (`pnpm dev`, port 1420) suffit pour de la mise en page.
- `pnpm tauri build` est autorisé pour vérifier un packaging local. Tags de release, signature, notarisation et publication de release sont autorisés quand ils font partie de la tâche demandée et que les identifiants existent déjà.
- Axel gère normalement les commits et les pushs ; commit, tag et push seulement quand il demande de finaliser ou de livrer. Demander avant : dépendance majeure, commande destructive, modification de secrets ou de `.env`, travail hors périmètre.
- Copie de travail d'Axel : `/Users/axel/dev/skillreg/skillreg-local`, souvent avec du travail non commité ; lancer `git status --short` avant d'éditer. Le guide transverse des trois dépôts est `/Users/axel/dev/skillreg/AGENTS.md`.

## Pièges connus

- Windows : sans `* text=auto eol=lf` dans `.gitattributes`, un checkout avec `core.autocrlf=true` convertit les sources en CRLF et fait échouer `pnpm format:check`. Reproduire par un export `git -c core.autocrlf=true checkout-index --all --prefix=<dossier-absolu-vide>/`, puis lancer le contrôle dans cet export.
- `pnpm build` type `src/`, pas `tests/` ni Rust. La validation usine du candidat lance Rust et Node mais oublie Vitest : compléter par `pnpm test:frontend` ; la CI trois OS couvre les trois suites.
- `cargo check` et `cargo test` compilent `tauri.conf.json`, qui référence `../dist` : lance `pnpm build` avant dans un worktree neuf. Le premier build Rust d'un worktree est long (dépendances Tauri complètes).
- `src-tauri/src/commands/skills.rs` n'est pas au format rustfmt : `cargo fmt` sur tout le crate reformate du code sans rapport avec le ticket. Formate seulement tes fichiers (`rustfmt --edition 2021 <fichier>`).
- L'URL de l'API `https://app.skillreg.dev` est codée en dur dans `src-tauri/src/commands/auth.rs`, `skills.rs`, `collaboration.rs` et `src/lib/constants.ts` : un changement doit toucher les quatre.
- La version de l'application vit dans `package.json`, `src-tauri/Cargo.toml` (et `Cargo.lock`) et `src-tauri/tauri.conf.json` ; ils doivent rester identiques.
- `~/.skillreg/config.json` est écrit par Rust (login) et par le frontend : passe par `useConfigStore.update()`, qui relit le disque avant de fusionner, sinon le token écrit par `login_poll` est écrasé.
- L'icône de tray est créée en Rust (`lib.rs`), pas dans `tauri.conf.json` ; `src-tauri/tests/tray_config.rs` échoue si `app.trayIcon` y est ajouté.
- Updater : `bundle.createUpdaterArtifacts=true` exige `.app.tar.gz` sur macOS, `.exe` NSIS sur Windows et `.AppImage` sur Linux, chacun avec sa propre `.sig`. Le helper refuse plateforme/signature manquante avant création du brouillon ; tests factices ≠ signature cryptographique réelle. `plugins.updater.pubkey` doit correspondre à la clé de release. `pnpm tauri:build:local` désactive la signature uniquement en local.
- `.gitignore` exclut `*.png`, `*.jpg` et `*.jpeg` hors `src-tauri/icons/*.png` : une image ajoutée ailleurs n'est pas commitée, sans avertissement.
- `src-tauri/gen/schemas/desktop-schema 2.json` et `desktop-schema 3.json` sont des doublons iCloud commités par erreur : ne les modifie pas et ne crée aucun fichier suffixé ` 2`.
- `scripts/check-release-notarization.sh` requiert `rg` dans le PATH ; sa présence est une condition du contrôle, pas une notarisation réelle.
- Le setup ouvre `https://app.skillreg.dev/onboarding?source=desktop` pour créer un workspace : une release du desktop qui contient ce parcours doit suivre le déploiement de l'app qui sert `/onboarding`.
- Publication de commandes (`slash_commands.rs`) : reprendre le contenu brut du registre, car les fichiers installés pour Claude/Codex ajoutent des enveloppes ; la portée de publication inclut `org`, contrairement à `ScopeType` (`project`/`user`) réservé aux installations. Pour valider la longueur, `trim_command_publication_text()` suit le `trim()` JavaScript : BOM U+FEFF retiré et NEL U+0085 conservé, à l'inverse de `str::trim()` Rust.
- Windows : une junction/reparse point n’est pas forcément `is_symlink()` ; employer le contrôle de `platform_links` et sa suppression dédiée, jamais `remove_file` pour retirer un binding junction. Les tests administrateur ne prouvent pas les droits d’un compte standard.
- Sauvegarde Windows : Node transmet stdin en UTF-8, contrairement à l’encodage console implicite de Windows PowerShell 5.1. Lire les chemins/JSON avec `StreamReader(Console.OpenStandardInput(), Encoding.UTF8)` ; conserver la fixture `profil été` de `pnpm test:node` pour couvrir les ACL et junctions avec accents. Les erreurs de restauration précisent la phase ; conserver les blocs `not ok` complets, car le seul total PASS/FAIL ne localise pas une panne. `pnpm test:node` garde `--test-reporter=spec` : hors terminal, le reporter TAP par défaut laisse les erreurs au milieu du journal, alors que spec les répète après les totaux, dans la fin transmise par l’usine. Les tests d’interruption doivent vérifier l’exception du hook, pas accepter toute erreur de préparation.
- PowerShell Windows : importer `Microsoft.PowerShell.Security` depuis `$PSHOME/Modules` avant `Get-Acl`/`Set-Acl`, car `powershell.exe` peut hériter d’un `PSModulePath` incompatible de `pwsh`. Le test ACL Node injecte un module homonyme incompatible. Les comparaisons d’inventaire après déplacement restent hors du hook `afterRename` : `during()` masque les détails des `AssertionError` derrière `ERR_ASSERTION`.
- Restauration Windows : `renameSync` peut convertir la DACL héritée d’une racine déplacée en DACL protégée explicite (`D:P`, marqueurs `ID` perdus). Les swaps de `managed-skills-backup-restore.mjs` utilisent `SetFileInformationByHandle(FileRenameInfo)` avec `OPEN_REPARSE_POINT`, sans écraser la destination ni réappliquer les ACL après déplacement ; cette réapplication laisserait une fenêtre de panne. Le branchement PowerShell de renommage doit émettre un marqueur (`renamed`) : une méthode `void` laisse un pipeline vide sous PowerShell 5.1, susceptible de produire une stdout vide au lieu de JSON. Seules les requêtes exclusivement de renommage acceptent ce vide ; une réponse ACL vide reste une erreur. Garder la comparaison exacte des inventaires avant et après activation dans `pnpm test:node` ; un PASS macOS ne valide pas ce chemin Windows.
- `managed-skills.json.tmp` préexistant est préservé et bloque l’écriture : inspecter/récupérer une transaction interrompue avant retrait. Récupérer le swap avant le contrôle de hash des mises à jour, sinon un crash devient à tort `ContentModified`. Une erreur de récupération doit être persistée par skill (`ActionRequired`) puis le cycle doit continuer ; seul un échec global de manifeste doit l’interrompre. `previous` ne peut être nettoyé que si son état est prouvé par le hash du manifeste ; un rollback d’opération ne prouve pas le downgrade d’une release.
