# Prueba el instalador de Windows de principio a fin, sin ventanas: instala dos veces
# (reinstalar no debe duplicar nada), comprueba el comando, el PATH y el doble clic en
# un .esp, desinstala y comprueba que no queda rastro.
# Uso: .\instaladores\windows\probar-instalador.ps1 -Instalador dist\EspanolCode-0.12.0-windows.exe -Version 0.12.0 [-Modo todos]
param(
    [Parameter(Mandatory = $true)][string]$Instalador,
    [Parameter(Mandatory = $true)][string]$Version,
    [ValidateSet('usuario', 'todos')][string]$Modo = 'usuario'
)
$ErrorActionPreference = 'Stop'

if ($Modo -eq 'todos') {
    $modoArg = '/ALLUSERS'
    $app = Join-Path $env:ProgramFiles 'EspanolCode'
    $claveEntorno = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
    $claveClases = 'HKLM:\Software\Classes'
} else {
    $modoArg = '/CURRENTUSER'
    $app = Join-Path $env:LOCALAPPDATA 'Programs\EspanolCode'
    $claveEntorno = 'HKCU:\Environment'
    $claveClases = 'HKCU:\Software\Classes'
}
$Instalador = (Resolve-Path -LiteralPath $Instalador).Path
$exe = Join-Path $app 'espanolcode.exe'
$carpetaProgramas = if ($Modo -eq 'todos') { [Environment]::GetFolderPath('CommonPrograms') } else { [Environment]::GetFolderPath('Programs') }
$menuInicio = Join-Path $carpetaProgramas ('Espa' + [char]0x00F1 + 'olCode')

function Fallar([string]$mensaje) { throw "PRUEBA FALLIDA: $mensaje" }

function Contar-EnPath([string]$carpeta) {
    # Lee el PATH tal cual está guardado, sin expandir variables como %USERPROFILE%.
    $path = [string](Get-Item -LiteralPath $claveEntorno).GetValue('Path', '', 'DoNotExpandEnvironmentNames')
    $buscada = $carpeta.TrimEnd('\')
    return @($path -split ';' | Where-Object { $_.Trim().TrimEnd('\') -ieq $buscada }).Count
}

function Valor-Predeterminado([string]$clave) {
    if (-not (Test-Path -LiteralPath $clave)) { return $null }
    return (Get-Item -LiteralPath $clave).GetValue('')
}

function Instalar {
    $p = Start-Process -FilePath $Instalador -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART $modoArg /TASKS=`"path,asociar`"" -Wait -PassThru
    if ($p.ExitCode -ne 0) { Fallar "el instalador terminó con código $($p.ExitCode)" }
}

Write-Host "== Modo: $Modo ($app)"
# Si .esp no estaba registrado antes, tras desinstalar no debe quedar ni la clave vacía.
$espExistiaAntes = Test-Path -LiteralPath "$claveClases\.esp"
Write-Host '1. Instalar y reinstalar encima'
Instalar
Instalar

Write-Host '2. Comando, ejemplo y menú Inicio'
if (-not (Test-Path -LiteralPath $exe)) { Fallar "no existe $exe" }
$salida = (& $exe --version) -join "`n"
if ($salida -notmatch [regex]::Escape($Version)) { Fallar "--version dice '$salida' y esperaba $Version" }
$salida = (& $exe (Join-Path $app 'ejemplos\hola.esp')) -join "`n"
if ($salida -notmatch 'Hola mundo') { Fallar "hola.esp no mostró 'Hola mundo': $salida" }

foreach ($acceso in @(('Espa' + [char]0x00F1 + 'olCode.lnk'), ('Ejemplos de Espa' + [char]0x00F1 + 'olCode.lnk'), ('Desinstalar Espa' + [char]0x00F1 + 'olCode.lnk'))) {
    if (-not (Test-Path -LiteralPath (Join-Path $menuInicio $acceso))) { Fallar "falta el acceso del menú Inicio: $acceso" }
}

Write-Host '3. PATH sin duplicados'
$veces = Contar-EnPath $app
if ($veces -ne 1) { Fallar "la carpeta aparece $veces veces en el PATH (esperaba 1)" }

Write-Host '4. Asociación y doble clic en una carpeta con espacios, &, paréntesis y eñe'
if ((Valor-Predeterminado "$claveClases\.esp") -ne 'EspanolCode.Programa') { Fallar '.esp no está asociado a EspanolCode.Programa' }
$comando = Valor-Predeterminado "$claveClases\EspanolCode.Programa\shell\open\command"
if ($comando -notmatch '^"([^"]+)"\s+(.*)$') { Fallar "comando de doble clic inesperado: $comando" }
$ejecutable = $Matches[1]
$plantilla = $Matches[2]
$carpeta = Join-Path $env:TEMP ("prueba espanolcode & " + [char]0x00F1 + " (1)")
New-Item -ItemType Directory -Force -Path $carpeta | Out-Null
$programa = Join-Path $carpeta 'mi programa.esp'
[IO.File]::WriteAllText($programa, 'escribir_archivo("resultado.txt", "funciona")')
$argumentos = $plantilla.Replace('%1', $programa)
$enter = Join-Path $env:TEMP 'espanolcode-prueba-enter.txt'
[IO.File]::WriteAllText($enter, "`r`n")
Start-Process -FilePath $ejecutable -ArgumentList $argumentos -RedirectStandardInput $enter -Wait -NoNewWindow
$resultado = Join-Path $carpeta 'resultado.txt'
if (-not (Test-Path -LiteralPath $resultado)) { Fallar 'el doble clic no ejecutó el programa (o no lo ejecutó en su carpeta)' }
if ((Get-Content -LiteralPath $resultado -Raw) -ne 'funciona') { Fallar 'resultado.txt no tiene el contenido esperado' }
Remove-Item -LiteralPath $carpeta -Recurse -Force
Remove-Item -LiteralPath $enter -Force

Write-Host '5. Desinstalar'
Start-Process -FilePath (Join-Path $app 'unins000.exe') -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -Wait
# El desinstalador se copia a TEMP y sigue trabajando desde allí; esperamos a que termine de verdad.
$limite = (Get-Date).AddMinutes(2)
while ((Test-Path -LiteralPath $exe) -and ((Get-Date) -lt $limite)) { Start-Sleep -Seconds 1 }
if (Test-Path -LiteralPath $exe) { Fallar 'el programa sigue instalado tras desinstalar' }
if ((Contar-EnPath $app) -ne 0) { Fallar 'la carpeta sigue en el PATH tras desinstalar' }
if ((Valor-Predeterminado "$claveClases\.esp") -eq 'EspanolCode.Programa') { Fallar '.esp sigue asociado tras desinstalar' }
if (-not $espExistiaAntes -and (Test-Path -LiteralPath "$claveClases\.esp")) { Fallar 'queda la clave .esp en el registro tras desinstalar' }
if (Test-Path -LiteralPath "$claveClases\EspanolCode.Programa") { Fallar 'la clave EspanolCode.Programa sigue en el registro' }
if (Test-Path -LiteralPath $menuInicio) { Fallar 'la carpeta del menú Inicio sigue ahí tras desinstalar' }

Write-Host "OK: el instalador funciona en modo $Modo"
