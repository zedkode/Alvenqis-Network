$ErrorActionPreference = "Stop"

& "$PSScriptRoot\check-release.ps1"

$outputDir = Join-Path (Resolve-Path ".") "release-artifacts\mainnet-candidate"
if (Test-Path $outputDir) {
  Remove-Item -LiteralPath $outputDir -Recurse -Force
}
New-Item -ItemType Directory -Path $outputDir | Out-Null

$cargoShim = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (Test-Path $cargoShim) {
  & $cargoShim +stable-x86_64-pc-windows-msvc build --workspace --release
} else {
  cargo build --workspace --release
}

Copy-Item README.md $outputDir
Copy-Item -Recurse configs $outputDir
Copy-Item -Recurse docs\release $outputDir

Write-Host "Release artifacts prepared in $outputDir"
