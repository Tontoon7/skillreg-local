import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
	constants,
	chmodSync,
	chownSync,
	closeSync,
	copyFileSync,
	fsyncSync,
	lchmodSync,
	lchownSync,
	lstatSync,
	mkdirSync,
	openSync,
	readFileSync,
	readdirSync,
	readlinkSync,
	realpathSync,
	renameSync,
	symlinkSync,
	writeFileSync,
} from "node:fs";
import { basename, dirname, isAbsolute, join, parse, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const roots = [".skillreg", ".claude", ".codex", ".cursor", ".agents"];
const windows = process.platform === "win32";
const hash = (value) => createHash("sha256").update(value).digest("hex");
const equal = (left, right) => JSON.stringify(left) === JSON.stringify(right);

function requireState(condition, message) {
	if (!condition) throw new Error(message);
}

function requireInventory(actual, expected, message) {
	if (equal(actual, expected)) return;
	requireState(actual.length === expected.length, `${message} (entry count)`);
	const index = actual.findIndex((entry, index) => !equal(entry, expected[index]));
	const fields = ["path", "type", "mode", "uid", "gid", "target", "sha256", "size", "acl"].filter(
		(field) => !equal(actual[index][field], expected[index][field]),
	);
	// Report only the index and known field names, never profile paths or descriptor values.
	throw new Error(`${message} (entry ${index}: ${fields.join(", ") || "metadata"})`);
}

function during(phase, operation) {
	try {
		return operation();
	} catch (error) {
		// Filesystem messages may contain private paths; retain only their stable error code.
		throw new Error(`${phase}: ${error.code || error.message}`);
	}
}

function stat(path) {
	try {
		return lstatSync(path);
	} catch (error) {
		if (error.code === "ENOENT") return null;
		throw error;
	}
}

function directory(path) {
	const info = stat(path);
	requireState(info?.isDirectory() && !info.isSymbolicLink(), "Expected a real directory");
}

function canonical(path) {
	requireState(typeof path === "string" && isAbsolute(path), "Paths must be explicit and absolute");
	const resolved = resolve(path);
	requireState(
		resolved !== parse(resolved).root,
		"A volume root cannot be used as a profile, backup or recovery",
	);
	if (stat(resolved)) {
		requireState(!lstatSync(resolved).isSymbolicLink(), "A profile or backup cannot be a link");
		return realpathSync(resolved);
	}
	directory(dirname(resolved));
	return join(realpathSync(dirname(resolved)), basename(resolved));
}

function separate(...paths) {
	for (let i = 0; i < paths.length; i++) {
		for (let j = i + 1; j < paths.length; j++) {
			const fold = (path) =>
				windows || process.platform === "darwin" ? path.normalize("NFC").toLowerCase() : path;
			const a = fold(paths[i]);
			const b = fold(paths[j]);
			requireState(
				a !== b && !a.startsWith(`${b}${sep}`) && !b.startsWith(`${a}${sep}`),
				"Profile, backup and recovery must not overlap",
			);
		}
	}
}

function native(command, args, input) {
	const result = spawnSync(command, args, { encoding: "utf8", input, windowsHide: true });
	const code =
		command === "powershell.exe"
			? /^\{"nativeError":([1-9]\d{0,9})\}$/.exec(result.stdout?.trim() || "")?.[1]
			: null;
	const detail = code && Number(code) <= 0xffffffff ? ` (Win32 ${code})` : "";
	requireState(
		!result.error && result.status === 0,
		`Required native metadata operation failed: ${command}${detail}`,
	);
	return result.stdout.trimEnd();
}

// Handles opened with OPEN_REPARSE_POINT operate on junctions themselves, never their targets.
const windowsMetadata = String.raw`
$ErrorActionPreference = 'Stop'
try {
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class ProfileMetadata {
  [StructLayout(LayoutKind.Sequential)]
  struct RenameInfo {
    public uint Flags;
    public IntPtr RootDirectory;
    public uint FileNameLength;
    public ushort FileName;
  }
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  static extern SafeFileHandle CreateFile(string name, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);
  [DllImport("kernel32.dll", SetLastError=true)]
  static extern bool SetFileInformationByHandle(SafeFileHandle handle, int kind, IntPtr info, uint size);
  [DllImport("advapi32.dll", SetLastError=true)]
  static extern bool GetKernelObjectSecurity(SafeFileHandle handle, uint info, IntPtr descriptor, uint size, out uint required);
  [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  static extern bool ConvertSecurityDescriptorToStringSecurityDescriptor(IntPtr descriptor, uint revision, uint info, out IntPtr value, out uint size);
  [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  static extern bool ConvertStringSecurityDescriptorToSecurityDescriptor(string value, uint revision, out IntPtr descriptor, out uint size);
  [DllImport("advapi32.dll", SetLastError=true)]
  static extern bool SetKernelObjectSecurity(SafeFileHandle handle, uint info, IntPtr descriptor);
  [DllImport("advapi32.dll", SetLastError=true)]
  static extern bool GetSecurityDescriptorOwner(IntPtr descriptor, out IntPtr owner, out bool defaulted);
  [DllImport("advapi32.dll", SetLastError=true)]
  static extern bool GetSecurityDescriptorGroup(IntPtr descriptor, out IntPtr group, out bool defaulted);
  [DllImport("advapi32.dll", SetLastError=true)]
  static extern bool GetSecurityDescriptorDacl(IntPtr descriptor, out bool present, out IntPtr dacl, out bool defaulted);
  [DllImport("advapi32.dll")]
  static extern uint SetSecurityInfo(SafeFileHandle handle, int kind, uint info, IntPtr owner, IntPtr group, IntPtr dacl, IntPtr sacl);
  [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr value);
  public static void Rename(string source, string destination) {
    // Rename the object itself without a post-rename ACL repair window.
    using (var handle = CreateFile(source, 0x10000, 7, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero)) {
      if (handle.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error());
      byte[] name = System.Text.Encoding.Unicode.GetBytes(destination);
      int offset = Marshal.OffsetOf(typeof(RenameInfo), "FileName").ToInt32();
      int size = Math.Max(Marshal.SizeOf(typeof(RenameInfo)), offset + name.Length + 2);
      IntPtr buffer = Marshal.AllocHGlobal(size);
      try {
        // Flags=0 refuses an existing destination; RootDirectory=0 uses the absolute path.
        var info = new RenameInfo { Flags = 0, RootDirectory = IntPtr.Zero, FileNameLength = (uint)name.Length, FileName = 0 };
        Marshal.StructureToPtr(info, buffer, false);
        Marshal.Copy(name, 0, IntPtr.Add(buffer, offset), name.Length);
        // The Win32 path conversion also needs a terminator, excluded from FileNameLength.
        Marshal.WriteInt16(buffer, offset + name.Length, 0);
        if (!SetFileInformationByHandle(handle, 3, buffer, (uint)size)) throw new Win32Exception(Marshal.GetLastWin32Error());
      } finally { Marshal.FreeHGlobal(buffer); }
    }
  }
  public static string Access(string path, string replacement, bool write) {
    uint flags = 4;
    if (write && replacement.Contains("O:")) flags |= 1;
    if (write && replacement.Contains("G:")) flags |= 2;
    uint access = write ? 0x60000u : 0x20000u;
    if (write && (flags & 3) != 0) access |= 0x80000u;
    using (var handle = CreateFile(path, access, 7, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero)) {
      if (handle.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error());
      if (write) {
        IntPtr next; uint size;
        if (!ConvertStringSecurityDescriptorToSecurityDescriptor(replacement, 1, out next, out size)) throw new Win32Exception(Marshal.GetLastWin32Error());
        try {
          if (replacement.Contains("D:P")) {
            // SetKernelObjectSecurity does not set file DACL protection. The file API must apply it explicitly.
            IntPtr owner, group, dacl; bool defaulted, present;
            if (!GetSecurityDescriptorOwner(next, out owner, out defaulted) ||
                !GetSecurityDescriptorGroup(next, out group, out defaulted) ||
                !GetSecurityDescriptorDacl(next, out present, out dacl, out defaulted)) throw new Win32Exception(Marshal.GetLastWin32Error());
            uint error = SetSecurityInfo(handle, 1, flags | 0x80000000u, owner, group, dacl, IntPtr.Zero);
            if (error != 0) throw new Win32Exception((int)error);
          } else {
            // New clones already allow inheritance; keep the saved ACEs without inheriting from the backup parent.
            if (!SetKernelObjectSecurity(handle, flags, next)) throw new Win32Exception(Marshal.GetLastWin32Error());
          }
        } finally { LocalFree(next); }
      }
      // Read the stored descriptor without interpreting inheritance against the current parent.
      uint required;
      if (!GetKernelObjectSecurity(handle, 7, IntPtr.Zero, 0, out required)) {
        int error = Marshal.GetLastWin32Error();
        if (error != 122) throw new Win32Exception(error);
      }
      IntPtr descriptor = Marshal.AllocHGlobal(checked((int)required));
      try {
        if (!GetKernelObjectSecurity(handle, 7, descriptor, required, out required)) throw new Win32Exception(Marshal.GetLastWin32Error());
        IntPtr text; uint length;
        if (!ConvertSecurityDescriptorToStringSecurityDescriptor(descriptor, 1, 7, out text, out length)) throw new Win32Exception(Marshal.GetLastWin32Error());
        try { return Marshal.PtrToStringUni(text); } finally { LocalFree(text); }
      } finally { Marshal.FreeHGlobal(descriptor); }
    }
  }
}
'@
$requests = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), [System.Text.Encoding]::UTF8).ReadToEnd() | ConvertFrom-Json
$results = @($requests | ForEach-Object {
  if ($null -ne $_.destination) {
    [ProfileMetadata]::Rename($_.path, $_.destination)
    'renamed'
  } else {
    $replacement = $_.acl
    if ($replacement -eq 'private') {
      $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
      $replacement = 'D:P(A;OICI;FA;;;' + $sid + ')'
    }
    [ProfileMetadata]::Access($_.path, $replacement, ($null -ne $_.acl))
  }
})
ConvertTo-Json -InputObject $results -Compress
} catch {
  $exception = $_.Exception
  while ($null -ne $exception.InnerException) { $exception = $exception.InnerException }
  if ($exception -is [System.ComponentModel.Win32Exception]) {
    ConvertTo-Json -InputObject @{nativeError=$exception.NativeErrorCode} -Compress
  }
  exit 1
}
`;

export function winMetadata(requests) {
	if (!requests.length) return [];
	const result = native(
		"powershell.exe",
		[
			"-NoProfile",
			"-NonInteractive",
			"-EncodedCommand",
			Buffer.from(windowsMetadata, "utf16le").toString("base64"),
		],
		JSON.stringify(requests),
	);
	// Windows PowerShell 5.1 can emit nothing for a void-only pipeline; ACLs still require JSON.
	if (!result && requests.every((request) => typeof request.destination === "string")) return [];
	// PowerShell can serialize a single result as a scalar instead of an array.
	const values = JSON.parse(result);
	return Array.isArray(values) ? values : [values];
}

function renameRoot(source, destination) {
	if (windows) winMetadata([{ path: source, destination }]);
	else renameSync(source, destination);
}

function privateDirectory(path) {
	mkdirSync(path, { mode: 0o700 });
	if (windows) winMetadata([{ path, acl: "private" }]);
	else if (process.platform === "darwin") native("/bin/chmod", ["-N", path]);
	else native("setfacl", ["--remove-all", "--remove-default", "--", path]);
	flushDirectory(dirname(path));
}

function flushDirectory(path) {
	if (windows) return;
	const descriptor = openSync(path, "r");
	try {
		fsyncSync(descriptor);
	} finally {
		closeSync(descriptor);
	}
}

function unixAcl(path) {
	if (process.platform === "darwin") {
		return native("/bin/ls", ["-lde", path])
			.split("\n")
			.slice(1)
			.map((line) => line.replace(/^\s*\d+: /, ""));
	}
	requireState(process.platform === "linux", "Unsupported metadata platform");
	return native("getfacl", ["--absolute-names", "--omit-header", "--numeric", "--", path]);
}

function inventory(base) {
	directory(base);
	const entries = [];
	function visit(path) {
		const absolute = join(base, path);
		const info = stat(absolute);
		if (!info) return;
		requireState(!path.includes("\\") && !path.includes(":"), "Nonportable inventory path");
		const entry = { path, type: "", mode: info.mode & 0o7777 };
		if (!windows) {
			entry.uid = info.uid;
			entry.gid = info.gid;
		}
		if (info.isSymbolicLink()) {
			entry.type = "link";
			entry.target = readlinkSync(absolute);
			if (windows) {
				const kind = native(
					"powershell.exe",
					[
						"-NoProfile",
						"-NonInteractive",
						"-Command",
						"$ErrorActionPreference='Stop'; $path=[System.IO.StreamReader]::new([Console]::OpenStandardInput(), [System.Text.Encoding]::UTF8).ReadToEnd(); (Get-Item -Force -LiteralPath $path).LinkType",
					],
					absolute,
				);
				requireState(
					kind === "Junction",
					"Only Windows junctions have a supported restoration protocol",
				);
			}
		} else if (info.isDirectory()) {
			entry.type = "directory";
		} else {
			requireState(
				info.isFile() && info.nlink === 1,
				"Special files and hard links cannot be backed up safely",
			);
			entry.type = "file";
			entry.sha256 = hash(readFileSync(absolute));
			entry.size = info.size;
		}
		if (!windows) {
			// Linux ACL tools dereference links; symlink permissions are fixed and carry no POSIX ACL.
			entry.acl = entry.type === "link" && process.platform === "linux" ? null : unixAcl(absolute);
		}
		entries.push(entry);
		if (entry.type === "directory") {
			for (const child of readdirSync(absolute).sort()) visit(`${path}/${child}`);
		}
	}
	for (const root of roots) visit(root);
	if (windows) {
		const acls = winMetadata(entries.map((entry) => ({ path: join(base, entry.path) })));
		entries.forEach((entry, index) => {
			entry.acl = acls[index];
		});
	}
	return entries;
}

function applySecurity(path, entry) {
	if (windows) {
		if (entry.type !== "link") chmodSync(path, entry.mode);
		return;
	}
	const info = lstatSync(path);
	if (entry.uid !== info.uid || entry.gid !== info.gid) {
		(entry.type === "link" ? lchownSync : chownSync)(path, entry.uid, entry.gid);
	}
	if (entry.type === "link") {
		if (process.platform === "darwin") {
			requireState(entry.acl.length === 0, "ACL on a macOS symlink cannot be restored safely");
			lchmodSync(path, entry.mode);
		}
		return;
	}
	chmodSync(path, entry.mode);
	if (process.platform === "darwin") {
		native("/bin/chmod", ["-N", path]);
		for (const acl of entry.acl) native("/bin/chmod", ["+a", acl, path]);
	} else {
		native("setfacl", ["--set-file=-", "--", path], entry.acl);
	}
}

function clone(source, destination, entries) {
	privateDirectory(destination);
	for (const entry of entries) {
		const from = join(source, entry.path);
		const to = join(destination, entry.path);
		if (entry.type === "directory") mkdirSync(to, { mode: 0o700 });
		else if (entry.type === "file") {
			copyFileSync(from, to, constants.COPYFILE_EXCL);
			if (!windows) {
				const descriptor = openSync(to, "r");
				try {
					fsyncSync(descriptor);
				} finally {
					closeSync(descriptor);
				}
			}
		} else symlinkSync(entry.target, to, windows ? "junction" : undefined);
	}
	if (windows) {
		winMetadata(entries.map((entry) => ({ path: join(destination, entry.path), acl: entry.acl })));
	}
	for (const entry of [...entries].reverse()) {
		applySecurity(join(destination, entry.path), entry);
		if (entry.type === "directory") flushDirectory(join(destination, entry.path));
	}
	flushDirectory(destination);
	requireInventory(
		inventory(destination),
		entries,
		"Copied content or permissions do not match the inventory",
	);
}

function writeDurable(path, value, exclusive = true) {
	const file = openSync(path, exclusive ? "wx" : "w", 0o600);
	try {
		writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
		fsyncSync(file);
	} finally {
		closeSync(file);
	}
	flushDirectory(dirname(path));
}

function snapshot(home, backup) {
	directory(home);
	requireState(!stat(backup), "Backup destination must be new");
	const entries = inventory(home);
	privateDirectory(backup);
	clone(home, join(backup, "data"), entries);
	requireState(
		equal(inventory(home), entries),
		"Profile changed during backup; snapshot is incomplete",
	);
	const manifest = { version: 1, home, platform: process.platform, roots, entries };
	writeDurable(join(backup, "inventory.json"), manifest);
	return manifest;
}

function verify(home, backup) {
	directory(backup);
	const manifestPath = join(backup, "inventory.json");
	requireState(
		stat(manifestPath)?.isFile() && !lstatSync(manifestPath).isSymbolicLink(),
		"Inventory must be a regular file",
	);
	const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
	requireState(
		manifest.version === 1 &&
			manifest.home === home &&
			manifest.platform === process.platform &&
			equal(manifest.roots, roots) &&
			Array.isArray(manifest.entries),
		"Backup does not describe this profile and platform",
	);
	const seen = new Set();
	for (const entry of manifest.entries) {
		requireState(
			typeof entry.path === "string" &&
				!isAbsolute(entry.path) &&
				!/[\\:\0]/.test(entry.path) &&
				roots.includes(entry.path.split("/")[0]) &&
				entry.path.split("/").every((part) => part && part !== "." && part !== "..") &&
				!seen.has(entry.path),
			"Unsafe or duplicate inventory path",
		);
		const parent = entry.path.includes("/")
			? entry.path.slice(0, entry.path.lastIndexOf("/"))
			: null;
		requireState(
			!parent ||
				manifest.entries.some(
					(candidate) => candidate.path === parent && candidate.type === "directory",
				),
			"Inventory parent is missing or linked",
		);
		seen.add(entry.path);
	}
	requireInventory(
		inventory(join(backup, "data")),
		manifest.entries,
		"Backup inventory is incomplete or corrupted",
	);
	return manifest;
}

function rootEntries(entries, root) {
	return entries.filter((entry) => entry.path === root || entry.path.startsWith(`${root}/`));
}

function resumeStates(home, recovery, before, target) {
	const live = inventory(home);
	const moved = inventory(join(recovery, "moved"));
	const staged = inventory(join(recovery, "staged"));
	return roots.map((root) => {
		const old = rootEntries(before.entries, root);
		const next = rootEntries(target.entries, root);
		const current = rootEntries(live, root);
		const saved = rootEntries(moved, root);
		const ready = rootEntries(staged, root);
		if (equal(current, next) && equal(saved, old) && !ready.length) return "done";
		if (equal(current, old) && !saved.length && equal(ready, next)) return "move";
		if (!current.length && equal(saved, old) && equal(ready, next)) return "install";
		throw new Error("Restore state changed or is ambiguous; retained backups must be inspected");
	});
}

function restore(home, backup, recovery, target, afterRename) {
	const journalPath = join(recovery, "restore-journal.json");
	const targetHash = hash(JSON.stringify(target));
	let before;
	if (!stat(recovery)) {
		requireState(
			lstatSync(home).dev === lstatSync(dirname(recovery)).dev,
			"Recovery must use the profile filesystem for atomic swaps",
		);
		before = during("Restore recovery snapshot", () => snapshot(home, recovery));
		during("Restore staging", () =>
			clone(join(backup, "data"), join(recovery, "staged"), target.entries),
		);
		during("Restore recovery preparation", () => privateDirectory(join(recovery, "moved")));
		during("Restore journal write", () =>
			writeDurable(journalPath, { version: 1, home, backup, targetHash, status: "pending" }),
		);
	} else {
		before = during("Restore recovery verification", () => verify(home, recovery));
		requireState(
			stat(journalPath)?.isFile() && !lstatSync(journalPath).isSymbolicLink(),
			"Recovery has no valid restore journal",
		);
		const journal = JSON.parse(readFileSync(journalPath, "utf8"));
		requireState(
			journal.version === 1 &&
				journal.home === home &&
				journal.backup === backup &&
				journal.targetHash === targetHash &&
				["pending", "complete"].includes(journal.status),
			"Recovery journal belongs to another restore",
		);
	}
	during("Restore state inspection", () => resumeStates(home, recovery, before, target));
	for (let index = 0; index < roots.length; index++) {
		const root = roots[index];
		const state = during("Restore state inspection", () =>
			resumeStates(home, recovery, before, target),
		)[index];
		if (state === "done") continue;
		if (state === "move" && stat(join(home, root))) {
			during("Restore profile root", () => {
				renameRoot(join(home, root), join(recovery, "moved", root));
				flushDirectory(home);
				flushDirectory(join(recovery, "moved"));
				afterRename?.();
			});
		}
		if (stat(join(recovery, "staged", root))) {
			requireState(!stat(join(home, root)), "Restore destination reappeared during swap");
			during("Restore activation", () => {
				renameRoot(join(recovery, "staged", root), join(home, root));
				flushDirectory(home);
				flushDirectory(join(recovery, "staged"));
				afterRename?.();
			});
		}
	}
	requireInventory(inventory(home), target.entries, "Restored profile does not match the backup");
	const pending = `${journalPath}.next`;
	const completed = { version: 1, home, backup, targetHash, status: "complete" };
	if (stat(pending)) {
		requireState(
			lstatSync(pending).isFile() &&
				!lstatSync(pending).isSymbolicLink() &&
				equal(JSON.parse(readFileSync(pending, "utf8")), completed),
			"A journal update is ambiguous; inspect the retained recovery",
		);
	} else writeDurable(pending, completed);
	during("Restore journal commit", () => renameSync(pending, journalPath));
	flushDirectory(recovery);
	return { status: "restored", entries: target.entries.length, recoveryPreserved: true };
}

export function execute(args, hooks = {}) {
	const [command, ...rest] = args;
	requireState(
		["snapshot", "verify", "restore"].includes(command),
		"Expected snapshot, verify or restore",
	);
	const options = {};
	for (let i = 0; i < rest.length; i++) {
		const key = rest[i];
		requireState(
			["--home", "--backup", "--recovery", "--apply"].includes(key) && !(key in options),
			"Unknown or repeated argument",
		);
		options[key] = key === "--apply" ? true : rest[++i];
	}
	const home = canonical(options["--home"]);
	const backup = canonical(options["--backup"]);
	directory(home);
	separate(home, backup);
	requireState(
		command === "restore" || !(options["--apply"] || options["--recovery"]),
		"Restore arguments are only valid for restore",
	);
	if (command === "snapshot")
		return { status: "saved", entries: snapshot(home, backup).entries.length };
	const target = verify(home, backup);
	if (command === "verify") return { status: "verified", entries: target.entries.length };
	if (!options["--apply"])
		return {
			status: "preview",
			roots,
			entries: target.entries.length,
			currentEntries: inventory(home).length,
		};
	const recovery = canonical(options["--recovery"]);
	separate(home, backup, recovery);
	return restore(home, backup, recovery, target, hooks.afterRename);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	try {
		process.stdout.write(`${JSON.stringify(execute(process.argv.slice(2)))}\n`);
	} catch (error) {
		// Native errors can include profile paths; never print file contents or native stderr.
		process.stderr.write(`Backup/restore failed: ${error.code || error.message}\n`);
		process.exitCode = 1;
	}
}
