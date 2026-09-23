#!/usr/bin/env bash
set -euo pipefail
set +x
umask 077

script_dir=$(cd "$(dirname "$0")" && pwd)
signer="$script_dir/linux-signing.sh"
[[ -f "$signer" ]] || { echo "Missing Linux signing implementation." >&2; exit 1; }
for tool in gpg gpgconf; do
  command -v "$tool" >/dev/null || { echo "Required test tool not found: $tool" >&2; exit 1; }
done

test_root=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/skillreg-gpg-tests.XXXXXX")
cleanup() {
  local status=$?
  trap - EXIT
  for key_home in "$test_root"/keys-*; do
    if [[ -d "$key_home" ]] && ! gpgconf --homedir "$key_home" --kill all; then
      status=1
    fi
  done
  rm -rf "$test_root"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$test_root/keys-main" "$test_root/keys-other" "$test_root/keys-expired" "$test_root/temp"
export TMPDIR="$test_root/temp"
export RUNNER_TEMP="$TMPDIR"
export GNUPGHOME="$test_root/untouched-home"

key_gpg() {
  local key_home=$1
  shift
  gpg --no-options --homedir "$key_home" --batch --yes --no-tty --pinentry-mode loopback "$@"
}
fingerprint() {
  key_gpg "$1" --with-colons --list-keys 2>/dev/null | awk -F: '$1 == "fpr" { print $10; exit }'
}
passphrase='ephemeral-test-passphrase'
key_gpg "$test_root/keys-main" --passphrase-fd 3 --quick-generate-key \
  'SkillReg test <signing@example.invalid>' ed25519 cert 1d 3<<<"$passphrase"
main_fingerprint=$(fingerprint "$test_root/keys-main")
key_gpg "$test_root/keys-main" --passphrase-fd 3 --quick-add-key \
  "$main_fingerprint" ed25519 sign 1d 3<<<"$passphrase"
key_gpg "$test_root/keys-other" --passphrase-fd 3 --quick-generate-key \
  'Other test <other@example.invalid>' ed25519 sign 1d 3<<<"$passphrase"
other_fingerprint=$(fingerprint "$test_root/keys-other")
key_gpg "$test_root/keys-expired" --faked-system-time 20000101T000000 \
  --passphrase-fd 3 --quick-generate-key 'Expired test <expired@example.invalid>' \
  ed25519 sign 1d 3<<<"$passphrase"
expired_fingerprint=$(fingerprint "$test_root/keys-expired")

export LINUX_SIGNING_PUBLIC_KEY
LINUX_SIGNING_PUBLIC_KEY=$(key_gpg "$test_root/keys-main" --armor --export "$main_fingerprint")
export LINUX_SIGNING_PRIVATE_KEY
LINUX_SIGNING_PRIVATE_KEY=$(key_gpg "$test_root/keys-main" --passphrase-fd 3 \
  --armor --export-secret-keys "$main_fingerprint" 3<<<"$passphrase")
export LINUX_SIGNING_KEY_FINGERPRINT="$main_fingerprint"
export LINUX_SIGNING_PASSPHRASE="$passphrase"

make_files() {
  mkdir "$test_root/$1"
  files=()
  for suffix in deb rpm AppImage AppImage.tar.gz; do
    files+=("$test_root/$1/SkillReg test.$suffix")
    printf 'isolated %s fixture\n' "$suffix" > "${files[${#files[@]}-1]}"
  done
}
expect_failure() {
  local reason=$1
  shift
  if "$@" >"$test_root/failure.log" 2>&1; then
    echo "Expected failure: $reason" >&2
    exit 1
  fi
  if ! grep -Fq "$reason" "$test_root/failure.log"; then
    cat "$test_root/failure.log" >&2
    echo "Wrong failure; expected: $reason" >&2
    exit 1
  fi
  if [[ -n "$(find "$TMPDIR" -name 'skillreg-gpg.*' -print)" ]]; then
    echo "Signing left a temporary keyring after failure." >&2
    exit 1
  fi
}

make_files valid
expect_failure 'Missing LINUX_SIGNING_PASSPHRASE' env LINUX_SIGNING_PASSPHRASE= bash "$signer" sign -- "${files[@]}"
expect_failure 'Expected one file per Linux format' bash "$signer" sign -- "${files[0]}"
expect_failure 'Expected one file per Linux format' bash "$signer" sign -- "${files[0]}" "${files[0]}" "${files[2]}" "${files[3]}"
expect_failure 'Unexpected OpenPGP primary fingerprint' env LINUX_SIGNING_KEY_FINGERPRINT="$other_fingerprint" bash "$signer" sign -- "${files[@]}"
other_private=$(key_gpg "$test_root/keys-other" --passphrase-fd 3 \
  --armor --export-secret-keys "$other_fingerprint" 3<<<"$passphrase")
expect_failure 'Unexpected OpenPGP primary fingerprint' env LINUX_SIGNING_PRIVATE_KEY="$other_private" bash "$signer" sign -- "${files[@]}"
expect_failure 'OpenPGP signing failed' env LINUX_SIGNING_PASSPHRASE=incorrect bash "$signer" sign -- "${files[@]}"
[[ ! -e "${files[0]}.asc" ]] || { echo 'Failed signing created an asset.' >&2; exit 1; }

bash "$signer" sign -- "${files[@]}"
env -u LINUX_SIGNING_PRIVATE_KEY -u LINUX_SIGNING_PASSPHRASE bash "$signer" verify -- "${files[@]}"
[[ -s "$test_root/valid/skillreg-linux-signing-key.asc" ]]
# The primary key certifies only: a valid signature must come from its signing subkey.
status=$(key_gpg "$test_root/keys-main" --status-fd 1 --verify "${files[0]}.asc" "${files[0]}" 2>/dev/null)
printf '%s\n' "$status" | awk -v expected="$main_fingerprint" '
  $2 == "VALIDSIG" && $3 != expected && $12 == expected { found=1 }
  END { exit !found }
'
cp "${files[0]}" "$test_root/original"
printf 'tampered\n' >> "${files[0]}"
expect_failure 'Invalid OpenPGP signature' bash "$signer" verify -- "${files[@]}"
cp "$test_root/original" "${files[0]}"
mv "${files[0]}.asc" "$test_root/original.asc"
expect_failure 'Missing or empty detached signature' bash "$signer" verify -- "${files[@]}"
: > "${files[0]}.asc"
expect_failure 'Missing or empty detached signature' bash "$signer" verify -- "${files[@]}"
key_gpg "$test_root/keys-other" --passphrase-fd 3 --armor --detach-sign \
  --output "${files[0]}.asc" "${files[0]}" 3<<<"$passphrase"
expect_failure 'Invalid OpenPGP signature' bash "$signer" verify -- "${files[@]}"
cp "$test_root/original.asc" "${files[0]}.asc"
cp "$test_root/valid/skillreg-linux-signing-key.asc" "$test_root/original-key.asc"
key_gpg "$test_root/keys-other" --armor --export > "$test_root/valid/skillreg-linux-signing-key.asc"
expect_failure 'Distributed public key differs' bash "$signer" verify -- "${files[@]}"
cp "$test_root/original-key.asc" "$test_root/valid/skillreg-linux-signing-key.asc"

make_files expired
for file in "${files[@]}"; do
  key_gpg "$test_root/keys-expired" --faked-system-time 20000101T010000 \
    --passphrase-fd 3 --armor --detach-sign --output "$file.asc" "$file" 3<<<"$passphrase"
done
expired_public=$(key_gpg "$test_root/keys-expired" --armor --export "$expired_fingerprint")
printf '%s\n' "$expired_public" > "$test_root/expired/skillreg-linux-signing-key.asc"
expect_failure 'Revoked, expired or invalid OpenPGP key' env LINUX_SIGNING_PUBLIC_KEY="$expired_public" \
  LINUX_SIGNING_KEY_FINGERPRINT="$expired_fingerprint" bash "$signer" verify -- "${files[@]}"

# An unused expired subkey must not invalidate the current signing subkey.
(
  rotation_home="$test_root/keys-rotation"
  mkdir "$rotation_home"
  key_gpg "$rotation_home" --faked-system-time 20000101T000000 --passphrase-fd 3 \
    --quick-generate-key 'Rotation test <rotation@example.invalid>' ed25519 cert 0 3<<<"$passphrase"
  rotation_fingerprint=$(fingerprint "$rotation_home")
  key_gpg "$rotation_home" --faked-system-time 20000101T000000 --passphrase-fd 3 \
    --quick-add-key "$rotation_fingerprint" ed25519 sign 1d 3<<<"$passphrase"
  old_subkey=$(key_gpg "$rotation_home" --with-colons --fingerprint --fingerprint --list-keys |
    awk -F: '$1 == "sub" { subkey=1 } subkey && $1 == "fpr" { print $10; exit }')
  [[ -n "$old_subkey" ]]
  make_files rotation
  key_gpg "$rotation_home" --faked-system-time 20000101T010000 --passphrase-fd 3 \
    --local-user "$old_subkey!" --digest-algo SHA256 --armor --detach-sign \
    --output "$test_root/old-subkey-signature.asc" "${files[0]}" 3<<<"$passphrase"
  key_gpg "$rotation_home" --passphrase-fd 3 --quick-add-key \
    "$rotation_fingerprint" ed25519 sign 1d 3<<<"$passphrase"
  export LINUX_SIGNING_KEY_FINGERPRINT="$rotation_fingerprint"
  LINUX_SIGNING_PUBLIC_KEY=$(key_gpg "$rotation_home" --armor --export "$rotation_fingerprint")
  LINUX_SIGNING_PRIVATE_KEY=$(key_gpg "$rotation_home" --passphrase-fd 3 \
    --armor --export-secret-keys "$rotation_fingerprint" 3<<<"$passphrase")
  bash "$signer" sign -- "${files[@]}"
  bash "$signer" verify -- "${files[@]}"
  status=$(key_gpg "$rotation_home" --status-fd 1 --verify "${files[0]}.asc" "${files[0]}" 2>/dev/null)
  printf '%s\n' "$status" | awk -v primary="$rotation_fingerprint" -v expired="$old_subkey" '
    $2 == "VALIDSIG" && $3 != expired && $12 == primary { valid=1 }
    END { exit !valid }
  '
  cp "$test_root/old-subkey-signature.asc" "${files[0]}.asc"
  expect_failure 'OpenPGP signature' bash "$signer" verify -- "${files[@]}"
)

make_files revoked
bash "$signer" sign -- "${files[@]}"
sed 's/^://' "$test_root/keys-main/openpgp-revocs.d/$main_fingerprint.rev" > "$test_root/revocation.asc"
key_gpg "$test_root/keys-main" --import "$test_root/revocation.asc"
revoked_public=$(key_gpg "$test_root/keys-main" --armor --export "$main_fingerprint")
expect_failure 'Revoked, expired or invalid OpenPGP key' env LINUX_SIGNING_PUBLIC_KEY="$revoked_public" \
  bash "$signer" verify -- "${files[@]}"

[[ ! -e "$GNUPGHOME" ]] || { echo 'The ambient GPG home was touched.' >&2; exit 1; }
[[ -z "$(find "$TMPDIR" -name 'skillreg-gpg.*' -print)" ]] || { echo 'Signing left a temporary keyring.' >&2; exit 1; }
echo 'Linux signing tests passed (real GnuPG, isolated keys and agents).'
