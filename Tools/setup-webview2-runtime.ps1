# CI prerequisite for native WebView2 tests. Detection and deployment follow:
# https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
$ErrorActionPreference = 'Stop'

function Get-WebView2Version {
    $client = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    foreach ($key in @(
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$client",
        "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$client",
        "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$client"
    )) {
        $candidate = Get-ItemPropertyValue -Path $key -Name 'pv' -ErrorAction SilentlyContinue
        [Version]$version = $null
        if ([Version]::TryParse($candidate, [ref]$version) -and $version -gt [Version]'0.0.0.0') {
            return $version
        }
    }
    return $null
}

$version = Get-WebView2Version
if ($null -ne $version) {
    Write-Host "WebView2 Evergreen runtime already installed: $version"
    exit 0
}

$installer = Join-Path ([System.IO.Path]::GetTempPath()) "pitex-webview2-$([Guid]::NewGuid()).exe"
$process = $null
try {
    Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $installer -TimeoutSec 60
    $signature = Get-AuthenticodeSignature -FilePath $installer
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') {
        throw 'WebView2 bootstrapper does not have a valid Microsoft signature'
    }
    $process = Start-Process -FilePath $installer -ArgumentList '/silent', '/install' -PassThru
    if (-not $process.WaitForExit(300000)) {
        $process.Kill($true)
        throw 'WebView2 installation exceeded five minutes'
    }
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        $version = Get-WebView2Version
        if ($null -ne $version) { break }
        Start-Sleep -Seconds 1
    }
    if ($null -eq $version) {
        throw "WebView2 runtime is unavailable after installation (exit code $($process.ExitCode))"
    }
    Write-Host "WebView2 Evergreen runtime ready: $version (installer exit code $($process.ExitCode))"
} finally {
    if ($null -ne $process) {
        if (-not $process.HasExited) { $process.Kill($true) }
        $process.Dispose()
    }
    Remove-Item -LiteralPath $installer -Force -ErrorAction SilentlyContinue
}
