#!/usr/bin/env sh
# Lee la versión de Cargo.toml y, si se pasa una etiqueta de release, comprueba que coincide.
# Uso: sh instaladores/comprobar-version.sh [etiqueta]   -> imprime la versión (p. ej. 0.12.0)
set -eu
raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=$(tr -d '\r' < "$raiz/Cargo.toml" | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
if [ -z "$version" ]; then
  echo "No encuentro la versión en Cargo.toml" >&2
  exit 1
fi
etiqueta=${1:-}
if [ -n "$etiqueta" ] && [ "$etiqueta" != "v$version" ]; then
  echo "La etiqueta de la release ($etiqueta) no coincide con la versión de Cargo.toml ($version)." >&2
  echo "Cambia 'version' en Cargo.toml o usa la etiqueta v$version." >&2
  exit 1
fi
echo "$version"
