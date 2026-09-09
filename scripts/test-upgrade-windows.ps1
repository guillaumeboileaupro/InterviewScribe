param(
    [Parameter(Mandatory = $true)][string]$OldInstaller,
    [Parameter(Mandatory = $true)][string]$NewInstaller
)

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

foreach ($installer in @($OldInstaller, $NewInstaller)) {
    if (-not (Test-Path -LiteralPath $installer -PathType Leaf) -or [IO.Path]::GetExtension($installer) -ne ".exe") {
        throw "Installateur NSIS introuvable: $installer"
    }
}
if (-not (Get-Command sqlite3 -ErrorAction SilentlyContinue)) {
    throw "sqlite3 est requis pour verifier la migration"
}

$installDir = Join-Path $env:RUNNER_TEMP "InterviewScribeUpgrade"
$profileDir = Join-Path $env:RUNNER_TEMP "InterviewScribeUpgradeProfile"
$appDataDir = Join-Path $profileDir "com.guillaumeboileau.interviewscribe"
$database = Join-Path $appDataDir "interviewscribe.sqlite3"
$previousAppData = $env:APPDATA
$previousLocalAppData = $env:LOCALAPPDATA

function Find-AppExecutable {
    Get-ChildItem -LiteralPath $installDir -Filter "*.exe" -Recurse |
        Where-Object { $_.Name -notlike "*ninstall*" } |
        Select-Object -First 1
}

try {
    if (Test-Path -LiteralPath $installDir) { Remove-Item -LiteralPath $installDir -Recurse -Force }
    if (Test-Path -LiteralPath $profileDir) { Remove-Item -LiteralPath $profileDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $appDataDir | Out-Null
    $env:APPDATA = $profileDir
    $env:LOCALAPPDATA = $profileDir

    Start-Process -FilePath $OldInstaller -ArgumentList "/S", "/D=$installDir" -Wait
    if (-not (Find-AppExecutable)) { throw "La version N-1 ne s'est pas installee" }

    Get-Content -LiteralPath "tests/fixtures/db/schema-v0.sql" -Raw | & sqlite3 $database
    if ($LASTEXITCODE -ne 0) { throw "Impossible de creer la fixture N-1" }

    Start-Process -FilePath $NewInstaller -ArgumentList "/S", "/D=$installDir" -Wait
    $appExe = Find-AppExecutable
    if (-not $appExe) { throw "La version N ne s'est pas installee" }

    # Poll for the migration's own completion signal instead of waiting a
    # fixed duration then killing and hoping: a GUI app never exits on its
    # own, so a fixed-wait-then-kill race can inspect the database before
    # schema::init has actually committed user_version - this was the real,
    # confirmed cause of a prior release failure (see
    # docs/TEST_IMPLEMENTATION_PLAN.md section 9).
    $process = Start-Process -FilePath $appExe.FullName -PassThru
    $migrated = $false
    $deadline = (Get-Date).AddSeconds(20)
    while ((Get-Date) -lt $deadline) {
        if ((& sqlite3 $database "PRAGMA user_version") -eq "1") {
            $migrated = $true
            break
        }
        if ($process.HasExited) {
            Write-Warning "Le processus de la version N s'est arrete avant la fin de la migration (code $($process.ExitCode))"
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $process.HasExited) {
        $process.Kill()
        $process.WaitForExit()
    }
    if (-not $migrated) { throw "user_version n'a pas atteint 1 dans le delai imparti (20s)" }
    if ((& sqlite3 $database "SELECT COUNT(*) FROM pragma_table_info('interview') WHERE name='notes'") -ne "1") { throw "colonne notes absente" }
    if ((& sqlite3 $database "SELECT COUNT(*) FROM pragma_table_info('edit') WHERE name='reverted_at'") -ne "1") { throw "colonne reverted_at absente" }
    foreach ($table in @("interview", "speaker", "segment", "edit", "setting")) {
        if ((& sqlite3 $database "SELECT COUNT(*) FROM $table") -ne "1") { throw "donnee perdue dans $table" }
    }
    if ((& sqlite3 $database "SELECT raw_text FROM segment WHERE id=1") -ne "Texte synthetique immuable.") {
        throw "texte brut modifie pendant la mise a niveau"
    }
    Write-Host "Mise a niveau Windows validee; schema et donnees conserves."
}
finally {
    Get-Process -Name "interviewscribe" -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
    $uninstaller = Get-ChildItem -LiteralPath $installDir -Filter "*ninstall*.exe" -Recurse -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($uninstaller) { Start-Process -FilePath $uninstaller.FullName -ArgumentList "/S" -Wait }
    if ($installDir.StartsWith($env:RUNNER_TEMP) -and (Test-Path -LiteralPath $installDir)) {
        Remove-Item -LiteralPath $installDir -Recurse -Force
    }
    if ($profileDir.StartsWith($env:RUNNER_TEMP) -and (Test-Path -LiteralPath $profileDir)) {
        Remove-Item -LiteralPath $profileDir -Recurse -Force
    }
    $env:APPDATA = $previousAppData
    $env:LOCALAPPDATA = $previousLocalAppData
}
