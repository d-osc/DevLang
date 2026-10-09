param(
    [string]$Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\DevLang'),
    [switch]$Uninstall,
    [switch]$NoPath,
    [switch]$NoRegistration
)
$ErrorActionPreference = 'Stop'
$Prefix = [IO.Path]::GetFullPath($Prefix)
$markerPath = Join-Path $Prefix '.devlang-install.json'
$registryKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\DevLang'
$previous = $null
function Notify-Environment {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DevLangEnvironment {
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr SendMessageTimeout(IntPtr window, uint message, UIntPtr wParam, string lParam, uint flags, uint timeout, out UIntPtr result);
}
'@
    $result = [UIntPtr]::Zero
    [void][DevLangEnvironment]::SendMessageTimeout([IntPtr]0xffff, 0x001a, [UIntPtr]::Zero, 'Environment', 2, 2000, [ref]$result)
}
if (Test-Path -LiteralPath $markerPath) {
    $previous = Get-Content -LiteralPath $markerPath -Raw | ConvertFrom-Json
    if ($previous.product -ne 'DevLang' -or $previous.prefix -ne $Prefix) { throw 'Invalid installation marker' }
}
function Remove-OwnedFiles($record) {
    $directories = @{}
    foreach ($name in $record.files) {
        $file = [IO.Path]::GetFullPath((Join-Path $Prefix $name))
        if (-not $file.StartsWith($Prefix + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid manifest path' }
        if (Test-Path -LiteralPath $file -PathType Leaf) { Remove-Item -LiteralPath $file -Force }
        $directory = [IO.Path]::GetDirectoryName($file)
        while ($directory -and $directory.StartsWith($Prefix + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            $directories[$directory] = $true
            $directory = [IO.Path]::GetDirectoryName($directory)
        }
    }
    foreach ($directory in @($directories.Keys | Sort-Object -Property Length -Descending)) {
        if ((Test-Path -LiteralPath $directory -PathType Container) -and @(Get-ChildItem -LiteralPath $directory -Force).Count -eq 0) {
            Remove-Item -LiteralPath $directory -Force
        }
    }
}
if ($Uninstall) {
    if (-not $previous) { throw 'No managed DevLang installation at this location' }
    if ($previous.pathAdded) {
        $parts = @([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | Where-Object { $_ -and $_.TrimEnd('\') -ine $Prefix.TrimEnd('\') })
        [Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'User')
        Notify-Environment
    }
    if ($previous.registered -and (Test-Path $registryKey)) {
        if ((Get-ItemProperty $registryKey).InstallLocation -eq $Prefix) { Remove-Item -LiteralPath $registryKey }
    }
    Remove-OwnedFiles $previous
    Remove-Item -LiteralPath $markerPath -Force
    if (@(Get-ChildItem -LiteralPath $Prefix -Force).Count -eq 0) { Remove-Item -LiteralPath $Prefix -Force }
    Write-Host "DevLang removed. Other files in $Prefix were preserved. Open a new terminal."
    exit 0
}
if ((Test-Path -LiteralPath $Prefix) -and -not $previous -and @(Get-ChildItem -LiteralPath $Prefix -Force).Count -gt 0) { throw 'Destination is not a managed DevLang installation; choose an empty directory' }
if (-not $NoRegistration -and (Test-Path $registryKey) -and (Get-ItemProperty $registryKey).InstallLocation -ne $Prefix) { throw 'Another DevLang installation is registered; use -NoRegistration for a separate copy' }
$payload = Join-Path $PSScriptRoot 'payload.zip'
if (-not (Test-Path -LiteralPath $payload)) { throw 'payload.zip must be next to install.ps1' }
$stage = Join-Path ([IO.Path]::GetTempPath()) ('devlang-install-' + [guid]::NewGuid().ToString('N'))
try {
    Expand-Archive -LiteralPath $payload -DestinationPath $stage
    foreach ($line in Get-Content -LiteralPath (Join-Path $stage 'SHA256SUMS')) {
        $digest, $name = $line -split '  ', 2
        $file = [IO.Path]::GetFullPath((Join-Path $stage $name))
        if (-not $file.StartsWith($stage + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid checksum path' }
        $stream = [IO.File]::OpenRead($file)
        $sha = [Security.Cryptography.SHA256]::Create()
        try { $actual = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '') }
        finally { $stream.Dispose(); $sha.Dispose() }
        if ($actual -ine $digest) { throw "Checksum mismatch: $name" }
    }
    $files = @(Get-ChildItem -LiteralPath $stage -Recurse -File | ForEach-Object { $_.FullName.Substring($stage.Length + 1) })
    # Preserve files created by users; refuse to overwrite unowned paths.
    foreach ($name in $files) {
        if ((Test-Path -LiteralPath (Join-Path $Prefix $name)) -and (-not $previous -or $previous.files -notcontains $name)) { throw "Unowned destination file: $name" }
    }
    New-Item -ItemType Directory -Path $Prefix -Force | Out-Null
    if ($previous) { Remove-OwnedFiles $previous }
    Copy-Item -Path (Join-Path $stage '*') -Destination $Prefix -Recurse -Force
    Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $Prefix 'uninstall.ps1') -Force
    $files += 'uninstall.ps1'
    $pathAdded = $previous -and $previous.pathAdded
    if (-not $NoPath) {
        $userPath = [string][Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not @($userPath -split ';' | Where-Object { $_.TrimEnd('\') -ieq $Prefix.TrimEnd('\') }).Count) {
            [Environment]::SetEnvironmentVariable('Path', (($userPath.TrimEnd(';') + ';' + $Prefix).TrimStart(';')), 'User')
            Notify-Environment
            $pathAdded = $true
        }
    }
    $registered = $previous -and $previous.registered
    if (-not $NoRegistration) {
        if ((Test-Path $registryKey) -and (Get-ItemProperty $registryKey).InstallLocation -ne $Prefix) { throw 'Another DevLang installation is registered; use -NoRegistration for a separate copy' }
        New-Item -Path $registryKey -Force | Out-Null
        $command = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "' + (Join-Path $Prefix 'uninstall.ps1') + '" -Uninstall -Prefix "' + $Prefix + '"'
        foreach ($entry in @{DisplayName='DevLang'; DisplayVersion='@VERSION@'; Publisher='d-osc'; InstallLocation=$Prefix; UninstallString=$command; DisplayIcon=(Join-Path $Prefix 'd.exe')}.GetEnumerator()) {
            New-ItemProperty -Path $registryKey -Name $entry.Key -Value $entry.Value -PropertyType String -Force | Out-Null
        }
        $registered = $true
    }
    @{product='DevLang'; version='@VERSION@'; prefix=$Prefix; files=$files; pathAdded=[bool]$pathAdded; registered=[bool]$registered} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $markerPath -Encoding UTF8
    Write-Host "Installed DevLang @VERSION@ in $Prefix. Open a new terminal and run: d --help"
} finally {
    if ([IO.Path]::GetFullPath($stage).StartsWith([IO.Path]::GetTempPath(), [StringComparison]::OrdinalIgnoreCase)) { Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue }
}
