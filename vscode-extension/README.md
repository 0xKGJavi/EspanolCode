# Extensión EspañolCode para Visual Studio Code

La extensión reconoce archivos `.esp`, colorea palabras reservadas y texto, muestra advertencias/errores básicos y ejecuta el programa en la terminal integrada de VS Code, para que puedas responder a preguntas interactivas. Incluye el logotipo EspañolCode.

## Instalar para probar mientras desarrollas

1. Instala Visual Studio Code y el ejecutable EspañolCode.
2. Abre una terminal en esta carpeta (`vscode-extension`).
3. Ejecuta `code --extensionDevelopmentPath="$PWD"` en macOS/Linux, o en PowerShell:

   ```powershell
   code --extensionDevelopmentPath="$PWD"
   ```

4. En la nueva ventana de VS Code, abre un archivo `.esp`. Pulsa `Ctrl+Alt+R` o abre la paleta de comandos con `Ctrl+Shift+P` y elige **EspañolCode: Ejecutar archivo actual**.

Para esta forma de prueba, `code` debe estar disponible en la terminal. También puedes abrir la carpeta `vscode-extension` desde VS Code y pulsar `F5` para abrir la ventana de desarrollo de extensiones.

## Crear un instalador `.vsix`

Un `.vsix` es el paquete instalable de una extensión de VS Code. Solo quien crea el paquete necesita Node.js y npm; quienes lo instalan no necesitan Python, Rust ni Node.js.

Desde esta carpeta, ejecuta:

```powershell
npx --yes @vscode/vsce package --allow-missing-repository
```

Esto crea `espanolcode-vscode-0.12.0.vsix`. Para instalarlo en VS Code:

```powershell
code --install-extension .\espanolcode-vscode-0.12.0.vsix
```

En macOS/Linux cambia la ruta de Windows por `./espanolcode-vscode-0.12.0.vsix`. También puedes abrir la vista **Extensiones**, pulsar `...` y elegir **Instalar desde VSIX...**.

## Ejecutable del lenguaje

Por defecto, la extensión ejecuta `espanolcode` encontrado mediante `PATH`. Si usaste una instalación portátil o no está en `PATH`, abre los ajustes de VS Code, busca **EspañolCode Ruta Ejecutable** y escribe la ruta completa al programa; por ejemplo:

```text
C:\Users\TuNombre\AppData\Local\Programs\EspanolCode\espanolcode.exe
```

## Qué comprueba por ahora

- `decir "Hola mundo"` muestra texto; `decir nombre` muestra el valor de una variable existente.
- `guardar nombre = "Ana"` crea o cambia una variable. También admite números sencillos y copiar otra variable.
- `guardar nombre = preguntar "¿Cómo te llamas?"` muestra una pregunta y guarda la respuesta.
- `decir 2 + 3` y `guardar total = precio * cantidad` aceptan operaciones matemáticas; `+` también une textos.
- `si` admite comparaciones con números o textos y ejecuta el bloque indentado; `si no` cubre el caso contrario.
- `repetir N veces` ejecuta el bloque indentado; el contador debe ser un entero de 0 a 100000.
- Las listas aceptan `[...]`, índices desde 0, `agregar lista = valor` y `cantidad(lista)`.
- Los archivos aceptan `leer_archivo("ruta")`, `escribir_archivo("ruta", texto)` y `agregar_archivo("ruta", texto)`.
- Las funciones integradas incluyen `absoluto`, `redondear`, `minimo`, `maximo`, `texto` y `numero`.
- Las funciones propias aceptan expresiones o bloques con `guardar`, `devolver`, `si`/`si no` y `repetir`; las variables locales no cambian las de fuera.
- Una instrucción mal formada, una condición sin comparación o el uso de una variable antes de guardarla se marca como error.
- Por ahora no se pueden definir funciones dentro de otras funciones.
- Los errores se muestran al pasar el cursor sobre el subrayado o en el panel **Problemas**.
- La salida aparece en la terminal integrada **EspañolCode**, donde también puedes escribir respuestas cuando el programa pregunte.
