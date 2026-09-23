# Desktop Auto Skill Updates Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Keep locally installed SkillReg skills up to date automatically from SkillReg Local while the app runs in the tray/menu bar.

**Architecture:** SkillReg Local remains a resident Tauri app. A lightweight Rust worker starts after app setup, reads a local installed-skills manifest, periodically checks the registry for newer approved versions, updates safe installations through the existing download/install path, and sends an OS notification with the result. The feature is enabled by default, can be disabled globally in Settings, and stops when the user explicitly quits SkillReg.

**Tech Stack:** Tauri v2, Rust/Tokio, React 19, TypeScript, Zustand, local JSON config under `~/.skillreg`, SkillReg REST API through Rust `reqwest`, Tauri notification plugin, Tauri autostart plugin.

---

## Product Decision

Use **Option A**:

- SkillReg Local starts at login.
- Closing the window hides it and leaves the tray/menu bar process running.
- The auto-update worker runs inside the Tauri process.
- Choosing `Quit` from the tray stops SkillReg and stops auto-updates.

Do not build a separate macOS LaunchAgent, Windows Service, or Linux systemd user service in this iteration.

## Scope

In scope:

- Local installed-skills manifest.
- Global auto-update setting enabled by default.
- Launch-at-login enabled by default.
- Background worker in the Tauri process.
- Safe automatic updates for SkillReg-managed installations.
- OS notification after automatic updates.
- Settings UI to disable/enable the feature and run a manual check.
- Installed page status that distinguishes managed, modified, update available, and auto-updated skills.

Out of scope:

- True OS daemon/service that runs after `Quit`.
- Per-org scheduling policies.
- Rollback UI.
- Update diff viewer.
- Forced enterprise policies.
- Auto-update of local-only skills not installed through SkillReg.

## Safety Rules

Auto-update may update a skill only when all conditions are true:

- Global `autoUpdateEnabled` is true.
- The installation exists in `~/.skillreg/installed.json`.
- The installation's per-skill `autoUpdateEnabled` is not false.
- The current local `SKILL.md` content hash matches the manifest hash from the last SkillReg install.
- The server has a newer approved version.
- The downloaded tarball SHA-256 matches the server checksum when present.

If any condition fails, the worker skips the skill and records a status. It must never overwrite a modified local skill automatically.

---

### Task 0: Baseline And Branch Hygiene

**Files:**
- Inspect: `src-tauri/src/commands/skills.rs`
- Inspect: `src-tauri/src/commands/local.rs`
- Inspect: `src-tauri/src/commands/config.rs`
- Inspect: `src/pages/Settings.tsx`
- Inspect: `src/pages/Installed.tsx`

**Step 1: Confirm branch**

Run:
```bash
git -C /Users/axel/Documents/skillreg/skillreg-local status --short --branch
```

Expected: branch is `feat/desktop-auto-skill-updates`.

**Step 2: Run baseline verification**

Run:
```bash
cd /Users/axel/Documents/skillreg/skillreg-local
pnpm format:check
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: all pass before feature edits. If an existing unrelated issue appears, record it before coding.

---

### Task 1: Extend Local Config Defaults

**Files:**
- Modify: `src-tauri/src/commands/config.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/store.ts`

**Step 1: Add config fields**

Add these optional fields to `SkillregConfig` in Rust and TypeScript:

```rust
pub auto_update_enabled: Option<bool>,
pub auto_update_interval_minutes: Option<u64>,
pub launch_at_login: Option<bool>,
```

```ts
autoUpdateEnabled?: boolean;
autoUpdateIntervalMinutes?: number;
launchAtLogin?: boolean;
```

**Step 2: Add default helpers**

In Rust, add constants:

```rust
pub const DEFAULT_AUTO_UPDATE_ENABLED: bool = true;
pub const DEFAULT_AUTO_UPDATE_INTERVAL_MINUTES: u64 = 60;
pub const DEFAULT_LAUNCH_AT_LOGIN: bool = true;
```

Add methods:

```rust
impl SkillregConfig {
    pub fn auto_update_enabled_value(&self) -> bool {
        self.auto_update_enabled.unwrap_or(DEFAULT_AUTO_UPDATE_ENABLED)
    }

    pub fn auto_update_interval_minutes_value(&self) -> u64 {
        self.auto_update_interval_minutes
            .unwrap_or(DEFAULT_AUTO_UPDATE_INTERVAL_MINUTES)
            .clamp(15, 24 * 60)
    }

    pub fn launch_at_login_value(&self) -> bool {
        self.launch_at_login.unwrap_or(DEFAULT_LAUNCH_AT_LOGIN)
    }
}
```

**Step 3: Keep frontend defaults stable**

In `src/lib/store.ts`, when config loads, do not eagerly write defaults to disk. Let missing values mean default true. Settings can write explicit values when the user saves.

**Step 4: Verify**

Run:
```bash
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

---

### Task 2: Add Installed Skills Manifest

**Files:**
- Create: `src-tauri/src/commands/installed_manifest.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`

**Step 1: Create manifest schema**

Create `src-tauri/src/commands/installed_manifest.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstalledManifest {
    pub version: u32,
    pub installations: Vec<TrackedInstallation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedInstallation {
    pub org: String,
    pub name: String,
    pub version: String,
    pub agent: String,
    pub scope: String,
    pub project_dir: Option<String>,
    pub install_path: String,
    pub content_hash: String,
    pub sha256: Option<String>,
    pub auto_update_enabled: Option<bool>,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<String>,
}
```

Add helpers:

- `manifest_path() -> PathBuf` returns `~/.skillreg/installed.json`.
- `read_installed_manifest() -> Result<InstalledManifest, String>`.
- `write_installed_manifest(manifest: InstalledManifest) -> Result<(), String>`.
- `upsert_tracked_installation(entry: TrackedInstallation) -> Result<(), String>`.
- `remove_tracked_installation(name, agent, scope, project_dir) -> Result<(), String>`.

Use a stable key:

```rust
(org, name, agent, scope, project_dir)
```

**Step 2: Add Tauri commands**

Expose:

```rust
#[tauri::command]
pub fn list_tracked_installations() -> Result<Vec<TrackedInstallation>, String>

#[tauri::command]
pub fn set_skill_auto_update(
    org: String,
    name: String,
    agent: String,
    scope: String,
    project_dir: Option<String>,
    enabled: bool,
) -> Result<(), String>
```

Register them in `src-tauri/src/lib.rs`.

**Step 3: Add TS types/wrappers**

In `src/lib/types.ts`, add `TrackedInstallation`.

In `src/lib/api.ts`, add:

```ts
export const listTrackedInstallations = () =>
  invoke<TrackedInstallation[]>("list_tracked_installations");
```

and a wrapper for `set_skill_auto_update`.

**Step 4: Verify**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
pnpm build
```

Expected: PASS.

---

### Task 3: Track Installs And Uninstalls

**Files:**
- Modify: `src-tauri/src/commands/skills.rs`
- Modify: `src-tauri/src/commands/installed_manifest.rs`

**Step 1: Refactor install helper**

In `skills.rs`, keep the public `pull_skill` command but extract the shared install logic into an internal helper:

```rust
pub async fn install_skill_from_registry(
    org: String,
    name: String,
    version: Option<String>,
    agent: String,
    scope: String,
    project_dir: Option<String>,
) -> Result<InstallResult, String>
```

`pull_skill` should call this helper.

**Step 2: Return enough metadata**

Ensure `InstallResult` has:

```rust
pub sha256: Option<String>,
pub content_hash: String,
```

If changing the public TS type is too disruptive, keep `InstallResult` stable and compute the tracked entry internally after install.

**Step 3: Upsert manifest after successful install**

After a successful install, write a `TrackedInstallation` with:

- org
- name
- installed version
- agent
- scope
- project_dir
- install_path
- current `SKILL.md` content hash
- expected server sha256 if available
- `auto_update_enabled: None`
- `last_updated_at: now`
- `last_error: None`

**Step 4: Remove manifest entry on uninstall**

In `uninstall_skill`, after deleting files successfully, remove the matching manifest entry.

**Step 5: Verify**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
pnpm build
```

Expected: PASS.

---

### Task 4: Auto-Update Worker Core

**Files:**
- Create: `src-tauri/src/commands/auto_update.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/skills.rs`

**Step 1: Add worker result types**

Create:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateRunSummary {
    pub checked: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub updated_skills: Vec<AutoUpdatedSkill>,
    pub skipped_skills: Vec<AutoUpdateSkippedSkill>,
}
```

Include name, agent, scope, old version, new version, and reason fields.

**Step 2: Implement one-shot update run**

Add:

```rust
pub async fn run_auto_update_once() -> Result<AutoUpdateRunSummary, String>
```

Algorithm:

1. Read config.
2. If no token or no org access, return empty summary.
3. Read manifest.
4. For each tracked installation:
   - skip if global disabled.
   - skip if per-skill disabled.
   - skip if local path missing.
   - scan current local `SKILL.md` hash.
   - skip if hash differs from manifest `content_hash`.
   - fetch registry detail or use an efficient org skill list cache.
   - skip if no newer approved version.
   - call `install_skill_from_registry(...)`.
   - update manifest with new version/hash/timestamps.
5. Record errors per skill, do not abort the whole run.

**Step 3: Prefer one registry fetch per org**

For lightness, do not call `GET /skills/:name` for every skill when many skills exist. Fetch registry skills by org pages of 200, build a `HashMap<name, latest_version>`, and only call detailed download flow for skills that need updating.

**Step 4: Add manual Tauri command**

Expose:

```rust
#[tauri::command]
pub async fn run_auto_update_now() -> Result<AutoUpdateRunSummary, String>
```

**Step 5: Verify**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

---

### Task 5: Start Resident Worker

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/auto_update.rs`

**Step 1: Add worker startup**

In `setup(|app| { ... })`, after tray setup, start the worker:

```rust
commands::auto_update::spawn_auto_update_worker(app.handle().clone());
```

**Step 2: Worker loop**

Use `tokio::spawn`:

- initial delay: 2 minutes after app launch.
- interval: config value, default 60 minutes.
- minimum interval: 15 minutes.
- jitter: small deterministic or random delay up to 5 minutes to avoid every install checking at the same time.
- read config on each loop so Settings changes apply without restart.
- if global auto-update disabled, sleep and continue.

**Step 3: Avoid overlapping runs**

Use a shared `Arc<tokio::sync::Mutex<()>>` or an atomic flag to ensure only one update run happens at a time.

**Step 4: Verify**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

---

### Task 6: Notifications And Events

**Files:**
- Modify: `src-tauri/src/commands/auto_update.rs`
- Modify: `src-tauri/src/lib.rs`
- Optionally modify: `src/lib/notifications.ts`

**Step 1: Send OS notifications from worker**

After a run:

- if one skill updated: `Skill updated`
- if multiple skills updated: `N skills updated`
- if only failures happened: do not spam by default; record status for UI.

Use the existing Tauri notification plugin from Rust. If Rust API details need confirmation during implementation, check official Tauri notification plugin docs before coding.

**Step 2: Emit frontend event**

Emit a Tauri event after each run:

```rust
app.emit("auto-update:completed", summary)
```

The UI can listen later, but the worker must not depend on a visible window.

**Step 3: Verify**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

---

### Task 7: Launch At Login

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json` only if the plugin requires frontend permission.
- Modify: `package.json` only if a JS API is used.

**Step 1: Add official Tauri autostart plugin**

Use the official Tauri v2 autostart plugin. This is the only new dependency expected for this feature.

If using Rust-only control, add Cargo dependency and initialize the plugin in `lib.rs`. If using JS control, also add the npm package and permissions.

**Step 2: Apply default launch-at-login**

On app setup:

- read config.
- if `launchAtLogin` is missing or true, enable autostart.
- if false, disable autostart.

**Step 3: Add command for Settings**

Expose:

```rust
#[tauri::command]
pub fn set_launch_at_login(enabled: bool) -> Result<(), String>
```

The command updates config and applies the OS autostart setting.

**Step 4: Verify**

Run:
```bash
pnpm install --frozen-lockfile
cargo check --manifest-path src-tauri/Cargo.toml
pnpm build
```

Expected: PASS. Lockfile changes are expected only if a JS package is added.

---

### Task 8: Window Close Behavior

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Hide instead of quitting on close**

For the main window close event:

- prevent close.
- hide window.
- keep tray and worker alive.

`Quit` in tray should still call `app.exit(0)`.

**Step 2: Verify manually**

Run:
```bash
pnpm tauri dev
```

Expected:

- closing the window hides it.
- tray "Show SkillReg" restores it.
- tray "Quit" exits the process.

---

### Task 9: Settings UI

**Files:**
- Modify: `src/pages/Settings.tsx`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`

**Step 1: Add controls**

In Settings, add an "Automatic skill updates" section:

- toggle: enabled/disabled, default enabled.
- interval select: 15 min, 30 min, 1 hour, 6 hours, 24 hours.
- toggle: launch at login, default enabled.
- button: "Check now".
- small status text: last run / last updated count / last error if exposed.

**Step 2: Wire config save**

Use existing `useConfigStore.update()` for config fields. Use `set_launch_at_login` command for applying OS autostart immediately.

**Step 3: Add manual run**

Add wrapper:

```ts
export const runAutoUpdateNow = () => invoke<AutoUpdateRunSummary>("run_auto_update_now");
```

Use it for "Check now" and show summary.

**Step 4: Verify**

Run:
```bash
pnpm build
pnpm format:check
```

Expected: PASS.

---

### Task 10: Installed Page Integration

**Files:**
- Modify: `src/pages/Installed.tsx`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`

**Step 1: Load manifest**

Load `listTrackedInstallations()` alongside local scan and registry metadata.

**Step 2: Update status model**

Add statuses:

- `managed_synced`
- `managed_update_available`
- `managed_modified_locally`
- `managed_auto_update_disabled`
- `local_only`

Keep existing visual density. Avoid adding a large new card layout.

**Step 3: Add per-skill auto-update toggle**

For managed skills only, add a compact toggle or menu action:

- "Auto-update on"
- "Auto-update off"

Toggling calls `set_skill_auto_update`.

**Step 4: Verify**

Run:
```bash
pnpm build
pnpm format:check
```

Expected: PASS.

---

### Task 11: Tests And Checks

**Files:**
- Add Rust unit tests in the modules where practical:
  - `src-tauri/src/commands/installed_manifest.rs`
  - `src-tauri/src/commands/auto_update.rs`

**Step 1: Manifest tests**

Test:

- upsert replaces an existing key.
- remove deletes only the matching installation.
- missing manifest returns empty default.

Use temp directories where possible. If path helpers need testability, add test-only path override functions behind `#[cfg(test)]`.

**Step 2: Auto-update decision tests**

Extract pure decision helpers from the worker:

```rust
fn should_update_installation(...)
```

Test:

- global disabled skips.
- per-skill disabled skips.
- local hash mismatch skips.
- same version skips.
- newer server version updates.

**Step 3: Run verification**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
pnpm format:check
pnpm build
```

Expected: PASS.

---

### Task 12: Docs And Roadmap

**Files:**
- Modify: `ROADMAP.md`
- Modify: `DEV-PLAN.md`

**Step 1: Update roadmap**

Add a new desktop phase item:

```md
- [x] Auto-update des skills installes via manifest local et worker tray
```

Keep it unchecked until implementation and verification are complete.

**Step 2: Update architecture docs**

In `DEV-PLAN.md`, document:

- `~/.skillreg/installed.json`
- default auto-update behavior
- close vs quit behavior
- safety rule for modified local skills
- OS notification behavior

**Step 3: Final verification**

Run:
```bash
git -C /Users/axel/Documents/skillreg/skillreg-local diff --stat
git -C /Users/axel/Documents/skillreg/skillreg-local diff --check
```

Expected: no whitespace errors and scoped diff.
