# macOS Notarization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ensure GitHub release builds for macOS are notarized and stapled so Gatekeeper no longer rejects downloaded SkillReg releases.

**Architecture:** Keep the existing Developer ID signing flow in GitHub Actions, reuse the existing Apple ID notarization credentials already stored in GitHub secrets for macOS jobs, let Tauri notarize the `.app`, then explicitly notarize and staple generated `.dmg` artifacts before publishing. Add a lightweight regression check script so the workflow cannot silently lose notarization wiring again.

**Tech Stack:** GitHub Actions, Tauri v2 build pipeline, Apple `notarytool`, Apple `stapler`, shell validation script

---

### Task 1: Add a failing workflow regression check

**Files:**
- Create: `scripts/check-release-notarization.sh`
- Test: `.github/workflows/release.yml`

**Step 1: Write the failing test**

Create a shell check that asserts the release workflow contains:
- Apple notarization credential wiring with `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID`
- `xcrun notarytool submit`
- `xcrun stapler staple`
- `xcrun stapler validate`

**Step 2: Run test to verify it fails**

Run: `bash scripts/check-release-notarization.sh`

Expected: FAIL because the current workflow signs macOS artifacts but does not configure notarization.

### Task 2: Wire notarization into the macOS release workflow

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Prepare Apple notarization credentials**

Add a macOS-only step that validates the required secrets exist:
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`

**Step 2: Keep Tauri app notarization enabled**

Pass the existing Apple notarization secrets into the `Build Tauri` step so generated `.app` bundles are notarized during build.

**Step 3: Notarize and staple DMG artifacts explicitly**

After `tauri build`, find each generated `.dmg`, submit it with `xcrun notarytool submit --wait`, staple it, and validate the stapled ticket.

**Step 4: Validate the app bundle**

Run `spctl -a -vv` and `xcrun stapler validate` against the built `.app` bundle to fail the workflow if notarization regresses.

### Task 3: Update project docs

**Files:**
- Modify: `ROADMAP.md`
- Modify: `CLAUDE.md`

**Step 1: Correct current status**

Replace the outdated “unsigned” / “code signing remains” notes with wording that reflects:
- release builds are Developer ID signed
- notarization now depends on GitHub release secrets and workflow

**Step 2: Document required secrets briefly**

Note the macOS release prerequisites:
- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`

### Task 4: Verify end to end

**Files:**
- Test: `scripts/check-release-notarization.sh`
- Test: `.github/workflows/release.yml`

**Step 1: Re-run the regression check**

Run: `bash scripts/check-release-notarization.sh`

Expected: PASS

**Step 2: Validate workflow syntax**

Run: `python - <<'PY' ...` or another YAML parser available locally to ensure the workflow remains syntactically valid.

Expected: workflow parses successfully

**Step 3: Summarize remaining operational prerequisite**

Record that the GitHub repository still needs valid Apple secrets configured for the next tagged release to notarize successfully.
