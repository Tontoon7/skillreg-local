[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$wrapper = Join-Path $PSScriptRoot 'windows-signing.ps1'
. $wrapper

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Throws {
    param([scriptblock]$Action, [string]$Message)
    $caught = $false
    try { & $Action } catch {
        $caught = $true
        Assert-True ($_.Exception.Message -like "*$Message*") "Unexpected error: $($_.Exception.Message)"
    }
    Assert-True $caught "Expected failure: $Message"
}

function New-TestZip {
    param([string]$Path, [hashtable]$Entries)
    if (Test-Path -LiteralPath $Path) { Remove-Item -LiteralPath $Path }
    $zip = [IO.Compression.ZipFile]::Open($Path, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $Entries.Keys) {
            $entry = $zip.CreateEntry($name)
            $writer = [IO.StreamWriter]::new($entry.Open())
            try { $writer.Write($Entries[$name]) } finally { $writer.Dispose() }
        }
    } finally { $zip.Dispose() }
}

$testRoot = Join-Path ([IO.Path]::GetTempPath()) "skillreg windows tests $([guid]::NewGuid())"
$originalTool = $env:WINDOWS_SIGNTOOL_PATH
$originalSubject = $env:WINDOWS_SIGNING_SUBJECT
$originalDlib = $env:WINDOWS_SIGNING_DLIB
$originalMetadata = $env:WINDOWS_SIGNING_METADATA
New-Item -ItemType Directory -Path $testRoot | Out-Null

try {
    $tool = Join-Path $testRoot 'fake signtool.ps1'
    @'
$global:WindowsSignToolCalls.Add(@($args))
$global:LASTEXITCODE = if ($args[0] -eq 'verify') { $global:WindowsTestVerifyExitCode } else { $global:WindowsTestExitCode }
'@ | Set-Content -LiteralPath $tool
    $global:WindowsSignToolCalls = [Collections.Generic.List[object]]::new()
    $global:WindowsTestExitCode = 0
    $global:WindowsTestVerifyExitCode = 0
    $env:WINDOWS_SIGNTOOL_PATH = $tool
    $env:WINDOWS_SIGNING_SUBJECT = 'CN=SkillReg test publisher, O=Test'
    $env:WINDOWS_SIGNING_DLIB = Join-Path $testRoot 'client dlib.dll'
    $env:WINDOWS_SIGNING_METADATA = Join-Path $testRoot 'signing metadata.json'
    'test DLL' | Set-Content -LiteralPath $env:WINDOWS_SIGNING_DLIB
    '{}' | Set-Content -LiteralPath $env:WINDOWS_SIGNING_METADATA
    $app = Join-Path $testRoot 'application with spaces.exe'
    'application' | Set-Content -LiteralPath $app
    $script:signature = [pscustomobject]@{
        Status = 'Valid'
        SignerCertificate = [pscustomobject]@{ Subject = $env:WINDOWS_SIGNING_SUBJECT }
        TimeStamperCertificate = [pscustomobject]@{ Subject = 'CN=Test timestamp' }
    }
    function Get-AuthenticodeSignature {
        param([string]$LiteralPath)
        Assert-True (Test-Path -LiteralPath $LiteralPath -PathType Leaf) 'Signature target must exist'
        return $script:signature
    }

    Invoke-WindowsSigning -Mode sign -FilePath $app
    Assert-True ($global:WindowsSignToolCalls.Count -eq 2) 'Sign must be followed by verify'
    $signArgs = $global:WindowsSignToolCalls[0]
    $verifyArgs = $global:WindowsSignToolCalls[1]
    Assert-True ($signArgs[-1] -ceq $app) 'Spaces in target path must be preserved'
    Assert-True ($signArgs[1] -eq '/fd' -and $signArgs[2] -eq 'SHA256') 'SHA256 file digest required'
    Assert-True ($signArgs[3] -eq '/tr' -and $signArgs[5] -eq '/td' -and $signArgs[6] -eq 'SHA256') 'RFC 3161 SHA256 timestamp required'
    Assert-True ($signArgs[8] -ceq $env:WINDOWS_SIGNING_DLIB) 'Dlib argument must preserve spaces'
    Assert-True ($signArgs[10] -ceq $env:WINDOWS_SIGNING_METADATA) 'Metadata argument must preserve spaces'
    Assert-True (($verifyArgs[0..3] -join ' ') -eq 'verify /pa /all /tw') 'All signatures and timestamps must be verified'

    foreach ($code in @(1, 2)) {
        $global:WindowsTestExitCode = $code
        $global:WindowsTestVerifyExitCode = $code
        Assert-Throws { Invoke-WindowsSigning -Mode sign -FilePath $app } "SignTool returned $code"
        Assert-Throws { Invoke-WindowsSigning -Mode verify -FilePath $app } "SignTool returned $code"
    }
    $global:WindowsTestExitCode = 0
    Assert-Throws { Invoke-WindowsSigning -Mode sign -FilePath $app } 'SignTool returned 2'
    $global:WindowsTestVerifyExitCode = 0
    foreach ($variable in @('WINDOWS_SIGNTOOL_PATH', 'WINDOWS_SIGNING_SUBJECT', 'WINDOWS_SIGNING_DLIB', 'WINDOWS_SIGNING_METADATA')) {
        $saved = [Environment]::GetEnvironmentVariable($variable)
        [Environment]::SetEnvironmentVariable($variable, '')
        Assert-Throws { Invoke-WindowsSigning -Mode sign -FilePath $app } $variable
        [Environment]::SetEnvironmentVariable($variable, $saved)
    }
    Assert-Throws { Invoke-WindowsSigning -Mode verify -FilePath '' } 'FilePath'
    Assert-Throws { Invoke-WindowsSigning -Mode verify -FilePath (Join-Path $testRoot 'absent.exe') } 'File does not exist'
    foreach ($status in @('NotSigned', 'HashMismatch', 'NotTrusted', 'UnknownError')) {
        $script:signature.Status = $status
        Assert-Throws { Invoke-WindowsSigning -Mode verify -FilePath $app } 'Authenticode status'
    }
    $script:signature.Status = 'Valid'
    $script:signature.SignerCertificate.Subject = 'CN=Wrong publisher'
    Assert-Throws { Invoke-WindowsSigning -Mode verify -FilePath $app } 'publisher does not match'
    $script:signature.SignerCertificate.Subject = $env:WINDOWS_SIGNING_SUBJECT
    $timestamp = $script:signature.TimeStamperCertificate
    $script:signature.TimeStamperCertificate = $null
    Assert-Throws { Invoke-WindowsSigning -Mode verify -FilePath $app } 'timestamp is missing'
    $script:signature.TimeStamperCertificate = $timestamp
    $signer = $script:signature.SignerCertificate
    $script:signature.SignerCertificate = $null
    Assert-Throws { Invoke-WindowsSigning -Mode verify -FilePath $app } 'signer certificate is missing'
    $script:signature.SignerCertificate = $signer

    $assets = Join-Path $testRoot 'collected artifacts'
    New-Item -ItemType Directory -Path $assets | Out-Null
    $installerName = 'SkillReg_1.0.0_x64-setup.exe'
    $installer = Join-Path $assets $installerName
    [IO.File]::WriteAllText($installer, 'installer')
    [IO.File]::WriteAllText((Join-Path $assets 'SkillReg_1.0.0_x64_en-US.msi'), 'msi')
    $archive = Join-Path $assets 'SkillReg_1.0.0_x64-setup.nsis.zip'
    New-TestZip $archive @{ $installerName = 'installer' }
    Invoke-WindowsSigning -Mode verify-artifacts -ArtifactDirectory $assets -ApplicationPath $app
    Assert-Throws { Invoke-WindowsSigning -Mode verify-artifacts -ArtifactDirectory $assets } 'ApplicationPath'
    foreach ($entries in @(@{}, @{ 'first.exe' = 'installer'; 'second.exe' = 'installer' }, @{ '../escape.exe' = 'installer' }, @{ 'wrong-name.exe' = 'installer' })) {
        New-TestZip $archive $entries
        Assert-Throws { Invoke-WindowsSigning -Mode verify-artifacts -ArtifactDirectory $assets -ApplicationPath $app } 'Archive must contain only'
    }
    New-TestZip $archive @{ $installerName = 'tampered' }
    Assert-Throws { Invoke-WindowsSigning -Mode verify-artifacts -ArtifactDirectory $assets -ApplicationPath $app } 'differs from distributed installer'
    New-TestZip $archive @{ $installerName = 'installer' }
    Copy-Item -LiteralPath $installer -Destination (Join-Path $assets 'duplicate.exe')
    Assert-Throws { Invoke-WindowsSigning -Mode verify-artifacts -ArtifactDirectory $assets -ApplicationPath $app } 'Exactly one .exe'
    Remove-Item -LiteralPath (Join-Path $assets 'duplicate.exe')
    Remove-Item -LiteralPath $archive
    Assert-Throws { Invoke-WindowsSigning -Mode verify-artifacts -ArtifactDirectory $assets -ApplicationPath $app } 'Exactly one .nsis.zip'
    Write-Host 'Windows signing contract tests passed (local doubles).'

    Remove-Item Function:\Get-AuthenticodeSignature
    if (-not $IsWindows) { throw 'Native signature rejection test requires Windows and SignTool.' }
    $env:WINDOWS_SIGNTOOL_PATH = $originalTool
    Assert-True (-not [string]::IsNullOrWhiteSpace($env:WINDOWS_SIGNTOOL_PATH)) 'WINDOWS_SIGNTOOL_PATH is required for native tests'
    $unsigned = Join-Path $testRoot 'unsigned executable.exe'
    Copy-Item -LiteralPath (Join-Path $PSHOME 'pwsh.exe') -Destination $unsigned
    & $env:WINDOWS_SIGNTOOL_PATH remove /s $unsigned
    Assert-True ($LASTEXITCODE -eq 0) 'Preparing native unsigned executable failed'
    Assert-True ((Get-AuthenticodeSignature -LiteralPath $unsigned).Status -eq 'NotSigned') 'Native test fixture must be an unsigned PE file'
    Assert-Throws { Invoke-WindowsSigning -Mode verify -FilePath $unsigned } 'SignTool returned'
    $process = Start-Process -FilePath (Join-Path $PSHOME 'pwsh.exe') -ArgumentList @(
        '-NoProfile', '-File', "`"$wrapper`"", '-Mode', 'verify', '-FilePath', "`"$unsigned`""
    ) -Wait -PassThru -NoNewWindow
    Assert-True ($process.ExitCode -ne 0) 'CLI must propagate native verification failure'
    Write-Host 'Windows native unsigned PE rejection and CLI exit propagation passed.'
} finally {
    $env:WINDOWS_SIGNTOOL_PATH = $originalTool
    $env:WINDOWS_SIGNING_SUBJECT = $originalSubject
    $env:WINDOWS_SIGNING_DLIB = $originalDlib
    $env:WINDOWS_SIGNING_METADATA = $originalMetadata
    Remove-Item -LiteralPath $testRoot -Recurse -Force
    Remove-Variable -Name WindowsSignToolCalls, WindowsTestExitCode, WindowsTestVerifyExitCode -Scope Global -ErrorAction SilentlyContinue
}

exit 0
