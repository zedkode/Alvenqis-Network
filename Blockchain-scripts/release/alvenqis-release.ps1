[CmdletBinding()]
param(
    [string]$RepoPath = ".",
    [string]$ArtifactsPath = "",
    [ValidateRange(1, 720)][int]$RecentHours = 24
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0
$script:ReleaseManagerVersion = "3.3.0"

function Write-Title {
    param([string]$Text)
    Write-Host ""
    Write-Host ("=" * 72) -ForegroundColor DarkCyan
    Write-Host "  $Text" -ForegroundColor Cyan
    Write-Host ("=" * 72) -ForegroundColor DarkCyan
}

function Write-Info {
    param([string]$Text)
    Write-Host "[INFO] $Text" -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Text)
    Write-Host "[OK]   $Text" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Text)
    Write-Host "[WARN] $Text" -ForegroundColor Yellow
}

function Write-Fail {
    param([string]$Text)
    Write-Host "[ERR]  $Text" -ForegroundColor Red
}

function Test-Tool {
    param([string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [string[]]$Arguments = @(),
        [switch]$Capture,
        [switch]$AllowFailure
    )

    if ($Capture) {
        $output = & $Command @Arguments 2>&1
        $exitCode = $LASTEXITCODE
        if (($exitCode -ne 0) -and (-not $AllowFailure)) {
            $message = ($output | Out-String).Trim()
            throw "Command failed: $Command $($Arguments -join ' ')`n$message"
        }
        return [pscustomobject]@{
            ExitCode = $exitCode
            Output   = (($output | Out-String).Trim())
        }
    }

    & $Command @Arguments
    $exitCode = $LASTEXITCODE
    if (($exitCode -ne 0) -and (-not $AllowFailure)) {
        throw "Command failed with exit code ${exitCode}: $Command $($Arguments -join ' ')"
    }

    return $exitCode
}

function Confirm-Action {
    param(
        [string]$Message,
        [bool]$DefaultYes = $false
    )

    $suffix = if ($DefaultYes) { "[Y/n]" } else { "[y/N]" }
    while ($true) {
        $answer = (Read-Host "$Message $suffix").Trim().ToLowerInvariant()
        if ([string]::IsNullOrWhiteSpace($answer)) {
            return $DefaultYes
        }
        if ($answer -in @("y", "yes")) {
            return $true
        }
        if ($answer -in @("n", "no")) {
            return $false
        }
        Write-Warn "Answer with y/yes or n/no."
    }
}

function Read-MenuChoice {
    param(
        [string]$Prompt,
        [string[]]$Allowed
    )

    while ($true) {
        $choice = (Read-Host $Prompt).Trim()
        if ($choice -in $Allowed) {
            return $choice
        }
        Write-Warn "Invalid option. Choose: $($Allowed -join ', ')."
    }
}

function Get-RepositoryRoot {
    param([string]$Path)

    $resolved = (Resolve-Path $Path).Path
    Push-Location $resolved
    try {
        $result = Invoke-Native -Command "git" -Arguments @("rev-parse", "--show-toplevel") -Capture
        return $result.Output
    }
    finally {
        Pop-Location
    }
}

function Get-CurrentBranch {
    $result = Invoke-Native -Command "git" -Arguments @("branch", "--show-current") -Capture
    return $result.Output.Trim()
}

function Get-DefaultBranch {
    if (Test-Tool "gh") {
        $result = Invoke-Native -Command "gh" -Arguments @("repo", "view", "--json", "defaultBranchRef", "--jq", ".defaultBranchRef.name") -Capture -AllowFailure
        if (($result.ExitCode -eq 0) -and (-not [string]::IsNullOrWhiteSpace($result.Output))) {
            return $result.Output.Trim()
        }
    }

    $symbolic = Invoke-Native -Command "git" -Arguments @("symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD") -Capture -AllowFailure
    if (($symbolic.ExitCode -eq 0) -and ($symbolic.Output -match "^origin/(.+)$")) {
        return $Matches[1]
    }

    return "main"
}

function Test-GhReady {
    if (-not (Test-Tool "gh")) {
        Write-Warn "GitHub CLI (gh) is not installed. The tag can start releases automatically, but restarting an individual workflow requires gh."
        return $false
    }

    $auth = Invoke-Native -Command "gh" -Arguments @("auth", "status") -Capture -AllowFailure
    if ($auth.ExitCode -ne 0) {
        Write-Warn "GitHub CLI is not authenticated. Run: gh auth login"
        return $false
    }

    return $true
}

function Test-SupportedReleaseArtifact {
    param([System.IO.FileInfo]$File)

    $name = $File.Name.ToLowerInvariant()
    if ($name -match "electron" -or $name -in @("latest.yml", "latest-linux.yml")) {
        return $false
    }

    if ($name.EndsWith(".tar.gz")) {
        return $true
    }

    return $File.Extension.ToLowerInvariant() -in @(
        ".exe", ".msi", ".zip", ".appimage", ".deb", ".rpm",
        ".gz", ".sha256", ".txt", ".json", ".sig", ".asc"
    )
}

function Get-RecentReleaseArtifacts {
    param(
        [string]$Path,
        [int]$Hours
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return @()
    }

    $cutoff = (Get-Date).AddHours(-$Hours)
    $files = Get-ChildItem -LiteralPath $Path -File -Recurse -ErrorAction SilentlyContinue |
        Where-Object {
            $_.LastWriteTime -ge $cutoff -and (Test-SupportedReleaseArtifact -File $_)
        } |
        Sort-Object LastWriteTime -Descending

    # GitHub Releases cannot contain two files with the same name. Keep the most recent file.
    $unique = @{}
    foreach ($file in $files) {
        $key = $file.Name.ToLowerInvariant()
        if (-not $unique.ContainsKey($key)) {
            $unique[$key] = $file
        }
    }

    return @($unique.Values | Sort-Object LastWriteTime -Descending)
}

function Format-FileSize {
    param([long]$Bytes)
    if ($Bytes -ge 1GB) { return ("{0:N2} GB" -f ($Bytes / 1GB)) }
    if ($Bytes -ge 1MB) { return ("{0:N2} MB" -f ($Bytes / 1MB)) }
    if ($Bytes -ge 1KB) { return ("{0:N2} KB" -f ($Bytes / 1KB)) }
    return "$Bytes B"
}

function Get-LocalArtifactPlatforms {
    param([System.IO.FileInfo[]]$Files)

    $platforms = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $Files) {
        $name = $file.Name.ToLowerInvariant()
        $extension = $file.Extension.ToLowerInvariant()

        if ($name -match "vps|control[-_ ]?plane|server[-_ ]?bundle" -or $name.EndsWith(".tar.gz")) {
            [void]$platforms.Add("vps")
            continue
        }

        if ($extension -in @(".appimage", ".deb", ".rpm") -or $name -match "linux|appimage") {
            [void]$platforms.Add("linux")
            continue
        }

        if ($extension -in @(".exe", ".msi") -or $name -match "windows|win64|win32|setup|installer|portable" -or $extension -eq ".zip") {
            [void]$platforms.Add("windows")
        }
    }

    return @($platforms | Sort-Object)
}

function Select-RecentLocalArtifacts {
    param(
        [string]$Path,
        [int]$DefaultHours
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        Write-Info "The local artifacts folder does not exist: $Path"
        return @()
    }

    $hours = $DefaultHours
    $enteredHours = (Read-Host "How many hours back should modified artifacts be searched? [$DefaultHours]").Trim()
    if (-not [string]::IsNullOrWhiteSpace($enteredHours)) {
        $parsedHours = 0
        if (-not [int]::TryParse($enteredHours, [ref]$parsedHours) -or $parsedHours -lt 1 -or $parsedHours -gt 720) {
            throw "Invalid interval. Enter a number between 1 and 720 hours."
        }
        $hours = $parsedHours
    }

    $files = @(Get-RecentReleaseArtifacts -Path $Path -Hours $hours)
    if ($files.Count -eq 0) {
        Write-Info "No eligible artifacts modified within the last $hours hours were found."
        return @()
    }

    Write-Host ""
    Write-Host "Recent local artifacts found in:" -ForegroundColor White
    Write-Host "  $Path" -ForegroundColor DarkGray
    for ($i = 0; $i -lt $files.Count; $i++) {
        $file = $files[$i]
        Write-Host ("  {0,2}. {1} | {2} | {3}" -f ($i + 1), $file.Name, (Format-FileSize -Bytes $file.Length), $file.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss"))
    }

    Write-Host ""
    Write-Host "  1. Upload all artifacts listed above and DO NOT rebuild Windows/Linux/VPS"
    Write-Host "  2. Select files manually"
    Write-Host "  0. Ignore local artifacts and let GitHub Actions build them"
    $choice = Read-MenuChoice -Prompt "Choose the release source" -Allowed @("0", "1", "2")

    if ($choice -eq "0") {
        return @()
    }
    if ($choice -eq "1") {
        return $files
    }

    while ($true) {
        $selection = (Read-Host "Enter comma-separated numbers, for example 1,2,5").Trim()
        $indexes = @()
        $valid = $true
        foreach ($part in ($selection -split ",")) {
            $number = 0
            if (-not [int]::TryParse($part.Trim(), [ref]$number) -or $number -lt 1 -or $number -gt $files.Count) {
                $valid = $false
                break
            }
            $indexes += ($number - 1)
        }
        if ($valid -and $indexes.Count -gt 0) {
            return @($indexes | Select-Object -Unique | ForEach-Object { $files[$_] })
        }
        Write-Warn "Invalid selection."
    }
}

function Ensure-GitHubPrerelease {
    param([string]$Tag)

    $existing = Invoke-Native -Command "gh" -Arguments @("release", "view", $Tag) -Capture -AllowFailure
    if ($existing.ExitCode -eq 0) {
        return
    }

    $notes = @"
## Mainnet Candidate prerelease - not public Mainnet

Windows, Linux, server-component, container-image, and Setup External outputs are published independently. Verify SHA256SUMS-LOCAL.txt before testing locally supplied assets. The generated changelog below comes from the commits included by the tagged release.
"@
    $lastError = $null
    for ($attempt = 1; $attempt -le 5; $attempt++) {
        $create = Invoke-Native -Command "gh" -Arguments @(
            "release", "create", $Tag,
            "--verify-tag",
            "--title", "Alvenqis $Tag",
            "--generate-notes",
            "--notes", $notes,
            "--prerelease"
        ) -Capture -AllowFailure

        if ($create.ExitCode -eq 0) {
            return
        }

        $view = Invoke-Native -Command "gh" -Arguments @("release", "view", $Tag) -Capture -AllowFailure
        if ($view.ExitCode -eq 0) {
            return
        }

        $lastError = $create.Output
        Start-Sleep -Seconds 2
    }
    throw "Could not create prerelease $Tag. $lastError"
}

function Publish-LocalArtifactsToRelease {
    param(
        [string]$Tag,
        [System.IO.FileInfo[]]$Files
    )

    if ($Files.Count -eq 0) {
        return
    }

    Write-Title "Publish local artifacts"
    Ensure-GitHubPrerelease -Tag $Tag

    $stage = Join-Path ([System.IO.Path]::GetTempPath()) ("alvenqis-local-release-{0}" -f ([guid]::NewGuid().ToString("N")))
    New-Item -ItemType Directory -Path $stage -Force | Out-Null

    try {
        $stagedFiles = @()
        foreach ($file in $Files) {
            $destination = Join-Path $stage $file.Name
            Copy-Item -LiteralPath $file.FullName -Destination $destination -Force
            $stagedFiles += Get-Item -LiteralPath $destination
        }

        $checksumPath = Join-Path $stage "SHA256SUMS-LOCAL.txt"
        $checksumLines = foreach ($file in ($stagedFiles | Sort-Object Name)) {
            $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            "$hash  $($file.Name)"
        }
        $checksumLines | Set-Content -LiteralPath $checksumPath -Encoding ascii
        $stagedFiles += Get-Item -LiteralPath $checksumPath

        Write-Info "Uploading $($stagedFiles.Count) files to release $Tag..."
        $arguments = @("release", "upload", $Tag)
        $arguments += @($stagedFiles | ForEach-Object { $_.FullName })
        Invoke-Native -Command "gh" -Arguments $arguments | Out-Null

        Write-Ok "The local artifacts were published to release $Tag."
        foreach ($file in $stagedFiles) {
            Write-Host "  - $($file.Name)"
        }
    }
    finally {
        Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Get-DesktopVersion {
    $packagePath = Join-Path (Get-Location) "Blockchain-prototype/alvenqis-desktop-v2/package.json"
    if (Test-Path $packagePath) {
        try {
            $package = Get-Content -Raw -Path $packagePath | ConvertFrom-Json
            $version = [string]$package.version
            if ($version -match "^\d+\.\d+\.\d+(?:[-+].+)?$") {
                return ($version -replace "[-+].*$", "")
            }
        }
        catch {
            Write-Warn "Could not read the version from Blockchain-prototype/alvenqis-desktop-v2/package.json."
        }
    }

    while ($true) {
        $version = (Read-Host "Base version (for example 1.0.0)").Trim()
        $version = $version.TrimStart("v")
        if ($version -match "^\d+\.\d+\.\d+$") {
            return $version
        }
        Write-Warn "The version must use the X.Y.Z format, for example 1.0.0."
    }
}

function Get-CandidateTags {
    param([string]$Version = "*")

    $pattern = if ($Version -eq "*") { "desktop-v*-candidate.*" } else { "desktop-v$Version-candidate.*" }
    $result = Invoke-Native -Command "git" -Arguments @("tag", "--list", $pattern, "--sort=-version:refname") -Capture
    if ([string]::IsNullOrWhiteSpace($result.Output)) {
        return @()
    }
    return @($result.Output -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function Get-NextCandidateTag {
    param([string]$Version)

    $escapedVersion = [regex]::Escape($Version)
    $max = 0
    foreach ($existingTag in (Get-CandidateTags -Version $Version)) {
        if ($existingTag -match "^desktop-v${escapedVersion}-candidate\.(\d+)$") {
            $number = [int]$Matches[1]
            if ($number -gt $max) {
                $max = $number
            }
        }
    }

    return "desktop-v$Version-candidate.$($max + 1)"
}

function Test-TagExistsLocally {
    param([string]$Tag)
    $result = Invoke-Native -Command "git" -Arguments @("show-ref", "--verify", "--quiet", "refs/tags/$Tag") -Capture -AllowFailure
    return $result.ExitCode -eq 0
}

function Test-TagExistsRemotely {
    param([string]$Tag)
    $result = Invoke-Native -Command "git" -Arguments @("ls-remote", "--exit-code", "--tags", "origin", "refs/tags/$Tag") -Capture -AllowFailure
    return $result.ExitCode -eq 0
}

function Assert-ReleaseTagFormat {
    param([string]$Tag)
    if ($Tag -notmatch "^desktop-v\d+\.\d+\.\d+-candidate\.\d+$") {
        throw "Invalid tag: $Tag. Accepted format: desktop-vX.Y.Z-candidate.N"
    }
}

function Sync-And-CheckRepository {
    Write-Info "Updating references and tags from origin..."
    Invoke-Native -Command "git" -Arguments @("fetch", "origin", "--tags", "--prune") | Out-Null

    $branch = Get-CurrentBranch
    $head = (Invoke-Native -Command "git" -Arguments @("rev-parse", "--short", "HEAD") -Capture).Output
    if ([string]::IsNullOrWhiteSpace($branch)) {
        Write-Warn "The repository is in detached HEAD state at commit $head."
        if (-not (Confirm-Action -Message "Continue and create the tag directly on this commit?" -DefaultYes $false)) {
            throw "Operation canceled."
        }
    }
    else {
        Write-Info "Current branch: $branch | commit: $head"
    }

    $status = (Invoke-Native -Command "git" -Arguments @("status", "--porcelain") -Capture).Output
    if (-not [string]::IsNullOrWhiteSpace($status)) {
        Write-Warn "There are uncommitted changes. They will NOT be included in the tag:"
        Write-Host $status
        if (-not (Confirm-Action -Message "Continue using only the latest commit (HEAD)?" -DefaultYes $false)) {
            throw "Commit the changes, then run the script again."
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($branch)) {
        $upstream = Invoke-Native -Command "git" -Arguments @("rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}") -Capture -AllowFailure
        if ($upstream.ExitCode -ne 0) {
            Write-Warn "Branch $branch does not have an upstream configured."
            if (Confirm-Action -Message "Publish it to origin/$($branch) now?" -DefaultYes $true) {
                Invoke-Native -Command "git" -Arguments @("push", "-u", "origin", $branch) | Out-Null
                Write-Ok "Branch published to origin/$branch."
            }
            else {
                throw "The tag cannot be created until the branch is published."
            }
        }
        else {
            $upstreamName = $upstream.Output.Trim()
            $counts = (Invoke-Native -Command "git" -Arguments @("rev-list", "--left-right", "--count", "$upstreamName...HEAD") -Capture).Output.Trim()
            $parts = $counts -split "\s+"
            $behind = [int]$parts[0]
            $ahead = [int]$parts[1]

            if ($behind -gt 0) {
                throw "The local branch is $behind commit(s) behind $upstreamName. Pull or rebase before creating a release."
            }

            if ($ahead -gt 0) {
                Write-Warn "The local branch has $ahead unpublished commit(s)."
                if (Confirm-Action -Message "Publish them to $($upstreamName) now?" -DefaultYes $true) {
                    Invoke-Native -Command "git" -Arguments @("push") | Out-Null
                    Write-Ok "The commits were published."
                }
                else {
                    throw "The tag cannot be created on unpublished commits."
                }
            }
        }
    }

    return $branch
}

function Create-And-PushCandidateTag {
    param([switch]$Custom)

    $tag = $null
    $branch = $null
    $version = $null

    Write-Title "Create a candidate tag and start the releases"
    $branch = Sync-And-CheckRepository
    $version = Get-DesktopVersion

    if ($Custom) {
        $defaultTag = Get-NextCandidateTag -Version $version
        $entered = (Read-Host "Requested tag [$defaultTag]").Trim()
        $tag = if ([string]::IsNullOrWhiteSpace($entered)) { $defaultTag } else { $entered }
    }
    else {
        $tag = Get-NextCandidateTag -Version $version
    }

    if ([string]::IsNullOrWhiteSpace([string]$tag)) {
        throw "Could not generate the candidate tag. Verify the application version and try again."
    }

    Assert-ReleaseTagFormat -Tag $tag

    $expectedPrefix = "desktop-v$version-candidate."
    if (-not $tag.StartsWith($expectedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Tag $tag does not match desktop version $version. The workflows will reject it."
    }

    if ((Test-TagExistsLocally -Tag $tag) -or (Test-TagExistsRemotely -Tag $tag)) {
        throw "Tag $tag already exists. Use the restart or upload option for an existing tag."
    }

    $ghReady = Test-GhReady
    $commit = (Invoke-Native -Command "git" -Arguments @("rev-parse", "--short", "HEAD") -Capture).Output.Trim()
    Write-Host ""
    Write-Host "Release summary:" -ForegroundColor White
    Write-Host "  Tag:       $tag"
    Write-Host "  Version:   $version"
    Write-Host "  Branch:    $(if ([string]::IsNullOrWhiteSpace($branch)) { 'detached HEAD' } else { $branch })"
    Write-Host "  Commit:    $commit"
    Write-Host "  Source:    GitHub Actions"
    Write-Host "  Starts:    Windows + Linux + server components + Setup External + container images + Quality (independent)"
    Write-Host ""

    if (-not (Confirm-Action -Message "Create and publish tag $($tag)?" -DefaultYes $false)) {
        Write-Warn "Operation canceled."
        return
    }

    $tagMessage = "Alvenqis candidate release $tag"

    Invoke-Native -Command "git" -Arguments @("tag", "-a", $tag, "-m", $tagMessage) | Out-Null
    try {
        Invoke-Native -Command "git" -Arguments @("push", "origin", $tag) | Out-Null
    }
    catch {
        Write-Fail "The tag was created locally, but publishing it to GitHub failed."
        Write-Host "You can retry manually with: git push origin $tag"
        throw
    }

    Write-Ok "Tag published: $tag"

    Write-Ok "GitHub Actions received the event. The six release/quality workflows run independently."
    Write-Host ""
    Write-Host "Important: you do not need to create the release manually." -ForegroundColor Yellow
    Write-Host "The first successful publish job creates the prerelease, and the others add their own files."

    if ($ghReady) {
        $repoUrl = (Invoke-Native -Command "gh" -Arguments @("repo", "view", "--json", "url", "--jq", ".url") -Capture).Output.Trim()
        if (-not [string]::IsNullOrWhiteSpace($repoUrl)) {
            Write-Host "Actions:  $repoUrl/actions"
            Write-Host "Releases: $repoUrl/releases/tag/$tag"
            if (Confirm-Action -Message "Open the release in a browser?" -DefaultYes $true) {
                Start-Process "$repoUrl/releases/tag/$tag"
            }
        }
    }
}

function Select-ExistingTag {
    $tags = @(Get-CandidateTags)
    if ($tags.Count -eq 0) {
        throw "No local candidate tags exist. Run git fetch origin --tags or create the first tag."
    }

    Write-Host ""
    Write-Host "Recent candidate tags:" -ForegroundColor White
    $limit = [Math]::Min($tags.Count, 15)
    for ($i = 0; $i -lt $limit; $i++) {
        Write-Host ("  {0,2}. {1}" -f ($i + 1), $tags[$i])
    }

    $entered = (Read-Host "Choose a number or enter the complete tag [1]").Trim()
    if ([string]::IsNullOrWhiteSpace($entered)) {
        return $tags[0]
    }

    $number = 0
    if ([int]::TryParse($entered, [ref]$number)) {
        if (($number -lt 1) -or ($number -gt $limit)) {
            throw "Invalid number."
        }
        return $tags[$number - 1]
    }

    Assert-ReleaseTagFormat -Tag $entered
    return $entered
}

function Start-WorkflowForTag {
    param(
        [string]$Workflow,
        [string]$Tag,
        [string]$DefaultBranch
    )

    Write-Info "Starting $Workflow for $Tag..."
    $arguments = @("workflow", "run", $Workflow, "--ref", $DefaultBranch, "-f", "tag=$Tag")
    Invoke-Native -Command "gh" -Arguments $arguments | Out-Null
    Write-Ok "Started: $Workflow"
}

function Restart-IndependentWorkflows {
    $tag = $null
    $defaultBranch = $null

    Write-Title "Restart an independent release"

    Invoke-Native -Command "git" -Arguments @("fetch", "origin", "--tags", "--prune") | Out-Null
    if (-not (Test-GhReady)) {
        throw "For a manual restart, install GitHub CLI and run gh auth login."
    }

    $tag = [string](Select-ExistingTag)
    if ([string]::IsNullOrWhiteSpace($tag)) {
        throw "No tag was selected."
    }
    if (-not (Test-TagExistsRemotely -Tag $tag)) {
        throw "Tag $tag does not exist on origin."
    }

    $defaultBranch = Get-DefaultBranch
    Write-Info "The workflows will be started from the default branch: $defaultBranch"

    Write-Host ""
    Write-Host "  1. Windows"
    Write-Host "  2. Linux"
    Write-Host "  3. Setup External bundle"
    Write-Host "  4. Linux server components"
    Write-Host "  5. Setup External container images"
    Write-Host "  6. Quality checks"
    Write-Host "  7. All six"
    Write-Host "  0. Back"
    $choice = Read-MenuChoice -Prompt "Choose the workflow" -Allowed @("0", "1", "2", "3", "4", "5", "6", "7")
    if ($choice -eq "0") {
        return
    }

    $workflows = @()
    switch ($choice) {
        "1" { $workflows = @("candidate-windows-release.yml") }
        "2" { $workflows = @("candidate-linux-release.yml") }
        "3" { $workflows = @("candidate-setup-external-release.yml") }
        "4" { $workflows = @("candidate-linux-components-release.yml") }
        "5" { $workflows = @("setup-external-images.yml") }
        "6" { $workflows = @("candidate-quality.yml") }
        "7" {
            $workflows = @(
                "candidate-windows-release.yml",
                "candidate-linux-release.yml",
                "candidate-setup-external-release.yml",
                "candidate-linux-components-release.yml",
                "setup-external-images.yml",
                "candidate-quality.yml"
            )
        }
    }

    Write-Host ""
    Write-Host "Tag: $tag"
    Write-Host "Workflows: $($workflows -join ', ')"
    if (-not (Confirm-Action -Message "Start the selected workflows?" -DefaultYes $false)) {
        Write-Warn "Operation canceled."
        return
    }

    foreach ($workflow in $workflows) {
        Start-WorkflowForTag -Workflow $workflow -Tag $tag -DefaultBranch $defaultBranch
    }

    $repoUrl = (Invoke-Native -Command "gh" -Arguments @("repo", "view", "--json", "url", "--jq", ".url") -Capture).Output.Trim()
    Write-Host "Actions: $repoUrl/actions"
    if (Confirm-Action -Message "Open GitHub Actions in a browser?" -DefaultYes $true) {
        Start-Process "$repoUrl/actions"
    }
}

function Upload-LocalArtifactsForExistingTag {
    $tag = $null

    Write-Title "Upload local artifacts to an existing tag"

    Invoke-Native -Command "git" -Arguments @("fetch", "origin", "--tags", "--prune") | Out-Null
    if (-not (Test-GhReady)) {
        throw "For a local upload, install GitHub CLI and run gh auth login."
    }

    $tag = [string](Select-ExistingTag)
    if ([string]::IsNullOrWhiteSpace($tag)) {
        throw "No tag was selected."
    }
    if (-not (Test-TagExistsRemotely -Tag $tag)) {
        throw "Tag $tag does not exist on origin."
    }

    $files = @(Select-RecentLocalArtifacts -Path $script:ResolvedArtifactsPath -DefaultHours $RecentHours)
    if ($files.Count -eq 0) {
        Write-Warn "No artifacts were selected."
        return
    }

    Write-Host ""
    Write-Host "Tag: $tag"
    Write-Host "Files: $($files.Count)"
    if (-not (Confirm-Action -Message "Upload the selected files and replace duplicate names?" -DefaultYes $false)) {
        Write-Warn "Operation canceled."
        return
    }

    Publish-LocalArtifactsToRelease -Tag $tag -Files $files
}

function Show-CandidateTags {
    Write-Title "Candidate tags"
    Invoke-Native -Command "git" -Arguments @("fetch", "origin", "--tags", "--prune") | Out-Null
    $tags = @(Get-CandidateTags)
    if ($tags.Count -eq 0) {
        Write-Warn "No candidate tags exist."
        return
    }
    foreach ($candidateTag in $tags) {
        Write-Host "  $candidateTag"
    }
}

function Open-GitHubPages {
    Write-Title "Open GitHub"
    if (-not (Test-GhReady)) {
        throw "Install GitHub CLI and run gh auth login."
    }

    $repoUrl = (Invoke-Native -Command "gh" -Arguments @("repo", "view", "--json", "url", "--jq", ".url") -Capture).Output.Trim()
    Write-Host "  1. Actions"
    Write-Host "  2. Releases"
    Write-Host "  0. Back"
    $choice = Read-MenuChoice -Prompt "Choose the page" -Allowed @("0", "1", "2")
    switch ($choice) {
        "1" { Start-Process "$repoUrl/actions" }
        "2" { Start-Process "$repoUrl/releases" }
    }
}

if (-not (Test-Tool "git")) {
    throw "Git is not installed or is not available in PATH."
}

$repoRoot = Get-RepositoryRoot -Path $RepoPath
Set-Location $repoRoot

if ([string]::IsNullOrWhiteSpace($ArtifactsPath)) {
    $script:ResolvedArtifactsPath = Join-Path $repoRoot "release-artifacts"
}
elseif ([System.IO.Path]::IsPathRooted($ArtifactsPath)) {
    $script:ResolvedArtifactsPath = $ArtifactsPath
}
else {
    $script:ResolvedArtifactsPath = Join-Path $repoRoot $ArtifactsPath
}

Write-Title "Alvenqis Independent Release Manager v$script:ReleaseManagerVersion"
Write-Ok "Repository: $repoRoot"
Write-Info "Local artifacts: $script:ResolvedArtifactsPath"

while ($true) {
    Write-Host ""
    Write-Host "  1. Create the next candidate tag and start all releases"
    Write-Host "  2. Create a custom candidate tag"
    Write-Host "  3. Restart an independent workflow for an existing tag"
    Write-Host "  4. Upload local artifacts to an existing tag"
    Write-Host "  5. Show candidate tags"
    Write-Host "  6. Open GitHub Actions or Releases"
    Write-Host "  0. Exit"
    Write-Host ""

    $choice = Read-MenuChoice -Prompt "Choose an option" -Allowed @("0", "1", "2", "3", "4", "5", "6")
    try {
        switch ($choice) {
            "0" {
                Write-Host "Goodbye."
                exit 0
            }
            "1" { Create-And-PushCandidateTag }
            "2" { Create-And-PushCandidateTag -Custom }
            "3" { Restart-IndependentWorkflows }
            "4" { Upload-LocalArtifactsForExistingTag }
            "5" { Show-CandidateTags }
            "6" { Open-GitHubPages }
        }
    }
    catch {
        $currentError = $_
        Write-Fail $currentError.Exception.Message
        if ($null -ne $currentError.InvocationInfo -and $currentError.InvocationInfo.ScriptLineNumber -gt 0) {
            Write-Host ("[DEBUG] File: {0}" -f $currentError.InvocationInfo.ScriptName) -ForegroundColor DarkGray
            Write-Host ("[DEBUG] Line: {0}" -f $currentError.InvocationInfo.ScriptLineNumber) -ForegroundColor DarkGray
            if (-not [string]::IsNullOrWhiteSpace($currentError.InvocationInfo.Line)) {
                Write-Host ("[DEBUG] Code: {0}" -f $currentError.InvocationInfo.Line.Trim()) -ForegroundColor DarkGray
            }
        }
    }
}
