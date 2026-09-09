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

    $process = Start-Process -FilePath $appExe.FullName -PassThru
    if (-not $process.WaitForExit(20000)) {
        $process.Kill()
        $process.WaitForExit()
    }
    elseif ($process.ExitCode -ne 0) {
        throw "Le lancement de la version N a echoue: code $($process.ExitCode)"
    }

    if ((& sqlite3 $database "PRAGMA user_version") -ne "1") { throw "user_version attendu: 1" }
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
