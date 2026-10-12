const GITHUB_REPO = "stackql/stackql-deploy-rs";
const RELEASE_BASE = `https://github.com/${GITHUB_REPO}/releases/latest/download`;
const RELEASES_URL = `https://github.com/${GITHUB_REPO}/releases/latest`;
const DOCS_URL = "https://stackql.io/docs/installing-stackql";

// Browser / known-OS callers send a User-Agent that identifies the platform, so
// the root URL can redirect them straight to the correct release asset. CLI
// download tools (curl, wget, PowerShell) don't carry the OS reliably, so we
// expose installer endpoints that detect OS + arch locally instead:
//
//   /install      - auto-detects the caller's shell from the User-Agent and
//                   serves the matching installer (sh for curl/wget, PowerShell
//                   for irm/iwr). This is the universal one-liner.
//   /install.sh   - always the POSIX sh installer (verbose / explicit).
//   /install.ps1  - always the PowerShell installer (verbose / explicit).
//
// The explicit endpoints also guard against being run in the wrong shell: if the
// sh installer is fetched by PowerShell (or the PowerShell installer by
// curl/wget), we serve a short message - written in the language of the shell
// that's about to run it - that points at the right command. Each installer also
// guards at runtime (uname / $PSVersionTable) for cases the User-Agent missed,
// e.g. curl inside Git Bash on Windows.
//
// The root URL behaviour is unchanged, so the existing
// `curl -L https://get-stackql-deploy.io | tar xzf -` one-liner keeps working.

function getAssetName(ua: string): string {
  if (/windows/i.test(ua)) return "stackql-deploy-windows-x86_64.zip";
  if (/darwin|macintosh|mac os/i.test(ua)) return "stackql-deploy-macos-universal.tar.gz";
  return "stackql-deploy-linux-x86_64.tar.gz";
}

// True when the caller is PowerShell (Invoke-WebRequest / Invoke-RestMethod),
// which sets a User-Agent like "Mozilla/5.0 (Windows NT...) WindowsPowerShell/5.1".
function isPowerShell(ua: string): boolean {
  return /powershell/i.test(ua);
}

// True when the caller is a POSIX download tool that will pipe the body into sh.
function isPosixShellTool(ua: string): boolean {
  return /\bcurl\b|\bwget\b/i.test(ua);
}

const INSTALL_SH = `#!/bin/sh
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
      *)
        echo "stackql-deploy: there's no prebuilt Linux binary for your CPU ($arch)." >&2
        echo "Prebuilt Linux builds cover x86_64 (amd64) and arm64 (aarch64)." >&2
        echo "Browse all downloads: ${RELEASES_URL}" >&2
        exit 1
        ;;
    esac
    ;;
  MINGW* | MSYS* | CYGWIN* | Windows_NT)
    echo "stackql-deploy: this looks like Windows in a POSIX shell ($os)." >&2
    echo "The Windows build installs from PowerShell. Open PowerShell and run:" >&2
    echo "" >&2
    echo "    irm https://get-stackql-deploy.io/install.ps1 | iex" >&2
    echo "" >&2
    echo "Already in WSL and want the Linux build? Run this from your WSL shell." >&2
    exit 1
    ;;
  *)
    echo "stackql-deploy: this installer doesn't recognize your system ($os $arch)." >&2
    echo "See the install guide for every option: ${DOCS_URL}" >&2
    echo "Or download a binary directly: ${RELEASES_URL}" >&2
    exit 1
    ;;
esac

echo "Downloading $asset ..."
curl -fsSL "$base/$asset" | tar xzf -
chmod +x stackql-deploy
echo "Installed ./stackql-deploy ($os/$arch). Run it with ./stackql-deploy or move it onto your PATH."
`;

const INSTALL_PS1 = `#Requires -Version 5
# stackql-deploy installer - https://get-stackql-deploy.io/install.ps1
# Detects the native Windows architecture (AMD64 or ARM64), downloads the matching
# latest release archive and extracts stackql-deploy.exe into the current directory.
# Usage: irm https://get-stackql-deploy.io/install.ps1 | iex
#
# Everything runs in a script block so 'irm | iex' leaves no variables or
# preference changes behind in the caller's session.
& {
  $ErrorActionPreference = 'Stop'
  # Invoke-WebRequest's progress bar slows large downloads badly on Windows
  # PowerShell 5.1.
  $ProgressPreference = 'SilentlyContinue'

  # Runtime guard: PowerShell also runs on macOS/Linux, where this Windows build
  # won't help. Point those callers at the sh installer instead.
  if ($PSVersionTable.Platform -eq 'Unix') {
    Write-Host "stackql-deploy: this is the Windows installer, but you're on PowerShell on a non-Windows OS."
    Write-Host "Install with:"
    Write-Host ""
    Write-Host "    curl -fsSL https://get-stackql-deploy.io/install.sh | sh"
    Write-Host ""
    return
  }

  $base = "${RELEASE_BASE}"

  # Detect the native OS architecture. $env:PROCESSOR_ARCHITECTURE describes this
  # PowerShell process, so it reads AMD64 when x64 PowerShell runs under emulation
  # on ARM64. The system value in the registry is the same for every process, so
  # prefer it, then the WOW64 variable, then the process variable.
  $arch = $null
  try {
    $arch = (Get-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment' -Name PROCESSOR_ARCHITECTURE -ErrorAction Stop).PROCESSOR_ARCHITECTURE
  } catch {}
  if (-not $arch) { $arch = $env:PROCESSOR_ARCHITEW6432 }
  if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
  if (-not $arch) {
    Write-Host "stackql-deploy: couldn't detect your CPU architecture, defaulting to x64 (AMD64)."
    $arch = 'AMD64'
  }

  if ($arch -eq 'ARM64') {
    $asset = 'stackql-deploy-windows-arm64.zip'
  } elseif ($arch -eq 'AMD64') {
    $asset = 'stackql-deploy-windows-x86_64.zip'
  } else {
    Write-Host "stackql-deploy: there's no prebuilt Windows binary for your CPU ($arch)."
    Write-Host "Prebuilt Windows builds cover x64 (AMD64) and ARM64."
    Write-Host "Browse all downloads: ${RELEASES_URL}"
    return
  }

  $dest = (Get-Location).Path
  $zip = Join-Path $dest $asset
  Write-Host "Downloading $asset ..."
  Invoke-WebRequest -Uri "$base/$asset" -OutFile $zip
  Expand-Archive -Path $zip -DestinationPath $dest -Force
  Remove-Item $zip
  Write-Host "Installed .\\stackql-deploy.exe (Windows/$arch). Run it with .\\stackql-deploy.exe or move it onto your PATH."
}
`;

// Served when the sh installer is fetched by PowerShell - valid PowerShell that
// just points the user at the Windows one-liner.
const GUIDE_USE_PS1 = `# stackql-deploy - wrong installer for this shell.
Write-Host "stackql-deploy: that's the Linux/macOS installer."
Write-Host "On Windows, install with:"
Write-Host ""
Write-Host "    irm https://get-stackql-deploy.io/install.ps1 | iex"
Write-Host ""
`;

// Served when the PowerShell installer is fetched by curl/wget - valid sh that
// just points the user at the Linux/macOS one-liner.
const GUIDE_USE_SH = `#!/bin/sh
# stackql-deploy - wrong installer for this shell.
echo "stackql-deploy: that's the Windows (PowerShell) installer."
echo "On macOS or Linux, install with:"
echo ""
echo "    curl -fsSL https://get-stackql-deploy.io/install.sh | sh"
echo ""
exit 1
`;

function shResponse(body: string): Response {
  return new Response(body, {
    headers: { "content-type": "text/x-shellscript; charset=utf-8" },
  });
}

function ps1Response(body: string): Response {
  return new Response(body, {
    headers: { "content-type": "text/plain; charset=utf-8" },
  });
}

export default {
  fetch(req: Request): Response {
    const url = new URL(req.url);
    const ua = req.headers.get("user-agent") ?? "";

    // Universal installer: pick the script that matches the calling shell.
    if (url.pathname === "/install") {
      return isPowerShell(ua) ? ps1Response(INSTALL_PS1) : shResponse(INSTALL_SH);
    }

    // Explicit POSIX installer. If PowerShell fetched it, hand back a PowerShell
    // message instead of sh it can't run.
    if (url.pathname === "/install.sh") {
      return isPowerShell(ua) ? ps1Response(GUIDE_USE_PS1) : shResponse(INSTALL_SH);
    }

    // Explicit PowerShell installer. If curl/wget fetched it (i.e. it's about to
    // be piped into sh), hand back an sh message instead of PowerShell.
    if (url.pathname === "/install.ps1") {
      return isPosixShellTool(ua) ? shResponse(GUIDE_USE_SH) : ps1Response(INSTALL_PS1);
    }

    if (url.pathname !== "/") {
      return Response.redirect("https://stackql-deploy.io", 301);
    }

    const asset = getAssetName(ua);
    return Response.redirect(`${RELEASE_BASE}/${asset}`, 302);
  },
} satisfies ExportedHandler;
