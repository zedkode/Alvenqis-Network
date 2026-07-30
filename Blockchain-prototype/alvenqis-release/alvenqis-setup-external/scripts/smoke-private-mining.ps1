param(
    [Parameter(Mandatory = $true)]
    [string]$PrivateRpcUrl,
    [Parameter(Mandatory = $true)]
    [string]$StratumHost,
    [int]$StratumPort = 3333,
    [Parameter(Mandatory = $true)]
    [string]$MinerAddress
)
$ErrorActionPreference = "Stop"

$template = Invoke-RestMethod `
    -Uri "$($PrivateRpcUrl.TrimEnd('/'))/mining/template?miner_address=$([uri]::EscapeDataString($MinerAddress))" `
    -TimeoutSec 45
if (-not $template.template_id -or $template.network_id -ne "alvenqis-mainnet-candidate") {
    throw "Docker-private pool mining RPC returned an invalid template."
}

$client = [System.Net.Sockets.TcpClient]::new()
try {
    $client.ConnectAsync($StratumHost, $StratumPort).Wait([TimeSpan]::FromSeconds(20))
    $tls = [System.Net.Security.SslStream]::new($client.GetStream(), $false)
    $tls.AuthenticateAsClient($StratumHost)
    if (-not $tls.IsAuthenticated -or -not $tls.IsEncrypted) {
        throw "Stratum connection is not authenticated and encrypted."
    }
    Write-Host "PASS: Docker-private pool mining template is valid and Stratum TLS authenticated."
} finally {
    $client.Dispose()
}
