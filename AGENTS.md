# skillreg-local : guide des agents

Application de bureau SkillReg (Tauri 2, Rust et React) qui permet d'installer, publier, mettre à jour et configurer des skills d'agents IA sur le poste de l'utilisateur, sans passer par la CLI.

## Produit

- **SkillReg** est un registre privé de skills d'agents IA (fichiers `SKILL.md`) pour Claude Code, Codex et Cursor : publier, versionner, gouverner, découvrir, installer. Produit de Kairia, porté par Axel. Direction : couche d'entreprise neutre entre agents, privée par défaut, avec une distribution sécurisée.
- **Ce dépôt** est l'application de bureau : interface locale pour les utilisateurs qui n'utilisent pas la CLI et pour tout ce qui touche au système de fichiers (installations, mises à jour, variables d'environnement, slash commands). C'est la **priorité produit n° 1** de SkillReg, devant `Tontoon7/skillreg-app` (web, API, CLI) et `Tontoon7/skillreg-website` (site public).
- Elle consomme la même API REST que la CLI (`https://app.skillreg.dev/api/v1`, contrat dans `skillreg-app/API-REFERENCE.md`) et partage ses fichiers de configuration locaux.
- **Distribution** : releases GitHub de `Tontoon7/skillreg-local` pour macOS (arm64 et x64, signées et notarisées), Linux (`.deb`, `.rpm`, AppImage) et Windows ; mise à jour automatique par le plugin updater de Tauri ; page de téléchargement sur https://skillreg.dev/download.
- **Docs de référence** : `DEV-PLAN.md` (architecture, commandes Rust, écrans, API), `ROADMAP.md` (avancement par phase), `docs/plans/2026-03-10-macos-notarization.md`, `docs/plans/2026-05-20-skillreg-env-engine-spec.md` (moteur de variables d'environnement par organisation).
- **État** : phases 1 à 6 terminées (squelette, auth et dashboard, catalogue et installation, skills locales et publication, variables d'environnement et réglages, packaging signé et notarisé), puis moteur de variables par organisation, slash commands, mises à jour automatiques des skills, badges de validation et catalogue public. Les secrets de signature et de notarisation Apple sont configurés dans GitHub. Le reste à faire est dans `ROADMAP.md`.

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
| `src-tauri/src/commands/auto_update.rs`, `installed_manifest.rs` | Mises à jour automatiques des skills, manifeste `~/.skillreg/installed.json` |
| `src-tauri/src/commands/slash_commands.rs` | Slash commands du registre, manifeste `~/.skillreg/commands.json` |
| `src-tauri/src/commands/api_error.rs` | Mise en forme des erreurs d'API (limites de plan, paiement) |
| `src-tauri/tauri.conf.json`, `capabilities/`, `icons/` | Configuration Tauri (identifiant `com.skillreg.local`, updater), permissions, icônes |
| `src-tauri/gen/schemas/` | Schémas générés par Tauri : ne pas éditer à la main |
| `src-tauri/tests/tray_config.rs` | Test d'intégration Rust sur `tauri.conf.json` |
| `src/App.tsx`, `src/main.tsx` | Routes, garde d'authentification, route de setup ; point d'entrée et restauration du thème |
| `src/pages/` | Login, Setup, Dashboard, Catalog, PublicCatalog, SkillDetail, Commands, Installed, EnvVars, Settings |
| `src/components/` | Dialogues (publication, proposition, suppression, variables), `UpdateChecker`, `ValidationBadge`, `layout/` (AppShell, Sidebar, Titlebar), `ui/` |
| `src/lib/api.ts` | Wrappers `invoke()` typés, seule porte vers le backend Rust |
| `src/lib/store.ts`, `types.ts`, `constants.ts` | Stores Zustand, types partagés Rust et TypeScript, `API_BASE_URL`, agents, portées |
| `src/lib/*.ts` (autres) | Logique sans IPC : inventaire des variables (`env-inventory.ts`) et regroupement des skills installées (`installed-skill-groups.ts`), testés dans `tests/` ; actions locales, notifications, couleurs de tags, `cn()` |
| `src/styles/globals.css` | Base Tailwind, variables des thèmes sombre et clair, prose |
| `tests/*.test.ts` | Tests `node:test` de la logique de `src/lib` |
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
cargo check --manifest-path src-tauri/Cargo.toml             # compilation Rust sans lancer l'app
cargo test --manifest-path src-tauri/Cargo.toml              # tests Rust (unitaires et src-tauri/tests)
cargo test --manifest-path src-tauri/Cargo.toml <nom>        # tests Rust dont le nom contient <nom>
pnpm test                                                    # tests TypeScript (node --test sur tests/*.test.ts)
node --test --experimental-strip-types tests/env-inventory.test.ts   # un fichier
bash scripts/check-release-notarization.sh                   # après une modification de release.yml (requiert rg)
```

**Validation de l'usine** (commande `check` de la configuration active) : `pnpm format:check && pnpm build`.

Elle ne couvre ni le Rust ni les tests TypeScript. Si tu modifies `src-tauri/`, lance aussi `cargo test --manifest-path src-tauri/Cargo.toml` après `pnpm build` (qui produit `dist/`, référencé par `tauri.conf.json`) ; si tu modifies `src/lib/` ou `tests/`, lance `pnpm test`. Signale dans ta conclusion ce que tu as lancé.

Dans l'usine, ne lance jamais `pnpm tauri dev`, `pnpm dev` ou `pnpm tauri build` : les deux premiers ne s'arrêtent pas, le troisième est un packaging lourd qui n'est pas demandé.

## Conventions

- Réponses à Axel en français, commentaires de code en anglais, seulement pour une logique non évidente.
- TypeScript strict, pas de `any`. Pas de sur-ingénierie.
- Biome pour le frontend : tabulations, 100 colonnes, imports triés ; `src-tauri/` est exclu. Rust : conventions rustfmt.
- **HTTP uniquement par Rust** : toute requête vers l'API passe par une commande Rust (`reqwest`) appelée via `invoke()` depuis `src/lib/api.ts`. Le frontend ne fait jamais de `fetch` : cela évite les problèmes CORS du webview et garde le réseau et le système de fichiers côté natif. Schéma : `invoke("commande")` → Rust `reqwest` → API → résultat Rust → frontend.
- Commandes Rust dans `src-tauri/src/commands/`, déclarées dans `commands/mod.rs` et enregistrées dans `lib.rs` ; elles renvoient `Result<T, String>`. Les structures échangées avec TypeScript dérivent `Serialize`/`Deserialize` en `camelCase` et restent alignées avec `src/lib/types.ts`.
- Pages dans `src/pages/`, composants réutilisables dans `src/components/`, stores dans `src/lib/store.ts`, wrappers IPC dans `src/lib/api.ts`. Les pages lisent directement les stores (`useAuthStore` : authentification, utilisateur, organisations ; `useConfigStore` : organisation, agent, portée, `setupDone`), sans prop drilling.
- **Auth** : device flow recommandé (`login_initiate` → POST `/api/v1/auth/cli/initiate`, `open_url`, affichage du `userCode`, `login_poll` toutes les 3 s jusqu'à `status: "complete"`) ou collage d'un token `sr_live_*`, `sr_test_*` ou `sk_*` (`login_with_token` vérifie le format puis appelle `whoami`). Le token est enregistré dans `~/.skillreg/config.json`.
- **Setup** : après la première connexion, si `setupDone` est faux, redirection vers `/setup` (organisation, agent par défaut claude/codex/cursor, portée par défaut project/user).
- **Thème** : sombre par défaut, bascule dans Settings, persisté dans `localStorage` (`skillreg-theme`), variables CSS dans `src/styles/globals.css` (classe `.light` sur `<html>`).
- **Interopérabilité avec la CLI** : mêmes fichiers (`~/.skillreg/config.json`, `.skillregrc`, `~/.skillreg/env/`), mêmes chemins d'installation (projet : `.claude/skills/`, `.codex/skills/`, `.cursor/skills/` ; utilisateur : les mêmes sous `~/`). Un changement de contrat d'API se coordonne avec `skillreg-app` (routes, CLI, `API-REFERENCE.md`).
- **TDD** pour la logique (installation, archives, variables, parsing, packaging, commandes Rust) : test Rust dans un module `#[cfg(test)]` ou `src-tauri/tests/`, test TypeScript dans `tests/`. Un test ne touche ni le vrai `~/.skillreg` ni le trousseau du système : dossier temporaire et `MemoryCredentialBackend`, comme les tests existants de `env.rs` et `installed_manifest.rs`.
- UI : garder l'esthétique sombre d'outil développeur et les composants existants.
- **Secrets** : ne jamais copier de valeur de `.env`, de `.claude/settings.local.json`, de clé de signature Tauri ou d'identifiant Apple dans le code, les tests ou la doc.
- `ROADMAP.md` : ne le modifie que si le ticket le demande (voir « Sessions avec Axel »).

## Livraison

- Branche cible : `main`. Aucune CI ne tourne sur les PR ni sur `main`.
- Release (`.github/workflows/release.yml`) : sur un tag `v*`, build Tauri pour macOS arm64 et x64 (signature Developer ID, notarisation et agrafage des DMG), Linux et Windows, puis création d'une release GitHub **en brouillon** avec les installeurs et `latest.json` pour l'updater. Axel publie ensuite le brouillon. Un build complet peut durer plus de deux heures.
- Une release commence par un commit `chore: release x.y.z` qui change la version dans `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` et `src-tauri/tauri.conf.json`, puis le tag `vx.y.z`. Ni tag ni bump de version sans demande d'Axel.
- Politique de l'usine pour ce dépôt : livraison `review`. L'usine ouvre une PR et la laisse en « À valider » ; Axel fusionne et publie une release manuellement.

## Travail dans l'usine

Les tickets de ce dépôt sont exécutés par l'usine de développement d'Axel (Codex, Claude Code ou Cursor sur son Mac mini). Dans ce cadre :
- ne commite pas, ne pousse pas, n'ouvre pas de PR et ne déploie pas : le moteur s'en charge après validation et revue. Cette règle prime sur toute consigne contraire de ce dépôt ;
- la validation bloquante est `pnpm format:check && pnpm build` ; fais-la passer avant de conclure ;
- mets ce fichier à jour quand ton travail change une commande, une convention ou l'architecture, et complète « Pièges connus ».

## Sessions avec Axel (hors usine)

- Avant de coder : relire `DEV-PLAN.md` et `ROADMAP.md`. Après avoir codé : mettre à jour `ROADMAP.md` (cocher les items terminés) et `DEV-PLAN.md` si l'architecture change.
- Vérifier le comportement réel dans l'application avec `pnpm tauri dev` (système de fichiers, IPC, tray) quand le changement touche l'utilisateur ; le serveur Vite seul (`pnpm dev`, port 1420) suffit pour de la mise en page.
- `pnpm tauri build` est autorisé pour vérifier un packaging local. Tags de release, signature, notarisation et publication de release sont autorisés quand ils font partie de la tâche demandée et que les identifiants existent déjà.
- Axel gère normalement les commits et les pushs ; commit, tag et push seulement quand il demande de finaliser ou de livrer. Demander avant : dépendance majeure, commande destructive, modification de secrets ou de `.env`, travail hors périmètre.
- Copie de travail d'Axel : `/Users/axel/dev/skillreg/skillreg-local`, souvent avec du travail non commité ; lancer `git status --short` avant d'éditer. Le guide transverse des trois dépôts est `/Users/axel/dev/skillreg/AGENTS.md`.

## Pièges connus

- La validation de l'usine ne compile pas le Rust : `pnpm build` ne type que `src/` (`tsconfig.json`, `include: ["src"]`) ; ni `src-tauri/` ni `tests/` ne sont vérifiés.
- Les tests `tests/*.test.ts` (`pnpm test`) ne sont lancés par aucune CI ni par la validation de l'usine ; `tsc` ne les type pas.
- `cargo check` et `cargo test` compilent `tauri.conf.json`, qui référence `../dist` : lance `pnpm build` avant dans un worktree neuf. Le premier build Rust d'un worktree est long (dépendances Tauri complètes).
- `src-tauri/src/commands/skills.rs` n'est pas au format rustfmt : `cargo fmt` sur tout le crate reformate du code sans rapport avec le ticket. Formate seulement tes fichiers (`rustfmt --edition 2021 <fichier>`).
- L'URL de l'API `https://app.skillreg.dev` est codée en dur dans `src-tauri/src/commands/auth.rs`, `skills.rs`, `collaboration.rs` et `src/lib/constants.ts` : un changement doit toucher les quatre.
- La version de l'application vit dans `package.json`, `src-tauri/Cargo.toml` (et `Cargo.lock`) et `src-tauri/tauri.conf.json` ; ils doivent rester identiques.
- `~/.skillreg/config.json` est écrit par Rust (login) et par le frontend : passe par `useConfigStore.update()`, qui relit le disque avant de fusionner, sinon le token écrit par `login_poll` est écrasé.
- L'icône de tray est créée en Rust (`lib.rs`), pas dans `tauri.conf.json` ; `src-tauri/tests/tray_config.rs` échoue si `app.trayIcon` y est ajouté.
- Updater : `plugins.updater.pubkey` doit correspondre au secret `TAURI_SIGNING_PRIVATE_KEY`. `bundle.createUpdaterArtifacts` vaut `true` (format natif Tauri 2, verrouillé par `tests/release-hardening.test.ts`) : macOS garde `.app.tar.gz` et `.sig`, mais Windows et Linux signent directement l'installeur (`.exe`, `.msi`, `.AppImage` avec leur `.sig`), alors que `release.yml` cherche encore `.nsis.zip` et `.AppImage.tar.gz` pour `latest.json`. À aligner avant la prochaine release, sinon Windows et Linux ne reçoivent plus de mise à jour automatique. `pnpm tauri:build:local` désactive ces artefacts (`src-tauri/tauri.local.conf.json`).
- `.gitignore` exclut `*.png`, `*.jpg` et `*.jpeg` hors `src-tauri/icons/*.png` : une image ajoutée ailleurs n'est pas commitée, sans avertissement.
- `src-tauri/gen/schemas/desktop-schema 2.json` et `desktop-schema 3.json` sont des doublons iCloud commités par erreur : ne les modifie pas et ne crée aucun fichier suffixé ` 2`.
- `scripts/check-release-notarization.sh` utilise `rg`, absent du PATH de l'usine sur le Mac mini : il y échoue même quand `release.yml` est correct.
- Le setup ouvre `https://app.skillreg.dev/onboarding?source=desktop` pour créer un workspace : une release du desktop qui contient ce parcours doit suivre le déploiement de l'app qui sert `/onboarding`.
