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
│   │   ├── commands/           ← Tauri commands (invoked from frontend)
│   │   │   ├── mod.rs          ← Module exports (auth, config, env, local, skills)
│   │   │   ├── auth.rs         ← login_initiate, login_poll, login_with_token, whoami, logout, open_url
│   │   │   ├── skills.rs       ← list_skills, get_skill, search_skills, pull_skill, push_skill, uninstall_skill, check_updates
│   │   │   ├── local.rs        ← scan_local_skills, parse_frontmatter
│   │   │   ├── config.rs       ← read_config, write_config
│   │   │   └── env.rs          ← get_env_vars, set_env_vars, delete_env_vars, list_all_env_vars, import_env_file
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/                  ← App icons (all sizes: .icns, .ico, .png)
├── src/                        ← Frontend React
│   ├── App.tsx                 ← Root component + router + AuthGate + SetupRoute
│   ├── main.tsx                ← Entry point + theme restoration
│   ├── pages/
│   │   ├── Login.tsx           ← Auth (2 tabs: device flow + token paste)
│   │   ├── Setup.tsx           ← First-run wizard (org, agent, scope)
│   │   ├── Dashboard.tsx       ← Orgs overview (cards cliquables)
│   │   ├── Catalog.tsx         ← Browse/search skills + pagination + SkillCard inline
│   │   ├── SkillDetail.tsx     ← Detail + 3 tabs (readme/versions/files) + install sidebar
│   │   ├── Installed.tsx       ← Local skills groupés par agent/scope + update + uninstall
│   │   ├── Publish.tsx         ← Push skill (file picker dialog + preview + dry-run)
│   │   ├── EnvVars.tsx         ← Env vars CRUD + masking + import .env
│   │   └── Settings.tsx        ← User info, org/agent/scope, theme toggle, sign out
│   ├── components/
│   │   ├── ui/                 ← shadcn/ui (badge, button, card, input, label, select)
│   │   └── layout/
│   │       ├── AppShell.tsx    ← Main layout (titlebar + sidebar + content)
│   │       ├── Sidebar.tsx     ← Navigation sidebar (7 items)
│   │       └── Titlebar.tsx    ← Custom window titlebar (draggable + min/max/close)
│   ├── lib/
│   │   ├── api.ts              ← 20 invoke() wrappers typés
│   │   ├── store.ts            ← Zustand stores (useAuthStore, useConfigStore)
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
// rm -rf the skill directory

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
fn get_env_vars(org: String, skill: String) -> Result<HashMap<String, String>, String>
// Read ~/.skillreg/env/{org}/{skill}.env

#[tauri::command]
fn set_env_vars(org: String, skill: String, vars: HashMap<String, String>) -> Result<(), String>
// Write to env file

#[tauri::command]
fn delete_env_vars(org: String, skill: String, keys: Vec<String>) -> Result<(), String>
// Remove specific keys from env file

#[tauri::command]
fn list_all_env_vars(org: String) -> Result<HashMap<String, HashMap<String, String>>, String>
// List env vars for all skills in org

#[tauri::command]
fn import_env_file(org: String, skill: String, content: String) -> Result<(), String>
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

### 6.2 Setup Wizard (premier lancement)

```
Step 1/3 — Organisation
┌─────────────────────────────┐
│ Sélectionnez votre org :    │
│                              │
│ ◉ acme-corp (owner)         │
│ ○ my-team (admin)           │
│                              │
│         [ Suivant → ]        │
└─────────────────────────────┘

Step 2/3 — Agent par défaut
┌─────────────────────────────┐
│ Quel agent utilisez-vous ?  │
│                              │
│ ◉ Claude                     │
│ ○ Cursor                     │
│ ○ Codex                      │
│                              │
│  [ ← Retour ] [ Suivant → ] │
└─────────────────────────────┘

Step 3/3 — Scope
┌─────────────────────────────┐
│ Installer les skills pour : │
│                              │
│ ◉ Ce projet (project)       │
│ ○ Tout l'utilisateur (user) │
│                              │
│  [ ← Retour ] [ Terminer ]  │
└─────────────────────────────┘
```

### 6.3 Dashboard

```
┌─────────┬────────────────────────────────────────┐
│ Sidebar │  Mes Organisations                      │
│         │                                          │
│ 🏠 Home │  ┌──────────────┐  ┌──────────────┐    │
│ 📦 Cat. │  │ acme-corp    │  │ my-team      │    │
│ 💻 Local│  │ 12 skills    │  │ 3 skills     │    │
│ ⬆ Push  │  │ 5 membres    │  │ 2 membres    │    │
│ 🔑 Env  │  │ Plan: Team   │  │ Plan: Free   │    │
│ ⚙ Prefs │  │ [ Ouvrir ]   │  │ [ Ouvrir ]   │    │
│         │  └──────────────┘  └──────────────┘    │
│         │                                          │
└─────────┴────────────────────────────────────────┘
```

### 6.4 Catalog

```
┌─────────┬────────────────────────────────────────┐
│ Sidebar │  acme-corp — Skills                     │
│         │                                          │
│         │  [🔍 Rechercher...                    ]  │
│         │                                          │
│         │  Filtres: [All Tags ▾] [Tri: Updated ▾] │
│         │                                          │
│         │  ┌────────────────────────────────────┐  │
│         │  │ code-review-expert        v2.1.0   │  │
│         │  │ Expert code review for PRs         │  │
│         │  │ [typescript] [review]              │  │
│         │  │ ⬇ 234 downloads    [ Install ✓ ]  │  │
│         │  └────────────────────────────────────┘  │
│         │  ┌────────────────────────────────────┐  │
│         │  │ api-design-guide          v1.0.3   │  │
│         │  │ REST API design best practices     │  │
│         │  │ [api] [design]                     │  │
│         │  │ ⬇ 89 downloads     [ Installed ]  │  │
│         │  └────────────────────────────────────┘  │
└─────────┴────────────────────────────────────────┘
```

### 6.5 Skill Detail

```
┌─────────┬───────────────────────────┬────────────┐
│ Sidebar │  code-review-expert       │ Metadata   │
│         │  Expert code review...    │            │
│         │                           │ v2.1.0     │
│         │  [README] [Versions] [Files] │ 234 ⬇   │
│         │  ─────────────────────────│ 12.4 KB    │
│         │                           │ 3 versions │
│         │  # Code Review Expert     │            │
│         │                           │ Tags:      │
│         │  This skill helps you     │ typescript │
│         │  perform thorough code    │ review     │
│         │  reviews on pull requests │            │
│         │  ...                      │ SHA256:    │
│         │                           │ a1b2c3...  │
│         │                           │            │
│         │  ┌────────────────────┐   │ Agent:     │
│         │  │ [ Install v2.1.0 ]│   │ [Claude ▾] │
│         │  └────────────────────┘   │ Scope:     │
│         │                           │ [Project▾] │
└─────────┴───────────────────────────┴────────────┘
```

### 6.6 Installed Skills

```
┌─────────┬────────────────────────────────────────┐
│ Sidebar │  Skills Installés                       │
│         │                                          │
│         │  Claude (project) — .claude/skills/      │
│         │  ┌────────────────────────────────────┐  │
│         │  │ ● code-review     v2.1.0           │  │
│         │  │   Expert code review               │  │
│         │  │   [Update ⬆ v2.2.0] [Uninstall]   │  │
│         │  ├────────────────────────────────────┤  │
│         │  │ ● api-design      v1.0.3           │  │
│         │  │   REST API design guide            │  │
│         │  │   [✓ À jour]        [Uninstall]    │  │
│         │  └────────────────────────────────────┘  │
│         │                                          │
│         │  Claude (user) — ~/.claude/skills/       │
│         │  ┌────────────────────────────────────┐  │
│         │  │ ● shared-utils    v0.5.0           │  │
│         │  │   ⚠ 2 env vars manquantes          │  │
│         │  │   [Configure Env] [Uninstall]      │  │
│         │  └────────────────────────────────────┘  │
└─────────┴────────────────────────────────────────┘
```

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

## 7. Fichiers de Config Partagés

L'app desktop utilise exactement les mêmes fichiers que le CLI :

| Fichier | Contenu |
|---------|---------|
| `~/.skillreg/config.json` | Token, apiUrl, org par défaut, agent, scope, setupDone |
| `.skillregrc` | Config par projet (org, skills) |
| `~/.skillreg/env/{org}/{skill}.env` | Variables d'environnement par skill |

Les chemins d'installation sont identiques au CLI :

```
claude:  .claude/skills/ (project)  ~/.claude/skills/ (user)
codex:   .codex/skills/  (project)  ~/.codex/skills/  (user)
cursor:  .cursor/skills/ (project)  ~/.cursor/skills/ (user)
```

---

## 8. Dépendances Rust

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"           # Ouvrir le navigateur
tauri-plugin-dialog = "2"          # File picker natif
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
- [x] Page settings : org, agent, scope, theme, sign out
- [x] Thème clair/sombre (toggle manuel + persistence localStorage)

### Phase 6 — Polish + Packaging ✅ (partiel)
- [x] Tray icon (menubar macOS, system tray Windows)
- [x] Auto-update (config Tauri updater plugin)
- [x] Icône app (toutes tailles: .icns, .ico, .png)
- [x] Custom titlebar (draggable, boutons min/max/close)
- [ ] Notifications OS (install success, update available)
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
| Phase 6 — Polish + Packaging | 🔶 ~85% (notifications OS, code signing, page dl restants) |

---

## 12. Interopérabilité CLI ↔ Desktop

L'app desktop et le CLI partagent :
- Les mêmes fichiers de config (`~/.skillreg/config.json`, `.skillregrc`)
- Les mêmes chemins d'installation (`.claude/skills/`, etc.)
- Les mêmes env vars (`~/.skillreg/env/`)
- La même API REST

Un utilisateur peut installer un skill via l'app desktop et le voir avec `skillreg local`.
Un développeur peut push via le CLI et les non-techniques installent via l'app.

---

## 13. Sécurité

- Tokens stockés dans `~/.skillreg/config.json` (même que le CLI)
- Envisager migration vers le keychain OS natif (Keychain macOS, Credential Manager Windows) en v2
- Checksum SHA-256 vérifié à chaque download
- react-markdown pour le rendu markdown
- Pas de secrets en clair dans l'UI (masquage des tokens et env vars)
- Auto-update signé (Tauri updater avec signature — endpoint configuré)
