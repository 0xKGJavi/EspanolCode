#!/usr/bin/env sh
# Crea el instalador .pkg de macOS (un solo programa para Intel y Apple Silicon).
# Antes hay que compilar las dos versiones:
#   cargo build --release --target x86_64-apple-darwin
#   cargo build --release --target aarch64-apple-darwin
# Uso: sh instaladores/macos/construir-pkg.sh 0.12.0
set -eu
version=$1
raiz=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
aqui="$raiz/instaladores/macos"
trabajo=$(mktemp -d)
carga="$trabajo/carga"
mkdir -p "$carga/usr/local/bin" "$carga/usr/local/share/espanolcode/ejemplos" "$raiz/dist"

lipo -create -output "$carga/usr/local/bin/espanolcode" \
  "$raiz/target/x86_64-apple-darwin/release/espanolcode" \
  "$raiz/target/aarch64-apple-darwin/release/espanolcode"
chmod 755 "$carga/usr/local/bin/espanolcode"
cp "$raiz"/ejemplos/*.esp "$carga/usr/local/share/espanolcode/ejemplos/"
cp "$raiz/README.md" "$carga/usr/local/share/espanolcode/"

pkgbuild --root "$carga" --identifier io.github.0xkgjavi.espanolcode \
  --version "$version" --install-location / "$trabajo/espanolcode.pkg"
sed "s/__VERSION__/$version/" "$aqui/distribution.xml" > "$trabajo/distribution.xml"
productbuild --distribution "$trabajo/distribution.xml" --resources "$aqui/recursos" \
  --package-path "$trabajo" "$raiz/dist/EspanolCode-$version-macos.pkg"
rm -rf "$trabajo"
echo "Instalador creado: dist/EspanolCode-$version-macos.pkg"
