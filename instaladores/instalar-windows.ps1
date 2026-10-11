$ErrorActionPreference = 'Stop'
$raiz = Split-Path -Parent $PSScriptRoot
$ejecutable = Join-Path $raiz 'target\release\espanolcode.exe'

if (-not (Test-Path -LiteralPath $ejecutable -PathType Leaf)) {
    throw "No encuentro $ejecutable. Primero compila con: cargo build --release"
}

$carpetaBin = Join-Path $env:LOCALAPPDATA 'Programs\EspanolCode'
New-Item -ItemType Directory -Force -Path $carpetaBin | Out-Null
Copy-Item -LiteralPath $ejecutable -Destination (Join-Path $carpetaBin 'espanolcode.exe') -Force

$pathUsuario = [Environment]::GetEnvironmentVariable('Path', 'User')
$partesPath = @($pathUsuario -split ';' | Where-Object { $_ })
if ($partesPath -notcontains $carpetaBin) {
    $nuevoPath = (@($partesPath) + $carpetaBin) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $nuevoPath, 'User')
}

Write-Host "EspañolCode se instaló en: $carpetaBin"
Write-Host 'Cierra y vuelve a abrir la terminal. Después prueba: espanolcode --ayuda'
