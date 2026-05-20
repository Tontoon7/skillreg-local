# SkillReg Env Engine Spec

> Date: 2026-05-20
> Status: Draft ready for implementation planning
> Owner: SkillReg Local, with CLI alignment required
> Related surfaces: `skillreg-local`, `skillreg-app/packages/cli`, `SKILL.md` contract

## Summary

SkillReg needs a first-class local environment variable engine. The current desktop and CLI can store variables per skill in `~/.skillreg/env/{org}/{skill}.env`, but that model forces users to think skill-by-skill, duplicates common secrets such as `OPENAI_API_KEY`, and does not reliably make those values available to the agent process.

The new feature turns environment variables into local, org-scoped resources:

- A variable is configured once per organization on the local machine.
- SkillReg scans installed skills to know which variables each skill requires.
- The desktop Environment page is centered on variables, not on a skill selector.
- Values are stored securely in the OS credential store where possible.
- SkillReg provides explicit injection paths so Claude Code, Codex, Cursor, and shells can receive the variables at runtime.
- The registry never stores secret values. It stores only declarations and metadata from `SKILL.md`.

## Problem

Today the Environment tab asks the user to select a skill, then manage variables for that one skill. That has several product problems:

1. Users do not know which skill to select first.
2. Shared variables are duplicated across skills.
3. There is no inventory of all required variables on the machine.
4. The UI does not answer "what is missing?" globally.
5. Stored values are local files, not secure OS secrets.
6. Agents will not naturally look inside SkillReg env files.
7. CLI injection currently mutates installed skill Markdown with visible values, which is useful as a workaround but is not the right long-term security boundary.

The user mental model should be:

> "What environment variables does my local agent setup need, which ones are missing, and how does SkillReg make them available to my agents?"

Not:

> "Which skill do I select so I can maybe discover which variables it needs?"

## Goals

- Provide a variable-first Environment page in the desktop app.
- Configure each variable once per org and local machine.
- Show which installed skills require or optionally use each variable.
- Detect missing variables from installed `SKILL.md` files and installed metadata.
- Store secret values in the OS credential store by default.
- Keep a migration path from existing per-skill `.env` files.
- Provide CLI commands for listing, setting, deleting, exporting, and checking env values.
- Provide a runtime injection mechanism for terminal-launched agents.
- Avoid writing secret values into `SKILL.md`, project files, shell config files, logs, screenshots, or registry records.

## Non-Goals

- Do not sync secret values to SkillReg cloud.
- Do not introduce per-skill overrides in the first version.
- Do not support team-wide shared secret distribution.
- Do not silently edit `~/.zshrc`, `~/.bashrc`, or project `.env` files without explicit user action.
- Do not guarantee automatic injection into already-running apps. Existing processes cannot receive new environment variables from SkillReg.
- Do not solve every GUI-agent integration in v1. Cursor and other GUI apps may need later launchers or agent-specific adapters.

## Product Principles

### One Local Variable, Many Skills

`OPENAI_API_KEY` configured for org `kairia` is available to every installed skill in `kairia` that declares `OPENAI_API_KEY`.

If two skills need different values, the skills should use different variable names. For v1, SkillReg should avoid skill-level overrides because they add hidden complexity and make the UI harder to understand.

### Variables Are Local Machine State

Variables are not registry content. They are local runtime configuration for this machine.

SkillReg may store non-secret metadata in files, for example variable names, descriptions, required/optional status, and which skills reference them. Values stay in the secure store.

### Agents Need Runtime Environment, Not Documentation Only

A skill can document that it expects `OPENAI_API_KEY`, but the agent process must actually receive `OPENAI_API_KEY` in its process environment or through an explicit SkillReg command.

SkillReg's responsibility is to bridge from secure local storage to agent runtime.

## Existing State

### Desktop

Current desktop environment behavior:

- `src-tauri/src/commands/env.rs` stores values in `~/.skillreg/env/{org}/{skill}.env`.
- `src/pages/EnvVars.tsx` uses a skill selector and edits variables for the selected skill.
- `src/components/EnvVarSetupDialog.tsx` appears after install if `pull_skill` returns `envVars`.
- `src-tauri/src/commands/skills.rs` parses `env:` from the installed `SKILL.md` after pulling a skill.

### CLI

Current CLI behavior:

- `skillreg env set <skill> KEY=value` writes `~/.skillreg/env/{org}/{skill}.env`.
- `skillreg pull` runs an env wizard from `env:` frontmatter or fallback content detection.
- `skillreg local` can inject stored values into installed skill Markdown between `skillreg:env` comments.
- `env-detect.ts` detects likely env names from content patterns and placeholders.

This gives useful pieces but not the desired product model.

## Proposed Model

### Storage Model

Each org has a local env namespace.

```text
org: kairia
variable: OPENAI_API_KEY
credential key: skillreg:kairia:env:OPENAI_API_KEY
```

The value lives in the OS credential store.

Recommended backend:

- macOS: Keychain
- Windows: Credential Manager
- Linux: Secret Service where available
- Fallback: encrypted or permission-restricted file only when secure storage is unavailable or explicitly enabled

Non-secret index data may live in:

```text
~/.skillreg/env/index.json
```

Example index:

```json
{
  "version": 1,
  "orgs": {
    "kairia": {
      "variables": {
        "OPENAI_API_KEY": {
          "configured": true,
          "updatedAt": "2026-05-20T10:00:00.000Z",
          "source": "keychain"
        }
      }
    }
  }
}
```

The index must not contain secret values.

### Variable Declarations

Skill authors declare required variables in `SKILL.md` frontmatter:

```yaml
---
name: github-reviewer
description: Review pull requests
env:
  - name: GITHUB_TOKEN
    description: GitHub token with pull request read access.
    required: true
    secret: true
  - name: GITHUB_OWNER
    description: Default GitHub owner.
    required: false
    secret: false
---
```

Fields:

| Field | Required | Description |
| --- | --- | --- |
| `name` | yes | Uppercase env variable name. |
| `description` | no | Human-readable setup guidance. |
| `required` | no | Defaults to `true`. |
| `secret` | no | Defaults to `true` for names ending in `_KEY`, `_TOKEN`, `_SECRET`, `_PASSWORD`; otherwise `false`. |
| `default` | no | Non-secret default value. Must not be used for credentials. |

Validation:

- `name` must match `^[A-Z][A-Z0-9_]{2,}$`.
- `default` is rejected for variables inferred as secret unless `secret: false`.
- Duplicate variable names in one skill collapse into one declaration.

### Detection Pipeline

SkillReg builds the variable inventory from installed skills:

1. Scan installed skill directories across Claude, Codex, and Cursor.
2. Parse `SKILL.md` frontmatter `env:`.
3. If no `env:` exists, run high-confidence content detection:
   - `${OPENAI_API_KEY}`
   - `process.env.OPENAI_API_KEY`
   - `os.environ["OPENAI_API_KEY"]`
   - documented patterns such as `Environment Variable: OPENAI_API_KEY`
4. Mark frontmatter declarations as `declared`.
5. Mark detected-only variables as `detected` and show lower confidence in the UI.
6. Join declarations by variable name across all installed skills in the selected org.

The desktop should prefer declarations over detection. Detection is a fallback, not a replacement for good skill metadata.

### Inventory Shape

The Rust/CLI shared model should produce a normalized inventory:

```ts
interface EnvInventory {
  org: string;
  variables: EnvVariableInventoryItem[];
  orphanedStoredVariables: EnvStoredVariable[];
}

interface EnvVariableInventoryItem {
  name: string;
  configured: boolean;
  secret: boolean;
  requiredBy: EnvSkillReference[];
  optionalFor: EnvSkillReference[];
  detectedIn: EnvSkillReference[];
  description: string | null;
  defaultValue: string | null;
  source: "declared" | "detected" | "mixed";
  updatedAt: string | null;
}

interface EnvSkillReference {
  skillName: string;
  agent: "claude" | "codex" | "cursor";
  scope: "user" | "project";
  path: string;
  version: string;
}

interface EnvStoredVariable {
  name: string;
  configured: boolean;
  referencedByInstalledSkills: boolean;
  updatedAt: string | null;
}
```

## Desktop UX

### Environment Page

Replace the skill selector page with a variable inventory.

Primary sections:

1. `Needs attention`
2. `Configured`
3. `Optional`
4. `Unused stored variables`

Example:

```text
Environment Variables

Needs attention
GITHUB_TOKEN        Required by github-reviewer, release-notes       Configure
OPENAI_API_KEY     Required by code-reviewer, copywriter, seo-audit  Configure

Configured
NOTION_API_KEY     Used by notion-sync                               Edit
RESEND_API_KEY     Used by email-promoter                            Edit

Optional
GITHUB_OWNER       Optional for github-reviewer                       Configure

Unused stored variables
SLACK_BOT_TOKEN    Not required by installed skills                   Delete
```

Each variable row shows:

- variable name
- configured/missing status
- required/optional/detected badges
- skills that use it, grouped compactly
- last updated timestamp if available
- actions: `Configure`, `Edit`, `Delete`, `Reveal`, `Copy`, `Export`

Secret values remain masked by default. Revealing requires an explicit click.

### Variable Detail Drawer

Clicking a variable opens a drawer or side panel:

```text
OPENAI_API_KEY
Status: Configured
Storage: macOS Keychain

Required by
- code-reviewer, Claude, user
- seo-audit, Codex, user
- copywriter, Cursor, project

Value
[masked secret value] [Reveal] [Copy]

Actions
[Save] [Delete value]
```

For missing variables, the primary action is a single input field:

```text
GITHUB_TOKEN
Required by github-reviewer and release-notes.

Value
[                                      ]

[Save]
```

### Install Flow

When installing a skill with required variables:

1. Install the files.
2. Parse required variables.
3. Check org-level vault values.
4. If all required values already exist, show "Environment ready".
5. If required values are missing, open a setup dialog listing only missing variables.
6. Save values into the org-level vault.
7. Return to skill detail with configured status.

The install flow should not ask for a value that is already configured for the org.

### Installed Page

Installed skill groups should show env status:

- `Env ready`
- `Env missing: 2`
- `Env optional`
- `Env not required`

Clicking the badge opens the Environment page filtered to variables used by that skill.

## CLI UX

The CLI should move from skill-first env commands to variable-first env commands while preserving compatibility.

### New Commands

```bash
skillreg env list [--org <slug>]
skillreg env get <KEY> [--org <slug>]
skillreg env set <KEY> [--org <slug>] [--value <value>]
skillreg env delete <KEY> [--org <slug>]
skillreg env status [--org <slug>]
skillreg env export [--org <slug>] [--format shell|json]
skillreg env doctor [--org <slug>]
```

Behavior:

- `env list` lists variables, not skills.
- `env status` shows configured/missing grouped by variable.
- `env export --format shell` prints shell-safe `export KEY=...` statements for configured variables.
- `env doctor` scans installed skills and reports missing required variables.
- `env get` prints a single value only when explicitly requested. It should warn when stdout is a TTY.

### Backward Compatibility

Keep current skill-first commands temporarily:

```bash
skillreg env set <skill> KEY=value
skillreg env list <skill>
skillreg env delete <skill> KEY
```

Migration behavior:

- Reads existing per-skill files.
- Offers migration to org-level variables.
- If the same key has the same value across multiple skill files, migrate automatically.
- If the same key has conflicting values, ask the user to pick one or rename variables in affected skills.

## Agent Runtime Injection

### Terminal-Launched Agents

For Claude Code, Codex CLI, and other terminal-launched agents, the primary injection path is shell export at runtime.

Recommended command:

```bash
eval "$(skillreg env export --org kairia --format shell)"
```

This exports variables into the current shell session. Any agent launched from that shell inherits them.

SkillReg may offer setup automation:

```bash
skillreg env shell setup --org kairia --shell zsh
```

This can add a managed block to `~/.zshrc`, but only after explicit confirmation:

```zsh
# skillreg env start
eval "$(skillreg env export --org kairia --format shell)"
# skillreg env end
```

The shell config must not contain raw secret values.

### Desktop App Launchers

For GUI apps that do not inherit the terminal environment, SkillReg can add explicit launch actions later:

```text
Launch Cursor with SkillReg env
Launch Claude Code with SkillReg env
Launch terminal with SkillReg env
```

In v1, this is optional. The spec requires the storage and export engine first.

### Agent Skill Instructions

Skill instructions should reference variable names, not values:

```md
This skill requires `OPENAI_API_KEY`.
Use the `OPENAI_API_KEY` environment variable when running API calls or scripts.
```

SkillReg should not inject the actual value into `SKILL.md` as the normal path.

## Security

### Required Rules

- Never send secret values to SkillReg API.
- Never store secret values in `SKILL.md`.
- Never include secret values in telemetry, logs, crash reports, or screenshots.
- Mask values by default in UI and CLI.
- Require explicit user action to reveal or copy a value.
- Avoid printing values except for `env get` and `env export`, both explicit commands.
- Shell setup writes commands, not values.
- Migration from plaintext files should remove or archive old files only after confirmation.

### Secure Store Choice

Preferred Rust crate: `keyring`, unless Tauri has an official secure storage plugin available and stable in this repo's dependency constraints.

Credential naming:

```text
service: skillreg
account: {org}:env:{KEY}
```

Metadata index stores only:

- key name
- configured boolean
- storage backend
- timestamps
- migration source

### Fallback Storage

Fallback storage is allowed only when secure store is unavailable.

Fallback requirements:

- File permissions should be owner-read/write only.
- UI must mark the storage as less secure.
- User can retry migration to secure store.
- Fallback file must live outside installed skill directories.

## Migration

Existing files:

```text
~/.skillreg/env/{org}/{skill}.env
```

Migration steps:

1. Scan all per-skill env files.
2. Build a map of `KEY -> values`.
3. If a key has exactly one unique value, migrate it to org-level secure store.
4. If a key has multiple values, mark as conflict.
5. Show conflicts in desktop:

```text
GITHUB_TOKEN has 2 different saved values:
- github-reviewer
- release-notes

Choose one value for GITHUB_TOKEN, or skip migration.
```

6. After successful migration, keep old files for one release as backup.
7. Later, offer cleanup.

The first implementation should read both secure store and legacy files so users are not broken mid-migration.

## Cross-Repo Impact

### `skillreg-local`

Likely changes:

- `src-tauri/src/commands/env.rs`
  - introduce secure store backend
  - add variable-first commands
  - keep legacy skill env commands
  - add inventory command
- `src-tauri/src/commands/local.rs`
  - scan env declarations and detected variables
  - return env metadata on local skills or a separate inventory response
- `src-tauri/src/commands/skills.rs`
  - after install, check org-level vault values
  - return missing env variables only
- `src/lib/api.ts`
  - add typed invoke wrappers for inventory, set/get/delete/export
- `src/lib/types.ts`
  - add `EnvInventory`, `EnvVariableInventoryItem`, `EnvSkillReference`
- `src/pages/EnvVars.tsx`
  - replace skill selector with variable-first dashboard
- `src/components/EnvVarSetupDialog.tsx`
  - adapt to variable-first saving
- `src/pages/Installed.tsx`
  - show env status per grouped skill

### `skillreg-app/packages/cli`

Likely changes:

- `src/config.ts`
  - add secure store abstraction or hand off to a native helper
  - keep legacy file readers for migration
- `src/commands/env.ts`
  - variable-first command surface
  - `env export`, `env status`, `env doctor`
- `src/commands/pull.ts`
  - ask only for missing org-level variables
  - save to org-level secure store
- `src/commands/local.ts`
  - stop writing secret values into `SKILL.md` by default
  - report missing env status
- `src/env-detect.ts`
  - reuse or publish shared detection rules

### `skillreg-app/packages/shared`

Potential future shared code:

- env declaration schema
- variable-name validation
- secret inference rules
- normalized inventory types if reused by API docs or CLI

No server-side secret storage is required.

## Acceptance Criteria

### Desktop

- Environment page lists variables first.
- Each variable row shows which installed skills require it.
- Required missing variables appear in `Needs attention`.
- Configuring `OPENAI_API_KEY` once marks every dependent installed skill as ready.
- Installed page shows env readiness per grouped skill.
- Install flow asks only for missing variables.
- Existing configured values are reused across skills.
- Values are masked by default and stored outside installed skill directories.

### CLI

- `skillreg env list` shows variable inventory.
- `skillreg env set OPENAI_API_KEY` stores one org-level value.
- `skillreg env status` reports missing variables from installed skills.
- `skillreg env export --format shell` exports configured values for the current org.
- Existing `skillreg env set <skill> KEY=value` does not break immediately.
- `skillreg pull` no longer asks for values already configured for the org.

### Security

- No secret value is written into `SKILL.md` in the default path.
- No secret value is stored in registry API/database/storage.
- No secret value appears in regular logs or UI without reveal/copy action.
- Shell setup stores only an `eval` command, not raw values.

## Phased Implementation

### Phase 1: Inventory and UX Model

- Keep legacy `.env` files as storage.
- Build variable-first inventory from installed skills and existing values.
- Replace desktop Environment page with variable-first UI.
- Add Installed env badges.
- Do not introduce secure store yet.

This phase validates the product model without taking on OS credential complexity.

### Phase 2: Org-Level Storage

- Add org-level storage while still using local files.
- Migrate from `{skill}.env` files into `{org}` variable namespace.
- Update CLI commands to variable-first UX.
- Update install flows to ask only for missing variables.

### Phase 3: Secure Store

- [x] Add OS credential store backend.
- [x] Store values in Keychain/Credential Manager/Secret Service where available.
- [x] Keep metadata index file without secrets.
- [x] Keep `variables.env` as a permission-restricted fallback when secure storage is unavailable.
- [x] Add explicit migration from the Phase 2 fallback file to the secure store.
- [ ] Add cleanup strategy for legacy per-skill files after a documented confirmation flow.

### Phase 4: Runtime Injection

- Add `skillreg env export --format shell`.
- Add shell setup command with explicit confirmation.
- Remove default CLI behavior that writes secret values into installed skill Markdown.
- Add `env doctor` checks.

### Phase 5: Agent Launchers

- Add desktop launchers for GUI agents where practical.
- Add project-level helper commands if needed.
- Document agent-specific limits.

## Testing Strategy

### Unit Tests

- Env declaration parser handles YAML list forms.
- Content detector finds high-confidence env variables.
- Inventory groups multiple skills under one variable.
- Missing status changes when a variable is configured once.
- Legacy migration resolves duplicate identical values.
- Legacy migration detects conflicts.
- Shell export escaping handles quotes, spaces, newlines, and empty values safely.

### Rust Tests

- Env secure-store abstraction can be tested with an in-memory backend.
- File fallback sets restrictive permissions where platform allows.
- Tauri commands return masked metadata and never secret values in inventory responses.

### CLI Tests

- `env list`, `env status`, `env export`, `env doctor`.
- Backward-compatible skill-first commands.
- Pull wizard skips already configured org-level variables.

### Manual Verification

- Install a skill that requires `OPENAI_API_KEY`.
- Configure `OPENAI_API_KEY` once.
- Install a second skill requiring `OPENAI_API_KEY`.
- Confirm there is no second prompt.
- Launch a terminal with `eval "$(skillreg env export --org kairia --format shell)"`.
- Confirm `env | grep OPENAI_API_KEY` shows the variable in that shell only.
- Confirm closing the shell removes the runtime export.

## Open Questions

1. Should org-level env variables be keyed only by org slug, or also by SkillReg account/user id to avoid collisions if two accounts use the same slug?
2. Should `NEXT_PUBLIC_` and similar non-secret variables be stored in the same vault or in a visible local config file?
3. Should desktop v1 offer shell setup, or should shell setup wait for the CLI phase?
4. Should `skillreg env export` export every configured variable or only variables referenced by installed skills?
5. Should `env get` exist, or is it too easy to leak secrets in terminal history/logs?

## Recommendation

Implement Phase 1 first. It delivers the UX correction immediately and creates the inventory model needed for every later phase.

The first shipping slice should be:

1. Add env metadata to desktop local scan or a new inventory Tauri command.
2. Build variable-first `EnvVars` page from installed skills.
3. Save values once per org variable, still using local files temporarily.
4. Show env readiness in Installed.
5. Leave secure store and shell injection for the next focused slice.
