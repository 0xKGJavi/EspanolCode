const vscode = require('vscode');
const fs = require('fs');
const os = require('os');
const path = require('path');
const funcionesIntegradas = new Set(['cantidad', 'leer_archivo', 'escribir_archivo', 'agregar_archivo', 'absoluto', 'redondear', 'minimo', 'maximo', 'texto', 'numero']);

function analizarExpresion(expresion, funciones = new Map()) {
  const patron = /\s*(?:(\d+(?:\.\d+)?)|("(?:[^"\\]|\\(?:\\|"|n|t))*")|([\p{L}_][\p{L}\p{N}_]*)|(\[|\]|\(|\)|,|[+\-*/]))/uy;
  const tokens = [];
  let posicion = 0;
  while (posicion < expresion.length) {
    if (!expresion.slice(posicion).trim()) break;
    patron.lastIndex = posicion;
    const coincidencia = patron.exec(expresion);
    if (!coincidencia) return null;
    posicion = patron.lastIndex;
    if (coincidencia[1] || coincidencia[2]) tokens.push({ tipo: 'valor' });
    else if (coincidencia[3]) tokens.push({ tipo: 'nombre', valor: coincidencia[3] });
    else {
      const signo = coincidencia[4];
      const tipos = { '(': 'abre', ')': 'cierra', '[': 'listaAbre', ']': 'listaCierra', ',': 'coma' };
      tokens.push({ tipo: tipos[signo] || 'operador', valor: signo });
    }
  }

  const referencias = [];
  let indice = 0;
  function llamada(cantidadEsperada) {
    if (tokens[indice]?.tipo !== 'abre') return false;
    indice += 1;
    if (tokens[indice]?.tipo === 'cierra') {
      indice += 1;
      return cantidadEsperada === 0;
    }
    let cantidad = 0;
    while (true) {
      if (!suma()) return false;
      cantidad += 1;
      if (tokens[indice]?.tipo === 'coma') {
        indice += 1;
        continue;
      }
      if (tokens[indice]?.tipo === 'cierra') {
        indice += 1;
        break;
      }
      return false;
    }
    return Array.isArray(cantidadEsperada)
      ? cantidadEsperada.includes(cantidad)
      : cantidad === cantidadEsperada;
  }
  function indices() {
    while (tokens[indice]?.tipo === 'listaAbre') {
      indice += 1;
      if (!suma() || tokens[indice]?.tipo !== 'listaCierra') return false;
      indice += 1;
    }
    return true;
  }
  function primario() {
    const token = tokens[indice];
    if (!token) return false;
    if (token.tipo === 'operador' && token.valor === '-') {
      indice += 1;
      return primario();
    }
    if (token.tipo === 'abre') {
      indice += 1;
      if (!suma() || tokens[indice]?.tipo !== 'cierra') return false;
      indice += 1;
      return indices();
    }
    if (token.tipo === 'listaAbre') {
      indice += 1;
      if (tokens[indice]?.tipo === 'listaCierra') {
        indice += 1;
        return indices();
      }
      while (true) {
        if (!suma()) return false;
        if (tokens[indice]?.tipo === 'coma') {
          indice += 1;
          if (tokens[indice]?.tipo === 'listaCierra') {
            indice += 1;
            break;
          }
          continue;
        }
        if (tokens[indice]?.tipo === 'listaCierra') {
          indice += 1;
          break;
        }
        return false;
      }
      return indices();
    }
    if (token.tipo === 'valor') {
      indice += 1;
      return indices();
    }
    if (token.tipo === 'nombre') {
      indice += 1;
      const aridades = { cantidad: 1, leer_archivo: 1, escribir_archivo: 2, agregar_archivo: 2, absoluto: 1, redondear: 1, minimo: [1, 2], maximo: [1, 2], texto: 1, numero: 1 };
      if (Object.hasOwn(aridades, token.valor)) {
        if (tokens[indice]?.tipo !== 'abre') {
          referencias.push(token.valor);
        } else if (!llamada(aridades[token.valor])) {
          return false;
        }
      } else if (funciones.has(token.valor)) {
        if (!llamada(funciones.get(token.valor))) return false;
      } else {
        referencias.push(token.valor);
      }
      return indices();
    }
    return false;
  }
  function producto() {
    if (!primario()) return false;
    while (tokens[indice]?.tipo === 'operador' && ['*', '/'].includes(tokens[indice].valor)) {
      indice += 1;
      if (!primario()) return false;
    }
    return true;
  }
  function suma() {
    if (!producto()) return false;
    while (tokens[indice]?.tipo === 'operador' && ['+', '-'].includes(tokens[indice].valor)) {
      indice += 1;
      if (!producto()) return false;
    }
    return true;
  }

  return suma() && indice === tokens.length ? referencias : null;
}

function analizarCondicion(expresion, funciones = new Map()) {
  let dentroDeTexto = false;
  let escapado = false;
  let parentesis = 0;
  for (let indice = 0; indice < expresion.length; indice += 1) {
    const caracter = expresion[indice];
    if (dentroDeTexto) {
      if (escapado) escapado = false;
      else if (caracter === '\\') escapado = true;
      else if (caracter === '"') dentroDeTexto = false;
      continue;
    }
    if (caracter === '"') { dentroDeTexto = true; continue; }
    if (caracter === '(') { parentesis += 1; continue; }
    if (caracter === ')') { parentesis = Math.max(0, parentesis - 1); continue; }
    if (parentesis > 0) continue;

    const operador = ['==', '!=', '>=', '<=', '>', '<'].find((candidato) => expresion.startsWith(candidato, indice));
    if (operador) {
      const izquierda = analizarExpresion(expresion.slice(0, indice), funciones);
      const derecha = analizarExpresion(expresion.slice(indice + operador.length), funciones);
      return izquierda && derecha ? [...izquierda, ...derecha] : null;
    }
  }
  return null;
}

function validar(documento, diagnosticos) {
  if (documento.languageId !== 'espanolcode') return;

  const encontrados = [];
  const variables = new Set();
  const funciones = new Map();
  const bloques = [];
  let funcionActual = null;
  const nombreValido = /^[\p{L}_][\p{L}\p{N}_]*$/u;
  const reservadas = new Set(['decir', 'guardar', 'preguntar', 'agregar', 'si', 'no', 'repetir', 'veces', 'funcion']);

  for (let indice = 0; indice < documento.lineCount; indice += 1) {
    const linea = documento.lineAt(indice);
    const texto = linea.text.trim();
    if (!texto) continue;
    const sangria = linea.text.match(/^[ \t]*/)[0].replace(/\t/g, '    ').length;
    if (funcionActual && sangria <= funcionActual.sangria) {
      if (!funcionActual.devolvio) {
        encontrados.push(new vscode.Diagnostic(
          funcionActual.linea.range,
          'La función necesita una línea devolver expresión.',
          vscode.DiagnosticSeverity.Error
        ));
      }
      funcionActual = null;
    }
    const variablesActivas = funcionActual ? funcionActual.variables : variables;
    if (funcionActual && sangria > funcionActual.sangria && /^funcion(?:\s|$)/u.test(texto)) {
      encontrados.push(new vscode.Diagnostic(
        linea.range,
        'Por ahora no se pueden definir funciones dentro de otra función.',
        vscode.DiagnosticSeverity.Error
      ));
      continue;
    }

    if (/^devolver(?:\s|$)/u.test(texto)) {
      const expresion = texto.replace(/^devolver\s*/u, '');
      const referencias = expresion && analizarExpresion(expresion, funciones);
      const desconocida = referencias && referencias.find((nombre) => !variablesActivas.has(nombre));
      if (!funcionActual) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          "'devolver' solo se puede usar dentro de una función.",
          vscode.DiagnosticSeverity.Error
        ));
      } else if (!referencias) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'Escribe una expresión después de devolver.',
          vscode.DiagnosticSeverity.Error
        ));
      } else if (desconocida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la variable '${desconocida}' para devolverla.`,
          vscode.DiagnosticSeverity.Error
        ));
      } else {
        funcionActual.devolvio = true;
      }
      continue;
    }

    if (texto === 'si no') {
      while (bloques.length && bloques[bloques.length - 1].sangria > sangria) bloques.pop();
      const bloque = bloques[bloques.length - 1];
      if (!bloque || bloque.tipo !== 'si' || bloque.sangria !== sangria || bloque.tieneAlternativa) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          "'si no' debe estar alineado con su 'si' y solo puede aparecer una vez.",
          vscode.DiagnosticSeverity.Error
        ));
      } else {
        bloque.tieneAlternativa = true;
      }
      continue;
    }

    while (bloques.length && bloques[bloques.length - 1].sangria >= sangria) bloques.pop();

    if (/^funcion(?:\s|$)/u.test(texto)) {
      const coincidencia = /^funcion\s+([\p{L}_][\p{L}\p{N}_]*)\s*\((.*?)\)\s*(?:=\s*(.*?))?\s*$/u.exec(texto);
      const nombre = coincidencia && coincidencia[1];
      const parametrosTexto = coincidencia && coincidencia[2];
      const expresion = coincidencia && coincidencia[3];
      const parametros = parametrosTexto && parametrosTexto.trim()
        ? parametrosTexto.split(',').map((parametro) => parametro.trim()) : [];
      const nombresProhibidos = new Set([...reservadas, ...funcionesIntegradas]);
      const parametrosMalos = parametros.some((parametro, indiceParametro) =>
        !nombreValido.test(parametro) || reservadas.has(parametro) || parametros.indexOf(parametro) !== indiceParametro
      );
      if (!coincidencia || (expresion !== undefined && !expresion) || nombresProhibidos.has(nombre) || !nombreValido.test(nombre) || parametrosMalos) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'Usa funcion nombre(parametros) = expresión, o una cabecera con bloque indentado.',
          vscode.DiagnosticSeverity.Error
        ));
        continue;
      }
      const firmas = new Map(funciones);
      firmas.set(nombre, parametros.length);
      if (expresion === undefined) {
        funciones.set(nombre, parametros.length);
        funcionActual = {
          sangria,
          linea,
          variables: new Set([...variables, ...parametros]),
          devolvio: false,
        };
        continue;
      }
      const referencias = analizarExpresion(expresion, firmas);
      const locales = new Set([...variables, ...parametros]);
      const desconocida = referencias && referencias.find((referencia) => !locales.has(referencia));
      if (!referencias) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'La función debe devolver una expresión válida.',
          vscode.DiagnosticSeverity.Error
        ));
      } else if (desconocida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la variable '${desconocida}' en la expresión de la función.`,
          vscode.DiagnosticSeverity.Error
        ));
      } else {
        funciones.set(nombre, parametros.length);
      }
      continue;
    }

    if (/^si(?:\s|$)/u.test(texto)) {
      const coincidencia = /^si\s+(.+)$/u.exec(texto);
      const referencias = coincidencia && analizarCondicion(coincidencia[1], funciones);
      const desconocida = referencias && referencias.find((nombre) => reservadas.has(nombre) || !variablesActivas.has(nombre));
      if (!referencias) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'Escribe una comparación después de si, por ejemplo: si edad >= 18.',
          vscode.DiagnosticSeverity.Error
        ));
      } else if (desconocida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la variable '${desconocida}' en esta condición.`,
          vscode.DiagnosticSeverity.Error
        ));
      } else {
        bloques.push({ sangria, tipo: 'si', tieneAlternativa: false });
      }
      continue;
    }

    if (/^decir(?:\s|$)/u.test(texto)) {
      const coincidencia = /^decir\s+(.+?)\s*$/u.exec(texto);
      const referencias = coincidencia && analizarExpresion(coincidencia[1], funciones);
      const desconocida = referencias && referencias.find((nombre) => reservadas.has(nombre) || !variablesActivas.has(nombre));
      if (!referencias) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'Usa decir con texto, números, variables u operaciones como decir 2 + 3.',
          vscode.DiagnosticSeverity.Error
        ));
      } else if (desconocida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la variable '${desconocida}'. Guárdala primero con guardar.`,
          vscode.DiagnosticSeverity.Error
        ));
      }
      continue;
    }

    if (/^guardar(?:\s|$)/u.test(texto)) {
      const coincidencia = /^guardar\s+([\p{L}_][\p{L}\p{N}_]*)\s*=\s*(.*?)\s*$/u.exec(texto);
      const nombre = coincidencia && coincidencia[1];
      const valor = coincidencia && coincidencia[2];
      const pregunta = valor && /^preguntar\s+(.+)$/u.exec(valor);
      const referencias = valor && analizarExpresion(pregunta ? pregunta[1] : valor, funciones);
      const desconocida = referencias && referencias.find((referencia) => reservadas.has(referencia) || !variablesActivas.has(referencia));
      if (!coincidencia || !referencias || reservadas.has(nombre)) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'Usa guardar nombre = expresión o guardar nombre = preguntar "¿Pregunta?".',
          vscode.DiagnosticSeverity.Error
        ));
      } else if (desconocida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la variable '${desconocida}'. Guárdala antes de usarla.`,
          vscode.DiagnosticSeverity.Error
        ));
      } else {
        variablesActivas.add(nombre);
      }
      continue;
    }

    if (/^agregar(?:\s|$)/u.test(texto)) {
      const coincidencia = /^agregar\s+([\p{L}_][\p{L}\p{N}_]*)\s*=\s*(.*?)\s*$/u.exec(texto);
      const nombre = coincidencia && coincidencia[1];
      const valor = coincidencia && coincidencia[2];
      const pregunta = valor && /^preguntar\s+(.+)$/u.exec(valor);
      const referencias = valor && analizarExpresion(pregunta ? pregunta[1] : valor, funciones);
      const desconocida = referencias && referencias.find((referencia) => reservadas.has(referencia) || !variablesActivas.has(referencia));
      if (!coincidencia || !referencias || reservadas.has(nombre)) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'Usa agregar lista = valor, por ejemplo: agregar frutas = "pera".',
          vscode.DiagnosticSeverity.Error
        ));
      } else if (!variablesActivas.has(nombre)) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la lista '${nombre}'. Créala primero con guardar.`,
          vscode.DiagnosticSeverity.Error
        ));
      } else if (desconocida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la variable '${desconocida}' que quieres agregar.`,
          vscode.DiagnosticSeverity.Error
        ));
      }
      continue;
    }

    if (/^repetir(?:\s|$)/u.test(texto)) {
      const coincidencia = /^repetir\s+(.+?)\s+veces$/u.exec(texto);
      const referencias = coincidencia && analizarExpresion(coincidencia[1], funciones);
      const literal = coincidencia && /^-?\d+(?:\.\d+)?$/u.test(coincidencia[1].trim())
        ? Number(coincidencia[1].trim()) : null;
      const cantidadLiteralValida = literal === null || (Number.isInteger(literal) && literal >= 0 && literal <= 100000);
      const desconocida = referencias && referencias.find((nombre) => reservadas.has(nombre) || !variablesActivas.has(nombre));
      if (!referencias || !cantidadLiteralValida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'Usa repetir un número entero entre 0 y 100000 veces, por ejemplo: repetir 3 veces.',
          vscode.DiagnosticSeverity.Error
        ));
      } else if (desconocida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la variable '${desconocida}' para el número de repeticiones.`,
          vscode.DiagnosticSeverity.Error
        ));
      } else {
        bloques.push({ sangria, tipo: 'repetir', tieneAlternativa: false });
      }
      continue;
    }

    if (/^(?:escribir_archivo|agregar_archivo)\b/u.test(texto)) {
      const referencias = analizarExpresion(texto, funciones);
      const desconocida = referencias && referencias.find((nombre) => !variablesActivas.has(nombre));
      if (!referencias) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          'Usa escribir_archivo("ruta", contenido) o agregar_archivo("ruta", contenido).',
          vscode.DiagnosticSeverity.Error
        ));
      } else if (desconocida) {
        encontrados.push(new vscode.Diagnostic(
          linea.range,
          `No encuentro la variable '${desconocida}' en el contenido del archivo.`,
          vscode.DiagnosticSeverity.Error
        ));
      }
      continue;
    }

    encontrados.push(new vscode.Diagnostic(
      linea.range,
      "Esta instrucción aún no está disponible en el intérprete. Por ahora puedes usar 'decir', 'guardar', 'preguntar', 'agregar', 'funcion', 'si', 'si no', 'repetir' y operaciones con archivos.",
      vscode.DiagnosticSeverity.Warning
    ));
  }
  if (funcionActual && !funcionActual.devolvio) {
    encontrados.push(new vscode.Diagnostic(
      funcionActual.linea.range,
      'La función necesita una línea devolver expresión.',
      vscode.DiagnosticSeverity.Error
    ));
  }
  diagnosticos.set(documento.uri, encontrados);
}

function entreComillasPowerShell(texto) {
  return `'${texto.replace(/'/g, "''")}'`;
}

function entreComillasShell(texto) {
  return `'${texto.replace(/'/g, `'"'"'`)}'`;
}

function comandoParaTerminal(ejecutable, archivo) {
  const terminalConfig = vscode.workspace.getConfiguration('terminal.integrated');
  const perfil = String(terminalConfig.get(`defaultProfile.${process.platform === 'win32' ? 'windows' : process.platform}`) || '');
  if (process.platform === 'win32' && /command prompt|(^|\b)cmd(\b|$)/iu.test(perfil)) {
    return `"${ejecutable}" "${archivo}"`;
  }
  if (process.platform === 'win32' && !/bash|zsh/i.test(perfil)) {
    return `& ${entreComillasPowerShell(ejecutable)} ${entreComillasPowerShell(archivo)}`;
  }
  return `${entreComillasShell(ejecutable)} ${entreComillasShell(archivo)}`;
}

async function ejecutarArchivo(diagnosticos) {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'espanolcode') {
    vscode.window.showWarningMessage('Abre primero un archivo .esp para ejecutarlo.');
    return;
  }

  const documento = editor.document;
  if (documento.isUntitled || documento.isDirty) {
    const guardado = await documento.save();
    if (!guardado) {
      vscode.window.showWarningMessage('Guarda el archivo antes de ejecutarlo.');
      return;
    }
  }

  validar(documento, diagnosticos);
  const errores = diagnosticos.get(documento.uri) || [];
  if (errores.some((diagnostico) => diagnostico.severity === vscode.DiagnosticSeverity.Error)) {
    vscode.window.showErrorMessage('Corrige los errores señalados antes de ejecutar el programa.');
    return;
  }

  const config = vscode.workspace.getConfiguration('espanolcode');
  let ejecutable = config.get('rutaEjecutable', 'espanolcode').trim() || 'espanolcode';
  if (ejecutable === 'espanolcode') {
    const posiblesRutas = process.platform === 'win32'
      ? [
          process.env.LOCALAPPDATA && path.join(process.env.LOCALAPPDATA, 'Programs', 'EspanolCode', 'espanolcode.exe'),
          path.resolve(__dirname, '..', 'target', 'release', 'espanolcode.exe')
        ]
      : [
          path.join(os.homedir(), '.local', 'bin', 'espanolcode'),
          path.resolve(__dirname, '..', 'target', 'release', 'espanolcode')
        ];
    ejecutable = posiblesRutas.find((ruta) => ruta && fs.existsSync(ruta)) || ejecutable;
  }
  const terminal = vscode.window.createTerminal({ name: 'EspañolCode' });
  terminal.show(true);
  terminal.sendText(comandoParaTerminal(ejecutable, documento.fileName), true);
}

function activate(context) {
  const diagnosticos = vscode.languages.createDiagnosticCollection('espanolcode');
  context.subscriptions.push(diagnosticos);

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((documento) => validar(documento, diagnosticos)),
    vscode.workspace.onDidChangeTextDocument((evento) => validar(evento.document, diagnosticos)),
    vscode.workspace.onDidCloseTextDocument((documento) => diagnosticos.delete(documento.uri)),
    vscode.commands.registerCommand('espanolcode.ejecutarArchivo', () => ejecutarArchivo(diagnosticos))
  );

  for (const documento of vscode.workspace.textDocuments) {
    validar(documento, diagnosticos);
  }
}

function deactivate() {}

module.exports = { activate, deactivate };
