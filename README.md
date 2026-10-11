# EspañolCode 0.12 — texto, variables, preguntas, cálculos, condiciones, repeticiones, listas, archivos, funciones y bloques visuales

![Logotipo de EspañolCode](vscode-extension/media/espanolcode-logo.png)

EspañolCode es un lenguaje de programación pequeño e independiente: escribes instrucciones en español en un archivo `.esp` y el programa EspañolCode las ejecuta. Esta versión entiende `decir`, variables con `guardar`, preguntas con `preguntar`, operaciones matemáticas, condiciones, repeticiones, listas, archivos de texto, funciones propias por expresión o bloques indentados, un editor visual de bloques estilo Scratch y algunas funciones útiles en español. No necesita Python.

## Prueba rápida

El programa de ejemplo está en `ejemplos/hola.esp` y contiene:

```text
decir "Hola mundo"
```

Tras compilar e instalarlo, abre una **terminal** (la ventana donde escribes comandos) y ejecuta:

```text
espanolcode ejemplos/hola.esp
```

Deberías ver `Hola mundo` y `Estoy aprendiendo EspañolCode`.

## Editor visual de bloques

En la terminal ejecuta `espanolcode --bloques`. El ejecutable abre un editor local en el navegador: arrastra o pulsa piezas de `decir`, `guardar`, `agregar`, `si`, `repetir` y funciones; coloca unas dentro de otras, ajusta sus campos y pulsa **Ejecutar**. También puedes descargar el programa como `.esp`. El navegador se comunica solo con el runtime local; no necesita Python ni conexión a Internet.

Es una primera versión de bloques: no incluye los personajes, disfraces, sonidos ni sensores de Scratch, y todavía no tiene una pieza visual para cada instrucción.

## Variables

Una variable es una cajita con nombre para guardar un valor. Crea un archivo `variables.esp` con estas líneas:

```text
guardar nombre = "Carlos"
decir nombre

guardar edad = 18
decir edad

guardar nombre = "Ana"
decir nombre
```

`guardar` crea la cajita o cambia lo que hay dentro. `decir` puede mostrar texto entre comillas, números o el nombre de una variable. Ejecuta el ejemplo incluido con `espanolcode ejemplos/variables.esp`.

## Preguntar al usuario

`preguntar` hace una pregunta y guarda lo que la persona escribe:

```text
guardar nombre = preguntar "¿Cómo te llamas? "
decir nombre
```

Ejecuta `espanolcode ejemplos/preguntar.esp`. Al escribir la respuesta y pulsar Enter, EspañolCode la guarda en `nombre`.

## Operaciones matemáticas sencillas

Puedes sumar, restar, multiplicar y dividir con `+`, `-`, `*` y `/`. Los paréntesis ayudan a agrupar los cálculos; `*` y `/` se hacen antes que `+` y `-`.

```text
guardar precio = 12
guardar cantidad = 3
guardar total = precio * cantidad + 5
decir total
decir (2 + 3) * 4
```

`decir "Hola " + nombre` también une textos. Prueba todos estos ejemplos con `espanolcode ejemplos/calculos.esp`.

## Crear funciones sencillas

Una función con nombre puede recibir valores y calcular un resultado. Escribe el cálculo después de `=`; EspañolCode devuelve ese resultado al llamar la función:

```text
funcion doble(numero) = numero * 2
funcion saludo(nombre) = "Hola " + nombre

decir doble(5)
decir saludo("Ana")
```

Puedes usar funciones en cálculos y condiciones, y llamar una función desde otra. Para más de un paso, indenta el cuerpo y termina con `devolver`:

```text
funcion area_rectangulo(ancho, alto)
    guardar area = ancho * alto
    devolver area
```

Las variables creadas dentro de la función son locales y no cambian las de fuera. El cuerpo admite `si`, `si no` y `repetir` anidados; cada camino ejecutado debe llegar a `devolver` o la llamada dará un error. Aún no se pueden definir funciones dentro de otra función. Prueba `espanolcode ejemplos/funciones.esp`.

## Funciones útiles

EspañolCode incluye algunas herramientas sencillas: `absoluto(n)` quita el signo negativo; `redondear(n)` redondea al entero más cercano; `minimo(...)` y `maximo(...)` aceptan dos números o una lista numérica; `texto(valor)` convierte un valor a texto y `numero(texto)` convierte texto numérico en número.

```text
decir absoluto(-7)
decir redondear(3.6)
decir minimo([8, 3, 5])
decir maximo(8, 3)
guardar edad = numero("12")
decir texto(edad + 1)
```

Prueba `espanolcode ejemplos/funciones_basicas.esp`. Estas funciones cubren usos comunes, no toda la biblioteca de Python.

## Elegir con `si` y `si no`

Una condición comprueba algo. Si es cierto, ejecuta las líneas indentadas debajo de `si`; si no, usa las líneas debajo de `si no`. Indenta el bloque con cuatro espacios:

```text
guardar edad = 17
si edad >= 18
    decir "Eres mayor de edad"
si no
    decir "Eres menor de edad"
```

Se aceptan comparaciones `==`, `!=`, `>`, `<`, `>=` y `<=`. Prueba `espanolcode ejemplos/condiciones.esp`.

## Repetir instrucciones

`repetir N veces` ejecuta el bloque indentado esa cantidad de veces. `N` puede ser un cálculo o una variable numérica; debe ser un entero entre 0 y 100000.

```text
guardar contador = 0
repetir 3 veces
    guardar contador = contador + 1
decir contador
```

Prueba `espanolcode ejemplos/repeticiones.esp` para ver también repeticiones anidadas.

## Guardar y usar listas

Una lista guarda varios valores en orden. Los índices empiezan en 0; `agregar` añade un elemento al final y `cantidad(...)` cuenta elementos de una lista o texto.

```text
guardar nombres = ["Ana", "Luis"]
agregar nombres = "Eva"
decir nombres[0]
decir cantidad(nombres)
```

También puedes usar listas vacías, listas dentro de listas y unir listas con `+`. Prueba `espanolcode ejemplos/listas.esp`.

## Importaciones de módulos

EspañolCode reconoce la sintaxis española para las formas habituales de importación:

```text
importar math como matematicas
importar random, statistics
desde math importar sqrt como raiz
desde paquete.subpaquete importar (una, dos)
desde .utilidades importar ayuda
```

Puedes escribir nombres de módulos y elementos en español o conservar los nombres habituales de Python; `como` permite indicar un alias en cada importación. Por ahora no hay un catálogo que traduzca automáticamente los nombres de todos los módulos estándar. Se valida únicamente la sintaxis: EspañolCode no carga módulos de Python ni hace disponibles sus funciones o nombres importados. Al ejecutar una importación muestra un aviso para dejar clara esta limitación. Esto no requiere instalar Python.

## Leer y escribir archivos de texto

`leer_archivo(ruta)` lee el texto de un archivo. `escribir_archivo(ruta, texto)` crea o reemplaza un archivo; `agregar_archivo(ruta, texto)` añade texto al final.

```text
escribir_archivo("salida-demo.txt", "Primera línea\n")
agregar_archivo("salida-demo.txt", "Segunda línea\n")
guardar contenido = leer_archivo("salida-demo.txt")
decir contenido
```

Las rutas relativas se resuelven desde la carpeta de la terminal donde ejecutas `espanolcode`. Prueba `espanolcode ejemplos/archivos.esp`; creará `salida-demo.txt` en esa carpeta.

También puedes abrir el modo interactivo escribiendo `espanolcode`. Prueba `decir "Hola mundo"` o guarda una variable. Para salir, escribe `salir`.

## Compilar en Windows

Compilar convierte el código fuente de este proyecto en un programa ejecutable. Para compilar hace falta instalar Rust una vez en el ordenador de quien construye el proyecto. La persona que use el ejecutable instalado **no necesita Python ni Rust**.

1. Instala Rust desde <https://rustup.rs/> (solo para compilar).
2. Abre PowerShell en la carpeta de este proyecto.
3. Ejecuta:

   ```powershell
   cargo build --release
   ```

4. Instala EspañolCode para tu cuenta:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\instaladores\instalar-windows.ps1
   ```

5. Cierra PowerShell, abre una ventana nueva y comprueba:

   ```powershell
   espanolcode --ayuda
   espanolcode .\ejemplos\hola.esp
   ```

El instalador copia `espanolcode.exe` a la carpeta de programas de tu cuenta y añade esa carpeta al `PATH` del usuario. `PATH` es la lista de carpetas donde Windows busca comandos; por eso luego puedes escribir `espanolcode` desde cualquier carpeta. Windows permite asociar `.esp` a un programa, pero no hace falta asociación para ejecutar un archivo: el comando `espanolcode archivo.esp` es más claro y funciona desde la terminal.

## Linux y macOS

Instala Rust en el ordenador que vaya a compilar (<https://rustup.rs/>), abre una terminal en la carpeta del proyecto y ejecuta:

```sh
cargo build --release
sh instaladores/instalar-unix.sh
```

El instalador copia el ejecutable a `~/.local/bin`. Si indica que esa carpeta no está en `PATH`, sigue el mensaje que imprime y abre una terminal nueva. Después prueba `espanolcode ejemplos/hola.esp`.

Para distribuirlo a otras personas sin Rust, compila una vez para cada sistema operativo y entrega el ejecutable compilado junto con el instalador correspondiente. Este proyecto todavía no genera instaladores gráficos (`.msi`, `.dmg` o paquetes de Linux); el script de Windows sí instala el comando para la cuenta actual.

## Escribir tu propio archivo

1. Abre un editor de texto sencillo, por ejemplo el Bloc de notas.
2. Escribe `decir "Hola mundo"`.
3. Guarda el archivo con el nombre `hola.esp` (comprueba que no termine como `hola.esp.txt`).
4. En una terminal, ve a la carpeta donde lo guardaste y ejecuta `espanolcode hola.esp`.

Las comillas indican dónde empieza y termina el texto. Puedes usar `\n` dentro del texto para crear una línea nueva y `\"` para mostrar comillas.

## Ayuda y errores

- `espanolcode --ayuda` muestra los comandos disponibles.
- `espanolcode --version` muestra la versión.
- Si una operación, comparación o sangría está mal escrita, falta una variable o no se puede abrir un archivo, EspañolCode muestra el número de línea y una explicación.

## Usar EspañolCode en Visual Studio Code

El proyecto incluye una extensión para reconocer archivos `.esp`, colorear instrucciones y cadenas, señalar errores básicos y ejecutar el archivo desde el editor con **Ctrl+Alt+R** o el comando **EspañolCode: Ejecutar archivo actual** en la paleta de comandos.

1. Instala Visual Studio Code.
2. Instala EspañolCode siguiendo los pasos de Windows o Linux/macOS anteriores. La extensión necesita que el comando `espanolcode` funcione en una terminal.
3. Instala la extensión desde la carpeta `vscode-extension` siguiendo [sus instrucciones](vscode-extension/README.md).
4. Abre cualquier archivo `.esp`. El código se muestra con colores. Para ejecutarlo, pulsa **Ctrl+Alt+R**; el programa se abre en la terminal integrada, donde también puedes responder a `preguntar`.

La extensión también lleva el logotipo, reconoce variables, preguntas, condiciones, repeticiones, listas, funciones sencillas y operaciones con archivos, y señala errores básicos. Las futuras palabras del lenguaje se colorean, pero aún no se pueden ejecutar.

## Funciones ya incluidas y pendientes

- `decir` — equivalente sencillo de `print`: muestra texto.
- `guardar` — crea o cambia una variable.
- `preguntar` — equivalente sencillo de `input`: solicita y guarda una respuesta.
- `+`, `-`, `*`, `/` y paréntesis — operaciones con números; `+` también une dos textos.
- `si` y `si no` — eligen qué bloque ejecutar según una comparación.
- `repetir N veces` — ejecuta un bloque un número limitado de veces.
- Listas con `[...]`, índices como `nombres[0]`, `agregar` y `cantidad(...)`.
- `leer_archivo`, `escribir_archivo` y `agregar_archivo` — lectura y escritura sencilla de texto.
- `funcion nombre(parámetros) = expresión` o funciones por bloques con variables locales, `devolver`, `si`/`si no` y `repetir`.
- `importar` y `desde ... importar ...` — reconocen la sintaxis de imports, pero todavía no cargan los módulos ni sus funciones.
- Funciones útiles: `cantidad`, `absoluto`, `redondear`, `minimo`, `maximo`, `texto` y `numero`.
- `espanolcode --bloques` — abre el editor visual de piezas; permite arrastrar, descargar `.esp` y ejecutar.
- Siguiente: funciones anidadas y ampliar los bloques visuales; no incluye los sprites, disfraces ni audio de Scratch.

Esto no copia todas las bibliotecas especializadas de Python; traduce gradualmente las ideas comunes para principiantes.
