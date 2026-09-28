# SkillReg Local — Desktop App (Tauri v2)

> Application desktop cross-platform pour SkillReg.
> Alternative visuelle au CLI pour les utilisateurs non-techniques.

---

## 1. Contexte

Le CLI SkillReg (`@skillreg/cli`) est puissant mais réservé aux développeurs. En organisation, les profils non-techniques (product managers, designers, etc.) ont besoin d'une interface graphique pour :

- Installer des skills sur leur machine (accès filesystem)
- Configurer les agents locaux (Claude, Cursor, Codex)
- Gérer les skills installés localement
- Publier des skills sans connaître la ligne de commande

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
│   │   ├── managed_skills/     ← Canonical content, manifest v2, transactions, bindings, migration/import
│   │   ├── commands/           ← Tauri commands (invoked from frontend)
│   │   │   ├── mod.rs          ← Module exports (including slash_commands)
│   │   │   ├── auth.rs         ← login_initiate, login_poll, login_with_token, whoami, logout, open_url
│   │   │   ├── skills.rs       ← list_skills, get_skill, search_skills, pull_skill, push_skill, uninstall_skill, delete_skill, check_updates
│   │   │   ├── slash_commands.rs ← slash command registry, install, update, remove, publish version
│   │   │   ├── local.rs        ← scan_local_skills, parse_frontmatter
│   │   │   ├── managed_skills.rs ← managed lifecycle and safe DTOs
│   │   │   ├── managed_migration.rs ← opt-in legacy migration and repair
│   │   │   ├── local_import.rs ← preview/run local import
│   │   │   ├── config.rs       ← atomic private read_config/write_config
│   │   │   └── env.rs          ← EnvStore org-level + legacy env commands
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/                  ← App icons (all sizes: .icns, .ico, .png)
├── src/                        ← Frontend React
│   ├── App.tsx                 ← Root component + router + AuthGate + SetupRoute
│   ├── main.tsx                ← Entry point + theme restoration
│   ├── pages/
│   │   ├── Login.tsx           ← Auth (2 tabs: device flow + token paste)
│   │   ├── Setup.tsx           ← Company, assistant detection, opt-in migration
│   │   ├── Dashboard.tsx       ← Health, actions, global auto-update and manual updates
│   │   ├── Catalog.tsx         ← Search and managed install in one action
│   │   ├── SkillDetail.tsx     ← Employee detail and managed primary action
│   │   ├── Commands.tsx        ← Browse/install/update/remove slash commands + publish version
│   │   ├── Installed.tsx       ← One canonical row, agent availability, repair/uninstall
│   │   ├── Publish.tsx         ← Push skill (file picker dialog + preview + dry-run)
│   │   ├── EnvVars.tsx         ← Env vars CRUD + masking + import .env
│   │   └── Settings.tsx        ← User info, org/agent/scope, theme toggle, sign out
│   ├── components/
│   │   ├── PublishCommandDialog.tsx ← Publish a version of an existing registry command
│   │   ├── ui/                 ← shadcn/ui (badge, button, card, input, label, select)
│   │   └── layout/
│   │       ├── AppShell.tsx    ← Main layout (titlebar + sidebar + content)
│   │       ├── Sidebar.tsx     ← Navigation sidebar (7 items)
│   │       └── Titlebar.tsx    ← Custom window titlebar (draggable + min/max/close)
│   ├── lib/
│   │   ├── api.ts              ← Wrappers invoke() typés, dont publishCommandVersion
│   │   ├── command-publishing.ts ← Pure draft preparation and command publication validation
│   │   ├── store.ts            ← Zustand auth/config/managed, stale-response protection
│   │   ├── employee-model.ts   ← Pure employee state and actions
│   │   ├── types.ts            ← Types partagés (Rust ↔ TS)
│   │   ├── constants.ts        ← API_BASE_URL (pour verificationUrl côté front)
│   │   └── utils.ts            ← cn() helper (clsx + tailwind-merge)
│   └── styles/
│       └── globals.css         ← Tailwind base + dark palette + prose styles
├── .github/workflows/
│   ├── ci.yml                  ← Format/build/Node/Vitest/Rust on three OS, no publishing
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
| **Commands** | `/api/v1/orgs/{org}/commands` | GET | Token | Lister slash commands |
| | `/api/v1/orgs/{org}/commands/{name}` | GET | Token | Détail + versions d'une command |
| | `/api/v1/orgs/{org}/commands/{name}/versions` | POST | Token `write` ou `admin` | Publier une version d'une commande existante |
| **Search** | `/api/v1/search` | GET | Non | Recherche full-text |
| **Tokens** | `/api/v1/orgs/{org}/tokens` | GET/POST | Token | Lister/créer tokens |
| | `/api/v1/orgs/{org}/tokens/{id}` | DELETE | Token | Révoquer token |

### Publication de versions de commandes

Le frontend appelle `publishCommandVersion(org, name, input)` dans `src/lib/api.ts`. Le wrapper invoque `publish_command_version` ; Rust envoie un unique POST JSON authentifié avec le jeton connecté et les quatre champs explicites :

```json
{
  "version": "1.0.1",
  "content": "Review the current diff...",
  "agentCompatibility": ["claude", "codex"],
  "scope": "org"
}
```

La réponse HTTP 201 est `{ "version": { ...CommandVersion } }`. L'API exige le scope de jeton `write` ou `admin`, refuse une commande absente avec 404 et une version déjà publiée avec 409. Le rôle d'organisation affiché par le desktop ne suffit pas à déterminer ce droit.

Le numéro est saisi explicitement, unique et conforme à `/^\d+\.\d+\.\d+(-[\w.]+)?(\+[\w.]+)?$/`. Le serveur désigne la version publiée comme `latestVersion`, incrémente `totalVersions` et reprend ses agents et sa portée, sans imposer de progression numérique. Le contenu est du texte brut, limité après `trim()` à 1–20 000 unités UTF-16 ; les espaces et retours internes sont conservés. Les agents sont `claude`, `codex`, `cursor` (un à trois), et la portée de publication est `org`, `project` ou `user` (`CommandPublicationScope`, distinct de `ScopeType` pour les installations).

La publication reprend uniquement le contenu du registre et ne modifie ni les fichiers de commandes installés ni leur manifeste. Les listes du registre et des installations locales sont rechargées après succès ; chaque installation conserve sa version jusqu'à une action explicite d'installation ou de mise à jour.

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
// rm -rf the skill directory (local only)

#[tauri::command]
async fn delete_skill(org: String, name: String) -> Result<bool, String>
// DELETE /orgs/{org}/skills/{name} — removes the skill + all versions from the registry

#[tauri::command]
async fn check_updates(api_url: String, token: String, org: String, local_skills: Vec<LocalSkill>)
    -> Result<Vec<UpdateAvailable>, String>
// Compare local versions vs registry latest versions
```

### Commands (publication dans le registre)

```rust
#[tauri::command]
async fn publish_command_version(
    org: String,
    name: String,
    input: PublishCommandVersionInput,
) -> Result<CommandVersion, String>
// Validate input, normalize name, encode URL segments, send one authenticated JSON POST
```

`PublishCommandVersionInput` est sérialisé en camelCase avec `version`, `content`, `agentCompatibility` et `scope`. Les contrôles Rust portent sur les champs requis, le contenu (longueur `encode_utf16().count()` après nettoyage compatible avec `trim()` JavaScript), les agents et la portée ; l'API reste responsable du format complet et de l'unicité de version. Les erreurs HTTP passent par `format_api_error()` avec le contexte `Command version publish failed`, les erreurs réseau et réponses invalides sont distinguées. Le client de publication désactive les retries et redirections HTTP, avec un délai maximal de 30 secondes ; ni le jeton ni le contenu ne sont journalisés.

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

### 6.1–6.6 Parcours collaborateur géré

- Login : connexion navigateur recommandée ; accès par token conservé en option avancée.
- Setup : entreprise automatique si unique, choix si plusieurs, création de workspace via
  navigateur s’il n’existe pas, détection puis aperçu de migration opt-in. Préparation en échec :
  état explicite et réessai ; aucune sélection d’agent/scope/version dans le parcours principal.
- Accueil : santé des assistants, skills et accès requis, réconciliation automatique des bindings,
  toggle global de mises à jour, vérification et mise à jour manuelles même avec toggle OFF.
- Catalogue et détail : recherche, description et action principale gérée ; sélection de version
  approuvée côté serveur, agents disponibles détectés automatiquement.
- Mes skills : une ligne canonique avec disponibilités, accès manquants, réparation et suppression.
  Désinstallation via dialogue natif modal, focus initial non destructif, retour au déclencheur,
  annulation bloquée durant l’opération ; les accès enregistrés sont conservés.
- Réglages : import des skills locales après aperçu ; conflits, projets, contenus modifiés et
  liens externes conservés. Pas de publication implicite, pas d’usage/compteur/classement.

`src/lib/employee-model.ts` calcule les états affichés, `useManagedSkillsStore` orchestre les
wrappers IPC et rejette les réponses périmées après reset/changement d’organisation. Les erreurs
ne transforment pas un inventaire d’accès inconnu en liste vide autorisant l’écrasement.
Les détails utilisateur sont dans `docs/managed-skills-user-guide.md` ; les preuves natives
et limites d’accessibilité sont dans `docs/managed-skills-release-validation.md`.

### 6.7 Publish (Push)

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

### 6.8 Env Vars Manager

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

### 6.9 Commands — Publier une version

Chaque carte du registre propose **Publish version**, indépendamment de l'installation locale, de l'agent et du dossier projet sélectionnés. `PublishCommandDialog` affiche l'organisation et le nom cibles, puis charge le détail via `getCommand()`.

Le formulaire reprend le contenu brut de la version référencée par `latestVersion` (la première version seulement si cette référence est absente), ses agents et sa portée, avec repli sur les métadonnées de la commande pour les agents vides ou la portée absente. Une référence `latestVersion` incohérente affiche une erreur et permet de recharger ; une commande sans version démarre avec un contenu vide. Une portée inconnue ou des agents toujours absents imposent une sélection explicite. Aucun contenu installé avec frontmatter ou titre généré pour l'agent n'est importé.

L'utilisateur saisit une nouvelle version, modifie le texte, choisit les agents compatibles et la portée de commande. Le formulaire rappelle la version courante et le fait que la publication devient la version courante. `command-publishing.ts` prépare ce brouillon et valide les champs sans IPC. Les champs invalides et les versions déjà présentes bloquent l'envoi.

Pendant le POST, les champs et fermetures ordinaires sont désactivés, et une garde empêche les doubles soumissions. Les erreurs conservent le brouillon. Au succès, le dialogue se ferme, la confirmation identifie `@organisation/commande@version` et le registre est rechargé ; un échec de ce rechargement laisse la confirmation visible et propose de réessayer le chargement.

Changer d'organisation ferme le dialogue ; les réponses obsolètes ne remplacent pas les données actives. Un POST déjà envoyé reste lié à son organisation d'origine. Le dialogue gère le focus initial, son confinement et sa restauration, la fermeture par Échap hors publication, les libellés des champs et les annonces accessibles d'erreur et de succès. La création d'une commande portant un nouveau nom reste hors de ce parcours.

---

## 7. État local et moteur géré

| Fichier | Contrat |
| --- | --- |
| `~/.skillreg/config.json` | Token, org active, réglages et état setup ; écriture atomique privée. |
| `~/.skillreg/installed.json` | Manifeste legacy v1, lecteur conservé, jamais remplacé par v2. |
| `~/.skillreg/managed-skills.json` | Manifeste v2 strict, skills canoniques, hashes, bindings, organisation active, erreurs structurées et tombstones bornées. |
| `~/.skillreg/installed-v1.backup.json` | Copie du manifeste avant migration ; **pas une sauvegarde complète de downgrade**. |
| `~/.skillreg/skills/<consumer-org>/<source-org>/<name>/content` | Copie canonique vérifiée ; dossiers `staging` et `previous` pour la transaction. |
| `~/.skillreg/commands.json` | Slash commands ; contrat de publication inchangé. |
| `.skillregrc` et dossiers agents projet | Scope projet legacy, conservé hors migration. |
| `~/.skillreg/env/<org>/` et credential store OS | Index sans secrets, fallback permissionné et accès locaux ; jamais effacés par désinstallation de skill. |

`managed_skills/` contient archive/confinement, manifeste, service transactionnel, adaptateurs,
bindings, réconciliation, migration et import. Toutes les mutations utilisent le même verrou.
Après interruption, le hash du manifeste décide entre `content` et `previous` ; un état ambigu
reste conservé avec erreur explicite. Un fichier temporaire préexistant n’est pas écrasé.
Sur Unix, `.skillreg` est privé (0700) et les nouveaux fichiers sensibles en 0600. Sur Windows,
les ACL ciblées du répertoire et des temporaires sont durcies et vérifiées avant remplacement ;
une preuve runtime en compte standard reste exigée.

Bindings utilisateur : Claude `.claude/skills`, Codex `.agents/skills`, Cursor `.cursor/skills`.
Les anciens dossiers Codex `.codex/skills` restent détectés, sans appropriation automatique.
macOS arm64 seul est activé dans le code ; macOS x64, Windows et Linux attendent leurs preuves
runtime. Windows emploie des junctions NTFS, jamais un fallback par copies physiques.

### IPC et API gérés

`install_managed_skill(consumer_org, source_org?, name)` résout la politique serveur, télécharge
et vérifie une archive puis crée les bindings possibles. Rust utilise
`GET /api/v1/orgs/{consumer}/managed-skills/{source}/{name}/target` et `/download` ; checksum et
métadonnées d’identité/version sont obligatoires. Le frontend ne choisit ni version, ni chemin,
ni agent. Les DTO du cycle de vie collaborateur excluent les chemins natifs (les aperçus avancés de
migration/import exposent les sources locales pour diagnostic) ; les erreurs exposent un code et le seul
statut HTTP validé, sans URL signée, jeton ni message brut.

Autres commandes : `detect_managed_agents`, `get_managed_overview`, `uninstall_managed_skill`,
`repair_managed_skill`, `repair_all_managed_skills`, `switch_active_org`,
`preview_managed_skills_migration`, `run_managed_skills_migration`, `repair_managed_skills`,
`preview_local_skills_import`, `run_local_skills_import`. Les commandes legacy restent protégées
contre une mutation concurrente du contenu géré. Les commandes de publication de slash commands
restent enregistrées dans `lib.rs` et exposées dans `api.ts`.

### Mises à jour

L’app reste résidente dans le tray ; Quit arrête le worker. Démarrage à la connexion désactivé
par défaut ; vérification automatique activée, intervalle par défaut 60 minutes (borné 15–1440).
Le worker géré respecte le toggle global, le hash de tout le contenu, l’origine locale exclue et
la politique de version approuvée. Vérification/mise à jour manuelles restent disponibles OFF.
L’événement `managed-update:completed` rafraîchit l’UI gérée après un run ;
`auto-update:completed` reste associé au worker legacy.

### Retour arrière

Rollback d’opération et downgrade de release sont distincts. Le second nécessite le snapshot
complet antérieur des racines `.skillreg`, `.claude`, `.codex`, `.cursor`, `.agents` et une preuve
avec binaire legacy identifié. L’outil support Node `scripts/managed-skills-backup-restore.mjs`
préserve modes/ACL/liens, refuse les chemins dangereux, propose un aperçu et conserve l’état
remplacé dans un dossier de récupération distinct. Il n’exporte jamais le trousseau système.
Protocole et limites : `docs/managed-skills-release-validation.md`, scénario 18 toujours ouvert.

## 8. Dépendances Rust

`src-tauri/Cargo.toml` et son lockfile font foi : Tauri 2.11.5, tauri-build 2.6.3, tar 0.4.46,
reqwest/tokio/serde/sha2/flate2/dirs/open et backends credential store par OS.
Le socle géré ajoute `uuid` et, uniquement sous Windows, `junction` et les primitives
`windows-sys` ciblées pour le remplacement atomique. Aucun nouvel accès système privilégié.

## 9. Frontend, tests et livraison

React 19, TypeScript strict, Vite 6, Tailwind 4, Zustand 5 ; versions effectives dans
`package.json` et `pnpm-lock.yaml`. Vitest/Testing Library/jsdom sont repris du socle pour les
composants/stores/modèles/IPC ; `pnpm test` lance Node puis Vitest.

La validation usine du candidat est `pnpm format:check && pnpm build && cargo test
--manifest-path src-tauri/Cargo.toml --locked && node --test --experimental-strip-types
tests/*.test.ts` (sur une ligne). Compléter par `pnpm test:frontend`, Cargo check et garde
notarisation. `tsc` ne type pas les fichiers `tests/*.test.ts`.

La CI autonome Linux/macOS/Windows construit `dist/` avant Cargo sur chaque OS, lockfiles figés,
Rust `--no-fail-fast --locked`, sans secret ni publication. La release sur tag conserve signature,
notarisation/agrafage DMG et brouillon. Le helper `generate-updater-manifest.mjs` prépare un
`latest.json` complet : `.app.tar.gz` macOS, `.exe` NSIS Windows, `.AppImage` Linux et leurs
signatures exactes. Aucune version/tag/release n’est produit pendant l’implémentation.

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
- [x] Setup wizard (3 étapes : org, agent, scope)
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
- [x] Sélecteur agent/scope au moment de l'install
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

---

## 12. Interopérabilité CLI ↔ Desktop

Configuration/env locaux et API REST restent partagés. Le contrat CLI attendu conserve les
installations de projet et refuse de muter un manifeste v2 au scope utilisateur. Cette preuve
app/CLI/site est externe à ce dépôt et doit être rattachée au candidat ; les résultats historiques
ne suffisent pas. Ne pas annoncer un support runtime sur la seule base des fixtures desktop.

## 13. Sécurité

- Tokens stockés dans `~/.skillreg/config.json` (même que le CLI)
- Env vars stockées dans le secure store OS quand disponible ; `variables.env` reste uniquement fallback local permissionné
- Checksum SHA-256 vérifié à chaque download
- Auto-update des skills bloqué si le contenu local ne correspond plus au hash enregistré
- react-markdown pour le rendu markdown
- Pas de secrets en clair dans l'UI (masquage des tokens et env vars)
- Auto-update signé (Tauri updater avec signature — endpoint configuré)

## 14. État du lot A et autorisation

Le socle managed est présent et testé par fixtures. La checklist de release et son registre de
preuves restent **NO-GO dogfood et activation générale** tant que CI distante, campagnes natives,
Windows standard, binaire legacy, accessibilité, signatures/updater et observations 72 h/J+7
manquent. Les phases historiques ci-dessus ne valent pas autorisation du nouveau candidat.
