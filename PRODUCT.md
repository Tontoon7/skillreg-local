# SkillReg Local — Product Context

## Product

SkillReg Local is the desktop surface that lets employees discover and use
company-approved AI capabilities without managing files, versions, agents, or command-line
tools.

The product is part of SkillReg:

- the desktop app handles local installation, updates, configuration, and repair;
- the web app handles governance, version approval, members, and administration;
- the CLI remains the advanced surface for authors, automation, and project-scoped installs.

## Primary user

The primary user is a non-technical business employee inside a company.

They understand the outcome they want—prepare a client meeting, summarize a report, analyze a
market, draft a response—but should not need to understand:

- filesystem paths;
- `user` versus `project` scope;
- Claude, Codex, or Cursor installation folders;
- semantic versions;
- checksums;
- symlinks or junctions;
- API tokens in the normal sign-in flow.

## Primary job to be done

> When I need my AI assistant to perform a company-approved task, I want to add the capability
> in one click and know that it is ready, trusted, configured, and kept up to date.

## Product promise

SkillReg manages the technical decisions on the employee's behalf:

1. the employee signs in with their company account;
2. SkillReg selects the relevant organization;
3. SkillReg detects compatible assistants;
4. the employee chooses a useful skill from the approved catalog;
5. SkillReg installs one canonical copy and makes it available to every compatible assistant;
6. SkillReg keeps it healthy and explains only actions the employee can take.

## User roles

### Employee

Can:

- browse the approved catalog;
- install, update, configure, repair, and remove optional skills;
- see whether assistants and skills are ready;
- control automatic local updates;
- later see reliable usage and cleanup suggestions.

Does not choose:

- the version;
- the installation target;
- the agent for each install;
- a local scope;
- a link mechanism.

### Administrator

Controls:

- allowed sources;
- approved or pinned versions;
- member access and governance;
- later, adoption visibility and distribution policy.

### Author or advanced user

Uses the web app and CLI for:

- publishing and versioning;
- project-scoped installs;
- automation and CI;
- proposals and advanced inspection.

## Experience principles

### Outcome before mechanism

Lead with what a skill enables. Technical details remain available for support, but never
compete with the primary action.

### One obvious action

Each state has one primary action:

- `Installer`;
- `Configurer`;
- `Mettre à jour`;
- `Réparer`;
- or no action when everything is ready.

### Honest state

SkillReg distinguishes:

- ready;
- action required;
- conflict;
- offline;
- unsupported;
- unknown.

Unknown information is never shown as a successful zero or a failure.

### Safe and reversible

SkillReg never overwrites an unmanaged folder. Installs and updates are transactional, and a
failed update keeps the previous working content.

### Privacy by default

SkillReg never collects prompts, conversations, responses, file contents, working
directories, environment values, or agent log lines.

Any future usage reporting is aggregated locally and enabled only when an agent exposes a
validated signal.

## Main employee journey

### First run

1. `Se connecter`
2. Browser authorization
3. Automatic organization selection when unambiguous
4. Automatic assistant detection
5. Safe migration preview when legacy installs exist
6. `Vos assistants sont prêts`

### Install

1. Open `Catalogue`
2. Understand the business outcome
3. Select `Installer`
4. Complete required configuration if needed
5. See `Prête à utiliser`

### Maintain

The home screen answers:

- Are my assistants ready?
- Are automatic updates on?
- Is there anything I need to do?
- Which skills are installed?

### Remove

Removal is always confirmed. It removes only SkillReg-owned bindings and canonical content.
External or modified installations stay untouched.

## Main navigation

- `Accueil`
- `Catalogue`
- `Mes skills`
- `Réglages`

Creator and diagnostic tools belong under `Avancé` or in the web/CLI surfaces.

## Vocabulary

### Preferred

- assistant;
- capability;
- skill;
- available;
- ready;
- action required;
- automatic updates;
- company;
- workspace;
- repair;
- configuration.

### Avoid in the primary journey

- agent target;
- scope;
- user scope;
- project scope;
- semantic version;
- install path;
- symlink;
- junction;
- tarball;
- checksum;
- manifest;
- API token.

These terms may appear in advanced diagnostics when they help support.

## Non-goals for the first managed-storage release

- mandatory assignments;
- group deployment;
- automatic removal for inactivity;
- multiple simultaneously active organizations;
- migration of slash commands;
- removal of project scope from the CLI;
- claiming usage visibility where no reliable signal exists;
- replacing the existing visual identity.

## Success measures

### Activation

- median sign-in-to-ready time;
- first-attempt installation success;
- percentage of employees who never encounter agent, scope, path, or version controls;
- compatible assistant detection rate.

### Reliability

- successful atomic updates;
- ready binding rate;
- successful rollback rate;
- zero unmanaged-folder overwrites;
- successful migration rate.

### Comprehension

- employee can identify the global health state;
- employee can find the automatic-update control without opening Settings;
- every error offers a safe next action;
- unknown usage is understood as unknown.

## Product anti-patterns

- dashboards made primarily of vanity metrics;
- setup wizards that teach filesystem concepts;
- multiple equally prominent actions;
- hidden destructive operations;
- auto-remediation that overwrites user content;
- productivity rankings;
- treating absence of telemetry as inactivity;
- exposing admin policy controls in the employee desktop.

## Related documents

- `../skillreg-app/docs/plans/2026-07-29-non-technical-employee-experience-design.md`
- `../skillreg-app/docs/plans/2026-07-29-non-technical-employee-experience-implementation-plan.md`
- `../skillreg-app/docs/plans/2026-07-29-admin-managed-skill-distribution-design.md`
