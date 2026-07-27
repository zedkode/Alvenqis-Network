$ErrorActionPreference = "Stop"

if ($args -contains "--help" -or $args -contains "-h" -or $args -contains "-Help") {
  Write-Host "Usage: Blockchain-scripts/security/check-repo-hygiene.ps1"
  Write-Host "Fails when tracked files match ignore rules, or when runtime/build artifacts can enter the repository."
  exit 0
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$issues = New-Object System.Collections.Generic.List[string]

# 1) Structural: any tracked path that is now ignored must fail the gate.
#    (git ls-files -c -i --exclude-standard)
$cachedIgnored = @(git ls-files -ci --exclude-standard 2>$null | Where-Object { $_ })
if ($cachedIgnored.Count -gt 0) {
  $sample = $cachedIgnored | Select-Object -First 40
  $more = if ($cachedIgnored.Count -gt 40) { "`n- ... and $($cachedIgnored.Count - 40) more" } else { "" }
  $issues.Add(
    ("Tracked files that match .gitignore (git ls-files -ci --exclude-standard); untrack with git rm --cached:`n- " +
      ($sample -join "`n- ") + $more)
  )
}

# 2) Content scan: tracked + unignored untracked paths
$candidates = @()
$candidates += git ls-files
$candidates += git ls-files --others --exclude-standard
$candidates = $candidates | Where-Object { $_ } | ForEach-Object { $_ -replace '\\', '/' } | Sort-Object -Unique

# Allowlisted placeholder keepers under control-plane state (must stay empty dirs in git).
$stateGitkeepAllow = @(
  'Blockchain-prototype/alvenqis-release/vps-control-plane/state/config/generated/.gitkeep',
  'Blockchain-prototype/alvenqis-release/vps-control-plane/state/secrets/.gitkeep',
  # legacy path (pre monorepo layout) — still reject anything else there
  'alvenqis-release/vps-control-plane/state/config/generated/.gitkeep',
  'alvenqis-release/vps-control-plane/state/secrets/.gitkeep'
)

$forbiddenPatterns = @(
  '(^|/)\.(alvenqis|vireon|veiron)-(dev|testnet|mainnet|local)(/|$)',
  '(^|/)(target|target-msvc|target-msvc-[^/]+|target-miner-test|target-rebrand|target-rebrand-msvc|node_modules|logs|devnet-data|node-data|\.artifacts|coverage)(/|$)',
  '(^|/)(\.review|\.agents|\.codex|\.cursor|\.grok|\.claude)(/|$)',
  '(^|/)Blockchain-docs/(internal|ai/rebrand-pack|human/source-info|human/internal)(/|$)',
  # Control-plane runtime state (monorepo + legacy roots). Only .gitkeep placeholders allowed.
  '(^|/)((Blockchain-prototype/)?)alvenqis-release/vps-control-plane/state/.+',
  '(^|/)chain\.jsonl$',
  '\.(log|pid|tmp|bak|orig|rej|db|sqlite|exe|dll|msi|AppImage|deb|rpm|apk|aab)$'
)

foreach ($file in $candidates) {
  $norm = $file -replace '\\', '/'

  if ($norm -match '^\.review/pipeline/(runs|worktrees)/' -and $norm -ne '.review/pipeline/runs/.gitkeep') {
    $issues.Add("Forbidden local pipeline artifact: $norm")
    continue
  }

  if ($stateGitkeepAllow -contains $norm) {
    continue
  }

  foreach ($pattern in $forbiddenPatterns) {
    if ($norm -match $pattern) {
      $issues.Add("Forbidden tracked or unignored artifact: $norm")
      break
    }
  }
}

if ($issues.Count -gt 0) {
  Write-Error ("Repository hygiene check failed:`n- " + ($issues -join "`n- "))
  exit 1
}

Write-Host "Repository hygiene check passed."
