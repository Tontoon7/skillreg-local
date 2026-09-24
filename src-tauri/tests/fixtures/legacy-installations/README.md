# Legacy installation fixtures

The migration integration tests create their mutable `installed.json` manifests and skill
directories in isolated temporary homes. This directory documents the fixture contract without
shipping machine-specific absolute paths:

- v1 records use `scope: "user"` for migration candidates;
- project-scoped records are never mutated;
- `contentHash` is the SHA-256 of the legacy `SKILL.md` content;
- `sourceOrg` falls back to `org` when absent;
- only regular directories at an adapter-owned user skill path can be replaced;
- untracked directories, symlinks, modified copies, and divergent duplicates stay untouched.
