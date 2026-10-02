#![cfg(target_os = "windows")]

mod support;

use skillreg_local_lib::managed_skills::{
    errors::ManagedErrorCode,
    manifest::{write_manifest_atomic, ManagedSkillsManifest},
    permissions::restrict_to_current_user,
    platform_links::{PlatformLinker, SystemPlatformLinker},
};
use std::{fs, path::Path, process::Command};
use support::TestHome;

fn powershell(path: &Path, script: &str) -> String {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("SKILLREG_TEST_PATH", path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn windows_profile_and_ntfs_junctions_are_verified_without_requesting_elevation() {
    let home = TestHome::new("permissions espaces été");
    let details = powershell(
        &home.path,
        r#"
$ErrorActionPreference = 'Stop'
$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
$drive = [System.IO.DriveInfo]::new([System.IO.Path]::GetPathRoot($env:SKILLREG_TEST_PATH))
if ($drive.DriveFormat -ne 'NTFS') { throw 'NTFS required for the release fixture' }
Write-Output ('filesystem=' + $drive.DriveFormat)
Write-Output ('administrator=' + $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator))
"#,
    );
    println!("Windows fixture profile: {details}");
    let target = home.path.join("cible été");
    let link = home.path.join("lien équipe");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("SKILL.md"), "protected content").unwrap();
    let linker = SystemPlatformLinker::current();
    linker.create_dir_link(&target, &link).unwrap();
    assert_eq!(
        fs::read(link.join("SKILL.md")).unwrap(),
        b"protected content"
    );
    linker.remove_link(&link, linker.link_kind()).unwrap();
    assert_eq!(
        fs::read(target.join("SKILL.md")).unwrap(),
        b"protected content"
    );
    assert!(!link.exists());
}

#[test]
fn broad_inheritance_is_removed_and_private_acl_survives_manifest_replacement() {
    let home = TestHome::new("broad-acl");
    let paths = home.paths();
    fs::create_dir(paths.skillreg_root()).unwrap();
    powershell(
        paths.skillreg_root(),
        r#"
$ErrorActionPreference = 'Stop'
Import-Module "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1"
$path = $env:SKILLREG_TEST_PATH
$acl = Get-Acl -LiteralPath $path
$rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
    [System.Security.Principal.SecurityIdentifier]::new('S-1-1-0'),
    'Modify', 'ContainerInherit, ObjectInherit', 'None', 'Allow')
$acl.AddAccessRule($rule)
Set-Acl -LiteralPath $path -AclObject $acl
"#,
    );
    let mut manifest = ManagedSkillsManifest::new();
    for active_org in [None, Some("acme".to_string())] {
        manifest.active_org = active_org;
        write_manifest_atomic(&paths, &manifest).unwrap();
        for path in [paths.skillreg_root().to_path_buf(), paths.manifest_path()] {
            powershell(
                &path,
                r#"
$ErrorActionPreference = 'Stop'
Import-Module "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1"
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$acl = Get-Acl -LiteralPath $env:SKILLREG_TEST_PATH
if (-not $acl.AreAccessRulesProtected) { throw 'Inheritance enabled' }
$rules = @($acl.GetAccessRules($true, $true, [System.Security.Principal.SecurityIdentifier]))
if ($rules.Count -ne 1 -or $rules[0].IdentityReference.Value -ne $sid -or
    $rules[0].AccessControlType -ne 'Allow' -or
    ($rules[0].FileSystemRights -band [System.Security.AccessControl.FileSystemRights]::FullControl) -ne
        [System.Security.AccessControl.FileSystemRights]::FullControl) { throw 'ACL is not private' }
"#,
            );
        }
    }
}

#[test]
fn hardening_refuses_a_foreign_junction_without_changing_its_manifest() {
    let home = TestHome::new("acl-foreign-target");
    let paths = home.paths();
    let target = home.path.join("external");
    fs::create_dir(&target).unwrap();
    let previous = b"preserve the previous manifest";
    fs::write(target.join("managed-skills.json"), previous).unwrap();
    let linker = SystemPlatformLinker::current();
    linker
        .create_dir_link(&target, paths.skillreg_root())
        .unwrap();
    assert!(restrict_to_current_user(paths.skillreg_root()).is_err());
    let result = write_manifest_atomic(&paths, &ManagedSkillsManifest::new());
    linker
        .remove_link(paths.skillreg_root(), linker.link_kind())
        .unwrap();
    assert_eq!(
        result.unwrap_err().code(),
        ManagedErrorCode::ManifestWriteFailed
    );
    assert_eq!(
        fs::read(target.join("managed-skills.json")).unwrap(),
        previous
    );
}

#[test]
fn denied_junction_creation_leaves_no_partial_binding_or_target_mutation() {
    let home = TestHome::new("junction-denied");
    let target = home.path.join("target");
    let parent = home.path.join("agent");
    let link = parent.join("review-helper");
    fs::create_dir(&target).unwrap();
    fs::create_dir(&parent).unwrap();
    fs::write(target.join("SKILL.md"), "protected content").unwrap();
    powershell(
        &parent,
        r#"
$ErrorActionPreference = 'Stop'
Import-Module "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1"
$path = $env:SKILLREG_TEST_PATH
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = Get-Acl -LiteralPath $path
$acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($sid, 'CreateDirectories', 'Deny'))
Set-Acl -LiteralPath $path -AclObject $acl
"#,
    );
    let result = SystemPlatformLinker::current().create_dir_link(&target, &link);
    restrict_to_current_user(&parent).unwrap();
    assert!(result.is_err());
    assert!(!link.exists());
    assert_eq!(
        fs::read(target.join("SKILL.md")).unwrap(),
        b"protected content"
    );
}
