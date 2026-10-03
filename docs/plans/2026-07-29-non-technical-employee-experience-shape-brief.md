# Non-technical Employee Experience — Shape Brief

**Date:** 2026-07-29

**Status:** implementation brief

**Inputs:** `PRODUCT.md`, `DESIGN.md`, and the cross-surface product design in
`skillreg-app/docs/plans/2026-07-29-non-technical-employee-experience-design.md`.

## Shape

SkillReg Local becomes an employee capability library. The desktop app should feel like a
small, trustworthy control surface that quietly keeps company skills available across the
employee's AI assistants.

The interaction model is:

```text
sign in → prepare automatically → browse → install → ready
```

Diagnostics and creator workflows still exist, but they are progressive disclosure rather
than the product's front door.

## Information architecture

### Primary navigation

```text
Accueil
Catalogue
Mes skills
Réglages
```

### Advanced navigation

Available from `Réglages > Avancé`:

- Commands;
- Environment;
- publishing/local inspection;
- technical diagnostics.

No existing advanced route is deleted during the transition.

## Window frame

```text
┌────────────────────────────────────────────────────────────────────┐
│ Native/custom titlebar                                             │
├────────────────┬───────────────────────────────────────────────────┤
│ SkillReg       │                                                   │
│                │ Main page                                         │
│ ● Accueil      │                                                   │
│ ○ Catalogue    │                                                   │
│ ○ Mes skills   │                                                   │
│ ○ Réglages     │                                                   │
│                │                                                   │
│ Avancé ▸       │                                                   │
│                │                                                   │
│ user@company   │                                                   │
└────────────────┴───────────────────────────────────────────────────┘
```

Retain the existing titlebar, brushed-metal sidebar, amber active state, and independent
content scrolling.

## First-run screens

### Sign in

```text
                    [SkillReg logo]

                Connectez-vous à SkillReg
       Retrouvez les capacités approuvées par votre entreprise.

                  [ Se connecter ]

                  Connexion avancée ▾
```

Expanding `Connexion avancée` reveals token sign-in. Browser sign-in remains the default.

States:

- idle;
- browser opened with device code;
- waiting;
- expired;
- offline;
- browser could not open;
- authenticated.

### Multiple organizations

Shown only when no valid prior choice can be reused:

```text
Choisissez votre entreprise

[ Kairia                         ]
[ Client Example                 ]
```

Use organization names. Role and slug are support details.

### Preparing assistants

```text
Préparation de vos assistants

✓ Espace d'entreprise
• Recherche des assistants
• Vérification de vos skills

This usually takes a few seconds.
```

Detection failure does not trap the user. It leads to a clear empty state with
`Relancer la détection`.

### Migration summary

```text
SkillReg peut simplifier vos installations

7 skills seront centralisées.
2 installations de projet resteront inchangées.
1 skill modifiée restera à son emplacement actuel.

[ Continuer ]  [ Plus tard ]
Voir le détail
```

Never use a generic confirmation without counts and categories.

## Home

### Standard ready state

```text
Vos assistants sont prêts                          3 connectés
Toutes vos skills gérées sont disponibles.

┌──────────────────────────────────────────────────────────────┐
│ Maintenir mes skills à jour                         [ ON ]    │
│ Dernière vérification aujourd'hui à 09:42        Vérifier    │
└──────────────────────────────────────────────────────────────┘

À faire
No action required.

Mes skills
Compte-rendu client        Prête                    À jour
Analyse de marché          Prête                    À jour
```

### Action required

Actions are sorted by severity and are directly actionable:

```text
Configurer une clé pour Analyse de marché      [ Configurer ]
Réparer le lien Cursor pour Compte-rendu       [ Réparer ]
Examiner une installation existante            [ Voir ]
```

### Automatic updates off

The toggle remains prominent:

```text
Maintenir mes skills à jour                        [ OFF ]
2 mises à jour approuvées sont disponibles.
                                             [ Tout mettre à jour ]
```

The employee still does not choose versions.

### No assistants

```text
Aucun assistant compatible détecté

Vous pouvez parcourir le catalogue maintenant. SkillReg rendra vos skills disponibles dès
qu'un assistant compatible sera installé.

[ Relancer la détection ]  [ Voir les assistants pris en charge ]
```

### Offline

Keep the last known local state:

```text
Vous êtes hors ligne
Vos skills déjà installées restent disponibles. Les vérifications reprendront automatiquement.
```

## Catalog

### Card anatomy

```text
Meeting Summary                                  Vérifiée
Transforme une réunion en compte-rendu structuré.
Par Kairia

                                                   [ Installer ]
```

Possible primary states:

- `Installer`;
- `Installation…`;
- `Configurer`;
- `Mettre à jour`;
- `Réparer`;
- `Installée`.

Do not show:

- version selector;
- agent checkboxes;
- scope;
- project directory;
- install command as a primary element.

### Detail page

Order:

1. outcome and trust;
2. primary action;
3. examples of requests;
4. required data or configuration;
5. detected assistants, informational only;
6. advanced technical details, collapsed.

## My skills

One row per canonical installation:

```text
Compte-rendu client
Prête · Claude, Codex et Cursor · À jour                    ···
```

The overflow menu may contain:

- view;
- repair;
- technical details;
- uninstall.

Uninstall requires a confirmation and explains that company environment values are retained
unless removed separately.

## Conflict

```text
SkillReg n'a pas remplacé votre installation existante

Un dossier portant le même nom existe déjà dans Cursor. Il a été conservé.

[ Voir le détail ]  [ Réessayer ]
```

No `Force` action in the employee flow.

## Usage and cleanup, future lot

The section is absent while the usage feature flag is off.

When reliable:

```text
Compte-rendu client        Utilisée 8 fois ces 30 derniers jours
```

When unavailable:

```text
Compte-rendu client        Usage inconnu
```

Cleanup appears only after complete exact coverage:

```text
Skill inutilisée depuis 60 jours
Cette suggestion repose sur 60 jours complets d'observation.

[ Désinstaller ] [ Conserver ] [ Plus tard ]
```

## Interaction rules

- one primary action per panel;
- optimistic UI only for reversible settings, with rollback on failure;
- destructive actions are never optimistic;
- loading placeholders preserve final dimensions;
- errors keep the last usable local state;
- network sync failure does not undo a successful local install;
- keyboard focus follows visual order;
- dialogs return focus to their trigger;
- reduced motion disables continuous glow/pulse.

## Copy rules

Use short, direct French:

- `Prête à utiliser`;
- `Action requise`;
- `Maintenir mes skills à jour`;
- `Vérifier maintenant`;
- `Relancer la détection`;
- `Usage inconnu`.

Avoid:

- `Pull`;
- `Scope`;
- `Target`;
- `Symlink`;
- `Manifest`;
- `Semver`;
- raw error bodies.

## QA states

Every changed page must be checked for:

- loading;
- empty;
- success;
- partial success;
- offline;
- forbidden;
- not found;
- conflict;
- unexpected error;
- keyboard-only use;
- reduced motion;
- long organization and skill names;
- 1024×700 and the configured minimum window size.

## Open visual decision

The existing dark industrial identity is retained. A complete light palette does not yet
exist and must not be improvised during this feature. If light mode is required for release,
define it as a separate token-level task and validate it across all primitives.
