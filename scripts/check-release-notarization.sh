#!/usr/bin/env bash

set -euo pipefail

workflow_path="${1:-.github/workflows/release.yml}"

if [[ ! -f "$workflow_path" ]]; then
  echo "Workflow not found: $workflow_path" >&2
  exit 1
fi

assert_contains() {
  local pattern="$1"

  if ! rg -q --fixed-strings "$pattern" "$workflow_path"; then
    echo "Missing workflow pattern: $pattern" >&2
    exit 1
  fi
}

assert_contains "APPLE_ID"
assert_contains "APPLE_PASSWORD"
assert_contains 'if [ "${#APPLE_TEAM_ID}" -ge 3 ]'
assert_contains "xcrun notarytool submit"
assert_contains "xcrun stapler staple"
assert_contains "xcrun stapler validate"

echo "Release workflow contains macOS notarization hooks."
