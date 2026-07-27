$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$count = if ($args.Length -gt 0) { [int]$args[0] } else { 1 }
if ($count -le 1) {
    cargo run -p alvenqis-node -- mine-dev-block
} else {
    cargo run -p alvenqis-node -- mine-dev-blocks $count
}
