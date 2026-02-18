# SkillReg Local — Roadmap

> Dernière mise à jour : 2026-02-18

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
- [x] Uninstall
- [x] Push : file picker / drag & drop
- [x] Preview SKILL.md
- [x] Version bumper
- [x] Dry-run
- [x] Upload + progress bar
- [x] Security scan warnings

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

---

## Phase 6 — Polish + Packaging

- [x] Tray icon
- [x] Auto-update (config)
- [x] Icône app (toutes tailles, .icns, .ico)
- [x] Custom titlebar
- [x] Notifications OS
- [x] Packaging (.dmg, .msi, .AppImage) — config prête
- [x] CI/CD GitHub Actions
- [x] ~~Code signing~~ — reporté (budget), distribution unsigned
- [ ] Page téléchargement skillreg.dev
