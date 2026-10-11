use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{self, Command};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const EDITOR_BLOQUES: &str = include_str!("../bloques/editor.html");

#[derive(Clone, Debug, PartialEq)]
enum Valor {
    Texto(String),
    Numero(f64),
    Lista(Vec<Valor>),
}

type Variables = HashMap<String, Valor>;

#[derive(Clone, Debug)]
enum CuerpoFuncion {
    Expresion(String),
    Bloque(Vec<LineaFuncion>),
}

#[derive(Clone, Debug)]
struct LineaFuncion {
    numero: usize,
    sangria: usize,
    instruccion: String,
}

#[derive(Clone, Debug)]
struct Funcion {
    parametros: Vec<String>,
    cuerpo: CuerpoFuncion,
    numero_linea: usize,
}

type Funciones = HashMap<String, Funcion>;

fn mostrar_error(mensaje: &str) {
    eprintln!("Error: {mensaje}");
}

fn mostrar_valor(valor: &Valor) -> String {
    match valor {
        Valor::Texto(texto) => texto.clone(),
        Valor::Numero(numero) if numero.fract() == 0.0 => format!("{numero:.0}"),
        Valor::Numero(numero) => numero.to_string(),
        Valor::Lista(valores) => {
            let elementos = valores
                .iter()
                .map(|valor| match valor {
                    Valor::Texto(texto) => format!("{texto:?}"),
                    otro => mostrar_valor(otro),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{elementos}]")
        }
    }
}

/// Interpreta texto entre comillas y entiende algunos caracteres especiales.
fn leer_texto(numero_linea: usize, texto: &str) -> Result<String, String> {
    let texto = texto.trim();
    if !texto.starts_with('"') || !texto.ends_with('"') || texto.len() < 2 {
        return Err(format!(
            "Línea {numero_linea}: escribe el texto entre comillas, por ejemplo: decir \"Hola\""
        ));
    }

    let interior = &texto[1..texto.len() - 1];
    let mut resultado = String::new();
    let mut caracteres = interior.chars();
    while let Some(caracter) = caracteres.next() {
        if caracter == '\\' {
            match caracteres.next() {
                Some('n') => resultado.push('\n'),
                Some('t') => resultado.push('\t'),
                Some('"') => resultado.push('"'),
                Some('\\') => resultado.push('\\'),
                Some(otro) => {
                    return Err(format!(
                        "Línea {numero_linea}: no conozco la secuencia \\\\{otro}."
                    ));
                }
                None => {
                    return Err(format!(
                        "Línea {numero_linea}: la barra final necesita otro carácter."
                    ));
                }
            }
        } else {
            resultado.push(caracter);
        }
    }
    Ok(resultado)
}

fn es_identificador(nombre: &str) -> bool {
    let mut letras = nombre.chars();
    match letras.next() {
        Some(inicial) if inicial == '_' || inicial.is_alphabetic() => {}
        _ => return false,
    }
    letras.all(|caracter| caracter == '_' || caracter.is_alphanumeric())
}

fn es_palabra_reservada(nombre: &str) -> bool {
    matches!(
        nombre,
        "decir"
            | "guardar"
            | "preguntar"
            | "agregar"
            | "si"
            | "no"
            | "repetir"
            | "veces"
            | "funcion"
            | "importar"
            | "desde"
            | "como"
    )
}

fn es_ruta_modulo(nombre: &str) -> bool {
    let ruta = nombre.trim_start_matches('.');
    if ruta.is_empty() {
        return nombre.starts_with('.');
    }
    ruta.split('.').all(es_identificador)
}

fn validar_elementos_importados(
    numero_linea: usize,
    elementos: &str,
    permitir_asterisco: bool,
    permitir_ruta: bool,
    permitir_parentesis: bool,
) -> Result<(), String> {
    let elementos = elementos.trim();
    let con_parentesis = elementos.starts_with('(') || elementos.ends_with(')');
    if con_parentesis && !permitir_parentesis {
        return Err(format!(
            "Línea {numero_linea}: no uses paréntesis en esta importación."
        ));
    }
    let elementos = if permitir_parentesis {
        elementos
            .strip_prefix('(')
            .and_then(|texto| texto.strip_suffix(')'))
            .unwrap_or(elementos)
            .trim()
    } else {
        elementos
    };
    if elementos.is_empty() {
        return Err(format!(
            "Línea {numero_linea}: escribe el módulo o los nombres que quieres importar."
        ));
    }

    let partes = elementos.split(',').map(str::trim).collect::<Vec<_>>();
    if partes.iter().any(|parte| parte.is_empty()) {
        return Err(format!(
            "Línea {numero_linea}: separa los elementos importados con comas y no dejes elementos vacíos."
        ));
    }
    if partes.iter().any(|parte| *parte == "*") {
        if !permitir_asterisco || partes.len() != 1 {
            return Err(format!(
                "Línea {numero_linea}: usa '*' solo y después de 'desde módulo importar'."
            ));
        }
        return Ok(());
    }

    for parte in partes {
        let mut secciones = parte.split(" como ");
        let nombre = secciones.next().unwrap_or_default().trim();
        let alias = secciones.next().map(str::trim);
        if secciones.next().is_some() {
            return Err(format!(
                "Línea {numero_linea}: usa 'como' una sola vez para cambiar el nombre importado."
            ));
        }
        let nombre_valido = if permitir_ruta {
            !nombre.starts_with('.') && es_ruta_modulo(nombre)
        } else {
            es_identificador(nombre)
        };
        if !nombre_valido {
            return Err(format!(
                "Línea {numero_linea}: revisa el nombre del módulo o elemento que quieres importar."
            ));
        }
        if let Some(alias) = alias {
            if !es_identificador(alias) || es_palabra_reservada(alias) {
                return Err(format!(
                    "Línea {numero_linea}: el nombre escrito después de 'como' no es válido."
                ));
            }
        }
        if !permitir_ruta && alias.is_none() && es_palabra_reservada(nombre) {
            return Err(format!(
                "Línea {numero_linea}: no puedes importar directamente una palabra reservada."
            ));
        }
    }
    Ok(())
}

fn validar_importacion(numero_linea: usize, instruccion: &str) -> Result<bool, String> {
    if let Some(resto) = instruccion.strip_prefix("importar") {
        if !resto.is_empty() && !resto.starts_with(char::is_whitespace) {
            return Ok(false);
        }
        validar_elementos_importados(numero_linea, resto, false, true, false)?;
        return Ok(true);
    }

    let Some(resto) = instruccion.strip_prefix("desde") else {
        return Ok(false);
    };
    if !resto.is_empty() && !resto.starts_with(char::is_whitespace) {
        return Ok(false);
    }
    let mut partes = resto.split_whitespace();
    let modulo = partes.next().unwrap_or_default();
    let palabra_importar = partes.next().unwrap_or_default();
    let elementos = partes.collect::<Vec<_>>().join(" ");
    if !es_ruta_modulo(modulo) || palabra_importar != "importar" {
        return Err(format!(
            "Línea {numero_linea}: usa 'desde módulo importar nombre'."
        ));
    }
    validar_elementos_importados(numero_linea, &elementos, true, false, true)?;
    Ok(true)
}

fn es_funcion_integrada(nombre: &str) -> bool {
    matches!(
        nombre,
        "cantidad"
            | "leer_archivo"
            | "escribir_archivo"
            | "agregar_archivo"
            | "absoluto"
            | "redondear"
            | "minimo"
            | "maximo"
            | "texto"
            | "numero"
    )
}

fn analizar_firma_funcion(
    numero_linea: usize,
    firma: &str,
) -> Result<(String, Vec<String>), String> {
    let firma = firma.trim();
    let Some(inicio_parametros) = firma.find('(') else {
        return Err(format!(
            "Línea {numero_linea}: escribe los parámetros entre paréntesis."
        ));
    };
    let Some(fin_parametros) = firma.rfind(')') else {
        return Err(format!(
            "Línea {numero_linea}: falta cerrar los paréntesis de los parámetros."
        ));
    };
    if fin_parametros != firma.len() - 1 || inicio_parametros >= fin_parametros {
        return Err(format!(
            "Línea {numero_linea}: revisa el nombre y los paréntesis de la función."
        ));
    }
    let nombre = firma[..inicio_parametros].trim();
    if !es_identificador(nombre) || es_palabra_reservada(nombre) || es_funcion_integrada(nombre) {
        return Err(format!(
            "Línea {numero_linea}: ese nombre de función no se puede usar."
        ));
    }

    let texto_parametros = &firma[inicio_parametros + 1..fin_parametros];
    let mut parametros = Vec::new();
    if !texto_parametros.trim().is_empty() {
        for parametro in texto_parametros.split(',').map(str::trim) {
            if !es_identificador(parametro) || es_palabra_reservada(parametro) {
                return Err(format!(
                    "Línea {numero_linea}: revisa el nombre de los parámetros."
                ));
            }
            if parametros.iter().any(|anterior| anterior == parametro) {
                return Err(format!(
                    "Línea {numero_linea}: no repitas el mismo parámetro."
                ));
            }
            parametros.push(parametro.to_string());
        }
    }
    Ok((nombre.to_string(), parametros))
}

fn analizar_definicion_funcion(
    numero_linea: usize,
    definicion: &str,
) -> Result<(String, Funcion), String> {
    let Some(definicion) = definicion.strip_prefix("funcion ") else {
        return Err(format!(
            "Línea {numero_linea}: empieza la definición con 'funcion'."
        ));
    };
    let Some((firma, expresion)) = definicion.split_once('=') else {
        return Err(format!(
            "Línea {numero_linea}: escribe funcion nombre(parámetros) = expresión o añade un bloque indentado."
        ));
    };
    let (nombre, parametros) = analizar_firma_funcion(numero_linea, firma)?;
    let expresion = expresion.trim();
    if expresion.is_empty() {
        return Err(format!(
            "Línea {numero_linea}: escribe una expresión que la función devolverá."
        ));
    }
    Ok((
        nombre,
        Funcion {
            parametros,
            cuerpo: CuerpoFuncion::Expresion(expresion.to_string()),
            numero_linea,
        },
    ))
}

fn es_numero(texto: &str) -> bool {
    let sin_signo = texto.strip_prefix('-').unwrap_or(texto);
    let mut partes = sin_signo.split('.');
    let entero = partes.next().unwrap_or_default();
    let decimal = partes.next();
    !entero.is_empty()
        && entero.chars().all(|caracter| caracter.is_ascii_digit())
        && decimal.map_or(true, |parte| {
            !parte.is_empty() && parte.chars().all(|caracter| caracter.is_ascii_digit())
        })
        && partes.next().is_none()
}

struct Analizador<'a> {
    caracteres: Vec<char>,
    posicion: usize,
    numero_linea: usize,
    variables: &'a Variables,
    funciones: &'a Funciones,
    profundidad: usize,
}

impl<'a> Analizador<'a> {
    fn nuevo(
        numero_linea: usize,
        expresion: &str,
        variables: &'a Variables,
        funciones: &'a Funciones,
        profundidad: usize,
    ) -> Self {
        Self {
            caracteres: expresion.chars().collect(),
            posicion: 0,
            numero_linea,
            variables,
            funciones,
            profundidad,
        }
    }

    fn mirar(&self) -> Option<char> {
        self.caracteres.get(self.posicion).copied()
    }

    fn saltar_espacios(&mut self) {
        while self.mirar().is_some_and(char::is_whitespace) {
            self.posicion += 1;
        }
    }

    fn error(&self, detalle: &str) -> String {
        format!("Línea {}: {detalle}", self.numero_linea)
    }

    fn analizar(mut self) -> Result<Valor, String> {
        let valor = self.suma_o_resta()?;
        self.saltar_espacios();
        if self.posicion != self.caracteres.len() {
            return Err(self.error("no entiendo esa parte de la operación."));
        }
        Ok(valor)
    }

    fn suma_o_resta(&mut self) -> Result<Valor, String> {
        let mut izquierda = self.multiplicar_o_dividir()?;
        loop {
            self.saltar_espacios();
            let operador = match self.mirar() {
                Some('+') => '+',
                Some('-') => '-',
                _ => break,
            };
            self.posicion += 1;
            let derecha = self.multiplicar_o_dividir()?;
            izquierda = match (operador, izquierda, derecha) {
                ('+', Valor::Numero(a), Valor::Numero(b)) => Valor::Numero(a + b),
                ('+', Valor::Texto(mut a), Valor::Texto(b)) => {
                    a.push_str(&b);
                    Valor::Texto(a)
                }
                ('+', Valor::Lista(mut a), Valor::Lista(b)) => {
                    a.extend(b);
                    Valor::Lista(a)
                }
                ('-', Valor::Numero(a), Valor::Numero(b)) => Valor::Numero(a - b),
                ('+', _, _) => {
                    return Err(self
                        .error("con + usa dos números, dos textos o dos listas del mismo tipo."));
                }
                ('-', _, _) => {
                    return Err(self.error("para restar, ambos valores deben ser números."));
                }
                _ => unreachable!(),
            };
        }
        self.comprobar_numero_finito(izquierda)
    }

    fn multiplicar_o_dividir(&mut self) -> Result<Valor, String> {
        let mut izquierda = self.unario()?;
        loop {
            self.saltar_espacios();
            let operador = match self.mirar() {
                Some('*') => '*',
                Some('/') => '/',
                _ => break,
            };
            self.posicion += 1;
            let derecha = self.unario()?;
            let (Valor::Numero(a), Valor::Numero(b)) = (izquierda, derecha) else {
                return Err(
                    self.error("para multiplicar o dividir, ambos valores deben ser números.")
                );
            };
            if operador == '/' && b == 0.0 {
                return Err(self.error("no se puede dividir entre cero."));
            }
            izquierda = Valor::Numero(if operador == '*' { a * b } else { a / b });
        }
        self.comprobar_numero_finito(izquierda)
    }

    fn unario(&mut self) -> Result<Valor, String> {
        self.saltar_espacios();
        if self.mirar() == Some('-') {
            self.posicion += 1;
            return match self.unario()? {
                Valor::Numero(numero) => self.comprobar_numero_finito(Valor::Numero(-numero)),
                Valor::Texto(_) | Valor::Lista(_) => {
                    Err(self.error("el signo - solo se puede usar con números."))
                }
            };
        }
        self.primario()
    }

    fn primario(&mut self) -> Result<Valor, String> {
        self.saltar_espacios();
        let mut valor = match self.mirar() {
            Some('(') => {
                self.posicion += 1;
                let valor = self.suma_o_resta()?;
                self.saltar_espacios();
                if self.mirar() != Some(')') {
                    return Err(self.error("falta cerrar el paréntesis."));
                }
                self.posicion += 1;
                valor
            }
            Some('"') => self.texto_entre_comillas()?,
            Some('[') => self.lista()?,
            Some(caracter) if caracter.is_ascii_digit() => self.numero()?,
            Some(caracter) if caracter == '_' || caracter.is_alphabetic() => self.variable()?,
            _ => return Err(self.error("esperaba un texto, número, lista, variable o paréntesis.")),
        };

        loop {
            self.saltar_espacios();
            if self.mirar() != Some('[') {
                break;
            }
            self.posicion += 1;
            let indice = self.suma_o_resta()?;
            self.saltar_espacios();
            if self.mirar() != Some(']') {
                return Err(self.error("falta cerrar el índice de la lista con ]."));
            }
            self.posicion += 1;
            let Valor::Numero(indice) = indice else {
                return Err(self.error("el índice de una lista debe ser un número entero."));
            };
            if indice < 0.0 || indice.fract() != 0.0 {
                return Err(
                    self.error("el índice de una lista debe ser un entero de 0 en adelante.")
                );
            }
            let Valor::Lista(elementos) = valor else {
                return Err(self.error("solo puedes usar un índice [n] con una lista."));
            };
            let Some(elemento) = elementos.get(indice as usize).cloned() else {
                return Err(self.error("ese índice no existe en la lista."));
            };
            valor = elemento;
        }
        Ok(valor)
    }

    fn lista(&mut self) -> Result<Valor, String> {
        self.posicion += 1;
        self.saltar_espacios();
        if self.mirar() == Some(']') {
            self.posicion += 1;
            return Ok(Valor::Lista(Vec::new()));
        }

        let mut elementos = Vec::new();
        loop {
            elementos.push(self.suma_o_resta()?);
            self.saltar_espacios();
            match self.mirar() {
                Some(',') => {
                    self.posicion += 1;
                    self.saltar_espacios();
                    if self.mirar() == Some(']') {
                        self.posicion += 1;
                        break;
                    }
                }
                Some(']') => {
                    self.posicion += 1;
                    break;
                }
                _ => {
                    return Err(
                        self.error("separa los elementos de la lista con comas y ciérrala con ].")
                    )
                }
            }
        }
        Ok(Valor::Lista(elementos))
    }

    fn texto_entre_comillas(&mut self) -> Result<Valor, String> {
        let inicio = self.posicion;
        self.posicion += 1;
        let mut escapado = false;
        while let Some(caracter) = self.mirar() {
            self.posicion += 1;
            if escapado {
                escapado = false;
            } else if caracter == '\\' {
                escapado = true;
            } else if caracter == '"' {
                let texto: String = self.caracteres[inicio..self.posicion].iter().collect();
                return leer_texto(self.numero_linea, &texto).map(Valor::Texto);
            }
        }
        Err(self.error("faltan las comillas de cierre del texto."))
    }

    fn numero(&mut self) -> Result<Valor, String> {
        let inicio = self.posicion;
        while self
            .mirar()
            .is_some_and(|caracter| caracter.is_ascii_digit() || caracter == '.')
        {
            self.posicion += 1;
        }
        let texto: String = self.caracteres[inicio..self.posicion].iter().collect();
        if !es_numero(&texto) {
            return Err(self.error("ese número no está escrito correctamente."));
        }
        let numero = texto
            .parse::<f64>()
            .map_err(|_| self.error("no pude leer ese número."))?;
        self.comprobar_numero_finito(Valor::Numero(numero))
    }

    fn variable(&mut self) -> Result<Valor, String> {
        let inicio = self.posicion;
        while self
            .mirar()
            .is_some_and(|caracter| caracter == '_' || caracter.is_alphanumeric())
        {
            self.posicion += 1;
        }
        let nombre: String = self.caracteres[inicio..self.posicion].iter().collect();
        self.saltar_espacios();
        if self.mirar() == Some('(') {
            let argumentos = self.argumentos_llamada()?;
            return self.llamar_funcion(&nombre, argumentos);
        }
        self.variables.get(&nombre).cloned().ok_or_else(|| {
            self.error(&format!(
                "no conozco la variable '{nombre}'. Primero asígnale un valor con guardar."
            ))
        })
    }

    fn argumentos_llamada(&mut self) -> Result<Vec<Valor>, String> {
        if self.mirar() != Some('(') {
            return Err(self.error("falta abrir la llamada con ( ."));
        }
        self.posicion += 1;
        self.saltar_espacios();
        if self.mirar() == Some(')') {
            self.posicion += 1;
            return Ok(Vec::new());
        }

        let mut argumentos = Vec::new();
        loop {
            argumentos.push(self.suma_o_resta()?);
            self.saltar_espacios();
            match self.mirar() {
                Some(',') => {
                    self.posicion += 1;
                    self.saltar_espacios();
                }
                Some(')') => {
                    self.posicion += 1;
                    break;
                }
                _ => return Err(self.error("separa los argumentos con comas y cierra con ).")),
            }
        }
        Ok(argumentos)
    }

    fn llamar_funcion(&self, nombre: &str, argumentos: Vec<Valor>) -> Result<Valor, String> {
        match nombre {
            "cantidad" if argumentos.len() == 1 => match &argumentos[0] {
                Valor::Lista(elementos) => Ok(Valor::Numero(elementos.len() as f64)),
                Valor::Texto(texto) => Ok(Valor::Numero(texto.chars().count() as f64)),
                _ => Err(self.error("cantidad cuenta los elementos de una lista o texto.")),
            },
            "cantidad" => {
                Err(self.error("usa cantidad con un solo valor, por ejemplo: cantidad(lista)."))
            }
            "absoluto" if argumentos.len() == 1 => match &argumentos[0] {
                Valor::Numero(numero) => Ok(Valor::Numero(numero.abs())),
                _ => Err(self.error("absoluto necesita un número.")),
            },
            "absoluto" => Err(self.error("usa absoluto con un número, por ejemplo: absoluto(-5).")),
            "redondear" if argumentos.len() == 1 => match argumentos[0] {
                Valor::Numero(numero) => Ok(Valor::Numero(numero.round())),
                _ => Err(self.error("redondear necesita un número.")),
            },
            "redondear" => Err(self.error("usa redondear con un solo número.")),
            "texto" if argumentos.len() == 1 => Ok(Valor::Texto(mostrar_valor(&argumentos[0]))),
            "texto" => Err(self.error("usa texto con un solo valor.")),
            "numero" if argumentos.len() == 1 => {
                match &argumentos[0] {
                    Valor::Numero(numero) => Ok(Valor::Numero(*numero)),
                    Valor::Texto(texto) => {
                        let numero = texto.trim().parse::<f64>().map_err(|_| {
                        self.error("numero no pudo convertir ese texto. Prueba, por ejemplo, numero(\"12.5\").")
                    })?;
                        self.comprobar_numero_finito(Valor::Numero(numero))
                    }
                    _ => Err(self
                        .error("numero convierte texto o conserva un número; no acepta listas.")),
                }
            }
            "numero" => Err(self.error("usa numero con un solo valor.")),
            "minimo" | "maximo" => {
                let numeros = match argumentos.as_slice() {
                    [Valor::Numero(a), Valor::Numero(b)] => vec![*a, *b],
                    [Valor::Lista(elementos)] => {
                        if elementos.is_empty() {
                            return Err(self.error("minimo y maximo necesitan al menos un número."));
                        }
                        elementos
                            .iter()
                            .map(|elemento| match elemento {
                                Valor::Numero(numero) => Ok(*numero),
                                _ => Err(self.error("minimo y maximo solo comparan números.")),
                            })
                            .collect::<Result<Vec<_>, _>>()?
                    }
                    _ => {
                        return Err(self.error(&format!(
                            "usa {nombre} con dos números o una lista de números."
                        )))
                    }
                };
                let mut valores = numeros.into_iter();
                let mut resultado = valores.next().expect("siempre hay al menos un número");
                for numero in valores {
                    resultado = if nombre == "minimo" {
                        resultado.min(numero)
                    } else {
                        resultado.max(numero)
                    };
                }
                Ok(Valor::Numero(resultado))
            }
            "leer_archivo" if argumentos.len() == 1 => {
                let Valor::Texto(ruta) = &argumentos[0] else {
                    return Err(self.error("leer_archivo necesita una ruta entre comillas."));
                };
                fs::read_to_string(ruta).map(Valor::Texto).map_err(|error| {
                    self.error(&format!("no pude leer el archivo '{ruta}': {error}"))
                })
            }
            "leer_archivo" => Err(self
                .error("usa leer_archivo con una ruta, por ejemplo: leer_archivo(\"nota.txt\").")),
            "escribir_archivo" | "agregar_archivo" if argumentos.len() == 2 => {
                let Valor::Texto(ruta) = &argumentos[0] else {
                    return Err(self.error("la ruta del archivo debe ir entre comillas."));
                };
                let contenido = mostrar_valor(&argumentos[1]);
                if nombre == "escribir_archivo" {
                    fs::write(ruta, &contenido).map_err(|error| {
                        self.error(&format!("no pude escribir el archivo '{ruta}': {error}"))
                    })?;
                } else {
                    let mut archivo = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(ruta)
                        .map_err(|error| {
                            self.error(&format!("no pude abrir el archivo '{ruta}': {error}"))
                        })?;
                    archivo.write_all(contenido.as_bytes()).map_err(|error| {
                        self.error(&format!(
                            "no pude añadir texto al archivo '{ruta}': {error}"
                        ))
                    })?;
                }
                Ok(Valor::Numero(contenido.chars().count() as f64))
            }
            "escribir_archivo" | "agregar_archivo" => Err(self.error(&format!(
                "usa {nombre}(ruta, contenido), con la ruta entre comillas y dos valores."
            ))),
            _ => {
                let Some(funcion) = self.funciones.get(nombre) else {
                    return Err(self.error(&format!("no conozco la función '{nombre}'.")));
                };
                if funcion.parametros.len() != argumentos.len() {
                    return Err(self.error(&format!(
                        "la función '{nombre}' necesita {} valor(es), pero recibió {}.",
                        funcion.parametros.len(),
                        argumentos.len()
                    )));
                }
                if self.profundidad >= 64 {
                    return Err(self.error("la función se llamó demasiadas veces; revisa si tiene una repetición infinita."));
                }
                let mut variables_locales = self.variables.clone();
                for (parametro, argumento) in funcion.parametros.iter().zip(argumentos) {
                    variables_locales.insert(parametro.clone(), argumento);
                }
                let profundidad = self.profundidad + 1;
                match &funcion.cuerpo {
                    CuerpoFuncion::Expresion(expresion) => leer_valor_con_funciones(
                        funcion.numero_linea,
                        expresion,
                        &variables_locales,
                        self.funciones,
                        profundidad,
                    ),
                    CuerpoFuncion::Bloque(lineas) => {
                        let mut funciones_locales = self.funciones.clone();
                        let mut indice = 0;
                        match ejecutar_bloque_funcion(
                            lineas,
                            &mut indice,
                            lineas[0].sangria,
                            &mut variables_locales,
                            &mut funciones_locales,
                            profundidad,
                        )? {
                            Some(valor) => Ok(valor),
                            None => Err(self.error(&format!(
                                "la función '{nombre}' terminó sin devolver un valor."
                            ))),
                        }
                    }
                }
            }
        }
    }

    fn comprobar_numero_finito(&self, valor: Valor) -> Result<Valor, String> {
        if matches!(&valor, Valor::Numero(numero) if !numero.is_finite()) {
            Err(self.error("el cálculo produjo un número demasiado grande."))
        } else {
            Ok(valor)
        }
    }
}

fn leer_valor_con_funciones(
    numero_linea: usize,
    expresion: &str,
    variables: &Variables,
    funciones: &Funciones,
    profundidad: usize,
) -> Result<Valor, String> {
    Analizador::nuevo(numero_linea, expresion, variables, funciones, profundidad).analizar()
}

fn leer_entrada(
    numero_linea: usize,
    prompt: &str,
    variables: &Variables,
    funciones: &Funciones,
    profundidad: usize,
) -> Result<Valor, String> {
    if prompt.trim().is_empty() {
        return Err(format!(
            "Línea {numero_linea}: escribe la pregunta entre comillas."
        ));
    }
    let mensaje =
        leer_valor_con_funciones(numero_linea, prompt, variables, funciones, profundidad)?;
    print!("{}", mostrar_valor(&mensaje));
    io::stdout()
        .flush()
        .map_err(|error| format!("no pude mostrar la pregunta: {error}"))?;

    let mut respuesta = String::new();
    let bytes = io::stdin()
        .read_line(&mut respuesta)
        .map_err(|error| format!("no pude leer tu respuesta: {error}"))?;
    if bytes == 0 {
        return Err(format!(
            "Línea {numero_linea}: no recibí respuesta a la pregunta."
        ));
    }
    Ok(Valor::Texto(
        respuesta.trim_end_matches(&['\r', '\n'][..]).to_string(),
    ))
}

fn leer_valor_guardado(
    numero_linea: usize,
    expresion: &str,
    variables: &Variables,
    funciones: &Funciones,
    profundidad: usize,
) -> Result<Valor, String> {
    if let Some(pregunta) = expresion.trim().strip_prefix("preguntar ") {
        leer_entrada(numero_linea, pregunta, variables, funciones, profundidad)
    } else {
        leer_valor_con_funciones(numero_linea, expresion, variables, funciones, profundidad)
    }
}

fn ejecutar_linea(
    numero_linea: usize,
    linea: &str,
    variables: &mut Variables,
    funciones: &mut Funciones,
    profundidad: usize,
) -> Result<(), String> {
    // Algunos editores de Windows guardan un marcador invisible al inicio del archivo.
    let instruccion = linea.trim().trim_start_matches('\u{feff}');
    if instruccion.is_empty() {
        return Ok(());
    }

    if validar_importacion(numero_linea, instruccion)? {
        eprintln!(
            "Aviso línea {numero_linea}: se reconoce la importación, pero EspañolCode todavía no carga módulos ni sus funciones."
        );
        return Ok(());
    }

    if instruccion.starts_with("funcion ") {
        let (nombre, funcion) = analizar_definicion_funcion(numero_linea, instruccion)?;
        funciones.insert(nombre, funcion);
        return Ok(());
    }

    if let Some(asignacion) = instruccion.strip_prefix("guardar ") {
        let Some((nombre, expresion)) = asignacion.split_once('=') else {
            return Err(format!(
                "Línea {numero_linea}: usa guardar nombre = valor, por ejemplo: guardar nombre = \"Ana\"."
            ));
        };
        let nombre = nombre.trim();
        if !es_identificador(nombre) || es_palabra_reservada(nombre) {
            return Err(format!(
                "Línea {numero_linea}: ese nombre no sirve. Usa una palabra como nombre o edad, sin espacios."
            ));
        }
        let valor =
            leer_valor_guardado(numero_linea, expresion, variables, funciones, profundidad)?;
        variables.insert(nombre.to_string(), valor);
        return Ok(());
    }

    if let Some(asignacion) = instruccion.strip_prefix("agregar ") {
        let Some((nombre, expresion)) = asignacion.split_once('=') else {
            return Err(format!(
                "Línea {numero_linea}: usa agregar lista = valor, por ejemplo: agregar nombres = \"Eva\"."
            ));
        };
        let nombre = nombre.trim();
        if !es_identificador(nombre) || es_palabra_reservada(nombre) {
            return Err(format!(
                "Línea {numero_linea}: escribe el nombre de una lista guardada."
            ));
        }
        match variables.get(nombre) {
            Some(Valor::Lista(_)) => {}
            Some(_) => {
                return Err(format!(
                    "Línea {numero_linea}: '{nombre}' existe, pero no es una lista."
                ))
            }
            None => {
                return Err(format!(
                    "Línea {numero_linea}: no existe la lista '{nombre}'. Créala con guardar."
                ))
            }
        }
        let valor =
            leer_valor_guardado(numero_linea, expresion, variables, funciones, profundidad)?;
        if let Some(Valor::Lista(elementos)) = variables.get_mut(nombre) {
            elementos.push(valor);
        }
        return Ok(());
    }

    for funcion in ["escribir_archivo", "agregar_archivo"] {
        if instruccion
            .strip_prefix(funcion)
            .is_some_and(|resto| resto.trim_start().starts_with('('))
        {
            let _ = leer_valor_con_funciones(
                numero_linea,
                instruccion,
                variables,
                funciones,
                profundidad,
            )?;
            return Ok(());
        }
    }

    if let Some(expresion) = instruccion.strip_prefix("decir ") {
        let valor =
            leer_valor_con_funciones(numero_linea, expresion, variables, funciones, profundidad)?;
        println!("{}", mostrar_valor(&valor));
        return Ok(());
    }

    if instruccion == "devolver" || instruccion.starts_with("devolver ") {
        return Err(format!(
            "Línea {numero_linea}: 'devolver' solo se puede usar dentro de una función."
        ));
    }

    Err(format!(
        "Línea {numero_linea}: todavía no conozco esa instrucción. Consulta 'espanolcode --ayuda' para ver lo que puedes usar."
    ))
}

fn separar_condicion(
    numero_linea: usize,
    expresion: &str,
) -> Result<(String, &'static str, String), String> {
    let caracteres: Vec<char> = expresion.chars().collect();
    let mut dentro_de_texto = false;
    let mut escapado = false;
    let mut parentesis = 0usize;
    let comparadores = [
        ("==", 2),
        ("!=", 2),
        (">=", 2),
        ("<=", 2),
        (">", 1),
        ("<", 1),
    ];

    for indice in 0..caracteres.len() {
        let caracter = caracteres[indice];
        if dentro_de_texto {
            if escapado {
                escapado = false;
            } else if caracter == '\\' {
                escapado = true;
            } else if caracter == '"' {
                dentro_de_texto = false;
            }
            continue;
        }
        if caracter == '"' {
            dentro_de_texto = true;
            continue;
        }
        if caracter == '(' {
            parentesis += 1;
            continue;
        }
        if caracter == ')' {
            parentesis = parentesis.saturating_sub(1);
            continue;
        }
        if parentesis != 0 {
            continue;
        }

        for (operador, largo) in comparadores {
            let simbolos: Vec<char> = operador.chars().collect();
            if caracteres[indice..].starts_with(&simbolos) {
                let izquierda: String = caracteres[..indice].iter().collect();
                let derecha: String = caracteres[indice + largo..].iter().collect();
                if izquierda.trim().is_empty() || derecha.trim().is_empty() {
                    return Err(format!(
                        "Línea {numero_linea}: completa los dos lados de la comparación, por ejemplo: si edad >= 18."
                    ));
                }
                return Ok((izquierda, operador, derecha));
            }
        }
    }
    Err(format!(
        "Línea {numero_linea}: escribe una comparación, por ejemplo: si edad >= 18."
    ))
}

fn evaluar_condicion(
    numero_linea: usize,
    expresion: &str,
    variables: &Variables,
    funciones: &Funciones,
    profundidad: usize,
) -> Result<bool, String> {
    let (izquierda, operador, derecha) = separar_condicion(numero_linea, expresion)?;
    let izquierda =
        leer_valor_con_funciones(numero_linea, &izquierda, variables, funciones, profundidad)?;
    let derecha =
        leer_valor_con_funciones(numero_linea, &derecha, variables, funciones, profundidad)?;

    match (izquierda, derecha) {
        (Valor::Numero(a), Valor::Numero(b)) => Ok(match operador {
            "==" => a == b,
            "!=" => a != b,
            ">" => a > b,
            "<" => a < b,
            ">=" => a >= b,
            "<=" => a <= b,
            _ => unreachable!(),
        }),
        (Valor::Texto(a), Valor::Texto(b)) if operador == "==" => Ok(a == b),
        (Valor::Texto(a), Valor::Texto(b)) if operador == "!=" => Ok(a != b),
        (Valor::Lista(a), Valor::Lista(b)) if operador == "==" => Ok(a == b),
        (Valor::Lista(a), Valor::Lista(b)) if operador == "!=" => Ok(a != b),
        (Valor::Lista(_), Valor::Lista(_)) => Err(format!(
            "Línea {numero_linea}: las listas solo se comparan con == o !=."
        )),
        (Valor::Texto(_), Valor::Texto(_)) => Err(format!(
            "Línea {numero_linea}: para usar >, <, >= o <= compara números."
        )),
        _ => Err(format!(
            "Línea {numero_linea}: compara dos números o dos textos, no mezcles los tipos."
        )),
    }
}

struct LineaPrograma<'a> {
    numero: usize,
    sangria: usize,
    instruccion: &'a str,
}

fn preparar_lineas(contenido: &str) -> Vec<LineaPrograma<'_>> {
    contenido
        .lines()
        .enumerate()
        .map(|(indice, linea)| {
            let mut sangria = 0usize;
            for caracter in linea.chars() {
                match caracter {
                    ' ' => sangria += 1,
                    '\t' => sangria += 4,
                    _ => break,
                }
            }
            LineaPrograma {
                numero: indice + 1,
                sangria,
                instruccion: linea.trim().trim_start_matches('\u{feff}').trim(),
            }
        })
        .collect()
}

fn siguiente_no_vacia(lineas: &[LineaPrograma<'_>], desde: usize) -> usize {
    let mut indice = desde;
    while indice < lineas.len() && lineas[indice].instruccion.is_empty() {
        indice += 1;
    }
    indice
}

fn saltar_bloque(lineas: &[LineaPrograma<'_>], indice: &mut usize, sangria_bloque: usize) {
    while *indice < lineas.len() {
        let linea = &lineas[*indice];
        if !linea.instruccion.is_empty() && linea.sangria < sangria_bloque {
            break;
        }
        *indice += 1;
    }
}

fn leer_cantidad_repeticiones(
    numero_linea: usize,
    expresion: &str,
    variables: &Variables,
    funciones: &Funciones,
    profundidad: usize,
) -> Result<usize, String> {
    let Some(cantidad) = expresion.trim().strip_suffix(" veces") else {
        return Err(format!(
            "Línea {numero_linea}: escribe repetir número veces, por ejemplo: repetir 3 veces."
        ));
    };
    match leer_valor_con_funciones(numero_linea, cantidad, variables, funciones, profundidad)? {
        Valor::Numero(numero) if numero >= 0.0 && numero.fract() == 0.0 && numero <= 100_000.0 => {
            Ok(numero as usize)
        }
        Valor::Numero(_) => Err(format!(
            "Línea {numero_linea}: el número de repeticiones debe ser entero, de 0 a 100000."
        )),
        Valor::Texto(_) | Valor::Lista(_) => Err(format!(
            "Línea {numero_linea}: el número de repeticiones debe ser un número."
        )),
    }
}

fn siguiente_no_vacia_funcion(lineas: &[LineaFuncion], desde: usize) -> usize {
    let mut indice = desde;
    while indice < lineas.len() && lineas[indice].instruccion.is_empty() {
        indice += 1;
    }
    indice
}

fn saltar_bloque_funcion(lineas: &[LineaFuncion], indice: &mut usize, sangria_bloque: usize) {
    while *indice < lineas.len() {
        let linea = &lineas[*indice];
        if !linea.instruccion.is_empty() && linea.sangria < sangria_bloque {
            break;
        }
        *indice += 1;
    }
}

fn ejecutar_bloque_funcion(
    lineas: &[LineaFuncion],
    indice: &mut usize,
    sangria: usize,
    variables: &mut Variables,
    funciones: &mut Funciones,
    profundidad: usize,
) -> Result<Option<Valor>, String> {
    while *indice < lineas.len() {
        let linea = &lineas[*indice];
        if linea.instruccion.is_empty() {
            *indice += 1;
            continue;
        }
        if linea.sangria < sangria {
            return Ok(None);
        }
        if linea.sangria > sangria {
            return Err(format!(
                "Línea {}: revisa la sangría dentro de la función.",
                linea.numero
            ));
        }

        if let Some(expresion) = linea.instruccion.strip_prefix("devolver ") {
            return leer_valor_con_funciones(
                linea.numero,
                expresion,
                variables,
                funciones,
                profundidad,
            )
            .map(Some);
        }
        if linea.instruccion == "devolver" {
            return Err(format!(
                "Línea {}: escribe un valor después de devolver.",
                linea.numero
            ));
        }
        if linea.instruccion == "si no" {
            return Err(format!(
                "Línea {}: 'si no' debe ir después de un bloque 'si'.",
                linea.numero
            ));
        }
        if linea.instruccion.starts_with("funcion ") {
            return Err(format!(
                "Línea {}: por ahora no se pueden definir funciones dentro de otra función.",
                linea.numero
            ));
        }

        if let Some(expresion) = linea.instruccion.strip_prefix("repetir ") {
            let numero_repeticion = linea.numero;
            let cantidad = leer_cantidad_repeticiones(
                numero_repeticion,
                expresion,
                variables,
                funciones,
                profundidad,
            )?;
            *indice += 1;
            let inicio_cuerpo = siguiente_no_vacia_funcion(lineas, *indice);
            if inicio_cuerpo >= lineas.len() || lineas[inicio_cuerpo].sangria <= sangria {
                return Err(format!(
                    "Línea {numero_repeticion}: añade debajo de 'repetir' una línea con sangría."
                ));
            }
            let sangria_cuerpo = lineas[inicio_cuerpo].sangria;
            if cantidad == 0 {
                *indice = inicio_cuerpo;
                saltar_bloque_funcion(lineas, indice, sangria_cuerpo);
            } else {
                let mut fin_cuerpo = inicio_cuerpo;
                for _ in 0..cantidad {
                    let mut indice_vuelta = inicio_cuerpo;
                    if let Some(valor) = ejecutar_bloque_funcion(
                        lineas,
                        &mut indice_vuelta,
                        sangria_cuerpo,
                        variables,
                        funciones,
                        profundidad,
                    )? {
                        return Ok(Some(valor));
                    }
                    fin_cuerpo = indice_vuelta;
                }
                *indice = fin_cuerpo;
            }
            continue;
        }
        if linea.instruccion == "repetir" {
            return Err(format!(
                "Línea {}: escribe repetir número veces, por ejemplo: repetir 3 veces.",
                linea.numero
            ));
        }

        if let Some(expresion) = linea.instruccion.strip_prefix("si ") {
            let numero_condicion = linea.numero;
            let condicion = evaluar_condicion(
                numero_condicion,
                expresion,
                variables,
                funciones,
                profundidad,
            )?;
            *indice += 1;
            let inicio_cuerpo = siguiente_no_vacia_funcion(lineas, *indice);
            if inicio_cuerpo >= lineas.len() || lineas[inicio_cuerpo].sangria <= sangria {
                return Err(format!(
                    "Línea {numero_condicion}: añade debajo de 'si' una línea con sangría."
                ));
            }
            let sangria_cuerpo = lineas[inicio_cuerpo].sangria;
            *indice = inicio_cuerpo;
            if condicion {
                if let Some(valor) = ejecutar_bloque_funcion(
                    lineas,
                    indice,
                    sangria_cuerpo,
                    variables,
                    funciones,
                    profundidad,
                )? {
                    return Ok(Some(valor));
                }
            } else {
                saltar_bloque_funcion(lineas, indice, sangria_cuerpo);
            }

            let posible_si_no = siguiente_no_vacia_funcion(lineas, *indice);
            if posible_si_no < lineas.len()
                && lineas[posible_si_no].sangria == sangria
                && lineas[posible_si_no].instruccion == "si no"
            {
                *indice = posible_si_no + 1;
                let inicio_alternativa = siguiente_no_vacia_funcion(lineas, *indice);
                if inicio_alternativa >= lineas.len()
                    || lineas[inicio_alternativa].sangria <= sangria
                {
                    return Err(format!(
                        "Línea {}: añade debajo de 'si no' una línea con sangría.",
                        lineas[posible_si_no].numero
                    ));
                }
                let sangria_alternativa = lineas[inicio_alternativa].sangria;
                *indice = inicio_alternativa;
                if condicion {
                    saltar_bloque_funcion(lineas, indice, sangria_alternativa);
                } else if let Some(valor) = ejecutar_bloque_funcion(
                    lineas,
                    indice,
                    sangria_alternativa,
                    variables,
                    funciones,
                    profundidad,
                )? {
                    return Ok(Some(valor));
                }
            }
            continue;
        }
        if linea.instruccion == "si" {
            return Err(format!(
                "Línea {}: escribe una comparación después de si.",
                linea.numero
            ));
        }

        ejecutar_linea(
            linea.numero,
            &linea.instruccion,
            variables,
            funciones,
            profundidad,
        )?;
        *indice += 1;
    }
    Ok(None)
}

fn ejecutar_bloque(
    lineas: &[LineaPrograma<'_>],
    indice: &mut usize,
    sangria: usize,
    variables: &mut Variables,
    funciones: &mut Funciones,
) -> Result<(), String> {
    while *indice < lineas.len() {
        let linea = &lineas[*indice];
        if linea.instruccion.is_empty() {
            *indice += 1;
            continue;
        }
        if linea.sangria < sangria {
            return Ok(());
        }
        if linea.sangria > sangria {
            return Err(format!(
                "Línea {}: esta línea está más indentada de lo esperado.",
                linea.numero
            ));
        }

        if linea.instruccion == "si no" || linea.instruccion.starts_with("si no ") {
            return Err(format!(
                "Línea {}: 'si no' debe ir después de un bloque 'si'.",
                linea.numero
            ));
        }

        if let Some(definicion) = linea.instruccion.strip_prefix("funcion ") {
            if !definicion.contains('=') {
                let numero_funcion = linea.numero;
                let (nombre, parametros) = analizar_firma_funcion(numero_funcion, definicion)?;
                *indice += 1;
                let inicio_cuerpo = siguiente_no_vacia(lineas, *indice);
                if inicio_cuerpo >= lineas.len() || lineas[inicio_cuerpo].sangria <= sangria {
                    return Err(format!(
                        "Línea {numero_funcion}: añade debajo de la función un bloque indentado."
                    ));
                }
                let mut cursor = inicio_cuerpo;
                let mut cuerpo = Vec::new();
                while cursor < lineas.len() {
                    let linea_cuerpo = &lineas[cursor];
                    if linea_cuerpo.instruccion.is_empty() {
                        cursor += 1;
                        continue;
                    }
                    if linea_cuerpo.sangria <= sangria {
                        break;
                    }
                    if linea_cuerpo.instruccion.starts_with("funcion ") {
                        return Err(format!(
                            "Línea {}: por ahora no se pueden definir funciones dentro de otra función.",
                            linea_cuerpo.numero
                        ));
                    }
                    if linea_cuerpo.instruccion == "devolver" {
                        return Err(format!(
                            "Línea {}: escribe un valor después de devolver.",
                            linea_cuerpo.numero
                        ));
                    }
                    cuerpo.push(LineaFuncion {
                        numero: linea_cuerpo.numero,
                        sangria: linea_cuerpo.sangria,
                        instruccion: linea_cuerpo.instruccion.to_string(),
                    });
                    cursor += 1;
                }
                if !cuerpo
                    .iter()
                    .any(|linea| linea.instruccion.starts_with("devolver "))
                {
                    return Err(format!(
                        "Línea {numero_funcion}: la función necesita una línea 'devolver expresión'."
                    ));
                }
                funciones.insert(
                    nombre,
                    Funcion {
                        parametros,
                        cuerpo: CuerpoFuncion::Bloque(cuerpo),
                        numero_linea: numero_funcion,
                    },
                );
                *indice = cursor;
                continue;
            }
        }

        if let Some(expresion) = linea.instruccion.strip_prefix("repetir ") {
            let numero_repeticion = linea.numero;
            let cantidad =
                leer_cantidad_repeticiones(numero_repeticion, expresion, variables, funciones, 0)?;
            *indice += 1;
            let inicio_cuerpo = siguiente_no_vacia(lineas, *indice);
            if inicio_cuerpo >= lineas.len() || lineas[inicio_cuerpo].sangria <= sangria {
                return Err(format!(
                    "Línea {numero_repeticion}: añade debajo de 'repetir' una línea con sangría."
                ));
            }
            let sangria_cuerpo = lineas[inicio_cuerpo].sangria;
            if cantidad == 0 {
                *indice = inicio_cuerpo;
                saltar_bloque(lineas, indice, sangria_cuerpo);
            } else {
                let mut fin_cuerpo = inicio_cuerpo;
                for _ in 0..cantidad {
                    let mut indice_vuelta = inicio_cuerpo;
                    ejecutar_bloque(
                        lineas,
                        &mut indice_vuelta,
                        sangria_cuerpo,
                        variables,
                        funciones,
                    )?;
                    fin_cuerpo = indice_vuelta;
                }
                *indice = fin_cuerpo;
            }
            continue;
        }

        if linea.instruccion == "repetir" {
            return Err(format!(
                "Línea {}: escribe repetir número veces, por ejemplo: repetir 3 veces.",
                linea.numero
            ));
        }

        if let Some(expresion) = linea.instruccion.strip_prefix("si ") {
            let numero_condicion = linea.numero;
            let condicion =
                evaluar_condicion(numero_condicion, expresion, variables, funciones, 0)?;
            *indice += 1;
            let inicio_cuerpo = siguiente_no_vacia(lineas, *indice);
            if inicio_cuerpo >= lineas.len() || lineas[inicio_cuerpo].sangria <= sangria {
                return Err(format!(
                    "Línea {numero_condicion}: añade debajo de 'si' una línea con sangría, por ejemplo: 4 espacios y luego decir."
                ));
            }
            let sangria_cuerpo = lineas[inicio_cuerpo].sangria;
            *indice = inicio_cuerpo;
            if condicion {
                ejecutar_bloque(lineas, indice, sangria_cuerpo, variables, funciones)?;
            } else {
                saltar_bloque(lineas, indice, sangria_cuerpo);
            }

            let posible_si_no = siguiente_no_vacia(lineas, *indice);
            if posible_si_no < lineas.len()
                && lineas[posible_si_no].sangria == sangria
                && lineas[posible_si_no].instruccion == "si no"
            {
                *indice = posible_si_no + 1;
                let inicio_alternativa = siguiente_no_vacia(lineas, *indice);
                if inicio_alternativa >= lineas.len()
                    || lineas[inicio_alternativa].sangria <= sangria
                {
                    return Err(format!(
                        "Línea {}: añade debajo de 'si no' una línea con sangría.",
                        lineas[posible_si_no].numero
                    ));
                }
                let sangria_alternativa = lineas[inicio_alternativa].sangria;
                *indice = inicio_alternativa;
                if condicion {
                    saltar_bloque(lineas, indice, sangria_alternativa);
                } else {
                    ejecutar_bloque(lineas, indice, sangria_alternativa, variables, funciones)?;
                }
            }
            continue;
        }

        ejecutar_linea(linea.numero, linea.instruccion, variables, funciones, 0)?;
        *indice += 1;
    }
    Ok(())
}

fn ejecutar_archivo(ruta: &Path) -> Result<(), String> {
    if ruta.extension().and_then(|ext| ext.to_str()) != Some("esp") {
        return Err("el archivo debe terminar en .esp".to_string());
    }
    let contenido = fs::read_to_string(ruta)
        .map_err(|error| format!("no pude abrir '{}': {error}", ruta.display()))?;
    let lineas = preparar_lineas(&contenido);
    let mut variables = Variables::new();
    let mut funciones = Funciones::new();
    let mut indice = 0;
    ejecutar_bloque(&lineas, &mut indice, 0, &mut variables, &mut funciones)
}

fn iniciar_consola() -> Result<(), String> {
    println!(
        "EspañolCode {VERSION} — prueba 'decir', 'guardar' o define una función; escribe 'salir' para terminar."
    );
    let stdin = io::stdin();
    let mut linea = String::new();
    let mut variables = Variables::new();
    let mut funciones = Funciones::new();
    loop {
        print!("> ");
        io::stdout().flush().map_err(|error| error.to_string())?;
        linea.clear();
        let bytes = stdin
            .read_line(&mut linea)
            .map_err(|error| error.to_string())?;
        if bytes == 0 || linea.trim() == "salir" {
            break;
        }
        if let Err(error) = ejecutar_linea(1, &linea, &mut variables, &mut funciones, 0) {
            mostrar_error(&error);
        }
    }
    Ok(())
}

fn ayuda() {
    println!("EspañolCode {VERSION}\n\nUso:\n  espanolcode              Abre la consola interactiva\n  espanolcode --bloques    Abre el editor visual estilo Scratch\n  espanolcode archivo.esp  Ejecuta un programa\n  espanolcode --ayuda      Muestra esta ayuda\n\nInstrucciones disponibles:\n  decir expresión          Muestra texto, números o variables\n  guardar nombre = valor  Crea o cambia una variable\n  guardar x = preguntar \"?\"  Pregunta y guarda la respuesta\n  agregar lista = valor  Añade un elemento a la lista\n  importar módulo         Reconoce una importación (todavía no carga módulos)\n  desde módulo importar nombre  Reconoce una importación selectiva\n  cantidad(lista)         Cuenta elementos de una lista o texto\n  absoluto(n), redondear(n)  Funciones matemáticas sencillas\n  minimo(lista), maximo(lista)  Busca el menor o mayor número\n  texto(valor), numero(texto)  Convierte entre texto y número\n  leer_archivo(ruta)      Lee un archivo de texto\n  escribir_archivo(ruta, texto)  Crea o reemplaza un archivo\n  agregar_archivo(ruta, texto)   Añade texto al final\n  funcion doble(n) = n * 2  Define una función que devuelve su resultado\n  funcion area(a, b)       Define una función con un bloque indentado\n  devolver expresión       Devuelve el resultado dentro de una función\n\nListas: [\"Ana\", \"Luis\"], índices desde 0\nOperaciones: + - * / y paréntesis\nCondiciones de archivos: si comparación / si no; indenta sus bloques\nRepeticiones de archivos: repetir N veces; máximo 100000");
}

fn json_cadena(texto: &str) -> String {
    let mut salida = String::from("\"");
    for caracter in texto.chars() {
        match caracter {
            '"' => salida.push_str("\\\""),
            '\\' => salida.push_str("\\\\"),
            '\n' => salida.push_str("\\n"),
            '\r' => salida.push_str("\\r"),
            '\t' => salida.push_str("\\t"),
            caracter if caracter.is_control() => {
                salida.push_str(&format!("\\u{:04x}", caracter as u32));
            }
            caracter => salida.push(caracter),
        }
    }
    salida.push('"');
    salida
}

fn responder_http(
    stream: &mut TcpStream,
    estado: &str,
    tipo_contenido: &str,
    contenido: &[u8],
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {estado}\r\nContent-Type: {tipo_contenido}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        contenido.len()
    )?;
    stream.write_all(contenido)
}

fn leer_peticion(stream: &mut TcpStream) -> io::Result<(String, String, String, Vec<u8>)> {
    let mut datos = Vec::new();
    let mut fragmento = [0u8; 4096];
    let mut fin_cabecera = None;
    let mut largo_cuerpo = 0usize;
    let mut token = String::new();
    loop {
        let leidos = stream.read(&mut fragmento)?;
        if leidos == 0 {
            break;
        }
        datos.extend_from_slice(&fragmento[..leidos]);
        if datos.len() > 8 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "petición demasiado grande",
            ));
        }
        if fin_cabecera.is_none() {
            if let Some(fin) = datos.windows(4).position(|ventana| ventana == b"\r\n\r\n") {
                let cabecera = String::from_utf8_lossy(&datos[..fin]);
                for linea in cabecera.lines().skip(1) {
                    if let Some((nombre, valor)) = linea.split_once(':') {
                        if nombre.eq_ignore_ascii_case("content-length") {
                            largo_cuerpo = valor.trim().parse::<usize>().unwrap_or(0);
                        } else if nombre.eq_ignore_ascii_case("x-espanolcode-token") {
                            token = valor.trim().to_string();
                        }
                    }
                }
                if largo_cuerpo > 4 * 1024 * 1024 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "código demasiado grande",
                    ));
                }
                fin_cabecera = Some(fin);
            }
        }
        if fin_cabecera.is_some_and(|fin| datos.len() >= fin + 4 + largo_cuerpo) {
            break;
        }
    }
    let Some(fin) = fin_cabecera else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cabecera HTTP incompleta",
        ));
    };
    if datos.len() < fin + 4 + largo_cuerpo {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "cuerpo HTTP incompleto",
        ));
    }
    let cabecera = String::from_utf8_lossy(&datos[..fin]);
    let primera = cabecera.lines().next().unwrap_or_default();
    let mut partes = primera.split_whitespace();
    let metodo = partes.next().unwrap_or_default().to_string();
    let ruta = partes.next().unwrap_or_default().to_string();
    let inicio = fin + 4;
    let cuerpo = datos[inicio..inicio + largo_cuerpo].to_vec();
    Ok((metodo, ruta, token, cuerpo))
}

fn ejecutar_codigo_bloques(codigo: &str) -> (bool, i32, String, String) {
    let marca_tiempo = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duracion| duracion.as_nanos())
        .unwrap_or_default();
    let temporal = env::temp_dir().join(format!(
        "espanolcode-bloques-{}-{marca_tiempo}.esp",
        process::id()
    ));
    if let Err(error) = fs::write(&temporal, codigo) {
        return (
            false,
            1,
            String::new(),
            format!("No pude crear el programa temporal: {error}"),
        );
    }
    let resultado = env::current_exe()
        .map_err(|error| error.to_string())
        .and_then(|ejecutable| {
            Command::new(ejecutable)
                .arg(&temporal)
                .output()
                .map_err(|error| error.to_string())
        });
    let _ = fs::remove_file(&temporal);
    match resultado {
        Ok(salida) => (
            salida.status.success(),
            salida.status.code().unwrap_or(1),
            String::from_utf8_lossy(&salida.stdout).into_owned(),
            String::from_utf8_lossy(&salida.stderr).into_owned(),
        ),
        Err(error) => (
            false,
            1,
            String::new(),
            format!("No pude ejecutar EspañolCode: {error}"),
        ),
    }
}

fn atender_peticion(stream: &mut TcpStream, token_sesion: &str) -> io::Result<bool> {
    let (metodo, ruta, token, cuerpo) = match leer_peticion(stream) {
        Ok(peticion) => peticion,
        Err(error) => {
            responder_http(
                stream,
                "400 Bad Request",
                "text/plain; charset=utf-8",
                error.to_string().as_bytes(),
            )?;
            return Ok(true);
        }
    };
    if metodo == "POST" && token != token_sesion {
        responder_http(
            stream,
            "403 Forbidden",
            "text/plain; charset=utf-8",
            "Token no válido".as_bytes(),
        )?;
        return Ok(true);
    }
    match (metodo.as_str(), ruta.as_str()) {
        ("GET", "/") => {
            let pagina = EDITOR_BLOQUES.replace("__ESPANOLCODE_TOKEN__", token_sesion);
            responder_http(
                stream,
                "200 OK",
                "text/html; charset=utf-8",
                pagina.as_bytes(),
            )?;
        }
        ("POST", "/run") => {
            let codigo = String::from_utf8_lossy(&cuerpo);
            let (correcto, codigo_salida, salida, error) = ejecutar_codigo_bloques(&codigo);
            let respuesta = format!(
                "{{\"ok\":{correcto},\"code\":{codigo_salida},\"stdout\":{},\"stderr\":{}}}",
                json_cadena(&salida),
                json_cadena(&error)
            );
            responder_http(
                stream,
                "200 OK",
                "application/json; charset=utf-8",
                respuesta.as_bytes(),
            )?;
        }
        ("POST", "/shutdown") => {
            responder_http(stream, "200 OK", "text/plain; charset=utf-8", b"cerrado")?;
            return Ok(false);
        }
        _ => responder_http(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"No encontrado",
        )?,
    }
    Ok(true)
}

#[cfg(target_os = "windows")]
fn abrir_navegador(url: &str) -> io::Result<()> {
    Command::new("cmd.exe")
        .args(["/C", "start", "", url])
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "macos")]
fn abrir_navegador(url: &str) -> io::Result<()> {
    Command::new("open").arg(url).spawn().map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn abrir_navegador(url: &str) -> io::Result<()> {
    Command::new("xdg-open").arg(url).spawn().map(|_| ())
}

fn iniciar_editor_bloques() -> Result<(), String> {
    let servidor = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let direccion = servidor.local_addr().map_err(|error| error.to_string())?;
    let marca_tiempo = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duracion| duracion.as_nanos())
        .unwrap_or_default();
    let token = format!(
        "{:x}-{:x}-{:x}",
        process::id(),
        marca_tiempo,
        direccion.port()
    );
    let url = format!("http://{direccion}/");
    println!("Abriendo el editor de bloques en {url}");
    if env::var_os("ESPANOLCODE_NO_ABRIR_NAVEGADOR").is_none() {
        if let Err(error) = abrir_navegador(&url) {
            eprintln!(
                "No pude abrir el navegador automáticamente: {error}\nCopia esta dirección: {url}"
            );
        }
    }
    println!("Elige 'Cerrar' en el editor o pulsa Ctrl+C para salir.");
    for peticion in servidor.incoming() {
        match peticion {
            Ok(mut stream) => match atender_peticion(&mut stream, &token) {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => eprintln!("Error del editor de bloques: {error}"),
            },
            Err(error) => eprintln!("No pude recibir una petición del editor: {error}"),
        }
    }
    Ok(())
}

fn ejecutar() -> Result<(), String> {
    let argumentos: Vec<String> = env::args().skip(1).collect();
    match argumentos.as_slice() {
        [] => iniciar_consola(),
        [opcion] if opcion == "--bloques" || opcion == "--blocks" => iniciar_editor_bloques(),
        [opcion] if opcion == "--ayuda" || opcion == "--help" || opcion == "-h" => {
            ayuda();
            Ok(())
        }
        [opcion] if opcion == "--version" || opcion == "-V" => {
            println!("EspañolCode {VERSION}");
            Ok(())
        }
        [archivo] => ejecutar_archivo(Path::new(archivo)),
        _ => Err("usa 'espanolcode archivo.esp' o 'espanolcode --ayuda'".to_string()),
    }
}

fn main() {
    if let Err(error) = ejecutar() {
        mostrar_error(&error);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::validar_importacion;

    #[test]
    fn reconoce_formas_de_importacion() {
        for instruccion in [
            "importar math",
            "importar math como matematicas, random",
            "desde math importar sqrt como raiz",
            "desde .utilidades importar ayuda",
            "desde paquete.subpaquete importar (uno, dos)",
            "desde math importar *",
        ] {
            assert_eq!(
                validar_importacion(1, instruccion),
                Ok(true),
                "{instruccion}"
            );
        }
    }

    #[test]
    fn rechaza_importaciones_mal_formadas() {
        for instruccion in [
            "importar",
            "importar math,",
            "importar .math",
            "importar (math)",
            "desde importar raiz",
            "desde math importa raiz",
            "desde math importar raiz,",
            "desde math importar *, raiz",
            "desde math importar raiz como",
        ] {
            assert!(
                validar_importacion(7, instruccion).is_err(),
                "{instruccion}"
            );
        }
    }
}
