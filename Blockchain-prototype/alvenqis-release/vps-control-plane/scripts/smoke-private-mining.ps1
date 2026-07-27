param(
    [string]$BaseUrl = "https://rpcnode.dohotstudio.com",
    [string]$StratumHost = "stratum.dohotstudio.com",
    [int]$StratumPort = 3333
)
$ErrorActionPreference = "Stop"

try {
    Invoke-WebRequest -Uri "$($BaseUrl.TrimEnd('/'))/mining/template" -TimeoutSec 20 -UseBasicParsing | Out-Null
    throw "Public mining HTTP unexpectedly returned success."
} catch {
    $status = [int]$_.Exception.Response.StatusCode
    if ($status -ne 410) {
        throw "Expected public mining HTTP 410, got $status."
    }
}

$client = [System.Net.Sockets.TcpClient]::new()
try {
    $client.ConnectAsync($StratumHost, $StratumPort).Wait([TimeSpan]::FromSeconds(20))
    $tls = [System.Net.Security.SslStream]::new($client.GetStream(), $false)
    $tls.AuthenticateAsClient($StratumHost)
    if (-not $tls.IsAuthenticated -or -not $tls.IsEncrypted) {
        throw "Stratum connection is not authenticated and encrypted."
    }
    Write-Host "PASS: HTTP mining is retired and Stratum TLS authenticated."
} finally {
    $client.Dispose()
}
