# Real Tauri E2E harness, Windows counterpart to scripts/e2e-linux.sh
# (docs/TEST_IMPLEMENTATION_PLAN.md item 2.6). Installs the real NSIS
# package silently to a known, fixed directory (same technique already
# verified in .github/workflows/release.yml's install/uninstall check -
# NSIS's /D= must be the last, unquoted argument), isolates app data to a
# throwaway profile, downloads the msedgedriver build matching this
# machine's installed Edge (tauri-driver does not do this itself - it only
# looks for msedgedriver.exe on PATH), then runs the same E2E suite as
# Linux against the installed .exe.
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$WdioArgs = @()
)
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

$installer = Get-ChildItem -Path "src-tauri/target/release/bundle/nsis" -Filter "*.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $installer) {
    Write-Error "No NSIS installer found under src-tauri/target/release/bundle/nsis. Build it first (pnpm tauri build --bundles nsis)."
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

# Isolate app data (Tauri resolves app_data_dir() from %APPDATA% on
# Windows) to a throwaway profile - never the real user's interviews.
$profileDir = Join-Path $env:RUNNER_TEMP "InterviewScribeE2EProfile"
New-Item -ItemType Directory -Force -Path $profileDir | Out-Null
$env:APPDATA = $profileDir
$env:LOCALAPPDATA = $profileDir

# tauri-driver needs msedgedriver.exe on PATH, matching the installed Edge
# version exactly - it does not fetch this itself.
$edgePaths = @(
    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
    "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe"
)
$edgeExe = $edgePaths | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edgeExe) {
    Write-Error "Microsoft Edge not found - required for msedgedriver version matching."
    exit 1
}
$edgeVersion = (Get-Item $edgeExe).VersionInfo.ProductVersion
Write-Host "Edge version: $edgeVersion"

$driverDir = Join-Path $env:RUNNER_TEMP "msedgedriver"
New-Item -ItemType Directory -Force -Path $driverDir | Out-Null
$zipPath = Join-Path $driverDir "edgedriver_win64.zip"
Invoke-WebRequest -Uri "https://msedgedriver.microsoft.com/$edgeVersion/edgedriver_win64.zip" -OutFile $zipPath
Expand-Archive -Path $zipPath -DestinationPath $driverDir -Force
$env:PATH = "$driverDir;$env:PATH"

$driverLog = if ($env:INTERVIEWSCRIBE_E2E_DRIVER_LOG) { $env:INTERVIEWSCRIBE_E2E_DRIVER_LOG } else { Join-Path $env:RUNNER_TEMP "tauri-driver.log" }
$driverProcess = Start-Process -FilePath "tauri-driver" -ArgumentList "--port", "4444" `
    -RedirectStandardOutput $driverLog -RedirectStandardError "$driverLog.err" -PassThru -NoNewWindow

$deadline = (Get-Date).AddSeconds(15)
$ready = $false
while ((Get-Date) -lt $deadline) {
    try {
        (New-Object System.Net.Sockets.TcpClient("127.0.0.1", 4444)).Close()
        $ready = $true
        break
    }
    catch {
        Start-Sleep -Milliseconds 200
    }
}
if (-not $ready) {
    Write-Error "tauri-driver never opened port 4444."
}

try {
    pnpm exec wdio run e2e/wdio.conf.mjs @WdioArgs
    $testExitCode = $LASTEXITCODE
}
finally {
    # Kill the whole tree: tauri-driver plus the app instance(s) it spawned,
    # which the Linux script needed a process-group fix for too (orphans
    # skew later runs' timing).
    Stop-Process -Id $driverProcess.Id -Force -ErrorAction SilentlyContinue
    Get-Process -Name ($appExe.BaseName) -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Remove-Item $installDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $profileDir -Recurse -Force -ErrorAction SilentlyContinue
}

exit $testExitCode
