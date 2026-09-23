# SkillReg Local — Desktop App (Tauri v2)

> Application desktop cross-platform pour SkillReg.
> Alternative visuelle au CLI pour les utilisateurs non-techniques.

---

## 1. Contexte

Le CLI SkillReg (`@skillreg/cli`) est puissant mais réservé aux développeurs. En organisation, les collaborateurs métiers (product managers, designers, sales, opérations, etc.) ont besoin d'une interface graphique pour :

- se connecter à leur entreprise sans choix technique ;
- installer une skill approuvée en un clic ;
- rendre automatiquement cette skill disponible dans les assistants détectés ;
- comprendre l'état, les mises à jour et les actions requises ;
- configurer les accès locaux sans les envoyer au registre.

Le dashboard web couvre le browse/search/gestion mais **ne peut pas** accéder au filesystem local.

---

## 2. Stack Technique

| Couche | Techno |
|--------|--------|
| Framework | **Tauri v2** |
| Backend | Rust (reqwest, serde, tar, flate2, sha2, dirs) |
| Frontend | React 19 + TypeScript + Vite |
| UI | Tailwind CSS v4 + shadcn/ui |
| State | Zustand |
| Routing | React Router v7 |
| Markdown | react-markdown + rehype-sanitize |
| Auto-update | @tauri-apps/plugin-updater |
| Tray | Tauri tray-icon (natif) |

### Pourquoi Tauri v2

| Critère | Tauri v2 | Electron |
|---------|----------|----------|
| Taille app | ~5-10 MB | ~100+ MB |
| RAM | ~30 MB | ~150+ MB |
| Backend | Rust (natif, rapide) | Node.js |
| Frontend | Web (React) | Web (React) |
| Filesystem | Natif via Rust | Node.js |
| Auto-update | Plugin intégré | electron-updater |
| Tray/menubar | Natif | Natif |
| Packaging | `.dmg` + `.msi` + `.AppImage` | Idem |

---

## 3. Architecture

```
skillreg-local/
├── src-tauri/                  ← Backend Rust
│   ├── src/
│   │   ├── main.rs             ← Entry point Tauri
│   │   ├── lib.rs              ← App setup, tray icon, command registration
│   │   ├── commands/           ← Tauri commands (invoked from frontend)
│   │   │   ├── mod.rs          ← Module exports
│   │   │   ├── auth.rs         ← login_initiate, login_poll, login_with_token, whoami, logout, open_url
│   │   │   ├── managed_skills.rs ← install, overview, repair, uninstall, active org
│   │   │   ├── managed_migration.rs ← preview/apply legacy migration
│   │   │   ├── local_import.rs  ← preview/import des dossiers locaux non suivis
│   │   │   ├── skills.rs       ← legacy/project install + registry CRUD
│   │   │   ├── slash_commands.rs ← registry/local slash command install, update, remove
│   │   │   ├── local.rs        ← scan_local_skills, parse_frontmatter
│   │   │   ├── config.rs       ← read_config, write_config
│   │   │   └── env.rs          ← EnvStore org-level + legacy env commands
│   │   ├── managed_skills/
│   │   │   ├── agents/         ← adapters Claude, Codex et Cursor
│   │   │   ├── archive.rs      ← extraction bornée et anti-traversal
│   │   │   ├── bindings.rs     ← plan/apply/verify des liens possédés
│   │   │   ├── local_import.rs  ← scan et import transactionnel sans publication
│   │   │   ├── manifest.rs     ← manifest v2 et écriture atomique
│   │   │   ├── migration.rs    ← classification et migration non destructive
│   │   │   ├── reconcile.rs    ← réparation, uninstall et switch d'org
│   │   │   └── service.rs      ← install/update transactionnels
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/                  ← App icons (all sizes: .icns, .ico, .png)
├── src/                        ← Frontend React
│   ├── App.tsx                 ← Root component + router + AuthGate + SetupRoute
│   ├── main.tsx                ← Entry point + theme restoration
│   ├── pages/
│   │   ├── Login.tsx           ← Auth (2 tabs: device flow + token paste)
│   │   ├── Setup.tsx           ← organisation + préparation automatique
│   │   ├── Dashboard.tsx       ← santé, actions et auto-update global
│   │   ├── Catalog.tsx         ← catalogue unifié et installation en un clic
│   │   ├── SkillDetail.tsx     ← résultats, exemples, accès et disponibilité
│   │   ├── Commands.tsx        ← Browse/install/update/remove slash commands
│   │   ├── Installed.tsx       ← une ligne canonique par skill + réparation/uninstall
│   │   ├── Publish.tsx         ← Push skill (file picker dialog + preview + dry-run)
│   │   ├── EnvVars.tsx         ← Env vars CRUD + masking + import .env
│   │   └── Settings.tsx        ← compte + réglages avancés repliés
│   ├── components/
│   │   ├── ui/                 ← shadcn/ui (badge, button, card, input, label, select)
│   │   └── layout/
│   │       ├── AppShell.tsx    ← Main layout (titlebar + sidebar + content)
│   │       ├── Sidebar.tsx     ← navigation principale en 4 entrées
│   │       └── Titlebar.tsx    ← Custom window titlebar (draggable + min/max/close)
│   ├── lib/
│   │   ├── api.ts              ← wrappers invoke() typés, aucun fetch direct
│   │   ├── store.ts            ← auth, config et modèle de skills gérées
│   │   ├── employee-model.ts   ← traduction des états techniques en actions métier
│   │   ├── types.ts            ← Types partagés (Rust ↔ TS)
│   │   ├── constants.ts        ← API_BASE_URL (pour verificationUrl côté front)
│   │   └── utils.ts            ← cn() helper (clsx + tailwind-merge)
│   └── styles/
│       └── globals.css         ← Tailwind base + dark/light themes + prose styles
├── .github/workflows/
│   └── release.yml             ← CI/CD cross-platform (macOS arm64/x86, Ubuntu, Windows)
├── package.json
├── tsconfig.json
├── vite.config.ts
├── components.json             ← shadcn/ui config
├── DEV-PLAN.md                 ← This file
├── ROADMAP.md                  ← Progress tracking
└── CLAUDE.md                   ← Conventions & instructions pour Claude Code
```

---

## 4. API Communication

L'app desktop consomme la même API REST que le CLI. Base URL hardcodée : `https://app.skillreg.dev` (invisible à l'utilisateur).

### Endpoints utilisés

| Catégorie | Endpoint | Méthode | Auth | Usage |
|-----------|----------|---------|------|-------|
| **Auth** | `/api/v1/auth/cli/initiate` | POST | Non | Démarrer device flow |
| | `/api/v1/auth/cli/poll` | GET | Non | Polling token |
| | `/api/v1/auth/whoami` | GET | Token | Info utilisateur |
| **Skills** | `/api/v1/orgs/{org}/skills` | GET | Token | Lister skills |
| | `/api/v1/orgs/{org}/skills` | POST | Token | Créer skill |
| | `/api/v1/orgs/{org}/skills/{name}` | GET | Token | Détail skill |
| | `/api/v1/orgs/{org}/skills/{name}` | PATCH | Token | Mettre à jour |
| | `/api/v1/orgs/{org}/skills/{name}` | DELETE | Token | Supprimer |
| **Versions** | `/api/v1/orgs/{org}/skills/{name}/versions` | GET | Token | Lister versions |
| | `/api/v1/orgs/{org}/skills/{name}/versions` | POST | Token | Publier version |
| | `/api/v1/orgs/{org}/skills/{name}/versions/{v}/download` | GET | Token | Télécharger tarball |
| **Install géré** | `/api/v1/orgs/{consumer}/managed-skills/{source}/{name}/target` | GET | Token | Résoudre la version approuvée exacte |
| | `/api/v1/orgs/{consumer}/managed-skills/{source}/{name}/download` | GET | Token | Télécharger avec identité, checksum et politique |
| | `/api/v1/orgs/{consumer}/managed-skills/policies/{sourceSkillId}` | PUT | Admin | Épingler ou suivre la dernière version approuvée |
| **Commands** | `/api/v1/orgs/{org}/commands` | GET | Token | Lister slash commands |
| | `/api/v1/orgs/{org}/commands/{name}` | GET | Token | Détail + versions d'une command |
| **Search** | `/api/v1/search` | GET | Non | Recherche full-text |
| **Tokens** | `/api/v1/orgs/{org}/tokens` | GET/POST | Token | Lister/créer tokens |
| | `/api/v1/orgs/{org}/tokens/{id}` | DELETE | Token | Révoquer token |

### Auth Flow (Device Authorization)

```
1. App: POST /api/v1/auth/cli/initiate
   → Reçoit { deviceCode, userCode, verificationUrl }

2. App: Ouvre le navigateur sur {baseUrl}{verificationUrl}
   → Affiche le userCode à l'utilisateur dans l'app

3. App: Poll GET /api/v1/auth/cli/poll?device_code={deviceCode}
   → Toutes les 3s, max 100 tentatives
   → Quand status="complete" → reçoit le token

4. App: Stocke le token dans ~/.skillreg/config.json
```

---

## 5. Backend Rust — Commandes Tauri

Toutes les interactions frontend ↔ backend passent par `invoke()`.

### Auth

```rust
#[tauri::command]
async fn login_initiate(api_url: String) -> Result<DeviceFlowResponse, String>

#[tauri::command]
async fn login_poll(api_url: String, device_code: String) -> Result<PollResponse, String>

#[tauri::command]
async fn login_with_token(token: String) -> Result<bool, String>
// Valide le format (sr_live_, sr_test_, sk_) et sauvegarde

#[tauri::command]
async fn whoami(api_url: String, token: String) -> Result<WhoamiResponse, String>

#[tauri::command]
async fn logout() -> Result<(), String>
// Supprime le token de la config
```

### Skills (API Registry)

```rust
#[tauri::command]
async fn list_skills(api_url: String, token: String, org: String, page: u32, limit: u32)
    -> Result<PaginatedSkills, String>

#[tauri::command]
async fn get_skill(api_url: String, token: String, org: String, name: String)
    -> Result<SkillDetail, String>

#[tauri::command]
async fn search_skills(api_url: String, query: String, org: Option<String>)
    -> Result<Vec<SearchResult>, String>
```

### Skills (Local Filesystem)

```rust
#[tauri::command]
async fn install_managed_skill(
    consumer_org: String, source_org: Option<String>, name: String
) -> Result<ManagedInstallResult, ManagedErrorDto>
// Le serveur résout la version ; une archive, une copie canonique, tous les bindings compatibles.

#[tauri::command]
fn get_managed_overview() -> Result<ManagedOverview, ManagedErrorDto>

#[tauri::command]
async fn uninstall_managed_skill(
    installation_id: String
) -> Result<ManagedUninstallResult, ManagedErrorDto>
// Vérifie hash et propriété de tous les liens avant retrait ; conserve les accès enregistrés.

#[tauri::command]
async fn repair_managed_skill(
    installation_id: String
) -> Result<ReconcileReport, ManagedErrorDto>

#[tauri::command]
async fn repair_all_managed_skills() -> Result<ReconcileReport, ManagedErrorDto>

#[tauri::command]
async fn switch_active_org(
    org: String
) -> Result<ActiveOrgSwitchReport, ManagedErrorDto>
// Autorisation distante, plan complet, apply/verify, puis écriture activeOrg/config.

#[tauri::command]
fn preview_local_skills_import() -> Result<LocalImportPreview, ManagedErrorDto>
// Lecture seule ; aucun chemin ni contenu n'est sérialisé vers le frontend.

#[tauri::command]
async fn run_local_skills_import(
    confirm: bool
) -> Result<LocalImportReport, ManagedErrorDto>
// Copie locale canonique, bindings vérifiés et rollback avant commit en cas d'échec.

// Contrat legacy/projet conservé pour les workflows avancés :
#[tauri::command]
async fn pull_skill(
    api_url: String, token: String,
    org: String, name: String, version: Option<String>,
    agent: String, scope: String
) -> Result<InstallResult, String>
// 1. GET version info → 2. Download tarball → 3. Verify SHA256 → 4. Extract to agent path

#[tauri::command]
async fn push_skill(
    api_url: String, token: String,
    org: String, dir_path: String,
    version: Option<String>, bump: Option<String>, tag: Option<String>,
    dry_run: bool
) -> Result<PushResult, String>
// 1. Read SKILL.md → 2. Create tarball → 3. Compute SHA256 → 4. Upload multipart

#[tauri::command]
async fn scan_local_skills(agent: Option<String>, scope: Option<String>)
    -> Result<Vec<LocalSkill>, String>
// Scan all AGENT_PATHS, parse SKILL.md frontmatter, detect symlinks

#[tauri::command]
async fn uninstall_skill(name: String, agent: String, scope: String) -> Result<bool, String>
// Parcours legacy uniquement ; la vue collaborateur utilise uninstall_managed_skill.

#[tauri::command]
async fn delete_skill(org: String, name: String) -> Result<bool, String>
// DELETE /orgs/{org}/skills/{name} — removes the skill + all versions from the registry

#[tauri::command]
async fn check_updates(api_url: String, token: String, org: String, local_skills: Vec<LocalSkill>)
    -> Result<Vec<UpdateAvailable>, String>
// Compare local versions vs registry latest versions
```

### Config

```rust
#[tauri::command]
fn read_config() -> Result<Config, String>
// Read ~/.skillreg/config.json

#[tauri::command]
fn write_config(config: Config) -> Result<(), String>
// Write ~/.skillreg/config.json

#[tauri::command]
```

### Env Vars

```rust
#[tauri::command]
fn get_org_env_var(org: String, key: String) -> Result<Option<String>, String>
// Read one org-level value from the OS secure store, or fallback file if secure storage is unavailable

#[tauri::command]
fn set_org_env_var(org: String, key: String, value: String) -> Result<(), String>
// Write one org-level variable to Keychain / Credential Manager / Secret Service

#[tauri::command]
fn delete_org_env_var(org: String, key: String) -> Result<(), String>
// Remove one org-level variable

#[tauri::command]
fn list_org_env_vars(org: String) -> Result<Vec<OrgEnvVariable>, String>
// List configured org-level variable metadata without values

#[tauri::command]
fn preview_legacy_env_migration(org: String) -> Result<EnvMigrationSummary, String>
// Report migratable legacy variables and conflicts without exposing values

#[tauri::command]
fn migrate_legacy_env_vars(org: String) -> Result<EnvMigrationSummary, String>
// Migrate only non-conflicting legacy values; keep old files as backup

#[tauri::command]
fn cleanup_legacy_env_vars(org: String) -> Result<LegacyCleanupSummary, String>
// Delete legacy values only when they exactly match the org-level stored value

#[tauri::command]
fn migrate_org_env_file_to_secure_store(org: String) -> Result<SecureStoreMigrationSummary, String>
// Explicitly move Phase 2 fallback values from variables.env to the OS secure store

#[tauri::command]
fn get_env_vars(org: String, skill: String) -> Result<HashMap<String, String>, String>
// Legacy compatibility: read ~/.skillreg/env/{org}/{skill}.env

#[tauri::command]
fn set_env_vars(org: String, skill: String, vars: HashMap<String, String>) -> Result<(), String>
// Legacy compatibility: write to per-skill env file

#[tauri::command]
fn delete_env_vars(org: String, skill: String, keys: Vec<String>) -> Result<(), String>
// Legacy compatibility: remove specific keys from per-skill env file

#[tauri::command]
fn list_all_env_vars(org: String) -> Result<Vec<SkillEnvVars>, String>
// Legacy compatibility: list per-skill env files only

#[tauri::command]
fn import_env_file(org: String, skill: String, file_path: String) -> Result<HashMap<String, String>, String>
// Parse .env content and merge into skill env vars
```

---

## 6. Écrans Détaillés

### 6.1 Login

```
┌──────────────────────────────────────────┐
│              SkillReg Local              │
│                                          │
│          [Logo SkillReg]                 │
│                                          │
│   ┌──────────────────────────────────┐   │
│   │  [ Login with Browser ]          │   │ ← Device flow (recommandé)
│   └──────────────────────────────────┘   │
│                                          │
│   ┌──────────────────────────────────┐   │
│   │  [ Login with Token ]            │   │ ← Input token manuellement
│   └──────────────────────────────────┘   │
│                                          │
│   Pas encore de compte ?                 │
│   Créez-en un sur app.skillreg.dev       │
└──────────────────────────────────────────┘
```

**Device flow UX** :
1. Clic "Login with Browser" → affiche un code (ex: `A3F7K2`)
2. Le navigateur s'ouvre sur la page d'autorisation
3. Spinner "En attente d'autorisation..." avec le code bien visible
4. Quand autorisé → transition vers Setup ou Dashboard

### 6.2 Préparation au premier lancement

- Une seule organisation autorisée : elle est choisie automatiquement.
- Plusieurs organisations : seuls leurs noms métier sont proposés.
- SkillReg détecte les assistants sans demander de choisir une cible.
- Le preview de migration est en lecture seule. Les installations projet, externes, inconnues ou
  modifiées restent intactes.
- Le collaborateur peut reporter la migration sans bloquer l'accès au catalogue.
- La fin du parcours confirme simplement « Vos assistants sont prêts ».

Les termes `agent`, `scope`, `user`, `project`, `version`, `semver` et les chemins locaux ne sont
pas exposés dans ce parcours.

### 6.3 Dashboard

Le dashboard répond d'abord à trois questions : les assistants sont-ils prêts, une action est-elle
requise, et les mises à jour automatiques sont-elles actives ? L'interrupteur global
« Maintenir mes skills à jour » et le bouton manuel « Vérifier maintenant » restent visibles sans
ouvrir les réglages. Une désactivation de l'automatique ne retire jamais l'action manuelle. À la
largeur minimale de 900 px, les contrôles se replient sans débordement horizontal.

### 6.4 Catalog

Le catalogue regroupe les skills privées de l'entreprise et les skills publiques autorisées par
sa politique. Une carte présente le résultat métier attendu et une seule action principale :
`Installer`, `Installer…`, `Configurer`, `Mettre à jour`, `Voir le problème` ou un état prêt.
L'installation n'ouvre aucun sélecteur technique.

### 6.5 Skill Detail

Le détail privilégie la description, les résultats, les exemples, les accès éventuellement requis
et les assistants dans lesquels la skill sera disponible. Les versions, checksums et chemins
restent dans les contrats techniques, pas dans l'action collaborateur.

### 6.6 Installed Skills

`Mes skills` affiche une seule ligne par installation canonique, même lorsque trois assistants y
sont liés. Un problème ouvre l'état des connexions et une réparation ciblée. `Désinstaller` reste
dans un menu secondaire, ouvre une confirmation nominative et précise que les accès enregistrés
sont conservés. Un lien modifié ou un dossier non géré bloque l'opération sans suppression.

### 6.7 Import local

Les Réglages exposent toujours « Rassembler mes skills locales » après connexion. L’aperçu est en
lecture seule et ne montre ni chemin, ni agent à choisir, ni scope, ni version. Une confirmation
explicite rassemble les copies identiques sous `~/.skillreg/skills`, crée les connexions gérées et
rafraîchit l’accueil. Les conflits, liens externes, dossiers incomplets et fichiers isolés restent
intacts. Aucun contenu n’est envoyé ou publié.

Les descriptions YAML sur une ou plusieurs lignes sont acceptées. Pour les anciennes skills, le
slug du dossier est l’identité d’installation et le champ `name` peut rester un libellé humain.

### 6.8 Publish (Push)

```
┌─────────┬────────────────────────────────────────┐
│ Sidebar │  Publier un Skill                       │
│         │                                          │
│         │  ┌────────────────────────────────────┐  │
│         │  │                                    │  │
│         │  │   Glissez un dossier skill ici     │  │
│         │  │   ou [ Parcourir... ]              │  │
│         │  │                                    │  │
│         │  └────────────────────────────────────┘  │
│         │                                          │
│         │  ── Preview ──                           │
│         │  Nom: code-review-expert                 │
│         │  Description: Expert code review for PRs │
│         │  Tags: typescript, review                │
│         │                                          │
│         │  Version: [1.0.3] [patch⬆] [minor⬆] [major⬆] │
│         │  Org: [acme-corp ▾]                      │
│         │  Tag: [latest ▾]                         │
│         │                                          │
│         │  Security scan: ✓ No issues found        │
│         │                                          │
│         │  [ Dry Run ] [ Publier ⬆ ]              │
└─────────┴────────────────────────────────────────┘
```

### 6.9 Env Vars Manager

```
┌─────────┬────────────────────────────────────────┐
│ Sidebar │  Variables d'Environnement              │
│         │                                          │
│         │  Skill: [code-review-expert ▾]           │
│         │                                          │
│         │  ┌──────────┬──────────────┬─────────┐  │
│         │  │ Clé      │ Valeur       │ Actions │  │
│         │  ├──────────┼──────────────┼─────────┤  │
│         │  │ API_KEY  │ sk_l****     │ ✏ 🗑    │  │
│         │  │ MODEL    │ gpt-4        │ ✏ 🗑    │  │
│         │  └──────────┴──────────────┴─────────┘  │
│         │                                          │
│         │  [ + Ajouter ] [ Importer .env ]         │
│         │                                          │
│         │  ⚠ OPENAI_KEY requis par le skill        │
│         │    mais non configuré                     │
└─────────┴────────────────────────────────────────┘
```

---

## 7. État local et propriété

Le desktop et le CLI partagent la configuration et les accès, mais seul le backend Rust du desktop
mute le stockage de skills gérées :

| Fichier | Contenu |
|---------|---------|
| `~/.skillreg/config.json` | Token, API, organisation active, onboarding, auto-update global et lancement à la connexion. Les anciens defaults agent/scope restent seulement lisibles pour le CLI. |
| `~/.skillreg/managed-skills.json` | Manifest v2 : installations canoniques, origine `registry` ou `local`, bindings possédés, organisation active, hashes, statuts et erreurs structurées. Écriture atomique. |
| `~/.skillreg/skills/<consumer>/<source>/<name>/content` | Copie canonique active, vérifiée par checksum d'archive ou hash d'arbre local. Les imports utilisent la source `local`. |
| `~/.skillreg/installed.json` | Manifest v1 conservé en lecture pendant la migration ; jamais écrasé par le lecteur v2. |
| `~/.skillreg/installed-v1.backup.json` | Backup créé lors d'une migration confirmée réussie. |
| `.skillregrc` | Config par projet (org, skills) |
| OS credential store | Valeurs org-level locales via macOS Keychain, Windows Credential Manager ou Linux Secret Service. |
| `~/.skillreg/env/{org}/index.json` | Métadonnées sans secrets : noms de variables, statut configuré, backend de stockage, timestamps. |
| `~/.skillreg/env/{org}/variables.env` | Fallback local permissionné si le secure store est indisponible, et source Phase 2 migrable explicitement. |
| `~/.skillreg/env/{org}/{skill}.env` | Fichiers legacy par skill, lus pour compatibilité et migration sûre. |

Une installation gérée crée au plus un lien possédé par agent activé :

```
Claude: ~/.claude/skills/<name>  -> ~/.skillreg/skills/.../<name>/content
Codex:  ~/.agents/skills/<name>  -> ~/.skillreg/skills/.../<name>/content
Cursor: ~/.cursor/skills/<name>  -> ~/.skillreg/skills/.../<name>/content
```

Les dossiers projet restent au CLI. Un chemin utilisateur historique est détecté mais n'est jamais
revendiqué implicitement. La matrice exacte et les plateformes activées vivent dans
`docs/agent-compatibility.md`.

### Auto-update des skills installés

SkillReg Local reste résident dans le tray/menu bar quand la fenêtre est fermée. Le bouton Quit du tray arrête explicitement le process et donc le worker d'auto-update.

Le toggle global d'auto-update est visible sur l'accueil et peut être désactivé sans retirer le
bouton manuel. Les valeurs absentes gardent le comportement par défaut sans réécrire la config.

Le worker ne met à jour que les installations du manifest v2 appartenant à l'organisation active.
Il vérifie le hash canonique, demande au serveur la cible approuvée, exige les headers d'identité et
de checksum, télécharge une fois, active atomiquement la nouvelle copie et conserve la précédente
jusqu'à la fin du commit. Un contenu modifié ou un changement de politique devient une action
requise ; il n'est jamais écrasé silencieusement.

Toutes les mutations gérées partagent un verrou local. Après un run réussi, SkillReg peut envoyer
une notification OS et émet l'événement frontend `auto-update:completed`. Les erreurs normalisées
sont enregistrées sans token, secret ou payload API arbitraire.

---

## 8. Dépendances Rust

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"           # Ouvrir le navigateur
tauri-plugin-dialog = "2"          # File picker natif
tauri-plugin-autostart = "2"       # Launch at login
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "multipart", "stream"] }
tokio = { version = "1", features = ["full"] }
sha2 = "0.10"                      # SHA-256 checksum
tar = "0.4"                        # Tarball creation/extraction
flate2 = "1.0"                     # Gzip compression
dirs = "5"                         # Home directory, config paths
open = "5"                         # Ouvrir URL dans le navigateur
```

---

## 9. Dépendances Frontend

```json
{
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "react-router": "^7.0.0",
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "@tauri-apps/plugin-dialog": "^2.0.0",
    "zustand": "^5.0.0",
    "react-markdown": "^9.0.0",
    "tailwindcss": "^4.0.0",
    "class-variance-authority": "^0.7.0",
    "clsx": "^2.0.0",
    "tailwind-merge": "^2.0.0",
    "lucide-react": "^0.400.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "typescript": "^5.7.0",
    "vite": "^6.0.0",
    "@vitejs/plugin-react": "^4.0.0"
  }
}
```

---

## 10. Phases de Développement

### Phase 1 — Squelette ✅
- [x] Init projet Tauri v2 (`pnpm create tauri-app`)
- [x] Setup Vite + React + TypeScript
- [x] Tailwind CSS v4 + shadcn/ui (copier composants de base)
- [x] Routing (React Router v7, pages vides)
- [x] Layout principal : AppShell + Sidebar + Titlebar
- [x] Commandes Rust : `read_config`, `write_config`
- [x] Types partagés (Rust structs ↔ TypeScript interfaces)
- [x] Design system dark theme
- [x] Custom titlebar (draggable)

### Phase 2 — Auth + Dashboard ✅
- [x] Device flow login (initiate + poll + open browser)
- [x] Token manual login (input + validation format sr_live_*, sr_test_*, sk_*)
- [x] Setup historique livré, puis remplacé par la préparation automatique de la Phase 7
- [x] Écran whoami / dashboard avec liste des orgs
- [x] Stockage token dans `~/.skillreg/config.json`
- [x] Logout
- [x] Route guards (redirect vers login si pas de token + redirect setup si !setupDone)
- [x] API URL hardcodée (https://app.skillreg.dev)

### Phase 3 — Catalog + Install ✅
- [x] Liste skills d'un org (GET API + pagination)
- [x] Recherche full-text (debounce 300ms)
- [x] Filtres : tags, tri (updated, downloads, name)
- [x] Skill detail page (markdown rendering, 3 onglets: readme/versions/files)
- [x] Pull/install : download tarball, vérif SHA-256, extraction
- [x] Sélecteur agent/scope legacy livré, retiré du parcours collaborateur en Phase 7
- [x] Badge "Installed" avec scan local
- [x] Progress indicator pendant download

### Phase 4 — Local Skills + Publish ✅
- [x] Scan local skills (tous agents/scopes)
- [x] Parse SKILL.md frontmatter pour chaque skill local
- [x] Détection symlinks (skills tiers vs propres)
- [x] Détection updates disponibles (compare versions locale vs registry)
- [x] Uninstall (suppression dossier)
- [x] Push skill : file picker natif (plugin-dialog)
- [x] Preview SKILL.md avant push
- [x] Version bumper (patch/minor/major)
- [x] Dry-run mode
- [x] Upload multipart avec progress bar
- [x] Affichage security scan warnings

### Phase 5 — Env Vars + Settings ✅
- [x] CRUD env vars par skill
- [x] Détection auto des vars requises depuis SKILL.md
- [x] Import depuis fichier .env
- [x] Masquage des valeurs (affichage partiel)
- [x] Warnings pour vars manquantes
- [x] Injection ${VAR} dans le scan local
- [x] Phase 2 Env Engine : stockage org-level temporaire dans `variables.env`, migration legacy sûre, conflits visibles
- [x] Phase 3 Env Engine : stockage OS secure store, index sans secrets, fallback fichier explicite, cleanup legacy sûr
- [x] Page settings : org, agent, scope, theme, sign out
- [x] Thème clair/sombre (toggle manuel + persistence localStorage)

### Phase 6 — Polish + Packaging ✅ (partiel)
- [x] Tray icon (menubar macOS, system tray Windows)
- [x] Auto-update (config Tauri updater plugin)
- [x] Auto-update des skills installés via manifest local et worker tray
- [x] Icône app (toutes tailles: .icns, .ico, .png)
- [x] Custom titlebar (draggable, boutons min/max/close)
- [x] Notifications OS (install success, auto-update completed)
- [x] Packaging config : `.dmg` (macOS), `.msi` (Windows), `.AppImage` (Linux)
- [x] GitHub Actions CI/CD : build cross-platform
- [x] Releases macOS signées + notarized en CI (nécessite secrets Apple GitHub)
- [ ] Page de téléchargement sur skillreg.dev

### Phase 7 — Expérience collaborateur métier et stockage central ✅ (QA cross-platform restante)

Guide utilisateur : `docs/managed-skills-user-guide.md`.

- [x] Onboarding sans choix de version, agent, scope ou chemin
- [x] Organisation unique automatique et switch multi-org transactionnel
- [x] Cible approuvée résolue côté serveur sans version envoyée par l'employé
- [x] Manifest v2 atomique séparant installations canoniques et bindings
- [x] Une copie sous `~/.skillreg/skills`, téléchargée une seule fois
- [x] Adaptateurs Claude, Codex et Cursor pilotés par une matrice datée
- [x] Liens possédés planifiés, appliqués, vérifiés et réparables
- [x] Migration v1 explicite, idempotente et non destructive
- [x] Import public des dossiers locaux non suivis, sans publication
- [x] Descriptions YAML multilignes et anciens noms d’affichage compatibles
- [x] Auto-update global visible, update manuel et rollback
- [x] Dashboard compact à 900×600, catalogue, détail et « Mes skills » simplifiés
- [x] Désinstallation confirmée, ciblée par installation ID et conservation des accès
- [x] CLI projet préservé ; manifest v2 lu en lecture seule
- [ ] Validation runtime complète macOS x64, Windows x64 et Linux x64

---

## 11. État d'avancement

| Phase | Statut |
|-------|--------|
| Phase 1 — Squelette | ✅ 100% |
| Phase 2 — Auth + Dashboard | ✅ 100% |
| Phase 3 — Catalog + Install | ✅ 100% |
| Phase 4 — Local + Publish | ✅ 100% |
| Phase 5 — Env Vars + Settings | ✅ 100% |
| Phase 6 — Polish + Packaging | 🔶 ~90% (page téléchargement restante) |
| Phase 7 — Expérience collaborateur | 🔶 implémentée, matrice cross-platform à terminer |

---

## 12. Interopérabilité CLI ↔ Desktop

L'app desktop et le CLI partagent :
- Les mêmes fichiers de config (`~/.skillreg/config.json`, `.skillregrc`)
- Les mêmes env vars (`~/.skillreg/env/`)
- La même API REST

Le desktop possède le user-scope géré. Le CLI lit `managed-skills.json` sans le modifier,
déduplique les bindings dans `skillreg local` et refuse un `pull --scope user` qui créerait une
copie divergente. Le scope projet reste intégralement pris en charge par le CLI. Un développeur
publie via le CLI ; un collaborateur installe en un clic via l'app.

---

## 13. Sécurité

- Tokens stockés dans `~/.skillreg/config.json` (même que le CLI)
- Env vars stockées dans le secure store OS quand disponible ; `variables.env` reste uniquement fallback local permissionné
- Checksum SHA-256 vérifié à chaque download
- Hash d'arbre vérifié avant update, réparation et désinstallation
- Extraction bornée refusant traversal, chemins absolus, symlinks et hardlinks
- Preuve de propriété exacte avant retrait d'un symlink ou d'une junction
- Manifest atomique, rollback et verrou global pour les mutations gérées
- Auto-update bloqué si le contenu local ne correspond plus au hash enregistré
- react-markdown pour le rendu markdown
- Pas de secrets en clair dans l'UI (masquage des tokens et env vars)
- Auto-update signé (Tauri updater avec signature — endpoint configuré)
- Aucun compteur d'usage ni snapshot admin dans le lot A ; aucun prompt, conversation ou fichier
  utilisateur n'est collecté.
