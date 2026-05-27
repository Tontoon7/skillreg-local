# SkillReg Local — Roadmap

> Dernière mise à jour : 2026-05-26

---

## Légende

| Symbole | Signification |
|---------|---------------|
| `[x]` | Terminé |
| `[~]` | En cours |
| `[ ]` | À faire |

---

## Phase 1 — Squelette

- [x] Init projet Tauri v2
- [x] Setup Vite + React 19 + TypeScript
- [x] Tailwind CSS v4 + shadcn/ui (composants de base)
- [x] React Router v7 (pages vides)
- [x] Layout : AppShell + Sidebar
- [x] Commandes Rust : `read_config`, `write_config`
- [x] Types partagés (Rust ↔ TypeScript)
- [x] Design system dark theme (cohérent avec app web)
- [x] Custom titlebar (draggable)

---

## Phase 2 — Auth + Dashboard

- [x] Device flow login (initiate + poll + open browser) — via Rust reqwest
- [x] Token manual login
- [x] Setup wizard (org, agent, scope)
- [x] Dashboard avec liste des orgs
- [x] Stockage token dans `~/.skillreg/config.json`
- [x] Logout
- [x] Route guards (redirect login)
- [x] API URL hardcodée (https://app.skillreg.dev) — invisible à l'utilisateur

---

## Phase 3 — Catalog + Install

- [x] Liste skills (pagination)
- [x] Recherche full-text (debounce)
- [x] Filtres : tags, tri
- [x] Skill detail (markdown, onglets)
- [x] Pull/install (download + SHA-256 + extraction)
- [x] Sélecteur agent/scope
- [x] Badge "Installed"
- [x] Progress indicator

---

## Phase 4 — Local Skills + Publish

- [x] Scan local skills (commande Rust)
- [x] Parse SKILL.md frontmatter
- [x] Détection symlinks
- [x] Détection updates
- [x] Vue Installed groupée par skill avec badges multi-agents/scopes
- [x] Uninstall
- [x] Push : file picker / drag & drop
- [x] Preview SKILL.md
- [x] Version bumper
- [x] Dry-run
- [x] Upload + progress bar
- [x] Security scan warnings

## Slash Commands

- [x] Lister les slash commands du registre depuis l'app desktop
- [x] Installer une command pour Claude, Codex, Cursor ou tous les agents compatibles
- [x] Support des chemins locaux commands : `.claude/commands`, `.cursor/commands`, `.codex/skills`
- [x] Manifest local partagé avec le CLI : `~/.skillreg/commands.json`
- [x] Vue des commands installées localement avec update et remove
- [ ] Création/publication de nouvelles versions de commands depuis le desktop

---

## Phase 5 — Env Vars + Settings

- [x] CRUD env vars
- [x] Détection auto vars requises
- [x] Import .env
- [x] Masquage valeurs
- [x] Warnings vars manquantes
- [x] Injection ${VAR}
- [x] Settings page (org, agent, scope, sign out)
- [x] Thème clair/sombre

## Env Engine — Phase 1

- [x] Inventaire variable-first depuis les skills installés et les fichiers `.env` legacy
- [x] Détection des variables déclarées dans `SKILL.md` via `env:`
- [x] Regroupement des skills dépendants par variable
- [x] Vue Environment centrée sur variables manquantes, configurées, optionnelles et inutilisées
- [x] Configuration d'une variable en une action pour les skills installés localement
- [x] Badges de statut env dans Installed : ready, missing, optional, not required

## Env Engine — Phase 2

- [x] Stockage org-level des variables dans `~/.skillreg/env/{org}/variables.env`
- [x] Couche Rust `EnvStore` : get, set, delete, list, aperçu/migration legacy
- [x] UI Environment branchée sur une sauvegarde unique par organisation
- [x] Inventaire et badges Installed compatibles org-level + legacy non conflictuel
- [x] Migration sûre des fichiers legacy identiques, avec conflits exposés sans écrasement
- [x] Flow d'installation : ne demande que les variables requises manquantes
- [x] Documentation du fichier org-level comme backend temporaire avant secure store

## Env Engine — Phase 3

- [x] Remplacer `variables.env` par Keychain / Credential Manager / Secret Service quand disponible
- [x] Ajouter `~/.skillreg/env/{org}/index.json` comme index sans secrets
- [x] Garder `variables.env` comme fallback permissionné si le secure store est indisponible
- [x] Ajouter une migration explicite du fallback Phase 2 vers le secure store
- [x] Afficher le backend de stockage dans Environment
- [x] Ajouter une stratégie de cleanup explicite pour les fichiers legacy migrés

---

## Phase 6 — Polish + Packaging

- [x] Tray icon
- [x] Auto-update (plugin + keypair + UI banner)
- [x] Auto-update des skills installés via manifest local et worker tray
- [x] Icône app (toutes tailles, .icns, .ico)
- [x] Custom titlebar
- [x] Notifications OS
- [x] Packaging (.dmg, .msi, .AppImage) — config prête
- [x] CI/CD GitHub Actions
- [x] Releases macOS signées + notarized en CI (requiert secrets Apple GitHub)
- [ ] Page téléchargement skillreg.dev
