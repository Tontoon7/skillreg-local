#!/usr/bin/env python3
"""Select verified build outputs and prepare an explicit, complete release.

Native signing scripts verify signatures before `record`. The inventory binds that
verification to the bytes transferred by Actions; it is not a digital signature.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import shutil
import sys


PLATFORMS = ("darwin-aarch64", "darwin-x86_64", "linux-x86_64", "windows-x86_64")
INVENTORY = "verified-files.json"
PUBLIC_KEY = "skillreg-linux-signing-key.asc"
LAYOUTS = {
    "darwin": {"dmg": ("dmg", ".dmg"), "updater": ("macos", ".app.tar.gz")},
    "linux": {
        "deb": ("deb", ".deb"), "rpm": ("rpm", ".rpm"),
        "appimage": ("appimage", ".AppImage"),
        "updater": ("appimage", ".AppImage.tar.gz"),
    },
    "windows": {
        "nsis": ("nsis", ".exe"), "msi": ("msi", ".msi"),
        "updater": ("nsis", ".nsis.zip"),
    },
}
DELIVERY_SUFFIXES = tuple(suffix for layout in LAYOUTS.values() for _, suffix in layout.values())
VERSION_PATTERN = r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def layout_for(platform):
    require(platform in PLATFORMS, f"Unsupported platform: {platform}")
    return LAYOUTS[platform.split("-", 1)[0]]


def safe_name(name):
    require(isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._+-]*", name),
            f"Unsafe asset name: {name!r}")
    return name


def directory_files(directory, allow_inventory=False):
    require(not directory.is_symlink() and directory.is_dir(), f"Not a regular directory: {directory}")
    files = {}
    folded = set()
    for path in sorted(directory.iterdir()):
        safe_name(path.name)
        require(path.name.casefold() not in folded, f"Ambiguous asset name: {path.name}")
        folded.add(path.name.casefold())
        require(not path.is_symlink() and path.is_file(), f"Not a regular file: {path}")
        require(path.stat().st_size > 0, f"Empty file: {path}")
        if allow_inventory and path.name == INVENTORY:
            continue
        files[path.name] = path
    return files


def signature_text(path):
    try:
        text = path.read_text(encoding="utf-8").strip()
    except UnicodeError as error:
        raise ValueError(f"Signature is not UTF-8 text: {path}") from error
    require(bool(text) and "\x00" not in text, f"Empty or invalid signature: {path}")
    return text


def select_files(directory, platform, final, allow_inventory=False):
    files = directory_files(directory, allow_inventory)
    layout = layout_for(platform)
    selected = {}
    for role, (_, suffix) in layout.items():
        candidates = [path for name, path in files.items() if name.endswith(suffix)]
        require(len(candidates) == 1, f"Expected exactly one {platform} {role}, found {len(candidates)}")
        selected[role] = candidates[0]
    required = {"updater-signature": selected["updater"].name + ".sig"}
    optional = {}
    if platform == "linux-x86_64":
        optional = {f"{role}-openpgp": path.name + ".asc" for role, path in selected.items()}
        optional["public-key"] = PUBLIC_KEY
        if final:
            required.update(optional)
    for role, name in required.items():
        require(name in files, f"Missing {platform} {role}: {name}")
    for role, name in {**optional, **required}.items():
        if name in files:
            signature_text(files[name])
            selected[role] = files[name]
    selected_names = {path.name for path in selected.values()}
    require(selected_names == set(files), f"Unexpected or orphan files: {sorted(set(files) - selected_names)}")
    return selected


def new_output(output):
    require(not output.exists() and not output.is_symlink(), f"Output already exists: {output}")


def collect(bundle, platform, output):
    layout = layout_for(platform)
    require(not bundle.is_symlink() and bundle.is_dir(), f"Missing bundle directory: {bundle}")
    new_output(output)
    candidates = []
    for subdirectory in sorted({directory for directory, _ in layout.values()}):
        parent = bundle / subdirectory
        require(not parent.is_symlink() and parent.is_dir(), f"Missing bundle directory: {parent}")
        for path in sorted(parent.iterdir()):
            # Tauri also produces intermediates and MSI updater archives, not distributed here.
            if path.name.endswith((".msi.zip", ".msi.zip.sig")):
                continue
            if not path.name.endswith(DELIVERY_SUFFIXES + (".sig", ".asc")):
                continue
            safe_name(path.name)
            require(not path.is_symlink() and path.is_file(), f"Not a regular asset: {path}")
            require(path.stat().st_size > 0, f"Empty asset: {path}")
            allowed = tuple(suffix for directory, suffix in layout.values() if directory == subdirectory)
            allowed += tuple(suffix + ".sig" for directory, suffix in layout.values()
                             if directory == subdirectory and suffix == layout["updater"][1])
            require(path.name.endswith(allowed), f"Unexpected asset in {subdirectory}: {path.name}")
            candidates.append(path)
    names = [path.name.casefold() for path in candidates]
    require(len(set(names)) == len(names), "Colliding collected asset names")
    # Validate in memory before creating the output, including exact signature pairing.
    for role, (subdirectory, suffix) in layout.items():
        matching = [path for path in candidates if path.parent.name == subdirectory and path.name.endswith(suffix)]
        require(len(matching) == 1, f"Expected exactly one {platform} {role}, found {len(matching)}")
        if role == "updater":
            signature = matching[0].with_name(matching[0].name + ".sig")
            require(signature in candidates, f"Missing updater signature: {signature.name}")
            signature_text(signature)
    require(len(candidates) == len(layout) + 1, "Unexpected or orphan bundle signatures")
    output.mkdir(parents=True)
    for path in candidates:
        shutil.copyfile(path, output / path.name)


def linux_files(directory):
    files = select_files(directory, "linux-x86_64", final=False, allow_inventory=True)
    return [files[role].resolve() for role in LAYOUTS["linux"]]


def validate_version(commit, version):
    require(isinstance(commit, str) and re.fullmatch(r"[0-9a-f]{40}", commit), "Expected full lowercase Git commit SHA")
    require(isinstance(version, str) and re.fullmatch(VERSION_PATTERN, version), f"Invalid release version: {version!r}")


def expected_identity(platform, windows_subject, linux_fingerprint):
    if platform == "windows-x86_64":
        require(isinstance(windows_subject, str) and windows_subject.strip()
                and all(ord(char) >= 32 for char in windows_subject), "Missing or invalid WINDOWS_SIGNING_SUBJECT")
        return {"windowsSubject": windows_subject}
    if platform == "linux-x86_64":
        require(isinstance(linux_fingerprint, str) and re.fullmatch(r"[0-9A-Fa-f]{40}|[0-9A-Fa-f]{64}", linux_fingerprint),
                "Missing or invalid LINUX_SIGNING_KEY_FINGERPRINT")
        return {"linuxFingerprint": linux_fingerprint.upper()}
    return {}


def file_entry(role, path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return {"role": role, "path": path.name, "size": path.stat().st_size, "sha256": digest.hexdigest()}


def write_json(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, ensure_ascii=False)
        stream.write("\n")


def record(directory, platform, commit, version, windows_subject=None, linux_fingerprint=None):
    validate_version(commit, version)
    identity = expected_identity(platform, windows_subject, linux_fingerprint)
    selected = select_files(directory, platform, final=True)
    manifest = {
        "schemaVersion": 1, "platform": platform, "commit": commit, "version": version,
        "identity": identity,
        "files": [file_entry(role, path) for role, path in sorted(selected.items())],
    }
    write_json(directory / INVENTORY, manifest)
    return manifest


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, f"Duplicate JSON key: {key}")
        value[key] = item
    return value


def verify_inventory(directory, platform, commit, version, windows_subject, linux_fingerprint):
    selected = select_files(directory, platform, final=True, allow_inventory=True)
    inventory = directory / INVENTORY
    require(inventory.is_file() and not inventory.is_symlink(), f"Missing verified inventory: {inventory}")
    value = json.loads(inventory.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    require(isinstance(value, dict) and set(value) == {"schemaVersion", "platform", "commit", "version", "identity", "files"},
            f"Invalid inventory fields: {inventory}")
    require(type(value["schemaVersion"]) is int and value["schemaVersion"] == 1, "Unsupported inventory schema")
    for field, expected in (("platform", platform), ("commit", commit), ("version", version),
                            ("identity", expected_identity(platform, windows_subject, linux_fingerprint))):
        require(value[field] == expected, f"Inventory {field} mismatch for {platform}")
    entries = value["files"]
    require(isinstance(entries, list) and len(entries) == len(selected), f"Incomplete inventory: {platform}")
    seen = set()
    for entry in entries:
        require(isinstance(entry, dict) and set(entry) == {"role", "path", "size", "sha256"}, "Invalid inventory file entry")
        safe_name(entry["path"])
        role = entry["role"]
        require(isinstance(role, str) and role in selected and role not in seen, f"Unknown or duplicate inventory role: {role}")
        seen.add(role)
        require(type(entry["size"]) is int and entry["size"] > 0, "Invalid inventory file size")
        require(entry == file_entry(role, selected[role]), f"Transferred file differs from verified inventory: {entry['path']}")
    return selected, {entry["role"]: entry for entry in entries}


def prepare(source, output, commit, tag, repository, windows_subject, linux_fingerprint, publication_path, notes_path):
    require(isinstance(tag, str) and tag.startswith("v"), "Release tag must start with v")
    version = tag[1:]
    validate_version(commit, version)
    require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository), "Invalid GitHub repository")
    require(source.is_dir() and not source.is_symlink(), f"Missing artifact download directory: {source}")
    require({path.name for path in source.iterdir()} == {f"build-{platform}" for platform in PLATFORMS},
            "Expected exactly the four build platform directories")
    new_output(output)
    for metadata in (publication_path, notes_path):
        new_output(metadata)
        require(output.resolve() not in metadata.resolve().parents, "Publication metadata must remain outside release assets")
    require(publication_path.resolve() != notes_path.resolve(), "Publication and notes paths must differ")
    copies = {}
    names = set()
    updater = {}
    for platform in PLATFORMS:
        selected, verified = verify_inventory(source / f"build-{platform}", platform, commit, version, windows_subject, linux_fingerprint)
        for role, path in selected.items():
            if role == "updater-signature":
                continue
            name = path.name
            if role == "updater" and platform.startswith("darwin"):
                arch = "aarch64" if platform == "darwin-aarch64" else "x64"
                name = f"SkillReg_{arch}.app.tar.gz"
            require(name.casefold() not in names, f"Release asset name collision: {name}")
            names.add(name.casefold())
            copies[name] = (path, verified[role])
            if role == "updater":
                updater[platform] = {"signature": signature_text(selected["updater-signature"]),
                                     "url": f"https://github.com/{repository}/releases/download/{tag}/{name}"}
    require(set(updater) == set(PLATFORMS), "Incomplete updater manifest")
    require("latest.json" not in names, "Release asset collides with latest.json")
    latest = {"version": version, "notes": f"SkillReg v{version}",
              "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"), "platforms": updater}
    # No externally visible release operation happens until all checks above succeeded.
    output.mkdir(parents=True)
    for name, (path, verified) in copies.items():
        destination = output / name
        shutil.copyfile(path, destination)
        require(file_entry(verified["role"], destination) == {**verified, "path": name},
                f"Prepared release file differs from verified inventory: {name}")
    write_json(output / "latest.json", latest)
    publication = {"tag": tag, "version": version,
                   "assets": [str((output / name).resolve()) for name in sorted([*copies, "latest.json"])]}
    write_json(publication_path, publication)
    fingerprint = expected_identity("linux-x86_64", windows_subject, linux_fingerprint)["linuxFingerprint"]
    with notes_path.open("x", encoding="utf-8") as stream:
        stream.write(
            f"SkillReg v{version}\n\n"
            "Windows downloads use Azure Artifact Signing. Linux downloads have detached OpenPGP .asc signatures.\n\n"
            f"Expected Windows publisher subject (exact): `{windows_subject}`\n\n"
            f"Linux signing primary fingerprint: `{fingerprint}`\n\n"
            "Download `skillreg-linux-signing-key.asc`, the Linux artifact and its matching `.asc` file. "
            "Compare the full fingerprint before importing the key into a dedicated GnuPG keyring:\n\n"
            "```sh\n"
            "gpg --show-keys --with-fingerprint skillreg-linux-signing-key.asc\n"
            "mkdir -m 700 ./skillreg-keyring\n"
            "gpg --homedir ./skillreg-keyring --import skillreg-linux-signing-key.asc\n"
            "gpg --homedir ./skillreg-keyring --verify FILE.asc FILE\n"
            "```\n\n"
            "Replace FILE with the exact downloaded filename. These .asc signatures are verified manually; "
            "they are separate from the Tauri updater signatures embedded in latest.json.\n"
        )
    return publication


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    collect_parser = commands.add_parser("collect")
    collect_parser.add_argument("--bundle", type=Path, required=True)
    collect_parser.add_argument("--output", type=Path, required=True)
    collect_parser.add_argument("--platform", choices=PLATFORMS, required=True)
    record_parser = commands.add_parser("record")
    record_parser.add_argument("--directory", type=Path, required=True)
    record_parser.add_argument("--platform", choices=PLATFORMS, required=True)
    record_parser.add_argument("--commit", required=True)
    record_parser.add_argument("--version", required=True)
    files_parser = commands.add_parser("files")
    files_parser.add_argument("--directory", type=Path, required=True)
    files_parser.add_argument("--platform", choices=["linux-x86_64"], required=True)
    prepare_parser = commands.add_parser("prepare")
    prepare_parser.add_argument("--input", dest="source", type=Path, required=True)
    prepare_parser.add_argument("--output", type=Path, required=True)
    prepare_parser.add_argument("--commit", required=True)
    prepare_parser.add_argument("--tag", required=True)
    prepare_parser.add_argument("--repository", required=True)
    prepare_parser.add_argument("--publication", dest="publication_path", type=Path, required=True)
    prepare_parser.add_argument("--notes", dest="notes_path", type=Path, required=True)
    for subparser in (record_parser, prepare_parser):
        subparser.add_argument("--windows-subject", required=subparser is prepare_parser)
        subparser.add_argument("--linux-fingerprint", required=subparser is prepare_parser)
    args = vars(parser.parse_args())
    command = args.pop("command")
    try:
        if command == "files":
            print(json.dumps([str(path) for path in linux_files(args["directory"])]))
        else:
            {"collect": collect, "record": record, "prepare": prepare}[command](**args)
    except (ValueError, OSError) as error:
        print(f"Release artifact validation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
