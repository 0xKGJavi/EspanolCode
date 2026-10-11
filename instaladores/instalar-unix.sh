#!/usr/bin/env sh
set -eu

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ejecutable="$raiz/target/release/espanolcode"
destino="${HOME}/.local/bin"

if [ ! -f "$ejecutable" ]; then
  echo "No encuentro $ejecutable. Primero compila con: cargo build --release" >&2
  exit 1
fi
mkdir -p "$destino"
install -m 755 "$ejecutable" "$destino/espanolcode"
printf 'EspañolCode se instaló en %s/espanolcode\n' "$destino"
case ":${PATH}:" in
  *":$destino:"*) ;;
  *) echo "Añade esta línea al archivo de inicio de tu terminal: export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
esac
