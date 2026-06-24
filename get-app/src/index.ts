const GITHUB_REPO = "stackql/stackql-deploy-rs";
const RELEASE_BASE = `https://github.com/${GITHUB_REPO}/releases/latest/download`;

// Browser / known-OS callers send a User-Agent that identifies the platform, so
// the root URL can redirect them straight to the correct release asset. CLI
// download tools (curl, wget) send a UA like "curl/8.4.0" with NO operating
// system in it, so we cannot pick a binary for them from the UA. For those we
// expose a dedicated installer at /install.sh that detects OS + arch locally
// with `uname` and downloads the right asset. This is what fixes "exec format
// error" on macOS - the bare root URL used to fall through to the linux x86_64
// asset for every curl caller regardless of the real platform.
//
// The root URL behaviour is unchanged, so the existing
// `curl -L https://get-stackql-deploy.io | tar xzf -` one-liner keeps working
// for linux / windows / browser callers. The script path is purely additive.

function getAssetName(ua: string): string {
  if (/windows/i.test(ua)) return "stackql-deploy-windows-x86_64.zip";
  if (/darwin|macintosh|mac os/i.test(ua)) return "stackql-deploy-macos-universal.tar.gz";
  return "stackql-deploy-linux-x86_64.tar.gz";
}

const INSTALL_SCRIPT = `#!/bin/sh
# stackql-deploy installer - https://get-stackql-deploy.io/install.sh
# Detects OS + architecture and downloads the matching release binary into the
# current directory.
set -eu

base="${RELEASE_BASE}"
os=$(uname -s)
arch=$(uname -m)

case "$os" in
  Darwin)
    asset="stackql-deploy-macos-universal.tar.gz"
    ;;
  Linux)
    case "$arch" in
      x86_64 | amd64)        asset="stackql-deploy-linux-x86_64.tar.gz" ;;
      aarch64 | arm64)       asset="stackql-deploy-linux-arm64.tar.gz" ;;
      *) echo "stackql-deploy: unsupported Linux architecture: $arch" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "stackql-deploy: unsupported OS: $os" >&2
    echo "On Windows use PowerShell - see https://stackql.io/docs/installing-stackql" >&2
    exit 1
    ;;
esac

echo "Downloading $asset ..."
curl -fsSL "$base/$asset" | tar xzf -
chmod +x stackql-deploy
echo "Installed ./stackql-deploy ($os/$arch). Run it with ./stackql-deploy or move it onto your PATH."
`;

// PowerShell equivalent of the installer for Windows callers, so the awkward
// Invoke-WebRequest + Expand-Archive dance collapses to:
//   irm https://get-stackql-deploy.io/install.ps1 | iex
// Windows ships a single x86_64 build; ARM64 Windows runs it under emulation, so
// both architectures resolve to the same asset.
const INSTALL_PS1 = `#Requires -Version 5
# stackql-deploy installer - https://get-stackql-deploy.io/install.ps1
# Downloads the latest Windows release binary into the current directory.
# Usage: irm https://get-stackql-deploy.io/install.ps1 | iex
$ErrorActionPreference = 'Stop'

$base = "${RELEASE_BASE}"
$asset = 'stackql-deploy-windows-x86_64.zip'
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne 'AMD64' -and $arch -ne 'ARM64') {
  throw "stackql-deploy: unsupported Windows architecture: $arch"
}

$dest = (Get-Location).Path
$zip = Join-Path $dest $asset
Write-Host "Downloading $asset ..."
Invoke-WebRequest -Uri "$base/$asset" -OutFile $zip
Expand-Archive -Path $zip -DestinationPath $dest -Force
Remove-Item $zip
Write-Host "Installed .\\stackql-deploy.exe. Run it with .\\stackql-deploy.exe or move it onto your PATH."
`;

export default {
  fetch(req: Request): Response {
    const url = new URL(req.url);

    // Dedicated installer endpoint for curl / wget (OS + arch detected client-side).
    if (url.pathname === "/install.sh" || url.pathname === "/install") {
      return new Response(INSTALL_SCRIPT, {
        headers: { "content-type": "text/x-shellscript; charset=utf-8" },
      });
    }

    // PowerShell installer for Windows: irm https://get-stackql-deploy.io/install.ps1 | iex
    if (url.pathname === "/install.ps1") {
      return new Response(INSTALL_PS1, {
        headers: { "content-type": "text/plain; charset=utf-8" },
      });
    }

    if (url.pathname !== "/") {
      return Response.redirect("https://stackql-deploy.io", 301);
    }

    const ua = req.headers.get("user-agent") ?? "";
    const asset = getAssetName(ua);
    return Response.redirect(`${RELEASE_BASE}/${asset}`, 302);
  },
} satisfies ExportedHandler;
