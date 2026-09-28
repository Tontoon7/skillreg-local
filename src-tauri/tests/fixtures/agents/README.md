# Agent compatibility fixtures

These fixtures validate agent discovery without touching the real user skill directories.

- `runtime-probe/SKILL.md` is intentionally minimal. A manual compatibility check creates a
  temporary agent skill root, links this directory into it, and invokes the skill explicitly.
- Temporary profiles must live under an explicit temporary directory and must be deleted after
  the check.
- A successful runtime check must record the candidate revision, agent version, operating system,
  architecture, filesystem, account elevation, date, link kind, observed output, and whether a
  restart was required in `docs/managed-skills-release-validation.md`.
- Use a profile path with spaces and Unicode. On Windows, run junction checks on NTFS with a
  standard account; an administrator CI runner is not evidence for that release gate.
- A filesystem-only unit test is not sufficient to mark an agent or platform as validated.
- Fixtures never contain credentials, environment values, or production skill content.
