# CLAUDE.md — SkillReg Local

## Workflow obligatoire

### Avant de coder
1. **Lire `DEV-PLAN.md`** — Architecture, commandes Rust, écrans, API
2. **Lire `ROADMAP.md`** — Identifier la prochaine tâche
3. **Lire ce fichier** — Conventions et état actuel

### Après avoir codé
1. **Mettre à jour `ROADMAP.md`** — Cocher les items terminés
2. **Vérifier** : `pnpm tauri dev` doit lancer sans erreur
3. **Lint** : `pnpm format:check` doit passer (Biome)

## Stack

| Couche | Techno |
|--------|--------|
| Framework | Tauri v2 (2.10.x) |
| Backend | Rust (reqwest, serde, sha2, tar, flate2, dirs, open) |
| Frontend | React 19 + TypeScript + Vite 6 |
| UI | Tailwind CSS v4 + shadcn/ui (copié, sans Radix) |
| State | Zustand 5 |
| Routing | React Router v7 |
| Markdown | react-markdown + rehype-sanitize |
| Linting | Biome (tabs, 100 cols) |
| Dialog | @tauri-apps/plugin-dialog |

## Conventions

- Langue de réponse : **français**
- Commentaires de code : **anglais**
- Types stricts, pas de `any`
- Pas de sur-ingénierie
- Commandes Rust dans `src-tauri/src/commands/`
- Pages React dans `src/pages/`
- Composants réutilisables dans `src/components/`
- Stores Zustand dans `src/lib/store.ts`
- Wrappers IPC dans `src/lib/api.ts`

## Commandes

```bash
pnpm tauri dev          # Dev mode (frontend + backend)
pnpm tauri build        # Build production
pnpm dev                # Frontend seul (Vite)
pnpm build              # Build frontend seul
pnpm format             # Biome auto-fix
pnpm format:check       # Biome check (CI)
```

## Architecture clé

### HTTP via Rust uniquement
**IMPORTANT** : Toutes les requêtes HTTP vers le serveur passent par **Rust** (`reqwest`) via IPC `invoke()`. Le frontend ne fait JAMAIS de `fetch` direct — ceci évite les problèmes CORS du webview Tauri.

Pattern : `Frontend invoke("command") → Rust reqwest → API server → Rust result → Frontend`

### API Base URL
Hardcodée dans `src-tauri/src/commands/auth.rs` et `skills.rs` : `https://app.skillreg.dev`
**L'utilisateur ne voit jamais cette URL.**

### Auth flow
Deux méthodes d'authentification :

**Device flow** (recommandé) :
1. Rust `login_initiate()` → POST `/api/v1/auth/cli/initiate` → `{deviceCode, userCode, verificationUrl}`
2. Rust `open_url()` → ouvre le navigateur sur la page d'autorisation
3. Frontend affiche le `userCode`, poll via Rust `login_poll()` toutes les 3s
4. Quand `status: "complete"` → token sauvé dans `~/.skillreg/config.json`

**Token manual** :
1. L'utilisateur colle un token (`sr_live_*`, `sr_test_*`, `sk_*`)
2. Rust `login_with_token()` → vérifie le format + appelle `whoami` pour valider
3. Token sauvé dans `~/.skillreg/config.json`

### Setup wizard
Après première connexion, si `setupDone` est `false`, redirect vers `/setup` :
- Step 1 : Sélection de l'organisation
- Step 2 : Agent par défaut (claude/codex/cursor)
- Step 3 : Scope par défaut (project/user)

### State management
- `useAuthStore` : auth state (authenticated, user, orgs)
- `useConfigStore` : config locale (org, agent, scope, setupDone)
- Pas de prop drilling — les pages lisent directement les stores

### Theme
- Dark par défaut, toggle clair/sombre dans Settings
- Persisté dans `localStorage` (`skillreg-theme`)
- Variables CSS dans `src/styles/globals.css` (classe `.light` sur `<html>`)

## Interop avec skillreg-app

L'app desktop consomme la même API REST que le CLI.
Les fichiers de config sont partagés : `~/.skillreg/config.json`, `.skillregrc`, `~/.skillreg/env/`.
Les chemins d'installation sont identiques : `.claude/skills/`, `.codex/skills/`, `.cursor/skills/`.

## Structure fichiers

```
src-tauri/src/
  commands/
    auth.rs       ← login_initiate, login_poll, login_with_token, whoami, logout, open_url
    config.rs     ← read_config, write_config
    env.rs        ← get_env_vars, set_env_vars, delete_env_vars, list_all_env_vars, import_env_file
    local.rs      ← scan_local_skills
    skills.rs     ← list_skills, get_skill, search_skills, pull_skill, push_skill, uninstall_skill, check_updates
    mod.rs        ← Module exports
  lib.rs          ← Tauri setup (tray icon, menu) + generate_handler! registration

src/
  App.tsx         ← Routes + AuthGate + SetupRoute
  main.tsx        ← Entry point + theme restore
  pages/
    Login.tsx     ← Auth (device flow + token paste)
    Setup.tsx     ← First-run wizard (org, agent, scope)
    Dashboard.tsx ← Orgs overview + org selection
    Catalog.tsx   ← Browse/search skills + pagination
    SkillDetail.tsx ← Detail + versions + README + install
    Installed.tsx ← Local skills scan + update + uninstall
    Publish.tsx   ← Push skill (file picker + dry-run)
    EnvVars.tsx   ← Env vars CRUD + import .env
    Settings.tsx  ← Config + theme toggle + logout
  components/
    layout/
      AppShell.tsx  ← Titlebar + Sidebar + main
      Sidebar.tsx   ← Navigation (7 items)
      Titlebar.tsx  ← Custom draggable titlebar + window controls
    ui/           ← badge, button, card, input, label, select
  lib/
    api.ts        ← invoke() wrappers typés (20 commandes)
    store.ts      ← Zustand stores (auth, config)
    types.ts      ← Types partagés (Rust ↔ TS)
    constants.ts  ← API_BASE_URL, AGENTS, SCOPES
    utils.ts      ← cn() helper
  styles/
    globals.css   ← Tailwind base + dark/light theme vars + prose
```

## Commandes Rust enregistrées (20)

| Module | Commandes |
|--------|-----------|
| auth | `login_initiate`, `login_poll`, `login_with_token`, `whoami`, `logout`, `open_url` |
| config | `read_config`, `write_config` |
| local | `scan_local_skills` |
| skills | `list_skills`, `get_skill`, `search_skills`, `pull_skill`, `push_skill`, `uninstall_skill`, `check_updates` |
| env | `get_env_vars`, `set_env_vars`, `delete_env_vars`, `list_all_env_vars`, `import_env_file` |

## État actuel

- **Phase 1** : ✅ terminée (skeleton, layout, config, dark theme, custom titlebar)
- **Phase 2** : ✅ terminée (device flow, token login, setup wizard, dashboard, route guards, logout)
- **Phase 3** : ✅ terminée (catalog, search, pagination, skill detail, pull/install, agent/scope picker)
- **Phase 4** : ✅ terminée (installed page, scan local, updates, uninstall, publish, push, dry-run)
- **Phase 5** : ✅ terminée (env vars CRUD, import .env, masquage, settings, thème clair/sombre)
- **Phase 6** : ✅ ~95% (tray icon, icônes app, CI/CD, titlebar, signature/notarization macOS release — reste : page de téléchargement)

## Reste à faire

- Releases macOS: secrets GitHub requis pour signature + notarization (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`)
- Page de téléchargement sur skillreg.dev
