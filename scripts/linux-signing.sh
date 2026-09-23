#!/usr/bin/env bash
set -euo pipefail
set +x
umask 077

fail() { echo "$*" >&2; exit 1; }
mode=${1:-}
[[ "$mode" == sign || "$mode" == verify ]] || fail 'Usage: linux-signing.sh sign|verify -- file.deb file.rpm file.AppImage file.AppImage.tar.gz'
shift
[[ "${1:-}" == -- ]] || fail 'An explicit file list after -- is required.'
shift
[[ $# == 4 ]] || fail 'Expected one file per Linux format (.deb, .rpm, .AppImage, .AppImage.tar.gz).'
for variable in LINUX_SIGNING_PUBLIC_KEY LINUX_SIGNING_KEY_FINGERPRINT; do
  [[ -n "${!variable:-}" ]] || fail "Missing $variable"
done
if [[ "$mode" == sign ]]; then
  for variable in LINUX_SIGNING_PRIVATE_KEY LINUX_SIGNING_PASSPHRASE; do
    [[ -n "${!variable:-}" ]] || fail "Missing $variable"
  done
fi
for tool in gpg gpgconf; do
  command -v "$tool" >/dev/null || fail "Required signing tool not found: $tool"
done

fingerprint=$(printf '%s' "$LINUX_SIGNING_KEY_FINGERPRINT" | tr '[:lower:]' '[:upper:]')
[[ "$fingerprint" =~ ^([A-F0-9]{40}|[A-F0-9]{64})$ ]] || fail 'LINUX_SIGNING_KEY_FINGERPRINT must be a full primary fingerprint.'
[[ "$LINUX_SIGNING_PUBLIC_KEY" == *'-----BEGIN PGP PUBLIC KEY BLOCK-----'* && "$LINUX_SIGNING_PUBLIC_KEY" != *'PRIVATE KEY BLOCK'* ]] || fail 'LINUX_SIGNING_PUBLIC_KEY must contain only public key material.'
files=()
formats=' '
output_dir=''
for file in "$@"; do
  [[ -f "$file" && -s "$file" && ! -L "$file" ]] || fail "Missing, empty or symbolic-link Linux asset: $file"
  [[ "$file" != *$'\n'* && "$file" != *$'\r'* ]] || fail 'Invalid Linux asset path.'
  directory=$(cd "$(dirname "$file")" && pwd -P)
  [[ -z "$output_dir" || "$output_dir" == "$directory" ]] || fail 'Linux assets must share one directory.'
  output_dir=$directory
  file="$directory/$(basename "$file")"
  case "$file" in
    *.AppImage.tar.gz) format=archive ;;
    *.AppImage) format=appimage ;;
    *.deb) format=deb ;;
    *.rpm) format=rpm ;;
    *) fail "Unexpected Linux asset: $file" ;;
  esac
  [[ "$formats" != *" $format "* ]] || fail 'Expected one file per Linux format.'
  formats+="$format "
  files+=("$file")
  if [[ "$mode" == sign ]]; then
    [[ ! -e "$file.asc" && ! -L "$file.asc" ]] || fail "Refusing to overwrite detached signature: $file.asc"
  else
    [[ -f "$file.asc" && -s "$file.asc" && ! -L "$file.asc" ]] || fail "Missing or empty detached signature: $file.asc"
  fi
done
public_asset="$output_dir/skillreg-linux-signing-key.asc"
if [[ "$mode" == sign ]]; then
  [[ ! -e "$public_asset" && ! -L "$public_asset" ]] || fail 'Refusing to overwrite the distributed public key.'
else
  [[ -f "$public_asset" && -s "$public_asset" && ! -L "$public_asset" ]] || fail 'Missing or empty distributed public key.'
fi

work=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/skillreg-gpg.XXXXXX")
created=()
cleanup() {
  local status=$?
  trap - EXIT
  for key_home in "$work"/public "$work"/private; do
    if [[ -d "$key_home" ]] && ! gpgconf --homedir "$key_home" --kill all; then
      echo 'Failed to stop the temporary GPG agent.' >&2
      status=1
    fi
  done
  if [[ "$status" != 0 && ${#created[@]} -gt 0 ]]; then
    rm -f "${created[@]}"
  fi
  rm -rf "$work"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$work/public" "$work/private"

isolated_gpg() {
  local key_home=$1
  shift
  gpg --no-options --homedir "$key_home" --batch --yes --no-tty --no-auto-key-retrieve "$@"
}
validate_primary() {
  local key_home=$1
  local key_kind=$2
  isolated_gpg "$key_home" --with-colons --fixed-list-mode --fingerprint \
    "--list-$key_kind-keys" > "$work/key-list"
  if ! awk -F: -v expected="$fingerprint" -v now="$(date +%s)" '
    $1 == "pub" || $1 == "sec" {
      count++; primary=1
      if ($2 ~ /[redi]/ || $12 ~ /D/ || ($7 != "" && $7 != "0" && $7 <= now)) invalid=1
      next
    }
    primary && $1 == "fpr" { fingerprints++; if ($10 != expected) mismatch=1; primary=0 }
    END { if (invalid) exit 2; if (count != 1 || fingerprints != 1 || mismatch) exit 3 }
  ' "$work/key-list"; then
    # Distinguish an unusable key from an identity/configuration mistake.
    if awk -F: -v now="$(date +%s)" '
      ($1 == "pub" || $1 == "sec") && ($2 ~ /[redi]/ || $12 ~ /D/ || ($7 != "" && $7 != "0" && $7 <= now)) { bad=1 }
      END { exit !bad }
    ' "$work/key-list"; then
      fail 'Revoked, expired or invalid OpenPGP key.'
    fi
    fail 'Unexpected OpenPGP primary fingerprint or multiple primary keys.'
  fi
}

printf '%s\n' "$LINUX_SIGNING_PUBLIC_KEY" | isolated_gpg "$work/public" --import >/dev/null 2>"$work/diagnostics" || fail 'Cannot import LINUX_SIGNING_PUBLIC_KEY.'
validate_primary "$work/public" public
isolated_gpg "$work/public" --export "$fingerprint" > "$work/expected-public.gpg"
isolated_gpg "$work/public" --armor --export "$fingerprint" > "$work/skillreg-linux-signing-key.asc"
[[ -s "$work/expected-public.gpg" ]] || fail 'Public key export is empty.'

if [[ "$mode" == sign ]]; then
  passphrase=$LINUX_SIGNING_PASSPHRASE
  private_key=$LINUX_SIGNING_PRIVATE_KEY
  unset LINUX_SIGNING_PRIVATE_KEY LINUX_SIGNING_PASSPHRASE
  [[ "$passphrase" != *$'\n'* && "$passphrase" != *$'\r'* ]] || fail 'LINUX_SIGNING_PASSPHRASE must be a single line.'
  printf '%s\n' "$private_key" | isolated_gpg "$work/private" --import >/dev/null 2>"$work/diagnostics" || fail 'Cannot import LINUX_SIGNING_PRIVATE_KEY.'
  unset private_key
  validate_primary "$work/private" secret
else
  isolated_gpg "$work/public" --output "$work/distributed-public.gpg" --dearmor "$public_asset" 2>"$work/diagnostics" || fail 'Distributed public key differs from trusted configuration.'
  cmp -s "$work/expected-public.gpg" "$work/distributed-public.gpg" || fail 'Distributed public key differs from trusted configuration.'
fi

index=0
for file in "${files[@]}"; do
  signature="$file.asc"
  if [[ "$mode" == sign ]]; then
    signature="$work/$index.asc"
    # The primary fingerprint explicitly selects its valid signing subkey, if any.
    isolated_gpg "$work/private" --pinentry-mode loopback --passphrase-fd 3 \
      --local-user "$fingerprint" --digest-algo SHA256 --armor --detach-sign \
      --output "$signature" "$file" 3<<<"$passphrase" 2>"$work/diagnostics" || fail "OpenPGP signing failed: $(basename "$file")"
  fi
  isolated_gpg "$work/public" --status-fd 1 --verify "$signature" "$file" \
    > "$work/status" 2>"$work/diagnostics" || fail "Invalid OpenPGP signature: $(basename "$file")"
  # KEYEXPIRED can describe an unused subkey; EXPKEYSIG identifies an expired signer.
  if ! awk -v expected="$fingerprint" -v now="$(date +%s)" '
    $1 != "[GNUPG:]" { invalid=1 }
    $2 ~ /^(BADSIG|ERRSIG|EXPSIG|EXPKEYSIG|REVKEYSIG|NO_PUBKEY|SIGEXPIRED|KEYREVOKED|FAILURE|ERROR)$/ { invalid=1 }
    $2 == "GOODSIG" { good++ }
    $2 == "VALIDSIG" {
      valid++
      primary=($12 == "" ? $3 : $12)
      if (primary != expected || ($6 != 0 && $6 <= now) || $10 !~ /^(8|9|10)$/) invalid=1
    }
    END { exit (invalid || good != 1 || valid != 1) }
  ' "$work/status"; then
    fail "Untrusted, expired or revoked OpenPGP signature: $(basename "$file")"
  fi
  index=$((index + 1))
done

if [[ "$mode" == sign ]]; then
  unset passphrase
  index=0
  for file in "${files[@]}"; do
    created+=("$file.asc")
    cp "$work/$index.asc" "$file.asc"
    index=$((index + 1))
  done
  created+=("$public_asset")
  cp "$work/skillreg-linux-signing-key.asc" "$public_asset"
fi
echo "Verified Linux OpenPGP signatures for all four formats with primary key $fingerprint."
