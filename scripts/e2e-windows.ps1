# Real Tauri E2E harness, Windows counterpart to scripts/e2e-linux.sh
# (docs/TEST_IMPLEMENTATION_PLAN.md item 2.6). Installs the real NSIS
# package silently to a known, fixed directory (same technique already
# verified in .github/workflows/release.yml's install/uninstall check -
# NSIS's /D= must be the last, unquoted argument), isolates app data to a
# throwaway profile, then runs the same E2E suite as Linux against the
# installed .exe. @wdio/tauri-service's embedded provider manages the
# app+driver lifecycle itself (no external msedgedriver/tauri-driver process
# to install or version-match anymore - see wdio.conf.mjs for why that
# approach was dropped).
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$WdioArgs = @()
)
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

$installer = Get-ChildItem -Path "src-tauri/target/release/bundle/nsis" -Filter "*.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $installer) {
    Write-Error "No NSIS installer found under src-tauri/target/release/bundle/nsis. Build it first (pnpm tauri build --bundles nsis --features wdio-e2e)."
    exit 1
}

$installDir = Join-Path $env:RUNNER_TEMP "InterviewScribeE2E"
if (Test-Path $installDir) { Remove-Item $installDir -Recurse -Force }
Start-Process -FilePath $installer.FullName -ArgumentList "/S", "/D=$installDir" -Wait

$appExe = Get-ChildItem -Path $installDir -Filter "*.exe" -Recurse |
Where-Object { $_.Name -notlike "*ninstall*" } | Select-Object -First 1
if (-not $appExe) {
    Write-Error "No application executable found under $installDir after install."
    exit 1
}
$env:INTERVIEWSCRIBE_E2E_BINARY = $appExe.FullName
Write-Host "Installed: $($appExe.FullName)"
$firewallRuleName = "InterviewScribe-E2E-Offline-$PID"

# Isolate app data (Tauri resolves app_data_dir() from %APPDATA% on
# Windows) to a throwaway profile - never the real user's interviews.
$profileDir = Join-Path $env:RUNNER_TEMP "InterviewScribeE2EProfile"
New-Item -ItemType Directory -Force -Path $profileDir | Out-Null
$env:APPDATA = $profileDir
$env:LOCALAPPDATA = $profileDir

try {
    if ($env:INTERVIEWSCRIBE_E2E_OFFLINE -eq "1") {
        New-NetFirewallRule -DisplayName $firewallRuleName -Direction Outbound -Program $appExe.FullName -Action Block | Out-Null
    }
    pnpm exec wdio run e2e/wdio.conf.mjs @WdioArgs
    $testExitCode = $LASTEXITCODE
}
finally {
    Remove-NetFirewallRule -DisplayName $firewallRuleName -ErrorAction SilentlyContinue
    Get-Process -Name ($appExe.BaseName) -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Remove-Item $installDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $profileDir -Recurse -Force -ErrorAction SilentlyContinue
}

exit $testExitCode
