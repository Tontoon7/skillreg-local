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

The release rollback fixture uses a profile path with spaces and Unicode. It snapshots the whole
`.skillreg`, `.claude`, `.codex`, `.cursor` and `.agents` roots through
`scripts/managed-skills-backup-restore.mjs` before invoking the real migration engine. It includes
fake access values, modified user copies, a project copy and an external link target. The same
script restores the snapshot and retains the complete post-migration profile separately.

Filesystem restoration does not prove compatibility with a legacy executable. Record that
executable's version and checksum, then read and operate on the restored fixture with that binary
before checking the downgrade gate. Never use a real home or real credentials.
