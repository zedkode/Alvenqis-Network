[CmdletBinding()]
param(
    [ValidateSet("", "Start", "Restart", "UpdateVps", "BuildWindows", "BuildLinux", "BuildBoth", "Full", "Exit")]
    [string]$Action = "",
    [string]$Environment = "",
    [ValidateSet("Prompt", "Include", "Abort")]
    [string]$DirtyPolicy = "Prompt",
    [switch]$Yes,
    [switch]$DryRun,
    [string]$WslDistro = "",
    [ValidateRange(60, 3600)]
    [int]$DeployTimeoutSeconds = 600,
    [string]$ProfileDirectory = ""
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = "Stop"
$env:GIT_TERMINAL_PROMPT = "0"
$env:GCM_INTERACTIVE = "Never"

$script:RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$script:ActivityRoot = "D:\Blockchain-Core\Activity Startup"
$script:RunDirectory = $null
$script:RunLog = $null
$script:SummaryPath = $null
$script:ErrorStatePath = $null
$script:VpsChangesPath = $null
$script:Secrets = New-Object System.Collections.Generic.List[string]
$script:RecentLogLines = New-Object System.Collections.Generic.List[string]
$script:EndpointResults = New-Object System.Collections.Generic.List[object]
$script:EndpointFailures = New-Object System.Collections.Generic.List[string]
$script:DownloadLinks = New-Object System.Collections.Generic.List[string]
$script:BuildStates = [ordered]@{
    Windows = "not requested"
    Linux   = "not requested"
    VPS     = "not requested"
}
$script:CurrentStep = "startup"
$script:FailureStep = $null
$script:FailureExitCode = $null
$script:FailureHint = $null
$script:FailureOutput = @()
$script:OverallSucceeded = $false
$script:Cancelled = $false
$script:VpsUpdateRan = $false
$script:VpsBefore = $null
$script:VpsAfter = $null
$script:VpsJobReport = ""
$script:SelectedProfile = $null
$script:SelectedEnvironment = ""
$script:CandidateTag = ""
$script:VpsReleaseTag = ""
$script:BuildCommit = ""
$script:InitialDirty = @()
$script:IncludeLocalChanges = $false
$script:VerifiedVpsStatusUrl = ""

function ConvertTo-RedactedText {
    param([AllowNull()][object]$Value)

    $text = if ($null -eq $Value) { "" } else { [string]$Value }
    foreach ($secret in $script:Secrets) {
        if (-not [string]::IsNullOrWhiteSpace($secret)) {
            $text = $text.Replace($secret, "[REDACTED]")
        }
    }

    $patterns = @(
        '(?i)(authorization\s*:\s*(?:bearer|token)\s+)[^\s,;]+',
        '(?i)((?:setup[_-]?token|github[_-]?pat|api[_-]?key|password|secret)\s*[:=]\s*)[^\s,;"]+',
        '(?i)\bgh[pousr]_[A-Za-z0-9_]{20,}\b',
        '(?i)\bgithub_pat_[A-Za-z0-9_]{20,}\b',
        '(?i)([?&](?:token|access_token|api_key)=)[^&\s]+',
        '(?i)(?:[A-Za-z]:\\|/)[^\r\n"]*(?:\.ssh[\\/][^\s"]+|id_rsa|id_ed25519|[^\\/\s"]+\.pem)\b',
        '(?s)-----BEGIN [^-]*PRIVATE KEY-----.*?-----END [^-]*PRIVATE KEY-----'
    )
    foreach ($pattern in $patterns) {
        $text = [regex]::Replace($text, $pattern, {
            param($match)
            if ($match.Groups.Count -gt 1 -and $match.Groups[1].Success) {
                return $match.Groups[1].Value + "[REDACTED]"
            }
            return "[REDACTED]"
        })
    }
    return $text
}

function Write-RunLine {
    param(
        [AllowNull()][object]$Message,
        [ValidateSet("INFO", "OK", "WARN", "ERROR", "OUTPUT")]
        [string]$Level = "INFO"
    )

    $safe = ConvertTo-RedactedText $Message
    $lines = @($safe -split "`r?`n")
    if ($lines.Count -eq 0) {
        $lines = @("")
    }
    foreach ($line in $lines) {
        $entry = "[{0}] [{1}] {2}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"), $Level, $line
        if ($script:RunLog) {
            Add-Content -LiteralPath $script:RunLog -Value $entry -Encoding UTF8
        }
        $script:RecentLogLines.Add($entry)
        while ($script:RecentLogLines.Count -gt 500) {
            $script:RecentLogLines.RemoveAt(0)
        }
        switch ($Level) {
            "OK" { Write-Host $line -ForegroundColor Green }
            "WARN" { Write-Host $line -ForegroundColor Yellow }
            "ERROR" { Write-Host $line -ForegroundColor Red }
            default { Write-Host $line }
        }
    }
}

function Initialize-RunArtifacts {
    New-Item -ItemType Directory -Path $script:ActivityRoot -Force | Out-Null
    $stamp = Get-Date
    do {
        $candidate = Join-Path $script:ActivityRoot $stamp.ToString("yyyyMMdd-HHmmss")
        if (-not (Test-Path -LiteralPath $candidate)) {
            break
        }
        $stamp = $stamp.AddSeconds(1)
    } while ($true)

    $script:RunDirectory = $candidate
    New-Item -ItemType Directory -Path $script:RunDirectory | Out-Null
    $script:RunLog = Join-Path $script:RunDirectory "run.log"
    $script:SummaryPath = Join-Path $script:RunDirectory "summary.md"
    $script:ErrorStatePath = Join-Path $script:RunDirectory "error-state.md"
    $script:VpsChangesPath = Join-Path $script:RunDirectory "vps-changes.md"
    New-Item -ItemType File -Path $script:RunLog | Out-Null
}

function Set-Failure {
    param(
        [string]$Step,
        [int]$ExitCode = 1,
        [string]$Hint = "Check the last logged error first.",
        [object[]]$Output = @()
    )
    if (-not $script:FailureStep) {
        $script:FailureStep = $Step
        $script:FailureExitCode = $ExitCode
        $script:FailureHint = $Hint
        $script:FailureOutput = @($Output | Select-Object -Last 50)
    }
}

function Get-FailureHint {
    param([string]$Text, [string]$Step)
    if ($Text -match '(?i)wsl|distribution|distro|webkit2gtk|nvcc|cuda') {
        return "WSL2 Ubuntu or a required Linux build dependency is unavailable; check the first matching error in run.log."
    }
    if ($Text -match '(?i)timed out|timeout') {
        if ($Step -match '(?i)deploy|vps') {
            return "VPS deploy timed out; check the admin-server and Docker broker logs."
        }
        return "The external operation timed out; check the named command and network availability."
    }
    if ($Text -match '(?i)401|403|unauthori|forbidden|auth') {
        return "Authentication was rejected; verify the existing local enrollment credentials without copying them into logs."
    }
    if ($Text -match '(?i)release.gate|fmt|clippy|cargo test|quality gate') {
        return "A release gate failed; fix the first reported fmt, test, clippy, or explorer error before retrying."
    }
    if ($Text -match '(?i)working tree|uncommitted|branch|origin/main|rebase') {
        return "Repository state is not release-ready; resolve the reported branch or working-tree condition first."
    }
    return "Check the first error reported for this step in run.log."
}

function Invoke-ExternalCommand {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$ArgumentList = @(),
        [Parameter(Mandatory = $true)][string]$DisplayName,
        [switch]$AllowFailure,
        [switch]$SkipInDryRun,
        [string]$WorkingDirectory = $script:RepoRoot
    )

    Write-RunLine "COMMAND START: $DisplayName"
    if ($DryRun -and $SkipInDryRun) {
        Write-RunLine "DRY RUN: command not executed: $DisplayName" "WARN"
        return [pscustomobject]@{ ExitCode = 0; Output = @("DRY RUN") }
    }
    if (-not (Get-Command $FilePath -ErrorAction SilentlyContinue)) {
        $message = "Command is not installed or could not be found: $DisplayName"
        Write-RunLine $message "ERROR"
        if (-not $AllowFailure) {
            Set-Failure -Step $script:CurrentStep -ExitCode 127 -Hint (Get-FailureHint -Text $message -Step $script:CurrentStep) -Output @($message)
            throw $message
        }
        return [pscustomobject]@{ ExitCode = 127; Output = @($message) }
    }

    $oldLocation = Get-Location
    $oldErrorActionPreference = $ErrorActionPreference
    $output = New-Object System.Collections.Generic.List[string]
    try {
        Set-Location -LiteralPath $WorkingDirectory
        # Windows PowerShell 5.1 turns redirected native stderr into ErrorRecord
        # objects. Keep those records in the merged output without treating an
        # otherwise successful native command as a PowerShell exception.
        $ErrorActionPreference = "Continue"
        & $FilePath @ArgumentList 2>&1 | ForEach-Object {
            $line = ConvertTo-RedactedText $_
            $output.Add($line)
            Write-RunLine $line "OUTPUT"
        }
        $exitCode = $LASTEXITCODE
    }
    catch {
        $line = ConvertTo-RedactedText $_.Exception.Message
        $output.Add($line)
        Write-RunLine $line "ERROR"
        $exitCode = 1
    }
    finally {
        $ErrorActionPreference = $oldErrorActionPreference
        Set-Location -LiteralPath $oldLocation
    }

    Write-RunLine "COMMAND END: $DisplayName (exit $exitCode)" -Level $(if ($exitCode -eq 0) { "OK" } else { "ERROR" })
    if ($exitCode -ne 0 -and -not $AllowFailure) {
        $joined = ($output | Select-Object -Last 50) -join "`n"
        $hint = Get-FailureHint -Text $joined -Step $script:CurrentStep
        Set-Failure -Step $script:CurrentStep -ExitCode $exitCode -Hint $hint -Output $output
        throw "$DisplayName failed with exit code $exitCode."
    }
    return [pscustomobject]@{ ExitCode = $exitCode; Output = @($output) }
}

function Invoke-PowerShellScript {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory = $true)][string]$DisplayName,
        [switch]$SkipInDryRun
    )
    $powershell = (Get-Command powershell.exe -ErrorAction Stop).Source
    $commandArguments = @("-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $Path) + $Arguments
    return Invoke-ExternalCommand -FilePath $powershell -ArgumentList $commandArguments -DisplayName $DisplayName -SkipInDryRun:$SkipInDryRun
}

function Get-PropertyValue {
    param([AllowNull()][object]$Object, [string]$Name, [AllowNull()][object]$Default = $null)
    if ($null -eq $Object) { return $Default }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) { return $Default }
    return $property.Value
}

function Get-ConfiguredEnvironments {
    $configRoot = Join-Path $script:RepoRoot "Blockchain-prototype\configs"
    $names = New-Object System.Collections.Generic.HashSet[string] ([System.StringComparer]::OrdinalIgnoreCase)
    Get-ChildItem -LiteralPath $configRoot -File -Filter "*.toml" | ForEach-Object {
        foreach ($line in (Get-Content -LiteralPath $_.FullName)) {
            if ($line -match '^\s*network\s*=\s*"([^"]+)"') {
                [void]$names.Add($Matches[1])
            }
        }
    }
    return @($names | Sort-Object)
}

function Resolve-ProfilePath {
    param([string]$EnvironmentName)

    $explicitFile = [Environment]::GetEnvironmentVariable("ALVENQIS_OPERATOR_PROFILE_FILE")
    if (-not [string]::IsNullOrWhiteSpace($explicitFile)) {
        return $explicitFile
    }
    $directory = $ProfileDirectory
    if ([string]::IsNullOrWhiteSpace($directory)) {
        $directory = [Environment]::GetEnvironmentVariable("ALVENQIS_OPERATOR_PROFILE_DIR")
    }
    if ([string]::IsNullOrWhiteSpace($directory)) {
        $directory = Join-Path $env:LOCALAPPDATA "Alvenqis\Operator\profiles"
    }
    return Join-Path $directory "$EnvironmentName.json"
}

function Read-OperatorProfile {
    param([string]$EnvironmentName, [switch]$Optional)

    $profilePath = Resolve-ProfilePath -EnvironmentName $EnvironmentName
    if (-not (Test-Path -LiteralPath $profilePath -PathType Leaf)) {
        if ($Optional) { return $null }
        throw "No local operator profile is configured for environment '$EnvironmentName'."
    }

    try {
        $profile = Get-Content -LiteralPath $profilePath -Raw | ConvertFrom-Json
    }
    catch {
        throw "The local operator profile for '$EnvironmentName' is not valid JSON."
    }

    $profileEnvironment = [string](Get-PropertyValue $profile "environment" "")
    if ($profileEnvironment -ne $EnvironmentName) {
        throw "The local operator profile environment does not match '$EnvironmentName'."
    }
    $adminUrl = ([string](Get-PropertyValue $profile "adminServerUrl" "")).TrimEnd("/")
    $token = [string](Get-PropertyValue $profile "setupToken" "")
    if ($adminUrl -notmatch '^https://|^http://(?:127\.0\.0\.1|localhost)(?::\d+)?$') {
        throw "The operator profile must use HTTPS, except for an explicit localhost SSH tunnel."
    }
    if ([string]::IsNullOrWhiteSpace($token)) {
        throw "The local operator profile does not contain the existing setup credential."
    }
    $script:Secrets.Add($token)

    return [pscustomobject]@{
        Environment    = $EnvironmentName
        AdminServerUrl = $adminUrl
        SetupToken     = $token
        DeployPayload  = Get-PropertyValue $profile "deployPayload" $null
        Endpoints      = Get-PropertyValue $profile "endpoints" $null
    }
}

function Invoke-AdminRequest {
    param(
        [Parameter(Mandatory = $true)][object]$Profile,
        [Parameter(Mandatory = $true)][ValidateSet("GET", "POST")][string]$Method,
        [Parameter(Mandatory = $true)][string]$Path,
        [AllowNull()][object]$Body = $null,
        [int]$TimeoutSec = 30,
        [switch]$AllowFailure
    )

    $uri = "$($Profile.AdminServerUrl)$Path"
    $headers = @{ "X-Alvenqis-Setup-Token" = $Profile.SetupToken; "Accept" = "application/json" }
    $parameters = @{
        UseBasicParsing = $true
        Uri             = $uri
        Method          = $Method
        Headers         = $headers
        TimeoutSec      = $TimeoutSec
        ErrorAction     = "Stop"
    }
    if ($null -ne $Body) {
        $parameters.ContentType = "application/json"
        $parameters.Body = ($Body | ConvertTo-Json -Depth 30 -Compress)
    }

    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $response = Invoke-WebRequest @parameters
        $watch.Stop()
        $parsed = $null
        if (-not [string]::IsNullOrWhiteSpace($response.Content)) {
            $parsed = $response.Content | ConvertFrom-Json
        }
        return [pscustomobject]@{
            Success = $true
            StatusCode = [int]$response.StatusCode
            ElapsedMs = [int]$watch.ElapsedMilliseconds
            Data = $parsed
            Error = ""
        }
    }
    catch {
        $watch.Stop()
        $status = 0
        if ($_.Exception.Response -and $_.Exception.Response.StatusCode) {
            $status = [int]$_.Exception.Response.StatusCode
        }
        $message = ConvertTo-RedactedText $_.Exception.Message
        if ($AllowFailure) {
            return [pscustomobject]@{
                Success = $false
                StatusCode = $status
                ElapsedMs = [int]$watch.ElapsedMilliseconds
                Data = $null
                Error = $message
            }
        }
        throw "Admin API $Method $Path failed (HTTP $status): $message"
    }
}

function Show-VpsStartupStatus {
    param([string[]]$Environments)
    Write-RunLine "VPS health and stack status before changes:"
    foreach ($name in $Environments) {
        try {
            $profile = Read-OperatorProfile -EnvironmentName $name -Optional
            if ($null -eq $profile) {
                Write-RunLine "  ${name}: local operator profile not configured; authenticated stack status unavailable." "WARN"
                continue
            }
            $health = Invoke-AdminRequest -Profile $profile -Method GET -Path "/health" -TimeoutSec 10 -AllowFailure
            $stack = Invoke-AdminRequest -Profile $profile -Method GET -Path "/api/stack" -TimeoutSec 15 -AllowFailure
            if ($health.Success) {
                Write-RunLine "  $name admin health: HTTP $($health.StatusCode), $($health.ElapsedMs) ms" "OK"
            }
            else {
                Write-RunLine "  $name admin health: unavailable (HTTP $($health.StatusCode))" "WARN"
            }
            if ($stack.Success -and $stack.Data) {
                $services = @(Get-PropertyValue $stack.Data "services" @())
                Write-RunLine "  $name stack: $($services.Count) reported service(s)"
                foreach ($service in $services) {
                    $serviceName = [string](Get-PropertyValue $service "Service" (Get-PropertyValue $service "Name" "unknown"))
                    $image = [string](Get-PropertyValue $service "Image" "unknown")
                    $state = [string](Get-PropertyValue $service "State" (Get-PropertyValue $service "Status" "unknown"))
                    Write-RunLine "    $serviceName | $image | $state"
                }
            }
            else {
                Write-RunLine "  $name authenticated stack status unavailable (HTTP $($stack.StatusCode))." "WARN"
            }
        }
        catch {
            Write-RunLine "  $name status check failed: $($_.Exception.Message)" "WARN"
        }
    }
}

function Invoke-StartupChecks {
    $script:CurrentStep = "startup checks"
    Write-RunLine "Alvenqis release orchestrator started."
    Write-RunLine "Run artifacts: $script:RunDirectory"

    Invoke-ExternalCommand -FilePath "git" -ArgumentList @("fetch", "origin", "--prune") -DisplayName "git fetch origin --prune" | Out-Null
    $branchResult = Invoke-ExternalCommand -FilePath "git" -ArgumentList @("branch", "--show-current") -DisplayName "read current branch"
    $branch = ($branchResult.Output -join "`n").Trim()
    $countResult = Invoke-ExternalCommand -FilePath "git" -ArgumentList @("rev-list", "--left-right", "--count", "HEAD...refs/remotes/origin/main") -DisplayName "compare HEAD with origin/main"
    $countLine = @($countResult.Output | Where-Object { $_ -match '^\s*\d+\s+\d+\s*$' } | Select-Object -Last 1)
    $parts = if ($countLine.Count -gt 0) { ($countLine[0].Trim() -split '\s+') } else { @() }
    if ($parts.Count -lt 2) {
        throw "Could not parse ahead/behind counts for origin/main."
    }
    $ahead = [int]$parts[0]
    $behind = [int]$parts[1]
    Write-RunLine "Current branch: $branch"
    Write-RunLine "Compared with origin/main: ahead $ahead, behind $behind"

    Write-RunLine "Last 5 commits on origin/main:"
    Invoke-ExternalCommand -FilePath "git" -ArgumentList @(
        "log", "-5", "--date=iso-strict", "--pretty=format:%h | %an | %ad | %s", "refs/remotes/origin/main"
    ) -DisplayName "show origin/main short log" | Out-Null

    $statusResult = Invoke-ExternalCommand -FilePath "git" -ArgumentList @("status", "--porcelain") -DisplayName "show git status --porcelain"
    $script:InitialDirty = @($statusResult.Output | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($script:InitialDirty.Count -gt 0) {
        Write-RunLine "Uncommitted changes were found. Nothing will be committed or pushed unless explicitly selected below." "WARN"
        foreach ($line in $script:InitialDirty) {
            Write-RunLine "  $line" "WARN"
        }
    }
    else {
        Write-RunLine "Working tree is clean." "OK"
    }

    $script:BuildCommit = ((Invoke-ExternalCommand -FilePath "git" -ArgumentList @("rev-parse", "HEAD") -DisplayName "read build commit").Output -join "").Trim()
    $environments = @(Get-ConfiguredEnvironments)
    if ($environments.Count -eq 0) {
        throw "No network environment names could be read from Blockchain-prototype/configs."
    }
    Write-RunLine "Configured network environments: $($environments -join ', ')"
    Show-VpsStartupStatus -Environments $environments
    return $environments
}

function Read-Action {
    if (-not [string]::IsNullOrWhiteSpace($Action)) { return $Action }
    Write-Host ""
    Write-Host "[1] Start services"
    Write-Host "[2] Restart services"
    Write-Host "[3] Update VPS docker containers"
    Write-Host "[4] Build Windows app"
    Write-Host "[5] Build Linux app"
    Write-Host "[6] Build both"
    Write-Host "[7] Full pipeline (build both + release + VPS update)"
    Write-Host "[0] Exit"
    $choice = (Read-Host "Action").Trim()
    $map = @{
        "0" = "Exit"; "1" = "Start"; "2" = "Restart"; "3" = "UpdateVps"
        "4" = "BuildWindows"; "5" = "BuildLinux"; "6" = "BuildBoth"; "7" = "Full"
    }
    if (-not $map.ContainsKey($choice)) {
        throw "Invalid action '$choice'. Run the orchestrator again and choose 0-7."
    }
    return $map[$choice]
}

function Test-VpsAction {
    param([string]$SelectedAction)
    return $SelectedAction -in @("UpdateVps", "Full")
}

function Select-Environment {
    param([string[]]$Environments)
    if (-not [string]::IsNullOrWhiteSpace($Environment)) {
        if ($Environment -notin $Environments) {
            throw "Environment '$Environment' is not present in Blockchain-prototype/configs."
        }
        return $Environment
    }
    Write-Host ""
    Write-Host "VPS target environments read from Blockchain-prototype/configs:"
    for ($index = 0; $index -lt $Environments.Count; $index++) {
        Write-Host ("[{0}] {1}" -f ($index + 1), $Environments[$index])
    }
    $choice = (Read-Host "Target environment").Trim()
    $number = 0
    if (-not [int]::TryParse($choice, [ref]$number) -or $number -lt 1 -or $number -gt $Environments.Count) {
        throw "Invalid VPS target selection."
    }
    return $Environments[$number - 1]
}

function Resolve-DirtyDecision {
    if ($script:InitialDirty.Count -eq 0) { return $false }
    if ($DirtyPolicy -eq "Include") { return $true }
    if ($DirtyPolicy -eq "Abort") {
        $script:Cancelled = $true
        return $false
    }
    Write-Host ""
    Write-Host "Uncommitted changes exist."
    Write-Host "[I] Include them through the existing commit + push synchronization path"
    Write-Host "[A] Abort and handle git manually first (default)"
    $choice = (Read-Host "Dirty working tree [A]").Trim().ToLowerInvariant()
    if ($choice -in @("i", "include")) { return $true }
    $script:Cancelled = $true
    return $false
}

function Get-ActionSummary {
    param([string]$SelectedAction)
    switch ($SelectedAction) {
        "Start" { return "start local node, RPC, indexer refresh, and explorer" }
        "Restart" { return "restart local node, RPC, indexer refresh, and explorer" }
        "UpdateVps" { return "run release gates, publish VPS release/images, and deploy through the authenticated admin API" }
        "BuildWindows" { return "build the Windows Control Center" }
        "BuildLinux" { return "prepare the existing WSL2 Ubuntu host and build the Linux Control Center" }
        "BuildBoth" { return "build Windows and Linux Control Center packages" }
        "Full" { return "build Windows and Linux, publish/watch releases and images, then update the VPS through the admin API" }
        default { return $SelectedAction }
    }
}

function Confirm-Plan {
    param([string]$SelectedAction)
    Write-Host ""
    Write-Host "FINAL PLAN"
    Write-Host "  Action: $(Get-ActionSummary $SelectedAction)"
    Write-Host "  Build targets: $(switch ($SelectedAction) {
        'BuildWindows' { 'Windows' }
        'BuildLinux' { 'Linux' }
        'BuildBoth' { 'Windows, Linux' }
        'Full' { 'Windows, Linux, VPS' }
        'UpdateVps' { 'VPS' }
        default { 'none' }
    })"
    Write-Host "  VPS environment: $(if (Test-VpsAction $SelectedAction) { $script:SelectedEnvironment } else { 'none' })"
    Write-Host "  Push local changes: $(if ($script:IncludeLocalChanges) { 'yes, through sync-and-release-vps.ps1 -SyncOnly' } else { 'no' })"
    Write-Host "  Mode: $(if ($DryRun) { 'DRY RUN (execution commands are not run)' } else { 'LIVE' })"
    if ($Yes) { return $true }
    $answer = (Read-Host "Proceed? [y/N]").Trim().ToLowerInvariant()
    return $answer -in @("y", "yes")
}

function Invoke-LocalServices {
    param([ValidateSet("start", "restart")][string]$Command)
    $script:CurrentStep = "$Command local services"
    $localManager = Join-Path $script:RepoRoot "Blockchain-scripts\local\alvenqis-local.ps1"
    Invoke-PowerShellScript -Path $localManager -Arguments @($Command) -DisplayName "$Command local Alvenqis services" -SkipInDryRun | Out-Null
}

function Invoke-ReleaseGate {
    $script:CurrentStep = "release gate"
    $gate = Join-Path $script:RepoRoot "Blockchain-scripts\release\release-gate.ps1"
    Invoke-PowerShellScript -Path $gate -DisplayName "Alvenqis release-gate.ps1" -SkipInDryRun | Out-Null
}

function Invoke-WindowsBuild {
    $script:CurrentStep = "Windows build"
    $script:BuildStates.Windows = "running"
    try {
        $builder = Join-Path $script:RepoRoot "Blockchain-scripts\release\build-windows-installer.ps1"
        Invoke-PowerShellScript -Path $builder -DisplayName "Windows Control Center build" -SkipInDryRun | Out-Null
        $script:BuildStates.Windows = if ($DryRun) { "dry-run" } else { "pass" }
        if (-not $DryRun) {
            $artifactRoot = Join-Path $script:RepoRoot "release-artifacts"
            Get-ChildItem -LiteralPath $artifactRoot -File -ErrorAction SilentlyContinue | ForEach-Object {
                $script:DownloadLinks.Add($_.FullName)
            }
        }
    }
    catch {
        $script:BuildStates.Windows = "fail"
        throw
    }
}

function Get-WslDistributions {
    $result = Invoke-ExternalCommand -FilePath "wsl.exe" -ArgumentList @("--list", "--quiet") -DisplayName "list installed WSL distributions" -AllowFailure
    if ($result.ExitCode -ne 0) { return @() }
    return @(($result.Output -join "`n").Replace([char]0, "") -split "`r?`n" |
        ForEach-Object { $_.Trim() } |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function Invoke-LinuxBuild {
    $script:CurrentStep = "Linux build"
    $script:BuildStates.Linux = "running"
    try {
        $distros = @(Get-WslDistributions)
        if ($distros.Count -eq 0) {
            throw "WSL2 has no installed distribution. Install WSL2 and Ubuntu 24.04, then rerun."
        }
        $distro = $WslDistro
        if ([string]::IsNullOrWhiteSpace($distro)) {
            $distro = [string]($distros | Where-Object { $_ -match '(?i)ubuntu' } | Select-Object -First 1)
        }
        if ([string]::IsNullOrWhiteSpace($distro) -or $distro -notin $distros) {
            throw "The required Ubuntu WSL distribution is not installed. Install Ubuntu 24.04 or pass -WslDistro with an installed Ubuntu name."
        }
        if ($distro -notmatch '(?i)ubuntu') {
            throw "The documented Linux build host is WSL2 Ubuntu; selected distribution '$distro' is unsupported."
        }

        $pathResult = Invoke-ExternalCommand -FilePath "wsl.exe" -ArgumentList @("-d", $distro, "--", "wslpath", "-a", "-u", $script:RepoRoot) -DisplayName "resolve repository path inside WSL"
        $wslRoot = ($pathResult.Output -join "`n").Trim()
        if ([string]::IsNullOrWhiteSpace($wslRoot)) {
            throw "WSL could not resolve the repository path."
        }
        $userResult = Invoke-ExternalCommand -FilePath "wsl.exe" -ArgumentList @("-d", $distro, "--", "sh", "-lc", "id -un; printf '\n'; printf '%s' `"`$HOME`"") -DisplayName "inspect WSL build user"
        $userLines = @(($userResult.Output -join "`n") -split "`r?`n" | Where-Object { $_ -ne "" })
        $wslHome = if ($userLines.Count -ge 2) { $userLines[-1].Trim() } else { "/root" }

        $rootSetup = "$wslRoot/Blockchain-scripts/release/setup-wsl-linux-build-host.sh"
        $userSetup = "$wslRoot/Blockchain-scripts/release/wsl-linux-setup.sh"
        $builder = "$wslRoot/Blockchain-scripts/release/build-linux-desktop.sh"

        Invoke-ExternalCommand -FilePath "wsl.exe" -ArgumentList @("-d", $distro, "-u", "root", "--", "bash", $rootSetup) -DisplayName "WSL root Linux build-host setup" -SkipInDryRun | Out-Null
        Invoke-ExternalCommand -FilePath "wsl.exe" -ArgumentList @("-d", $distro, "-u", "root", "--", "env", "HOME=$wslHome", "bash", $userSetup) -DisplayName "WSL Linux desktop dependency setup" -SkipInDryRun | Out-Null
        Invoke-ExternalCommand -FilePath "wsl.exe" -ArgumentList @("-d", $distro, "--", "bash", $builder) -DisplayName "Linux Control Center build" -SkipInDryRun | Out-Null
        $script:BuildStates.Linux = if ($DryRun) { "dry-run" } else { "pass" }
        if (-not $DryRun) {
            $artifactRoot = Join-Path $script:RepoRoot "Blockchain-prototype\alvenqis-release\apps\linux"
            Get-ChildItem -LiteralPath $artifactRoot -File -ErrorAction SilentlyContinue | ForEach-Object {
                $script:DownloadLinks.Add($_.FullName)
            }
        }
    }
    catch {
        $script:BuildStates.Linux = "fail"
        if (-not $script:FailureStep) {
            $message = $_.Exception.Message
            Set-Failure -Step $script:CurrentStep -ExitCode 1 -Hint (Get-FailureHint -Text $message -Step $script:CurrentStep) -Output @($message)
        }
        throw
    }
}

function Invoke-SyncAndReleaseVps {
    param([switch]$SyncOnly)
    $script:CurrentStep = if ($SyncOnly) { "commit and push confirmed local changes" } else { "VPS release" }
    $sync = Join-Path $script:RepoRoot "Blockchain-scripts\github\sync-and-release-vps.ps1"
    $arguments = @()
    if ($SyncOnly) { $arguments += "-SyncOnly" }
    Invoke-PowerShellScript -Path $sync -Arguments $arguments -DisplayName $(if ($SyncOnly) { "conservative confirmed sync through existing VPS sync script" } else { "VPS tag/release through existing sync script" }) -SkipInDryRun | Out-Null
    if (-not $DryRun) {
        $script:BuildCommit = ((Invoke-ExternalCommand -FilePath "git" -ArgumentList @("rev-parse", "HEAD") -DisplayName "read synchronized commit").Output -join "").Trim()
    }
}

function Get-TagPointingAtHead {
    param([string]$Pattern)
    $result = Invoke-ExternalCommand -FilePath "git" -ArgumentList @("tag", "--points-at", "HEAD", "--list", $Pattern) -DisplayName "read release tag at HEAD"
    return @($result.Output | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1)
}

function Get-GhExecutable {
    $command = Get-Command gh.exe -ErrorAction SilentlyContinue
    if (-not $command) { $command = Get-Command gh -ErrorAction SilentlyContinue }
    if (-not $command) {
        throw "GitHub CLI is required. Install GitHub.cli and authenticate with gh auth login."
    }
    return $command.Source
}

function Wait-GitHubWorkflow {
    param(
        [string]$Workflow,
        [string]$HeadBranch,
        [datetime]$NotBefore = (Get-Date).AddMinutes(-2)
    )
    $gh = Get-GhExecutable
    $script:CurrentStep = "GitHub workflow $Workflow"
    $runId = ""
    for ($attempt = 0; $attempt -lt 30 -and [string]::IsNullOrWhiteSpace($runId); $attempt++) {
        if ($attempt -gt 0) { Start-Sleep -Seconds 2 }
        $result = Invoke-ExternalCommand -FilePath $gh -ArgumentList @(
            "run", "list", "--workflow", $Workflow, "--limit", "30",
            "--json", "databaseId,headBranch,createdAt,status,conclusion,url"
        ) -DisplayName "discover $Workflow run" -AllowFailure
        if ($result.ExitCode -ne 0) { continue }
        try {
            $runs = (($result.Output -join "`n") | ConvertFrom-Json)
            $matching = @($runs | Where-Object {
                ([string]::IsNullOrWhiteSpace($HeadBranch) -or $_.headBranch -eq $HeadBranch) -and
                ([datetime]$_.createdAt -ge $NotBefore.ToUniversalTime())
            } | Sort-Object { [datetime]$_.createdAt } -Descending | Select-Object -First 1)
            if ($matching.Count -gt 0) { $runId = [string]$matching[0].databaseId }
        }
        catch {
            Write-RunLine "Could not parse gh run list output for $Workflow." "WARN"
        }
    }
    if ([string]::IsNullOrWhiteSpace($runId)) {
        throw "GitHub Actions run for $Workflow was not discovered."
    }
    Invoke-ExternalCommand -FilePath $gh -ArgumentList @("run", "watch", $runId, "--exit-status") -DisplayName "watch $Workflow run $runId" -SkipInDryRun | Out-Null
}

function New-CandidateTag {
    $packagePath = Join-Path $script:RepoRoot "Blockchain-prototype\alvenqis-desktop-tauri\package.json"
    $version = [string]((Get-Content -LiteralPath $packagePath -Raw | ConvertFrom-Json).version)
    $version = $version -replace '[-+].*$', ''
    if ($version -notmatch '^\d+\.\d+\.\d+$') {
        throw "Desktop package version is not a candidate-release semantic version."
    }
    $remoteResult = Invoke-ExternalCommand -FilePath "git" -ArgumentList @("ls-remote", "--tags", "origin", "v$version-candidate.*") -DisplayName "read existing candidate tags"
    $maximum = 0
    foreach ($line in $remoteResult.Output) {
        if ($line -match "refs/tags/v$([regex]::Escape($version))-candidate\.(\d+)$") {
            $number = [int]$Matches[1]
            if ($number -gt $maximum) { $maximum = $number }
        }
    }
    return "v$version-candidate.$($maximum + 1)"
}

function Invoke-CandidateRelease {
    $script:CurrentStep = "candidate release"
    $gh = Get-GhExecutable
    Invoke-ExternalCommand -FilePath $gh -ArgumentList @("auth", "status", "--hostname", "github.com") -DisplayName "verify GitHub CLI authentication" | Out-Null
    $tag = New-CandidateTag
    $started = Get-Date
    Invoke-ExternalCommand -FilePath "git" -ArgumentList @("tag", "-a", $tag, "-m", "Alvenqis orchestrated candidate release $tag") -DisplayName "create candidate tag $tag" -SkipInDryRun | Out-Null
    try {
        Invoke-ExternalCommand -FilePath "git" -ArgumentList @("push", "origin", $tag) -DisplayName "push candidate tag $tag" -SkipInDryRun | Out-Null
    }
    catch {
        if (-not $DryRun) {
            Invoke-ExternalCommand -FilePath "git" -ArgumentList @("tag", "-d", $tag) -DisplayName "remove candidate tag created by failed push" -AllowFailure | Out-Null
        }
        throw
    }
    $script:CandidateTag = $tag
    if (-not $DryRun) {
        foreach ($workflow in @("candidate-windows-release.yml", "candidate-linux-release.yml", "candidate-vps-release.yml")) {
            Wait-GitHubWorkflow -Workflow $workflow -HeadBranch $tag -NotBefore $started
        }
    }
}

function Invoke-DockerImagesWorkflow {
    param([string]$Version)
    if ([string]::IsNullOrWhiteSpace($Version)) {
        throw "An immutable Docker image version is required."
    }
    $script:CurrentStep = "Docker control-plane images"
    $gh = Get-GhExecutable
    $started = Get-Date
    Invoke-ExternalCommand -FilePath $gh -ArgumentList @(
        "workflow", "run", "docker-control-plane-images.yml", "--ref", "main", "-f", "version=$Version"
    ) -DisplayName "trigger Docker control-plane images workflow" -SkipInDryRun | Out-Null
    if (-not $DryRun) {
        Wait-GitHubWorkflow -Workflow "docker-control-plane-images.yml" -HeadBranch "main" -NotBefore $started
    }
}

function Copy-DeployPayloadWithVersion {
    param([object]$Payload, [string]$Version)
    if ($null -eq $Payload) {
        throw "The selected local operator profile has no deployPayload; refusing to guess VPS configuration."
    }
    $copy = (($Payload | ConvertTo-Json -Depth 30) | ConvertFrom-Json)
    foreach ($required in @("base_domain", "node_name", "admin_email")) {
        if ([string]::IsNullOrWhiteSpace([string](Get-PropertyValue $copy $required ""))) {
            throw "The selected local operator profile deployPayload is missing required field '$required'."
        }
    }
    $copy | Add-Member -NotePropertyName "ALVENQIS_version" -NotePropertyValue $Version -Force
    return $copy
}

function Get-ServiceSnapshot {
    param([object]$StackData)
    $snapshot = [ordered]@{}
    foreach ($service in @(Get-PropertyValue $StackData "services" @())) {
        $name = [string](Get-PropertyValue $service "Service" (Get-PropertyValue $service "Name" "unknown"))
        $snapshot[$name] = [pscustomobject]@{
            Image = [string](Get-PropertyValue $service "Image" "unknown")
            State = [string](Get-PropertyValue $service "State" (Get-PropertyValue $service "Status" "unknown"))
        }
    }
    return $snapshot
}

function Invoke-VpsDeploy {
    param([string]$Version)
    $script:CurrentStep = "VPS deploy"
    $script:BuildStates.VPS = "running"
    try {
        $before = Invoke-AdminRequest -Profile $script:SelectedProfile -Method GET -Path "/api/stack" -TimeoutSec 30
        $script:VpsBefore = Get-ServiceSnapshot $before.Data
        $payload = Copy-DeployPayloadWithVersion -Payload $script:SelectedProfile.DeployPayload -Version $Version
        if ($DryRun) {
            Write-RunLine "DRY RUN: authenticated POST /api/deploy was not sent." "WARN"
            $script:BuildStates.VPS = "dry-run"
            return
        }

        $script:VpsUpdateRan = $true
        Write-RunLine "Submitting authenticated VPS deploy request for environment '$script:SelectedEnvironment'."
        $accepted = Invoke-AdminRequest -Profile $script:SelectedProfile -Method POST -Path "/api/deploy" -Body $payload -TimeoutSec 30
        if (-not [bool](Get-PropertyValue $accepted.Data "accepted" $false)) {
            throw "Admin server did not accept the deploy request."
        }

        $deadline = (Get-Date).AddSeconds($DeployTimeoutSeconds)
        do {
            Start-Sleep -Seconds 5
            $job = Invoke-AdminRequest -Profile $script:SelectedProfile -Method GET -Path "/api/job" -TimeoutSec 30
            $running = [bool](Get-PropertyValue $job.Data "running" $false)
            if (-not $running) {
                $success = [bool](Get-PropertyValue $job.Data "success" $false)
                $script:VpsJobReport = [string](Get-PropertyValue $job.Data "output" "")
                Write-RunLine $script:VpsJobReport "OUTPUT"
                if (-not $success) {
                    throw "The admin-server deploy job reported failure."
                }
                break
            }
            Write-RunLine "VPS deploy is still running."
        } while ((Get-Date) -lt $deadline)

        if ($running) {
            throw "VPS deploy timed out after $DeployTimeoutSeconds seconds."
        }
        $after = Invoke-AdminRequest -Profile $script:SelectedProfile -Method GET -Path "/api/stack" -TimeoutSec 30
        $script:VpsAfter = Get-ServiceSnapshot $after.Data
        $script:BuildStates.VPS = "pass"
        $script:VerifiedVpsStatusUrl = "$($script:SelectedProfile.AdminServerUrl)/api/stack"
    }
    catch {
        $script:BuildStates.VPS = "fail"
        if (-not $script:FailureStep) {
            $message = $_.Exception.Message
            Set-Failure -Step $script:CurrentStep -ExitCode 1 -Hint (Get-FailureHint -Text $message -Step $script:CurrentStep) -Output @($message, $script:VpsJobReport)
        }
        throw
    }
}

function Add-EndpointResult {
    param([string]$Label, [string]$Url, [int]$StatusCode, [int]$ElapsedMs, [bool]$Alive)
    $script:EndpointResults.Add([pscustomobject]@{
        Label = $Label
        Url = $Url
        StatusCode = $StatusCode
        ElapsedMs = $ElapsedMs
        Alive = $Alive
    })
}

function Test-HttpEndpoint {
    param(
        [string]$Label,
        [string]$Url,
        [ValidateSet("GET", "HEAD")][string]$Method = "GET",
        [hashtable]$Headers = @{}
    )
    if ([string]::IsNullOrWhiteSpace($Url)) { return }
    for ($attempt = 1; $attempt -le 2; $attempt++) {
        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        $status = 0
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri $Url -Method $Method -Headers $Headers -TimeoutSec 20 -MaximumRedirection 5
            $watch.Stop()
            $status = [int]$response.StatusCode
            $alive = $status -ge 200 -and $status -lt 400
            Write-RunLine "HTTP verify [$Label] attempt ${attempt}: $status in $($watch.ElapsedMilliseconds) ms"
            if ($alive) {
                Add-EndpointResult -Label $Label -Url $Url -StatusCode $status -ElapsedMs ([int]$watch.ElapsedMilliseconds) -Alive $true
                return $true
            }
        }
        catch {
            $watch.Stop()
            if ($_.Exception.Response -and $_.Exception.Response.StatusCode) {
                $status = [int]$_.Exception.Response.StatusCode
            }
            Write-RunLine "HTTP verify [$Label] attempt $attempt failed: HTTP $status in $($watch.ElapsedMilliseconds) ms" "WARN"
        }
    }
    Add-EndpointResult -Label $Label -Url $Url -StatusCode $status -ElapsedMs ([int]$watch.ElapsedMilliseconds) -Alive $false
    $script:EndpointFailures.Add($Label)
    return $false
}

function Get-ProfileEndpointPairs {
    param([object]$Profile)
    $pairs = @()
    if ($null -eq $Profile -or $null -eq $Profile.Endpoints) { return $pairs }
    foreach ($name in @("rpcHealth", "explorer", "website")) {
        $url = [string](Get-PropertyValue $Profile.Endpoints $name "")
        if (-not [string]::IsNullOrWhiteSpace($url)) {
            $pairs += [pscustomobject]@{ Label = $name; Url = $url }
        }
    }
    return $pairs
}

function Add-ReleaseAssetsAndVerify {
    param([string]$Tag)
    if ([string]::IsNullOrWhiteSpace($Tag) -or $DryRun) { return }
    $gh = Get-GhExecutable
    $result = Invoke-ExternalCommand -FilePath $gh -ArgumentList @("release", "view", $Tag, "--json", "url,assets") -DisplayName "read release assets for $Tag"
    $release = ($result.Output -join "`n") | ConvertFrom-Json
    $releaseUrl = [string](Get-PropertyValue $release "url" "")
    if (-not [string]::IsNullOrWhiteSpace($releaseUrl)) {
        $script:DownloadLinks.Add($releaseUrl)
        Test-HttpEndpoint -Label "GitHub release $Tag" -Url $releaseUrl -Method HEAD
    }
    foreach ($asset in @(Get-PropertyValue $release "assets" @())) {
        $name = [string](Get-PropertyValue $asset "name" "release asset")
        $url = [string](Get-PropertyValue $asset "url" "")
        if (-not [string]::IsNullOrWhiteSpace($url)) {
            $script:DownloadLinks.Add($url)
            Test-HttpEndpoint -Label $name -Url $url -Method HEAD
        }
    }
}

function Invoke-LinkVerification {
    $script:CurrentStep = "link and endpoint verification"
    if ($DryRun) {
        Write-RunLine "DRY RUN: endpoint verification skipped because no build/release/deploy was executed." "WARN"
        return
    }
    $failuresBefore = $script:EndpointFailures.Count
    Add-ReleaseAssetsAndVerify -Tag $script:CandidateTag
    Add-ReleaseAssetsAndVerify -Tag $script:VpsReleaseTag
    if ($script:SelectedProfile) {
        $headers = @{ "X-Alvenqis-Setup-Token" = $script:SelectedProfile.SetupToken }
        Test-HttpEndpoint -Label "VPS admin health" -Url "$($script:SelectedProfile.AdminServerUrl)/health"
        Test-HttpEndpoint -Label "VPS stack status" -Url "$($script:SelectedProfile.AdminServerUrl)/api/stack" -Headers $headers
        foreach ($pair in (Get-ProfileEndpointPairs $script:SelectedProfile)) {
            Test-HttpEndpoint -Label $pair.Label -Url $pair.Url
        }
    }
    if ($script:EndpointFailures.Count -gt $failuresBefore) {
        $failedLabels = @($script:EndpointFailures | Select-Object -Skip $failuresBefore)
        throw "Endpoint verification failed after retry for: $($failedLabels -join ', ')."
    }
}

function Write-VpsChanges {
    if (-not $script:VpsUpdateRan) { return }
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# VPS changes")
    $lines.Add("")
    $lines.Add("- Environment: $script:SelectedEnvironment")
    $lines.Add("- Source: authenticated admin-server `/api/stack` snapshots and `/api/job` report")
    $lines.Add("")
    $lines.Add("| Service | Image before | Image after | State before | State after |")
    $lines.Add("|---|---|---|---|---|")

    $names = New-Object System.Collections.Generic.HashSet[string] ([System.StringComparer]::OrdinalIgnoreCase)
    if ($script:VpsBefore) { foreach ($name in $script:VpsBefore.Keys) { [void]$names.Add($name) } }
    if ($script:VpsAfter) { foreach ($name in $script:VpsAfter.Keys) { [void]$names.Add($name) } }
    $changeCount = 0
    foreach ($name in ($names | Sort-Object)) {
        $before = if ($script:VpsBefore -and $script:VpsBefore.Contains($name)) { $script:VpsBefore[$name] } else { $null }
        $after = if ($script:VpsAfter -and $script:VpsAfter.Contains($name)) { $script:VpsAfter[$name] } else { $null }
        $beforeImage = if ($before) { $before.Image } else { "(absent)" }
        $afterImage = if ($after) { $after.Image } else { "(absent)" }
        $beforeState = if ($before) { $before.State } else { "(absent)" }
        $afterState = if ($after) { $after.State } else { "(absent)" }
        if ($beforeImage -ne $afterImage -or $beforeState -ne $afterState) {
            $lines.Add("| $name | $beforeImage | $afterImage | $beforeState | $afterState |")
            $changeCount++
        }
    }
    if ($changeCount -eq 0) {
        $lines.Add("| (none reported) | - | - | - | - |")
    }
    $lines.Add("")
    $lines.Add("## Services changed according to the admin report")
    $lines.Add("")
    $reportLines = @(($script:VpsJobReport -split "`r?`n") | Where-Object {
        $_ -match '(?i)\b(created|recreated|restarted|started|stopped|removed|running)\b'
    })
    if ($reportLines.Count -eq 0) {
        $lines.Add("No service restart/change lines were reported by the admin-server.")
    }
    else {
        foreach ($line in $reportLines) {
            $lines.Add("- " + (ConvertTo-RedactedText $line))
        }
    }
    $lines | Set-Content -LiteralPath $script:VpsChangesPath -Encoding UTF8
}

function Write-Summary {
    $lines = New-Object System.Collections.Generic.List[string]
    if ($script:OverallSucceeded -and -not $DryRun) {
        $lines.Add("# $([System.Char]::ConvertFromUtf32(0x2705)) SUCCESS")
    }
    elseif ($script:Cancelled) {
        $lines.Add("# CANCELLED")
    }
    elseif ($DryRun -and -not $script:FailureStep) {
        $lines.Add("# DRY RUN - no execution commands were run")
    }
    else {
        $lines.Add("# FAILED")
    }
    $lines.Add("")
    $lines.Add("- Commit SHA built/planned: $script:BuildCommit")
    $lines.Add("- Action: $Action")
    $lines.Add("- VPS environment: $(if ($script:SelectedEnvironment) { $script:SelectedEnvironment } else { 'not selected' })")
    $lines.Add("- Local changes pushed: $(if ($script:IncludeLocalChanges -and -not $DryRun) { 'yes' } else { 'no' })")
    $lines.Add("")
    $lines.Add("## Build targets")
    $lines.Add("")
    $lines.Add("| Target | State |")
    $lines.Add("|---|---|")
    foreach ($entry in $script:BuildStates.GetEnumerator()) {
        $lines.Add("| $($entry.Key) | $($entry.Value) |")
    }
    $lines.Add("")
    $lines.Add("## Verified links and endpoints")
    $lines.Add("")
    $lines.Add("| Name | URL | HTTP | Response time | Result |")
    $lines.Add("|---|---|---:|---:|---|")
    if ($script:EndpointResults.Count -eq 0) {
        $lines.Add("| No relevant URL was verified | - | - | - | - |")
    }
    else {
        foreach ($result in $script:EndpointResults) {
            $state = if ($result.Alive) { "alive" } else { "dead" }
            $lines.Add("| $($result.Label) | $($result.Url) | $($result.StatusCode) | $($result.ElapsedMs) ms | $state |")
        }
    }

    if ($script:OverallSucceeded -and -not $DryRun) {
        $lines.Add("")
        $lines.Add("## Final build and download links")
        $lines.Add("")
        foreach ($link in ($script:DownloadLinks | Select-Object -Unique)) {
            $lines.Add("- $link")
        }
        if ($script:VerifiedVpsStatusUrl) {
            $lines.Add("- VPS status: $script:VerifiedVpsStatusUrl")
        }
    }
    $lines | Set-Content -LiteralPath $script:SummaryPath -Encoding UTF8
}

function Write-ErrorState {
    if (-not $script:FailureStep) { return }
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# Error state")
    $lines.Add("")
    $lines.Add("- Failed step: $script:FailureStep")
    $lines.Add("- Exit code: $script:FailureExitCode")
    $lines.Add("- Check first: $script:FailureHint")
    $lines.Add("")
    $lines.Add("## Last relevant log lines")
    $lines.Add("")
    $lines.Add('```text')
    $relevant = if ($script:FailureOutput.Count -gt 0) {
        @($script:FailureOutput | Select-Object -Last 50)
    }
    else {
        @($script:RecentLogLines | Select-Object -Last 50)
    }
    foreach ($line in $relevant) {
        $lines.Add((ConvertTo-RedactedText $line))
    }
    $lines.Add('```')
    $lines | Set-Content -LiteralPath $script:ErrorStatePath -Encoding UTF8
}

Initialize-RunArtifacts
$exitCode = 1
try {
    Set-Location -LiteralPath $script:RepoRoot
    $configuredEnvironments = @(Invoke-StartupChecks)
    $script:CurrentStep = "operator selection"
    $Action = Read-Action
    if ($Action -eq "Exit") {
        $script:Cancelled = $true
        $exitCode = 0
        Write-RunLine "Operator selected Exit."
        throw [System.OperationCanceledException]::new("Operator selected Exit.")
    }

    if (Test-VpsAction $Action) {
        $script:SelectedEnvironment = Select-Environment -Environments $configuredEnvironments
        $script:SelectedProfile = Read-OperatorProfile -EnvironmentName $script:SelectedEnvironment
    }
    $script:IncludeLocalChanges = Resolve-DirtyDecision
    if ($script:Cancelled) {
        Write-RunLine "Run cancelled because the working tree has uncommitted changes." "WARN"
        $exitCode = 2
        throw [System.OperationCanceledException]::new("Run cancelled because the working tree has uncommitted changes.")
    }
    if (-not (Confirm-Plan -SelectedAction $Action)) {
        $script:Cancelled = $true
        Write-RunLine "Run cancelled at final confirmation." "WARN"
        $exitCode = 2
        throw [System.OperationCanceledException]::new("Run cancelled at final confirmation.")
    }

    if ($script:IncludeLocalChanges) {
        Invoke-SyncAndReleaseVps -SyncOnly
    }

    switch ($Action) {
        "Start" {
            Invoke-LocalServices -Command "start"
        }
        "Restart" {
            Invoke-LocalServices -Command "restart"
        }
        "BuildWindows" {
            Invoke-WindowsBuild
            Invoke-LinkVerification
        }
        "BuildLinux" {
            Invoke-LinuxBuild
            Invoke-LinkVerification
        }
        "BuildBoth" {
            Invoke-WindowsBuild
            Invoke-LinuxBuild
            Invoke-LinkVerification
        }
        "UpdateVps" {
            $script:BuildStates.VPS = "running"
            Invoke-ReleaseGate
            Invoke-SyncAndReleaseVps
            if (-not $DryRun) {
                $script:VpsReleaseTag = [string](Get-TagPointingAtHead -Pattern "vps-control-v*-rc.*")
                if ([string]::IsNullOrWhiteSpace($script:VpsReleaseTag)) {
                    throw "The existing VPS release script completed without a VPS release tag at HEAD."
                }
            }
            else {
                $script:VpsReleaseTag = "dry-run-vps-release"
            }
            Invoke-DockerImagesWorkflow -Version $script:VpsReleaseTag
            Invoke-VpsDeploy -Version $script:VpsReleaseTag
            Invoke-LinkVerification
        }
        "Full" {
            Invoke-WindowsBuild
            Invoke-LinuxBuild
            Invoke-ReleaseGate
            Invoke-SyncAndReleaseVps
            if (-not $DryRun) {
                $script:VpsReleaseTag = [string](Get-TagPointingAtHead -Pattern "vps-control-v*-rc.*")
            }
            Invoke-CandidateRelease
            Invoke-DockerImagesWorkflow -Version $script:CandidateTag
            Invoke-VpsDeploy -Version $script:CandidateTag
            Invoke-LinkVerification
        }
    }

    $script:OverallSucceeded = $true
    $exitCode = 0
    Write-RunLine $(if ($DryRun) { "Dry run completed successfully." } else { "Requested pipeline completed successfully." }) "OK"
}
catch {
    if ($_.Exception -is [System.OperationCanceledException]) {
        # Cancellation was logged immediately before raising this control-flow exception.
    }
    else {
        $message = ConvertTo-RedactedText $_.Exception.Message
        Write-RunLine $message "ERROR"
        if (-not $script:FailureStep) {
            Set-Failure -Step $script:CurrentStep -ExitCode 1 -Hint (Get-FailureHint -Text $message -Step $script:CurrentStep) -Output @($message)
        }
        $exitCode = if ($script:FailureExitCode) { [int]$script:FailureExitCode } else { 1 }
    }
}
finally {
    try { Write-VpsChanges } catch { Write-RunLine "Could not write vps-changes.md: $($_.Exception.Message)" "ERROR" }
    try { Write-Summary } catch { Write-RunLine "Could not write summary.md: $($_.Exception.Message)" "ERROR"; $exitCode = 1 }
    try { Write-ErrorState } catch { Write-RunLine "Could not write error-state.md: $($_.Exception.Message)" "ERROR"; $exitCode = 1 }
    Write-RunLine "Overall exit code: $exitCode"
    Write-Host ""
    Write-Host "Run artifacts: $script:RunDirectory"
}
exit $exitCode
