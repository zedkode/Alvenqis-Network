# Unregister alvenqis-browser-host native messaging entries (Windows).

[CmdletBinding()]
param(
    [ValidateSet("Chrome", "Edge", "Brave", "All")]
    [string]$Browser = "All",

    [switch]$RemoveInstallDir
)

$ErrorActionPreference = "Stop"
$HostName = "com.alvenqis.browser_host"

$paths = @()
switch ($Browser) {
    "Chrome" { $paths = @("HKCU:\Software\Google\Chrome\NativeMessagingHosts\$HostName") }
    "Edge" { $paths = @("HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\$HostName") }
    "Brave" { $paths = @("HKCU:\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\$HostName") }
    "All" {
        $paths = @(
            "HKCU:\Software\Google\Chrome\NativeMessagingHosts\$HostName",
            "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\$HostName",
            "HKCU:\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\$HostName"
        )
    }
}

foreach ($p in $paths) {
    if (Test-Path $p) {
        Remove-Item -Path $p -Recurse -Force
        Write-Host "Removed $p"
    } else {
        Write-Host "Skip (missing): $p"
    }
}

if ($RemoveInstallDir) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Alvenqis\browser-host"
    if (Test-Path $InstallDir) {
        Remove-Item -Recurse -Force $InstallDir
        Write-Host "Removed $InstallDir"
    }
}

Write-Host "Unregister done."
