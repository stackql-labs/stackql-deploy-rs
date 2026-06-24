# Client-side smoke test for the stackql-deploy installer (Windows).
# Confirms the origin is Cloudflare, exercises every installer path / shell-guard,
# then runs the real installer and checks the binary downloads and runs.
# Works on Windows PowerShell 5.1 and PowerShell 7+.

$ErrorActionPreference = 'Stop'

$Bin = 'stackql-deploy.exe'
$Base = 'https://get-stackql-deploy.io'
$InstallUrl = "$Base/install.ps1"

# User-Agents the worker routes on: PowerShell vs a POSIX download tool.
$UaPs = 'Mozilla/5.0 (Windows NT 10.0) WindowsPowerShell/5.1'
$UaCurl = 'curl/8.4.0'

foreach ($f in @('stackql-deploy', 'stackql', 'stackql-deploy.exe', 'stackql-deploy.zip')) {
  if (Test-Path $f) { Remove-Item $f -Force }
}

Add-Type -AssemblyName System.Net.Http

# Fetch a URL with a given User-Agent without following redirects. Returns the
# status, Location header, Server header, and body so each check can assert.
function Get-Resp {
  param([string]$Url, [string]$Ua)
  $handler = [System.Net.Http.HttpClientHandler]::new()
  $handler.AllowAutoRedirect = $false
  $client = [System.Net.Http.HttpClient]::new($handler)
  try {
    $msg = [System.Net.Http.HttpRequestMessage]::new([System.Net.Http.HttpMethod]::Get, $Url)
    [void]$msg.Headers.TryAddWithoutValidation('User-Agent', $Ua)
    $resp = $client.SendAsync($msg).Result
    $body = $resp.Content.ReadAsStringAsync().Result
    $location = ''
    if ($resp.Headers.Location) { $location = $resp.Headers.Location.ToString() }
    $server = ''
    $vals = $null
    if ($resp.Headers.TryGetValues('Server', [ref]$vals)) { $server = ($vals -join '') }
    [pscustomobject]@{
      Status   = [int]$resp.StatusCode
      Location = $location
      Server   = $server
      Body     = $body
    }
  } finally {
    $client.Dispose()
    $handler.Dispose()
  }
}

function Assert-Body {
  param([string]$Name, [string]$Url, [string]$Ua, [string]$Expect)
  $resp = Get-Resp -Url $Url -Ua $Ua
  if ($resp.Body -like "*$Expect*") {
    Write-Host "  ok: $Name"
  } else {
    $first = ($resp.Body -split "`n" | Select-Object -First 1)
    Write-Host "FAIL: $Name"
    Write-Host "      expected body to contain: $Expect"
    Write-Host "      got first line: $first"
    exit 1
  }
}

function Assert-Location {
  param([string]$Name, [string]$Url, [string]$Ua, [string]$Expect)
  $resp = Get-Resp -Url $Url -Ua $Ua
  if ($resp.Location -like "*$Expect*") {
    Write-Host "  ok: $Name -> $($resp.Location)"
  } else {
    Write-Host "FAIL: $Name"
    Write-Host "      expected Location containing: $Expect"
    if ($resp.Location) { Write-Host "      got: $($resp.Location)" } else { Write-Host "      got: <none>" }
    exit 1
  }
}

function Write-Box {
  param([string]$Msg)
  $line = '-' * ($Msg.Length + 4)
  Write-Host "+$line+"
  Write-Host "|  $Msg  |"
  Write-Host "+$line+"
}

Write-Box "Installing StackQL Deploy for Windows"

Write-Host "Origin check:"
$origin = Get-Resp -Url $InstallUrl -Ua $UaPs
if ($origin.Server) { Write-Host "  server: $($origin.Server)" } else { Write-Host "  server: <none>" }
if ($origin.Server -like '*cloudflare*') {
  Write-Host "  ok: served by Cloudflare"
} else {
  Write-Host "FAIL: expected Cloudflare origin, got '$($origin.Server)'"
  exit 1
}
Write-Host ""

Write-Host "Endpoint routing:"
# /install auto-detects the calling shell.
Assert-Body "/install (powershell)  -> ps1 installer"     "$Base/install"     $UaPs   '#Requires -Version 5'
Assert-Body "/install (curl)        -> sh installer"      "$Base/install"     $UaCurl '#!/bin/sh'
# Explicit endpoints serve their real script for the matching shell.
Assert-Body "/install.ps1 (ps)      -> ps1 installer"     "$Base/install.ps1" $UaPs   '#Requires -Version 5'
Assert-Body "/install.sh (curl)     -> sh installer"      "$Base/install.sh"  $UaCurl '#!/bin/sh'
# Wrong-shell guards point at the correct command instead of erroring.
Assert-Body "/install.ps1 (curl)    -> 'use install.sh'"  "$Base/install.ps1" $UaCurl 'install.sh | sh'
Assert-Body "/install.sh (ps)       -> 'use install.ps1'" "$Base/install.sh"  $UaPs   'install.ps1 | iex'
Write-Host ""

Write-Host "Root + fallback redirects:"
Assert-Location "/ (windows UA)"    "$Base/" $UaPs                                       'stackql-deploy-windows-x86_64.zip'
Assert-Location "/ (linux UA)"      "$Base/" $UaCurl                                     'stackql-deploy-linux-x86_64.tar.gz'
Assert-Location "/ (macOS UA)"      "$Base/" 'Mozilla/5.0 (Macintosh; Intel Mac OS X)'   'stackql-deploy-macos-universal.tar.gz'
Assert-Location "/some/other/path"  "$Base/some/other/path" $UaCurl                      'stackql-deploy.io'
Write-Host ""

Write-Host "Running installer:"
Invoke-RestMethod $InstallUrl | Invoke-Expression

if (-not (Test-Path $Bin)) {
  Write-Host "FAIL: $Bin was not downloaded"
  exit 1
}
Write-Host ""

Write-Host "Binary:"
$item = Get-Item $Bin
Write-Host ("  {0}  {1:N0} bytes" -f $item.Name, $item.Length)
Write-Host ""

Write-Host "Execution check:"
try {
  & ".\$Bin" --version
  Write-Host ""
  Write-Host "PASS: runnable $Bin for Windows/$env:PROCESSOR_ARCHITECTURE"
} catch {
  Write-Host ""
  Write-Host "FAIL: $Bin did not run on this platform"
  exit 1
}
