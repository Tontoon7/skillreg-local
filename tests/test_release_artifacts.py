"""Contract tests for release selection and transfer; fixtures are not signed binaries."""

import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "release-artifacts.py"
SPEC = importlib.util.spec_from_file_location("release_artifacts", SCRIPT)
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)

COMMIT = "a" * 40
VERSION = "0.3.25"
SUBJECT = "CN=Release Test Publisher, O=Test"
FINGERPRINT = "1234567890ABCDEF1234567890ABCDEF12345678"
LAYOUTS = {
    "darwin-aarch64": {
        "dmg/SkillReg_0.3.25_aarch64.dmg": b"arm dmg",
        "macos/SkillReg.app.tar.gz": b"arm archive",
        "macos/SkillReg.app.tar.gz.sig": b"arm tauri signature\n",
    },
    "darwin-x86_64": {
        "dmg/SkillReg_0.3.25_x64.dmg": b"x64 dmg",
        "macos/SkillReg.app.tar.gz": b"x64 archive",
        "macos/SkillReg.app.tar.gz.sig": b"x64 tauri signature\n",
    },
    "linux-x86_64": {
        "deb/SkillReg_0.3.25_amd64.deb": b"deb",
        "rpm/SkillReg-0.3.25-1.x86_64.rpm": b"rpm",
        "appimage/SkillReg_0.3.25_amd64.AppImage": b"appimage",
        "appimage/SkillReg_0.3.25_amd64.AppImage.tar.gz": b"linux archive",
        "appimage/SkillReg_0.3.25_amd64.AppImage.tar.gz.sig": b"linux tauri signature\n",
    },
    "windows-x86_64": {
        "nsis/SkillReg_0.3.25_x64-setup.exe": b"nsis",
        "msi/SkillReg_0.3.25_x64_en-US.msi": b"msi",
        "nsis/SkillReg_0.3.25_x64-setup.nsis.zip": b"windows archive",
        "nsis/SkillReg_0.3.25_x64-setup.nsis.zip.sig": b"windows tauri signature\n",
    },
}


class ReleaseArtifactsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="release artifacts ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.downloaded = self.root / "downloaded"
        self.downloaded.mkdir()

    def bundle(self, platform):
        directory = self.root / f"bundle-{platform}"
        for name, content in LAYOUTS[platform].items():
            file = directory / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_bytes(content)
        return directory

    def collected(self, platform):
        output = self.downloaded / f"build-{platform}"
        release.collect(self.bundle(platform), platform, output)
        if platform == "linux-x86_64":
            for file in release.linux_files(output):
                Path(str(file) + ".asc").write_text("OpenPGP signature\n")
            (output / "skillreg-linux-signing-key.asc").write_text("OpenPGP public key\n")
        return output

    def recorded(self, platform):
        output = self.collected(platform)
        release.record(output, platform, COMMIT, VERSION, SUBJECT, FINGERPRINT)
        return output

    def all_recorded(self):
        return {platform: self.recorded(platform) for platform in LAYOUTS}

    def prepare(self):
        return release.prepare(
            self.downloaded, self.root / "assets", COMMIT, "v" + VERSION,
            "Tontoon7/skillreg-local", SUBJECT, FINGERPRINT,
            self.root / "publication.json", self.root / "notes.md",
        )

    def edit_manifest(self, directory, edit):
        path = directory / "verified-files.json"
        value = json.loads(path.read_text())
        edit(value)
        path.write_text(json.dumps(value))

    def test_complete_release_and_updater_contract(self):
        directories = self.all_recorded()
        publication = self.prepare()
        assets = self.root / "assets"
        names = {Path(path).name for path in publication["assets"]}
        self.assertEqual(names, {path.name for path in assets.iterdir()})
        self.assertEqual(len(names), 17)
        self.assertNotIn("verified-files.json", names)
        self.assertFalse(any(name.endswith(".sig") for name in names))
        self.assertEqual(len([name for name in names if name.endswith(".asc")]), 5)
        self.assertEqual((assets / "SkillReg_aarch64.app.tar.gz").read_bytes(), b"arm archive")
        self.assertEqual((assets / "SkillReg_x64.app.tar.gz").read_bytes(), b"x64 archive")
        latest = json.loads((assets / "latest.json").read_text())
        self.assertEqual(set(latest["platforms"]), set(LAYOUTS))
        self.assertEqual(latest["version"], VERSION)
        for platform, entry in latest["platforms"].items():
            file = entry["url"].rsplit("/", 1)[1]
            self.assertIn(file, names)
            self.assertTrue(entry["url"].startswith(
                "https://github.com/Tontoon7/skillreg-local/releases/download/v0.3.25/"))
            expected = next(directories[platform].glob("*.sig")).read_text().strip()
            self.assertEqual(entry["signature"], expected)
        self.assertIn(FINGERPRINT, (self.root / "notes.md").read_text())
        self.assertIn(SUBJECT, (self.root / "notes.md").read_text())
        self.assertEqual(json.loads((self.root / "publication.json").read_text()), publication)

    def test_collect_requires_each_role(self):
        for platform in LAYOUTS:
            for name in LAYOUTS[platform]:
                with self.subTest(platform=platform, missing=name):
                    bundle = self.bundle(platform)
                    (bundle / name).unlink()
                    with self.assertRaises(ValueError):
                        release.collect(bundle, platform, self.root / "output")

    def test_collect_rejects_duplicate_empty_orphan_and_wrong_location(self):
        bundle = self.bundle("linux-x86_64")
        problems = [
            ("deb/duplicate.deb", b"duplicate"),
            ("appimage/orphan.AppImage.tar.gz.sig", b"orphan"),
            ("rpm/wrong.deb", b"wrong directory"),
            ("deb/empty.deb", b""),
        ]
        for name, content in problems:
            with self.subTest(name=name):
                path = bundle / name
                path.write_bytes(content)
                with self.assertRaises(ValueError):
                    release.collect(bundle, "linux-x86_64", self.root / "output")
                path.unlink()

    def test_collect_ignores_intermediates_and_msi_updater(self):
        bundle = self.bundle("windows-x86_64")
        (bundle / "msi" / "unused.msi.zip").write_bytes(b"not distributed")
        (bundle / "nsis" / "nested").mkdir()
        (bundle / "nsis" / "nested" / "intermediate.exe").write_bytes(b"intermediate")
        output = self.root / "output"
        release.collect(bundle, "windows-x86_64", output)
        self.assertEqual(len(list(output.iterdir())), 4)

    def test_collect_rejects_missing_bundle_family(self):
        bundle = self.bundle("linux-x86_64")
        shutil.rmtree(bundle / "appimage")
        with self.assertRaises(ValueError):
            release.collect(bundle, "linux-x86_64", self.root / "output")

    def test_collect_rejects_whitespace_signature(self):
        bundle = self.bundle("darwin-aarch64")
        (bundle / "macos" / "SkillReg.app.tar.gz.sig").write_text(" \n\t")
        with self.assertRaises(ValueError):
            release.collect(bundle, "darwin-aarch64", self.root / "output")

    def test_record_requires_linux_signatures_and_key(self):
        output = self.collected("linux-x86_64")
        for path in sorted(output.glob("*.asc")):
            content = path.read_bytes()
            for replacement in (None, b"", b" \n"):
                with self.subTest(path=path.name, replacement=replacement):
                    if replacement is None:
                        path.unlink()
                    else:
                        path.write_bytes(replacement)
                    with self.assertRaises(ValueError):
                        release.record(output, "linux-x86_64", COMMIT, VERSION, SUBJECT, FINGERPRINT)
                    path.write_bytes(content)

    def test_missing_platform_and_unexpected_directory(self):
        directories = self.all_recorded()
        path = directories["darwin-aarch64"]
        path.rename(self.downloaded / "unexpected")
        with self.assertRaises(ValueError):
            self.prepare()

    def test_missing_platform_without_replacement(self):
        directories = self.all_recorded()
        shutil.rmtree(directories["linux-x86_64"])
        with self.assertRaises(ValueError):
            self.prepare()

    def test_tauri_signatures_are_included_in_transfer_integrity(self):
        directories = self.all_recorded()
        for directory in directories.values():
            signature = next(directory.glob("*.sig"))
            original = signature.read_bytes()
            for content in (b"", b" \n", b"different tauri signature"):
                with self.subTest(platform=directory.name, content=content):
                    signature.write_bytes(content)
                    with self.assertRaises(ValueError):
                        self.prepare()
                    signature.write_bytes(original)

    def test_changed_file_hash_and_size(self):
        directories = self.all_recorded()
        file = next(directories["windows-x86_64"].glob("*.exe"))
        for value in (b"NSIS", b"longer modified bytes"):
            with self.subTest(value=value):
                file.write_bytes(value)
                with self.assertRaises(ValueError):
                    self.prepare()
                self.assertFalse((self.root / "assets").exists())

    def test_copy_corruption_never_produces_publication_list(self):
        self.all_recorded()
        copy = release.shutil.copyfile

        def corrupt_copy(source, target):
            copy(source, target)
            target.write_bytes(b"corrupt copy")

        with mock.patch.object(release.shutil, "copyfile", side_effect=corrupt_copy):
            with self.assertRaises(ValueError):
                self.prepare()
        self.assertFalse((self.root / "publication.json").exists())

    def test_extra_missing_and_nested_files_refused(self):
        directories = self.all_recorded()
        directory = directories["linux-x86_64"]
        extra = directory / "unexpected.txt"
        extra.write_text("extra")
        with self.assertRaises(ValueError):
            self.prepare()
        extra.unlink()
        nested = directory / "nested"
        nested.mkdir()
        with self.assertRaises(ValueError):
            self.prepare()
        nested.rmdir()
        next(directory.glob("*.asc")).unlink()
        with self.assertRaises(ValueError):
            self.prepare()

    def test_manifest_metadata_and_identity_must_match(self):
        directories = self.all_recorded()
        for platform, field, value in [
            ("windows-x86_64", "identity", {"windowsSubject": "CN=Other"}),
            ("linux-x86_64", "identity", {"linuxFingerprint": "0" * 40}),
            ("linux-x86_64", "identity", {}),
            ("darwin-aarch64", "commit", "b" * 40),
            ("darwin-aarch64", "version", "0.3.24"),
            ("darwin-aarch64", "platform", "darwin-x86_64"),
            ("darwin-aarch64", "schemaVersion", 2),
        ]:
            with self.subTest(platform=platform, field=field):
                path = directories[platform] / "verified-files.json"
                original = path.read_bytes()
                self.edit_manifest(directories[platform], lambda manifest: manifest.update({field: value}))
                with self.assertRaises(ValueError):
                    self.prepare()
                path.write_bytes(original)

    def test_invalid_manifest_entries(self):
        directories = self.all_recorded()
        directory = directories["darwin-aarch64"]
        path = directory / "verified-files.json"
        original = path.read_bytes()
        modifications = [
            lambda value: value["files"].append(value["files"][0]),
            lambda value: value["files"].pop(),
            lambda value: value["files"][0].update(path="../escape.dmg"),
            lambda value: value["files"][0].update(path="/absolute.dmg"),
            lambda value: value["files"][0].update(path="nested\\escape.dmg"),
            lambda value: value["files"][0].update(size=True),
            lambda value: value["files"][0].update(sha256="wrong"),
            lambda value: value["files"][0].update(role="unexpected"),
            lambda value: value.update(unknown="ignored?"),
        ]
        for modification in modifications:
            with self.subTest(modification=modification):
                self.edit_manifest(directory, modification)
                with self.assertRaises(ValueError):
                    self.prepare()
                path.write_bytes(original)

    def test_duplicate_json_keys_refused(self):
        directories = self.all_recorded()
        path = directories["darwin-aarch64"] / "verified-files.json"
        value = path.read_text()
        path.write_text(value.replace('"schemaVersion": 1', '"schemaVersion": 2, "schemaVersion": 1'))
        with self.assertRaises(ValueError):
            self.prepare()

    def test_symlinks_refused(self):
        directories = self.all_recorded()
        directory = directories["linux-x86_64"]
        path = next(directory.glob("*.deb"))
        destination = self.root / "original.deb"
        path.rename(destination)
        path.symlink_to(destination)
        with self.assertRaises(ValueError):
            self.prepare()

    def test_collision_of_publication_names_refused(self):
        for platform in LAYOUTS:
            directory = self.collected(platform)
            if platform.startswith("darwin"):
                next(directory.glob("*.dmg")).rename(directory / "collision.dmg")
            release.record(directory, platform, COMMIT, VERSION, SUBJECT, FINGERPRINT)
        with self.assertRaises(ValueError):
            self.prepare()

    def test_record_requires_valid_trusted_inputs(self):
        output = self.collected("windows-x86_64")
        for commit, version, subject in [("", VERSION, SUBJECT), (COMMIT, "../1", SUBJECT), (COMMIT, VERSION, "")]:
            with self.subTest(commit=commit, version=version, subject=subject):
                with self.assertRaises(ValueError):
                    release.record(output, "windows-x86_64", commit, version, subject, FINGERPRINT)
        linux = self.collected("linux-x86_64")
        with self.assertRaises(ValueError):
            release.record(linux, "linux-x86_64", COMMIT, VERSION, SUBJECT, "short key id")

    def test_cli_end_to_end_and_failure_exit(self):
        for platform in LAYOUTS:
            bundle = self.bundle(platform)
            output = self.downloaded / f"build-{platform}"
            result = subprocess.run([sys.executable, str(SCRIPT), "collect", "--bundle", str(bundle),
                "--platform", platform, "--output", str(output)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            if platform == "linux-x86_64":
                result = subprocess.run([sys.executable, str(SCRIPT), "files", "--directory", str(output),
                    "--platform", platform], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                files = json.loads(result.stdout)
                self.assertEqual(len(files), 4)
                for file in files:
                    Path(file + ".asc").write_text("OpenPGP signature")
                (output / "skillreg-linux-signing-key.asc").write_text("Public key")
            result = subprocess.run([sys.executable, str(SCRIPT), "record", "--directory", str(output),
                "--platform", platform, "--commit", COMMIT, "--version", VERSION,
                "--windows-subject", SUBJECT, "--linux-fingerprint", FINGERPRINT], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
        args = [sys.executable, str(SCRIPT), "prepare", "--input", str(self.downloaded),
            "--output", str(self.root / "assets"), "--commit", COMMIT, "--tag", "v" + VERSION,
            "--repository", "Tontoon7/skillreg-local", "--windows-subject", SUBJECT,
            "--linux-fingerprint", FINGERPRINT, "--publication", str(self.root / "publication.json"),
            "--notes", str(self.root / "notes.md")]
        result = subprocess.run(args, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        result = subprocess.run(args, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)


class ReleaseWorkflowTests(unittest.TestCase):
    def test_build_barrier_and_native_checks_are_ordered(self):
        workflow = (SCRIPT.parents[1] / ".github/workflows/release.yml").read_text()
        build = workflow.split("  publish:\n", 1)[0]
        steps = ["Test release artifact contract", "Validate signing prerequisites", "Build Tauri",
                 "Select release artifacts", "Sign and verify Linux artifacts", "Verify Windows artifacts",
                 "Record verified inventory", "Upload artifacts"]
        positions = [build.index("name: " + step) for step in steps]
        self.assertEqual(positions, sorted(positions))
        for required in ["test-linux-signing.sh", "test-windows-signing.ps1", "linux-signing.sh",
                         "windows-signing.ps1", "release-artifacts.py", "if-no-files-found: error",
                         "TAURI_SIGNING_PRIVATE_KEY", "--config"]:
            self.assertIn(required, build)
        hook = build.split("name: Configure Windows signing hook", 1)[1].split("\n      - ", 1)[0]
        self.assertIn("if: runner.os == 'Windows'", hook)
        self.assertIn("signCommand = @{", hook)
        self.assertIn("args = @('-NoProfile', '-File', $wrapper, '-Mode', 'sign', '-FilePath', '%1')", hook)
        build_step = build.split("name: Build Tauri", 1)[1].split("\n      - ", 1)[0]
        self.assertIn('if [ "$RUNNER_OS" = Windows ]; then\n            args+=(--config "$WINDOWS_SIGNING_CONFIG")\n          fi', build_step)
        self.assertIn('"${args[@]}"', build_step)
        windows_check = build.split("name: Verify Windows artifacts", 1)[1].split("\n      - ", 1)[0]
        self.assertIn("windows-signing.ps1 -Mode verify-artifacts", windows_check)
        self.assertIn("-ApplicationPath", windows_check)

    def test_publication_only_after_all_checks(self):
        workflow = (SCRIPT.parents[1] / ".github/workflows/release.yml").read_text()
        publish = workflow.split("  publish:\n", 1)[1]
        self.assertIn("needs: build", publish)
        steps = ["Prepare verified release", "Verify transferred Linux signatures", "Create draft release"]
        positions = [publish.index("name: " + step) for step in steps]
        self.assertEqual(positions, sorted(positions))
        before_create = publish[:positions[-1]]
        self.assertNotIn("gh release", before_create)
        self.assertNotIn("continue-on-error", workflow)
        self.assertNotIn("|| true", workflow)
        self.assertNotIn("always()", publish)
        self.assertIn("--draft", publish)
        self.assertIn("release-artifacts.py prepare", publish)
        self.assertIn("LINUX_SIGNING_KEY_FINGERPRINT", publish)
        self.assertIn("LINUX_SIGNING_PUBLIC_KEY", publish)
        self.assertNotIn("--clobber", publish)


if __name__ == "__main__":
    unittest.main()
