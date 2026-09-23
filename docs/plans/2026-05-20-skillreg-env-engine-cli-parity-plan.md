# SkillReg Env Engine CLI Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Align the SkillReg CLI, website docs, and release/download surface with the desktop Env Engine model shipped in SkillReg Local v0.3.17.

**Architecture:** The desktop app is now the source implementation for local org-level env behavior. The CLI must use the same local layout under `~/.skillreg/env/{org}`: org-level values in a secure-store-ready abstraction, metadata without secret values, and legacy per-skill files as compatibility input only. The website/docs must describe org-level local storage, not per-skill primary storage.

**Tech Stack:** TypeScript CLI in `skillreg-app/packages/cli`, Next.js website docs in `skillreg-website`, Tauri/Rust desktop docs and roadmap in `skillreg-local`.

---

## Preflight

The current `/Users/axel/Documents/skillreg/skillreg-app` worktree is dirty on branch `feat/skill-follow-notifications`. Do not implement this plan in that worktree unless Axel explicitly asks to mix the work.

**Step 1: Create or switch to a clean worktree**

Recommended:

```bash
cd /Users/axel/Documents/skillreg
mkdir -p worktrees
git -C skillreg-app fetch origin
git -C skillreg-app worktree add ../worktrees/skillreg-app-env-cli origin/main
```

Expected: a clean app worktree at `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli`.

**Step 2: Check repo status**

Run:

```bash
git -C /Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli status --short
git -C /Users/axel/Documents/skillreg/skillreg-website status --short
git -C /Users/axel/Documents/skillreg/skillreg-local status --short
```

Expected:
- app worktree clean
- website may only have unrelated local `CLAUDE.md`
- local may still have unrelated `CLAUDE.md` / `CLAUDE 2.md`

---

## Behavior Contract

The CLI and desktop should agree on these rules:

- Primary env storage is org-level: one variable value per local org, shared by every skill that declares the same variable.
- Primary path scope remains under `~/.skillreg/env/{org}`.
- CLI must not write secrets into `SKILL.md`, docs, tests, fixtures, or logs.
- CLI output must mask values unless a future explicit reveal command is designed.
- Existing `~/.skillreg/env/{org}/{skill}.env` files are legacy compatibility files.
- Legacy files are read for readiness and migration, but not used as the default write target.
- If legacy values conflict, CLI must show a conflict and never overwrite.
- Cleanup of migrated legacy backups must be explicit and confirmed.
- Cloud registry must not store env secret values.

---

## Task 1: Add CLI Env Store Helpers

**Files:**
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/config.ts`
- Create: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/env-store.ts`
- Test: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/__tests__/env-store.test.ts`

**Step 1: Write failing tests for org-level paths and parsing**

Create tests that assert:
- `getOrgEnvFilePath("kairia")` returns `~/.skillreg/env/kairia/variables.env`
- `getOrgEnvIndexPath("kairia")` returns `~/.skillreg/env/kairia/index.json`
- `setOrgEnvVar` writes a normalized key once
- `listOrgEnvVars` returns names and metadata, never values
- invalid keys are rejected

Use a temp home directory by setting a test-only env override, for example `SKILLREG_HOME`.

Expected test skeleton:

```ts
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  deleteOrgEnvVar,
  getOrgEnvVar,
  getOrgEnvFilePath,
  listOrgEnvVars,
  setOrgEnvVar,
} from "../../env-store";

let tempHome = "";

beforeEach(() => {
  tempHome = mkdtempSync(join(tmpdir(), "skillreg-env-"));
  process.env.SKILLREG_HOME = tempHome;
});

afterEach(() => {
  delete process.env.SKILLREG_HOME;
  rmSync(tempHome, { recursive: true, force: true });
});

it("stores one org-level value and lists metadata only", () => {
  setOrgEnvVar("kairia", "openai_api_key", "secret-value");

  expect(getOrgEnvFilePath("kairia")).toBe(join(tempHome, ".skillreg", "env", "kairia", "variables.env"));
  expect(getOrgEnvVar("kairia", "OPENAI_API_KEY")).toBe("secret-value");
  expect(listOrgEnvVars("kairia")).toEqual([
    expect.objectContaining({ name: "OPENAI_API_KEY", configured: true, storage: "fallback_file" }),
  ]);
});
```

**Step 2: Run tests and verify failure**

Run:

```bash
cd /Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli
pnpm test -- --run packages/cli/src/commands/__tests__/env-store.test.ts
```

Expected: FAIL because `env-store.ts` does not exist.

**Step 3: Implement minimal file-backed EnvStore for CLI**

Create `packages/cli/src/env-store.ts`.

Implementation requirements:
- Use only Node core modules.
- Respect `SKILLREG_HOME` in tests, otherwise use `homedir()`.
- Write fallback file `~/.skillreg/env/{org}/variables.env` with mode `0o600` where possible.
- Write metadata index `~/.skillreg/env/{org}/index.json` without values.
- Normalize env names to uppercase.
- Reject keys not matching `/^[A-Z][A-Z0-9_]{2,}$/`.
- Do not log values.

Minimum API:

```ts
export interface OrgEnvVariable {
  name: string;
  configured: boolean;
  updatedAt: string | null;
  storage: "fallback_file";
}

export function getOrgEnvDir(org: string): string;
export function getOrgEnvFilePath(org: string): string;
export function getOrgEnvIndexPath(org: string): string;
export function getOrgEnvVar(org: string, key: string): string | undefined;
export function setOrgEnvVar(org: string, key: string, value: string): void;
export function deleteOrgEnvVar(org: string, key: string): boolean;
export function listOrgEnvVars(org: string): OrgEnvVariable[];
export function readOrgEnvVars(org: string): Record<string, string>;
```

**Step 4: Make tests pass**

Run:

```bash
pnpm test -- --run packages/cli/src/commands/__tests__/env-store.test.ts
```

Expected: PASS.

**Step 5: Commit**

```bash
git add packages/cli/src/env-store.ts packages/cli/src/commands/__tests__/env-store.test.ts packages/cli/src/config.ts
git commit -m "feat(cli): add org-level env store"
```

---

## Task 2: Preserve Legacy Per-Skill Env Compatibility

**Files:**
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/env-store.ts`
- Test: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/__tests__/env-store.test.ts`

**Step 1: Write failing tests for legacy reads**

Add tests for:
- `readLegacySkillEnvVars("kairia")` reads `*.env` except `variables.env`
- same variable with same value across two skill files is `migratable`
- same variable with different values is `conflict`
- org-level variable takes precedence over legacy in readiness helpers

Expected test concepts:

```ts
it("reports safe legacy migration when values match", () => {
  writeLegacySkillEnvVars("kairia", "reviewer", { OPENAI_API_KEY: "same" });
  writeLegacySkillEnvVars("kairia", "writer", { OPENAI_API_KEY: "same" });

  expect(previewLegacyEnvMigration("kairia").migratable).toEqual([
    { name: "OPENAI_API_KEY", skills: ["reviewer", "writer"] },
  ]);
});

it("reports conflicts without selecting a value", () => {
  writeLegacySkillEnvVars("kairia", "reviewer", { OPENAI_API_KEY: "one" });
  writeLegacySkillEnvVars("kairia", "writer", { OPENAI_API_KEY: "two" });

  expect(previewLegacyEnvMigration("kairia").conflicts).toEqual([
    { name: "OPENAI_API_KEY", skills: ["reviewer", "writer"], valueCount: 2 },
  ]);
});
```

**Step 2: Implement legacy helpers**

Add:

```ts
export interface LegacyEnvVariableSummary {
  name: string;
  configured: boolean;
  skills: string[];
  valueCount: number;
  status: "migratable" | "conflict" | "alreadyConfigured";
}

export function readLegacySkillEnvVars(org: string): Array<{ skill: string; vars: Record<string, string> }>;
export function previewLegacyEnvMigration(org: string): EnvMigrationSummary;
export function migrateLegacyEnvVars(org: string): EnvMigrationSummary;
export function writeLegacySkillEnvVars(org: string, skill: string, vars: Record<string, string>): void;
```

Migration rules:
- migrate only same-value variables
- skip variables already configured org-level and mark as `alreadyConfigured`
- never delete legacy files in this task
- never expose values in returned summaries

**Step 3: Run tests**

Run:

```bash
pnpm test -- --run packages/cli/src/commands/__tests__/env-store.test.ts
```

Expected: PASS.

**Step 4: Commit**

```bash
git add packages/cli/src/env-store.ts packages/cli/src/commands/__tests__/env-store.test.ts
git commit -m "feat(cli): read and migrate legacy env files"
```

---

## Task 3: Update `skillreg env` Commands

**Files:**
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/env.ts`
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/index.ts`
- Test: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/__tests__/env-command.test.ts`

**Step 1: Define CLI contract**

New primary usage:

```bash
skillreg env set KEY=value [KEY2=value2 ...] --org kairia
skillreg env list --org kairia
skillreg env delete KEY [KEY2 ...] --org kairia
skillreg env migrate --org kairia
skillreg env legacy --org kairia
```

Compatibility:

```bash
skillreg env set <skill> KEY=value
skillreg env list <skill>
skillreg env delete <skill> KEY
```

Compatibility behavior:
- Supported for now.
- Prints a deprecation note in non-JSON output.
- Writes to legacy only for explicit old syntax.
- Does not silently move values into org-level unless `env migrate` is run.

**Step 2: Write failing command tests**

Tests should cover:
- `env set OPENAI_API_KEY=value` writes org-level
- `env list --json` returns metadata only, no value
- `env delete OPENAI_API_KEY` removes org-level
- old syntax still writes/reads legacy
- `env migrate` migrates safe variables and reports conflicts

Expected JSON shape:

```json
{
  "variables": [
    {
      "name": "OPENAI_API_KEY",
      "configured": true,
      "storage": "fallback_file"
    }
  ],
  "legacy": [],
  "conflicts": []
}
```

**Step 3: Update Commander registrations**

Change `packages/cli/src/index.ts`:

```ts
envCmd
  .command("set <args...>")
  .description("Set org-level env vars (KEY=value)")
```

Inside the action:
- If first arg contains `=`, call org-level mode.
- If first arg does not contain `=` and remaining args contain `=`, route to legacy compatibility mode.
- If ambiguous, print usage.

Add commands:

```ts
envCmd.command("legacy").description("Show legacy per-skill env files");
envCmd.command("migrate").description("Migrate safe legacy env vars to org-level storage");
```

**Step 4: Update implementation**

In `packages/cli/src/commands/env.ts`:
- rename old functions internally to `legacyEnvSetCommand`, `legacyEnvListCommand`, `legacyEnvDeleteCommand`
- add primary `envSetCommand(args, options)`
- add primary `envListCommand(options)`
- add primary `envDeleteCommand(keys, options)`
- add `envLegacyCommand(options)`
- add `envMigrateCommand(options)`

Output rules:
- never print raw values
- `--json` must still not include secret values
- text output should say “stored once for @org”

**Step 5: Run focused tests**

Run:

```bash
pnpm test -- --run packages/cli/src/commands/__tests__/env-command.test.ts
pnpm --filter @skillreg/cli typecheck
```

Expected: PASS.

**Step 6: Commit**

```bash
git add packages/cli/src/commands/env.ts packages/cli/src/index.ts packages/cli/src/commands/__tests__/env-command.test.ts
git commit -m "feat(cli): make env commands org-level by default"
```

---

## Task 4: Update CLI Pull and Local Readiness

**Files:**
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/pull.ts`
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/local.ts`
- Test: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/__tests__/pull-env.test.ts`
- Test: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/__tests__/local-env.test.ts`

**Step 1: Write failing tests**

Cases:
- `pull` sees `OPENAI_API_KEY` already configured org-level and skips prompting it.
- `pull` prompts only missing required variables.
- `pull` saves newly entered variables org-level.
- `local` readiness treats org-level variables as configured for every skill.
- `local` warning text suggests `skillreg env set OPENAI_API_KEY=<value>`, not per-skill syntax.

**Step 2: Refactor env wizard**

In `pull.ts`:
- replace `readEnvFile(org, skillName)` with org-level `getOrgEnvVar(org, key)`
- when prompting, skip configured keys
- write results via `setOrgEnvVar(org, key, value)`
- keep `--no-env` behavior unchanged
- keep legacy fallback read as configured if no org-level value and no conflict

**Step 3: Refactor local warnings**

In `local.ts`:
- readiness should call an org-level + legacy compatibility helper
- warning command should be:

```txt
run: skillreg env set OPENAI_API_KEY=<value>
```

Use `--org <slug>` in the hint only when no default org can be resolved.

**Step 4: Run focused tests and typecheck**

Run:

```bash
pnpm test -- --run packages/cli/src/commands/__tests__/pull-env.test.ts
pnpm test -- --run packages/cli/src/commands/__tests__/local-env.test.ts
pnpm --filter @skillreg/cli typecheck
```

Expected: PASS.

**Step 5: Commit**

```bash
git add packages/cli/src/commands/pull.ts packages/cli/src/commands/local.ts packages/cli/src/commands/__tests__/pull-env.test.ts packages/cli/src/commands/__tests__/local-env.test.ts
git commit -m "feat(cli): use org-level env readiness in pull and local"
```

---

## Task 5: Add Explicit Legacy Cleanup Strategy

**Files:**
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/env-store.ts`
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/commands/env.ts`
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/packages/cli/src/index.ts`
- Optional later desktop follow-up: `/Users/axel/Documents/skillreg/skillreg-local/src/pages/EnvVars.tsx`

**Step 1: Write failing tests for cleanup**

Cases:
- cleanup removes only migrated legacy keys that match org-level configured keys
- cleanup leaves conflicting keys untouched
- cleanup leaves unrelated legacy keys untouched
- cleanup requires `--confirm` or equivalent non-interactive confirmation

Command:

```bash
skillreg env cleanup-legacy --org kairia --confirm
```

**Step 2: Implement cleanup helper**

Add:

```ts
export interface LegacyCleanupSummary {
  cleaned: Array<{ name: string; skills: string[] }>;
  skippedConflicts: Array<{ name: string; skills: string[]; valueCount: number }>;
  remainingLegacyFiles: string[];
}

export function cleanupMigratedLegacyEnvVars(org: string): LegacyCleanupSummary;
```

Rules:
- do not delete a file if it still contains unrelated keys
- if a file becomes empty except comments, remove it
- do not print values

**Step 3: Add CLI command**

Add:

```bash
skillreg env cleanup-legacy --org kairia --confirm
```

Without `--confirm`, print summary and exit without mutation.

**Step 4: Run tests**

Run:

```bash
pnpm test -- --run packages/cli/src/commands/__tests__/env-store.test.ts
pnpm test -- --run packages/cli/src/commands/__tests__/env-command.test.ts
pnpm --filter @skillreg/cli typecheck
```

Expected: PASS.

**Step 5: Commit**

```bash
git add packages/cli/src/env-store.ts packages/cli/src/commands/env.ts packages/cli/src/index.ts packages/cli/src/commands/__tests__
git commit -m "feat(cli): add explicit legacy env cleanup"
```

---

## Task 6: Update App and Website CLI Docs

**Files:**
- Modify: `/Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli/apps/web/src/app/(docs)/docs/cli-reference/page.tsx`
- Modify: `/Users/axel/Documents/skillreg/skillreg-website/src/content/docs/cli-reference.tsx`
- Modify: `/Users/axel/Documents/skillreg/skillreg-website/src/content/docs/environment-variables.tsx`
- Possibly modify: `/Users/axel/Documents/skillreg/skillreg-app/TUTORIAL-CLI.md`
- Possibly modify: `/Users/axel/Documents/skillreg/skillreg-app/API-REFERENCE.md` only if API docs mention env storage

**Step 1: Update CLI reference syntax**

Replace:

```txt
skillreg env set <skill> <pairs...>
skillreg env list [skill]
skillreg env delete <skill> <keys...>
```

With:

```txt
skillreg env set <pairs...>
skillreg env list
skillreg env delete <keys...>
skillreg env legacy
skillreg env migrate
skillreg env cleanup-legacy --confirm
```

Mention legacy skill argument as compatibility, not primary syntax.

**Step 2: Update environment variables guide**

Required language:
- “Environment variables are stored locally per organization.”
- “One variable value can satisfy every installed skill that declares the same variable.”
- “Desktop uses OS secure store when available; CLI currently uses the local org-level fallback file until secure-store CLI support is implemented.”
- “Legacy per-skill files remain readable.”
- “Values are never uploaded to the registry.”

Storage path:

```txt
~/.skillreg/env/{org}/variables.env
~/.skillreg/env/{org}/index.json
~/.skillreg/env/{org}/{skill}.env  # legacy
```

**Step 3: Remove examples with fake secrets**

Avoid examples like `API_KEY=sk-123`. Use placeholders:

```bash
skillreg env set OPENAI_API_KEY=<value>
```

**Step 4: Run docs checks**

For app worktree:

```bash
pnpm --filter web typecheck
pnpm format:check
```

For website:

```bash
npm run lint
npm run build
```

Expected: PASS, except unrelated local files should be called out and not modified.

**Step 5: Commit app docs**

```bash
cd /Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli
git add apps/web/src/app/'(docs)'/docs/cli-reference/page.tsx TUTORIAL-CLI.md API-REFERENCE.md
git commit -m "docs: update CLI env documentation"
```

**Step 6: Commit website docs**

```bash
cd /Users/axel/Documents/skillreg/skillreg-website
git add src/content/docs/cli-reference.tsx src/content/docs/environment-variables.tsx
git commit -m "docs: document org-level env variables"
```

---

## Task 7: Verify Download Surface for v0.3.17

**Files:**
- Inspect: `/Users/axel/Documents/skillreg/skillreg-website/src/lib/github.ts`
- Inspect: `/Users/axel/Documents/skillreg/skillreg-website/src/lib/downloads.ts`
- Inspect: `/Users/axel/Documents/skillreg/skillreg-website/src/app/download/page.tsx`
- Test: existing website build/lint

**Step 1: Verify latest release lookup**

Run:

```bash
cd /Users/axel/Documents/skillreg/skillreg-website
rg -n "Tontoon7/skillreg-local|latest|0\\.3\\.16|0\\.3\\.17" src/lib src/app/download src/components
```

Expected:
- no stale hardcoded `0.3.16` fallback unless deliberately acceptable
- latest GitHub release resolves to `0.3.17`

**Step 2: Build website**

Run:

```bash
npm run lint
npm run build
```

Expected: PASS.

**Step 3: Browser check**

Run website locally:

```bash
npm run dev -- --port 3001
```

Open:

```txt
http://localhost:3001/download
```

Verify:
- page shows `v0.3.17`
- macOS download links point to `SkillReg_0.3.17_aarch64.dmg` / x64 where applicable
- no console errors

**Step 4: Commit only if code/docs changed**

```bash
git add src/lib/github.ts src/lib/downloads.ts src/app/download/page.tsx src/components
git commit -m "chore: align download page with latest desktop release"
```

---

## Task 8: Cross-Repo Verification

**Files:**
- No edits unless failures identify a real issue.

**Step 1: CLI tests**

Run:

```bash
cd /Users/axel/Documents/skillreg/worktrees/skillreg-app-env-cli
pnpm test -- --run packages/cli/src/commands/__tests__
pnpm --filter @skillreg/cli typecheck
pnpm format:check
```

Expected: PASS.

**Step 2: App docs build/typecheck**

Run:

```bash
pnpm --filter web typecheck
pnpm build
```

Expected: PASS or report unrelated pre-existing failures.

**Step 3: Website checks**

Run:

```bash
cd /Users/axel/Documents/skillreg/skillreg-website
npm run lint
npm run build
```

Expected: PASS.

**Step 4: Desktop compatibility smoke test**

Do not make a new desktop release unless desktop code changes.

Smoke test:
- open SkillReg Local v0.3.17
- configure `OPENAI_API_KEY` once in Environment
- run CLI `skillreg env list --org <org>`
- verify CLI sees org-level metadata, not secret value
- install or inspect two skills declaring `OPENAI_API_KEY`
- verify both surfaces treat the variable as configured

**Step 5: Final status**

Report:
- commits created per repo
- checks run
- any skipped checks and why
- any remaining Phase 3+ work, especially CLI secure-store parity if not implemented

---

## Follow-Up Decisions

These should not be bundled into the CLI parity implementation unless Axel explicitly asks:

- Add OS secure-store support directly to the Node CLI.
- Add `skillreg env export`.
- Add cloud org-shared secrets.
- Delete legacy files automatically.
- Change registry APIs to store secret values.

Recommended next follow-up after this plan:

1. Implement CLI org-level fallback + legacy compatibility.
2. Update docs and download surface.
3. Later decide whether CLI should use OS secure store too, or delegate secret editing primarily to desktop.
