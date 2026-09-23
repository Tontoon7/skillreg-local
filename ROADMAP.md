# SkillReg Local — Roadmap

> Dernière mise à jour : 2026-07-29

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
- [x] Setup historique livré ; remplacé par la préparation automatique en Phase 7
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
- [x] Sélecteur agent/scope legacy livré ; retiré du parcours collaborateur en Phase 7
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
- [x] Messages desktop lisibles pour limites de plan et upgrade requis côté API
- [x] Icône app (toutes tailles, .icns, .ico)
- [x] Custom titlebar
- [x] Notifications OS
- [x] Packaging (.dmg, .msi, .AppImage) — config prête
- [x] CI/CD GitHub Actions
- [x] Releases macOS signées + notarized en CI (requiert secrets Apple GitHub)
- [ ] Page téléchargement skillreg.dev

---

## Phase 7 — Expérience collaborateur métier et stockage central

> Design cross-surface :
> `skillreg-app/docs/plans/2026-07-29-non-technical-employee-experience-design.md`
>
> Plan d'implémentation détaillé :
> `skillreg-app/docs/plans/2026-07-29-non-technical-employee-experience-implementation-plan.md`.

### Lot A — Prioritaire

- [x] Créer et valider `PRODUCT.md`, `DESIGN.md` et le shape brief desktop
- [x] Introduire le manifest géré v2 et les écritures atomiques cross-platform
- [x] Valider les chemins agents et les symlinks macOS pour Claude, Codex et Cursor
- [x] Construire le gestionnaire de bindings sûr, idempotent et non destructif
- [x] Remplacer le wizard organisation/agent/scope par une préparation automatique
- [x] Installer une skill sans exposer agent, scope, chemin ou version
- [x] Stocker une seule copie sous `~/.skillreg/skills`
- [x] Exposer cette copie aux agents détectés via des liens gérés sûrs
- [x] Migrer `installed.json` sans toucher aux installations projet, externes ou modifiées
- [x] Recentrer l'auto-update sur la copie canonique
- [x] Afficher le toggle global d'auto-update sur l'accueil
- [x] Garantir un dashboard sans débordement et des actions visibles à 900×600
- [x] Simplifier la navigation, le catalogue, le détail et « Mes skills »
- [x] Exposer « Rassembler mes skills locales » à tous les utilisateurs connectés
- [x] Importer transactionnellement les dossiers locaux sans publication ni choix technique
- [x] Accepter les descriptions YAML multilignes et les anciens noms d’affichage
- [x] Dogfood macOS arm64 : 47 skills locales, 141 bindings prêts, lien `decktype` intact
- [x] Ajouter réparation ciblée/globale, désinstallation sûre et switch d’organisation transactionnel
- [x] Aligner le CLI en lecture seule du manifest v2 tout en préservant les installations projet
- [x] Passer les tests, typechecks, builds et packaging macOS arm64 local non signé
- [x] Ajouter les gates CI Rust Linux, macOS et Windows
- [~] Valider les agents en runtime, la QA Tauri et le downgrade sur macOS, Windows et Linux
  ([checklist de release](docs/plans/2026-07-29-managed-skills-release-checklist.md))

### Lot B — Différé après stabilisation du lot A

- [ ] Synchroniser un inventaire sans chemins locaux ni contenu utilisateur
- [ ] Activer uniquement les observers d'usage prouvés par fixtures
- [ ] Afficher `Usage inconnu` lorsqu'aucun signal fiable n'existe
- [ ] Suggérer, sans automatiser, la désinstallation après 60 jours fiables
- [ ] Alimenter les vues d'adoption owner/admin

---

## Backlog différé — Distribution administrée aux collaborateurs métiers

> Décision cross-surface détaillée dans
> `skillreg-app/docs/plans/2026-07-29-admin-managed-skill-distribution-design.md`.

- [ ] Consommer les modes de distribution **libre**, **recommandée** et **obligatoire**
- [ ] Appliquer les assignations administrateur sans exposer version, agent ou scope au collaborateur
- [ ] Empêcher la désinstallation locale d'une skill obligatoire
- [ ] Ne jamais interpréter une télémétrie indisponible comme une absence d'usage
