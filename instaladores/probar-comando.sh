#!/usr/bin/env sh
# Comprueba que EspañolCode quedó instalado: el comando existe, dice su versión
# y ejecuta un ejemplo. Lo usan las pruebas de Linux y macOS.
# Uso: sh instaladores/probar-comando.sh <carpeta de ejemplos> <versión>
set -eu
ejemplos=$1
version=$2
ruta=$(command -v espanolcode) || { echo "No encuentro el comando espanolcode en el PATH" >&2; exit 1; }
echo "Comando instalado en $ruta"
espanolcode --version | grep -F "$version"
cd "$(mktemp -d)"
salida=$(espanolcode "$ejemplos/hola.esp")
printf '%s\n' "$salida"
printf '%s\n' "$salida" | grep -q "Hola mundo"
echo "OK: EspañolCode funciona"
