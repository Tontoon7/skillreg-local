<p align="center">
  <img src="src-tauri/icons/icon.svg" alt="SkillReg" width="96" height="96">
</p>

# SkillReg Desktop

Desktop app for [SkillReg](https://skillreg.dev), the private registry of AI agent skills
(`SKILL.md`) for Claude Code, Codex and Cursor.

It is made for employees who want to use the AI skills their company approved, without a command
line, file paths or version numbers.

## What it does

- Sign in with your company account.
- Browse the skills approved by your company.
- Install a skill in one click, for every compatible assistant (Claude Code, Codex, Cursor)
  detected on your computer.
- Keep installed skills up to date automatically.
- Configure the access a skill needs, stored locally on your computer.
- Repair or uninstall a skill without overwriting files SkillReg does not own.

Governance, version approval and administration stay in the web app at
[app.skillreg.dev](https://app.skillreg.dev).

## Download

Get SkillReg Desktop from https://skillreg.dev/download.

## Command-line interface

Authors and automation can use the SkillReg CLI to publish and version skills, install them in a
project and run in CI:

```bash
npm i -g @skillreg/cli
```

## Privacy

SkillReg does not collect your prompts, conversations, responses or file contents.

## Learn more

- https://skillreg.dev — SkillReg product site
- https://app.skillreg.dev — SkillReg web app
