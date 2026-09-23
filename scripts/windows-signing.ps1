[CmdletBinding()]
param(
    [ValidateSet('sign', 'verify', 'verify-artifacts')]
    [string]$Mode = 'sign',
    [string]$FilePath,
    [string]$ArtifactDirectory,
    [string]$ApplicationPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-WindowsSigningSetting {
    param([string]$Name)
    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) { throw "Missing required setting: $Name" }
    return $value
}

function Resolve-WindowsSigningFile {
    param([string]$Path, [string]$ParameterName)
    if ([string]::IsNullOrWhiteSpace($Path)) { throw "Missing required parameter: $ParameterName" }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "File does not exist: $Path" }
    $file = Get-Item -LiteralPath $Path -Force
    if ($file.Length -eq 0) { throw "File is empty: $Path" }
    if (($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Symbolic links are not accepted: $Path"
    }
    return $file.FullName
}

function Invoke-WindowsSignTool {
    param([string[]]$Arguments)
    $tool = Resolve-WindowsSigningFile (Get-WindowsSigningSetting 'WINDOWS_SIGNTOOL_PATH') 'WINDOWS_SIGNTOOL_PATH'
    $PSNativeCommandUseErrorActionPreference = $false
    & $tool @Arguments | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "SignTool returned $LASTEXITCODE for $($Arguments[0]); warnings also block release."
    }
}

function Assert-WindowsFileSignature {
    param([string]$Path)
    $file = Resolve-WindowsSigningFile $Path 'FilePath'
    $subject = Get-WindowsSigningSetting 'WINDOWS_SIGNING_SUBJECT'
    Invoke-WindowsSignTool -Arguments @('verify', '/pa', '/all', '/tw', $file)
    $signature = Get-AuthenticodeSignature -LiteralPath $file
    if ($signature.Status -ne 'Valid') {
        throw "Authenticode status is $($signature.Status): $file"
    }
    if ($null -eq $signature.SignerCertificate) { throw "Authenticode signer certificate is missing: $file" }
    if (-not [string]::Equals($signature.SignerCertificate.Subject, $subject, [StringComparison]::Ordinal)) {
        throw "Authenticode publisher does not match WINDOWS_SIGNING_SUBJECT: $file"
    }
    if ($null -eq $signature.TimeStamperCertificate) { throw "Authenticode timestamp is missing: $file" }
}

function Assert-WindowsReleaseArtifacts {
    param([string]$Directory, [string]$Application)
    if ([string]::IsNullOrWhiteSpace($Directory) -or -not (Test-Path -LiteralPath $Directory -PathType Container)) {
        throw 'ArtifactDirectory must be an existing directory.'
    }
    $app = Resolve-WindowsSigningFile $Application 'ApplicationPath'
    Assert-WindowsFileSignature $app
    $selected = @{}
    foreach ($suffix in @('.exe', '.msi', '.nsis.zip')) {
        $files = @(Get-ChildItem -LiteralPath $Directory -File -Force | Where-Object {
            $_.Name.EndsWith($suffix, [StringComparison]::OrdinalIgnoreCase)
        })
        if ($files.Count -ne 1) { throw "Exactly one $suffix artifact is required; found $($files.Count)." }
        $selected[$suffix] = $files[0]
        Resolve-WindowsSigningFile $files[0].FullName 'Artifact' | Out-Null
    }
    $installer = $selected['.exe']
    Assert-WindowsFileSignature $installer.FullName
    Assert-WindowsFileSignature $selected['.msi'].FullName

    $temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) "skillreg-nsis-verify-$([guid]::NewGuid())"
    New-Item -ItemType Directory -Path $temporaryDirectory | Out-Null
    try {
        $archive = [IO.Compression.ZipFile]::OpenRead($selected['.nsis.zip'].FullName)
        try {
            if ($archive.Entries.Count -ne 1 -or $archive.Entries[0].FullName -cne $installer.Name) {
                throw "Archive must contain only the expected installer: $($installer.Name)"
            }
            $entry = $archive.Entries[0]
            if ($entry.Length -ne $installer.Length) { throw 'Updater archive differs from distributed installer.' }
            # Extract to a fixed path rather than trusting archive paths.
            $extracted = Join-Path $temporaryDirectory $installer.Name
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $extracted)
        } finally { $archive.Dispose() }
        $expectedHash = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA256).Hash
        $archiveHash = (Get-FileHash -LiteralPath $extracted -Algorithm SHA256).Hash
        if ($archiveHash -cne $expectedHash) { throw 'Updater archive differs from distributed installer.' }
        Assert-WindowsFileSignature $extracted
    } finally {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}

function Invoke-WindowsSigning {
    param(
        [ValidateSet('sign', 'verify', 'verify-artifacts')][string]$Mode,
        [string]$FilePath,
        [string]$ArtifactDirectory,
        [string]$ApplicationPath
    )
    switch ($Mode) {
        'sign' {
            $file = Resolve-WindowsSigningFile $FilePath 'FilePath'
            Get-WindowsSigningSetting 'WINDOWS_SIGNING_SUBJECT' | Out-Null
            $dlib = Resolve-WindowsSigningFile (Get-WindowsSigningSetting 'WINDOWS_SIGNING_DLIB') 'WINDOWS_SIGNING_DLIB'
            $metadata = Resolve-WindowsSigningFile (Get-WindowsSigningSetting 'WINDOWS_SIGNING_METADATA') 'WINDOWS_SIGNING_METADATA'
            Invoke-WindowsSignTool -Arguments @(
                'sign', '/fd', 'SHA256', '/tr', 'http://timestamp.acs.microsoft.com', '/td', 'SHA256',
                '/dlib', $dlib, '/dmdf', $metadata, $file
            )
            Assert-WindowsFileSignature $file
        }
        'verify' { Assert-WindowsFileSignature $FilePath }
        'verify-artifacts' { Assert-WindowsReleaseArtifacts $ArtifactDirectory $ApplicationPath }
    }
}

if ($MyInvocation.InvocationName -ne '.') {
    try {
        Invoke-WindowsSigning -Mode $Mode -FilePath $FilePath -ArtifactDirectory $ArtifactDirectory -ApplicationPath $ApplicationPath
    } catch {
        Write-Error -Message $_.Exception.Message -ErrorAction Continue
        exit 1
    }
}
