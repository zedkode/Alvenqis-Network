# Register a Windows Scheduled Task that probes Mainnet Candidate health.
#
# Usage:
#   .\scripts\browser\register-health-task.ps1
#   .\scripts\browser\register-health-task.ps1 -IntervalMinutes 15 -Strict
#   .\scripts\browser\register-health-task.ps1 -MaxIndexerLag 2
#   .\scripts\browser\register-health-task.ps1 -WebhookUrl $env:ALVENQIS_HEALTH_WEBHOOK_URL
#   .\scripts\browser\register-health-task.ps1 -Unregister
#
# Logs: %LOCALAPPDATA%\Alvenqis\health\

[CmdletBinding()]
param(
    [string]$TaskName = "AlvenqisCandidateChainHealth",
    [int]$IntervalMinutes = 30,
    [switch]$Strict,
    [Nullable[uint64]]$MaxIndexerLag = $null,
    [string]$WebhookUrl = "",
    [string]$Rpc = "",
    [switch]$Unregister
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$ProbeScript = Join-Path $RepoRoot "scripts\browser\probe-chain.ps1"
if (-not (Test-Path $ProbeScript)) {
    throw "Missing probe script: $ProbeScript"
}

if ($Unregister) {
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "Unregistered task: $TaskName"
    return
}

if ($IntervalMinutes -lt 5) {
    Write-Host "Clamping interval to 5 minutes minimum." -ForegroundColor Yellow
    $IntervalMinutes = 5
}

if ([string]::IsNullOrWhiteSpace($WebhookUrl) -and $env:ALVENQIS_HEALTH_WEBHOOK_URL) {
    $WebhookUrl = $env:ALVENQIS_HEALTH_WEBHOOK_URL
}

$LogDir = Join-Path $env:LOCALAPPDATA "Alvenqis\health"
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

# Persistent wrapper invoked by the task (keeps logs + exit code).
$Wrapper = Join-Path $LogDir "run-probe.ps1"
$probeEsc = $ProbeScript.Replace("'", "''")
$logEsc = $LogDir.Replace("'", "''")

$extra = @()
if ($Strict) { $extra += "-Strict" }
if ($null -ne $MaxIndexerLag) { $extra += "-MaxIndexerLag $MaxIndexerLag" }
if ($Rpc) { $extra += "-Rpc '$($Rpc.Replace("'","''"))'" }
if ($WebhookUrl) { $extra += "-WebhookUrl '$($WebhookUrl.Replace("'","''"))'" }
$extraLine = ($extra -join " ")

$WrapperBody = @"
`$ErrorActionPreference = 'Continue'
`$logDir = '$logEsc'
`$probe = '$probeEsc'
`$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
`$summary = Join-Path `$logDir ("probe-`$stamp.log")
`$outLog = Join-Path `$logDir ("probe-`$stamp-out.log")
`$errLog = Join-Path `$logDir ("probe-`$stamp-err.log")
"=== `$(Get-Date -Format o) user=$env:USERNAME host=$env:COMPUTERNAME ===" | Set-Content -Path `$summary -Encoding UTF8
`$argList = @(
  '-NoProfile',
  '-ExecutionPolicy', 'Bypass',
  '-File', `$probe,
  '-Quiet'
  $(if ($Strict) { ", '-Strict'" } else { "" })
  $(if ($null -ne $MaxIndexerLag) { ", '-MaxIndexerLag', '$MaxIndexerLag'" } else { "" })
  $(if ($Rpc) { ", '-Rpc', '$($Rpc.Replace("'","''"))'" } else { "" })
  $(if ($WebhookUrl) { ", '-WebhookUrl', '$($WebhookUrl.Replace("'","''"))'" } else { "" })
)
try {
  `$p = Start-Process -FilePath 'powershell.exe' ``
    -ArgumentList `$argList ``
    -Wait -PassThru -NoNewWindow ``
    -RedirectStandardOutput `$outLog ``
    -RedirectStandardError `$errLog
  "exit=`$(`$p.ExitCode)" | Add-Content -Path `$summary
  if (Test-Path `$outLog) { Get-Content `$outLog -ErrorAction SilentlyContinue | Add-Content -Path `$summary }
  if (Test-Path `$errLog) { Get-Content `$errLog -ErrorAction SilentlyContinue | Add-Content -Path `$summary }
  exit `$p.ExitCode
} catch {
  `$_ | Add-Content -Path `$summary
  exit 1
}
"@
# Build wrapper without broken nested expansion — write a clean fixed script:
$WrapperBody = @"
param()
`$ErrorActionPreference = 'Continue'
`$logDir = '$logEsc'
`$probe = '$probeEsc'
`$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
`$summary = Join-Path `$logDir ("probe-`$stamp.log")
`$outLog = Join-Path `$logDir ("probe-`$stamp-out.log")
`$errLog = Join-Path `$logDir ("probe-`$stamp-err.log")
"=== `$(Get-Date -Format o) ===" | Set-Content -Path `$summary -Encoding UTF8
`$args = New-Object System.Collections.Generic.List[string]
`$args.Add('-NoProfile') | Out-Null
`$args.Add('-ExecutionPolicy') | Out-Null
`$args.Add('Bypass') | Out-Null
`$args.Add('-File') | Out-Null
`$args.Add(`$probe) | Out-Null
`$args.Add('-Quiet') | Out-Null
"@

if ($Strict) {
    $WrapperBody += "`n`$args.Add('-Strict') | Out-Null`n"
}
if ($null -ne $MaxIndexerLag) {
    $WrapperBody += "`n`$args.Add('-MaxIndexerLag') | Out-Null`n`$args.Add('$MaxIndexerLag') | Out-Null`n"
}
if ($Rpc) {
    $rpcEsc = $Rpc.Replace("'", "''")
    $WrapperBody += "`n`$args.Add('-Rpc') | Out-Null`n`$args.Add('$rpcEsc') | Out-Null`n"
}
if ($WebhookUrl) {
    $whEsc = $WebhookUrl.Replace("'", "''")
    $WrapperBody += "`n`$args.Add('-WebhookUrl') | Out-Null`n`$args.Add('$whEsc') | Out-Null`n"
}

$WrapperBody += @"

try {
  `$p = Start-Process -FilePath 'powershell.exe' -ArgumentList `$args.ToArray() -Wait -PassThru -NoNewWindow -RedirectStandardOutput `$outLog -RedirectStandardError `$errLog
  "exit=`$(`$p.ExitCode)" | Add-Content -Path `$summary
  if (Test-Path `$outLog) { Get-Content `$outLog -ErrorAction SilentlyContinue | Add-Content -Path `$summary }
  if (Test-Path `$errLog) { Get-Content `$errLog -ErrorAction SilentlyContinue | Add-Content -Path `$summary }
  exit `$p.ExitCode
} catch {
  `$_ | Add-Content -Path `$summary
  exit 1
}
"@

Set-Content -Path $Wrapper -Value $WrapperBody -Encoding UTF8

$action = New-ScheduledTaskAction `
    -Execute "powershell.exe" `
    -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$Wrapper`""

$trigger = New-ScheduledTaskTrigger `
    -Once `
    -At (Get-Date).AddMinutes(1) `
    -RepetitionInterval (New-TimeSpan -Minutes $IntervalMinutes) `
    -RepetitionDuration (New-TimeSpan -Days 3650)

$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -MultipleInstances IgnoreNew

$principal = New-ScheduledTaskPrincipal `
    -UserId $env:USERNAME `
    -LogonType Interactive `
    -RunLevel Limited

Register-ScheduledTask `
    -TaskName $TaskName `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -Principal $principal `
    -Force | Out-Null

Write-Host "Registered scheduled task: $TaskName" -ForegroundColor Green
Write-Host "  Interval : every $IntervalMinutes minutes (first run ~1 min from now)"
Write-Host "  Wrapper  : $Wrapper"
Write-Host "  Logs     : $LogDir"
Write-Host "  Strict   : $Strict"
if ($null -ne $MaxIndexerLag) { Write-Host "  MaxLag   : $MaxIndexerLag" }
if ($WebhookUrl) { Write-Host "  Webhook  : set" }
Write-Host ""
Write-Host "Manage:"
Write-Host "  Get-ScheduledTask -TaskName '$TaskName' | Format-List"
Write-Host "  Start-ScheduledTask -TaskName '$TaskName'"
Write-Host "  Get-ChildItem `"$LogDir`" | Sort-Object LastWriteTime -Descending | Select-Object -First 5"
Write-Host "  .\scripts\browser\register-health-task.ps1 -Unregister"
