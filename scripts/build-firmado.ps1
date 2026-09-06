<#
    Compila Desktop Organizer y firma los artefactos del updater con el par de
    claves vigente (el mismo que verifica la aplicacion instalada).

    Uso:  npm run build:firmado
          powershell -ExecutionPolicy Bypass -File scripts\build-firmado.ps1

    La clave privada nunca vive en el repositorio: se lee de la carpeta de
    claves del usuario y solo se expone como variable de entorno de este proceso.
#>

$ErrorActionPreference = 'Stop'

$keyDir   = Join-Path $env:APPDATA 'Desktop Organizer\updater'
$keyFile  = Join-Path $keyDir 'tauri-updater.key'
$pubFile  = Join-Path $keyDir 'tauri-updater.key.pub'
$passFile = Join-Path $keyDir 'tauri-updater-password.txt'

$repoRoot = Split-Path -Parent $PSScriptRoot
$confFile = Join-Path $repoRoot 'src-tauri\tauri.conf.json'

Write-Host '=== Clave de firma del updater ===' -ForegroundColor Cyan

foreach ($f in @($keyFile, $pubFile, $passFile)) {
    if (-not (Test-Path $f)) {
        Write-Host "FALTA el archivo de clave: $f" -ForegroundColor Red
        Write-Host 'Sin ese archivo no se pueden firmar actualizaciones.' -ForegroundColor Red
        exit 1
    }
}

# La clave publica del par debe coincidir con la que lleva incrustada la app.
# Si no coinciden, la actualizacion automatica fallaria en el equipo del usuario.
$pubKey  = (Get-Content $pubFile -Raw).Trim()
$confPub = (Get-Content $confFile -Raw | ConvertFrom-Json).plugins.updater.pubkey

if ($pubKey -ne $confPub) {
    Write-Host 'ERROR: la clave publica del par no coincide con tauri.conf.json' -ForegroundColor Red
    Write-Host "  par de claves : $pubKey"
    Write-Host "  tauri.conf    : $confPub"
    Write-Host 'Firmar asi dejaria a los usuarios instalados sin poder actualizar.' -ForegroundColor Red
    exit 1
}

Write-Host '  Par de claves y tauri.conf.json coinciden.' -ForegroundColor Green

$env:TAURI_SIGNING_PRIVATE_KEY          = (Get-Content $keyFile  -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = (Get-Content $passFile -Raw).Trim()

try {
    Write-Host ''
    Write-Host '=== Compilando e instalador firmado ===' -ForegroundColor Cyan
    Push-Location $repoRoot
    try {
        & npm run tauri build
        if ($LASTEXITCODE -ne 0) { throw "npm run tauri build fallo con codigo $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
}
finally {
    # No dejar la clave viva en el entorno mas alla de la compilacion.
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY          -ErrorAction SilentlyContinue
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
}

$bundle = Join-Path $repoRoot 'src-tauri\target\release\bundle\nsis'
Write-Host ''
Write-Host '=== Artefactos generados ===' -ForegroundColor Cyan
Get-ChildItem $bundle -Filter '*.exe*' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 4 Name, Length, LastWriteTime |
    Format-Table -AutoSize

Write-Host 'Listo: el .exe y su .sig quedaron firmados con la clave vigente.' -ForegroundColor Green
