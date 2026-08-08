$ErrorActionPreference = "Stop"

$cargoShim = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (Test-Path $cargoShim) {
  $cargo = $cargoShim
  $cargoArgsPrefix = @("+stable-x86_64-pc-windows-msvc")
} else {
  $cargo = "cargo"
  $cargoArgsPrefix = @()
}

Write-Host "Running Alvenqis mainnet-candidate release checks..."
& $cargo @cargoArgsPrefix fmt --all --check
& $cargo @cargoArgsPrefix test --workspace --tests
& $cargo @cargoArgsPrefix clippy --workspace --all-targets -- -D warnings

if (Test-Path "alvenqis-explorer\package.json") {
  Push-Location alvenqis-explorer
  npm install
  npm run build
  Pop-Location
}

Write-Host "Release checks passed."
