; Instalador de EspañolCode para Windows (Inno Setup 6).
; Lo fabrica GitHub Actions (.github/workflows/instaladores.yml). A mano, desde la raíz del proyecto:
;   cargo build --release
;   cd vscode-extension && npx --yes @vscode/vsce@4.0.0 package --allow-missing-repository -o ..\target\espanolcode.vsix && cd ..
;   iscc /DVersion=0.12.0 instaladores\windows\espanolcode.iss
; El resultado queda en dist\.

#ifndef Version
  #error Falta la versión. Compila con: iscc /DVersion=X.Y.Z instaladores\windows\espanolcode.iss
#endif

#define Raiz "..\.."

[Setup]
AppId={{AE62A7FD-1456-4C24-8DAF-92CC00E4B5CA}
AppName=EspañolCode
AppVersion={#Version}
AppVerName=EspañolCode {#Version}
AppPublisher=0xKGJavi
AppPublisherURL=https://github.com/0xKGJavi/EspanolCode
AppSupportURL=https://github.com/0xKGJavi/EspanolCode/issues
DefaultDirName={autopf}\EspanolCode
DefaultGroupName=EspañolCode
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ChangesEnvironment=yes
ChangesAssociations=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#Raiz}\dist
OutputBaseFilename=EspanolCode-{#Version}-windows
SetupIconFile=espanolcode.ico
UninstallDisplayIcon={app}\espanolcode.ico
UninstallDisplayName=EspañolCode
WizardStyle=modern
Compression=lzma2
SolidCompression=yes

[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "path"; Description: "Poder usar el comando espanolcode en cualquier terminal"; GroupDescription: "Opciones:"
Name: "asociar"; Description: "Ejecutar los archivos .esp con doble clic (clic derecho > Editar para cambiarlos)"; GroupDescription: "Opciones:"
Name: "vscode"; Description: "Instalar la extensión de EspañolCode en Visual Studio Code"; GroupDescription: "Opciones:"; Check: VSCodeEncontrado

[Files]
Source: "{#Raiz}\target\release\espanolcode.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "ejecutar-esp.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "espanolcode.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Raiz}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Raiz}\ejemplos\*.esp"; DestDir: "{app}\ejemplos"; Flags: ignoreversion
Source: "{#Raiz}\target\espanolcode.vsix"; DestDir: "{tmp}"; Flags: deleteafterinstall; Tasks: vscode

[Icons]
Name: "{group}\EspañolCode"; Filename: "{app}\espanolcode.exe"; WorkingDir: "{userdocs}"; IconFilename: "{app}\espanolcode.ico"; Comment: "Consola de EspañolCode: escribe instrucciones y mira qué hacen"
Name: "{group}\Ejemplos de EspañolCode"; Filename: "{app}\ejemplos"
Name: "{group}\Desinstalar EspañolCode"; Filename: "{uninstallexe}"
; Cuando el editor de bloques esté terminado, quita el ";" del principio de la línea siguiente:
; Name: "{group}\Editor de bloques de EspañolCode"; Filename: "{app}\espanolcode.exe"; Parameters: "--bloques"; WorkingDir: "{userdocs}"; IconFilename: "{app}\espanolcode.ico"

[Registry]
Root: HKA; Subkey: "Software\Classes\.esp"; ValueType: string; ValueName: ""; ValueData: "EspanolCode.Programa"; Flags: uninsdeletevalue uninsdeletekeyifempty; Tasks: asociar
Root: HKA; Subkey: "Software\Classes\EspanolCode.Programa"; ValueType: string; ValueName: ""; ValueData: "Programa de EspañolCode"; Flags: uninsdeletekey; Tasks: asociar
Root: HKA; Subkey: "Software\Classes\EspanolCode.Programa\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\espanolcode.ico"; Tasks: asociar
Root: HKA; Subkey: "Software\Classes\EspanolCode.Programa\shell"; ValueType: string; ValueName: ""; ValueData: "open"; Tasks: asociar
Root: HKA; Subkey: "Software\Classes\EspanolCode.Programa\shell\open"; ValueType: string; ValueName: ""; ValueData: "Ejecutar"; Tasks: asociar
Root: HKA; Subkey: "Software\Classes\EspanolCode.Programa\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{cmd}"" /s /c """"{app}\ejecutar-esp.cmd"" ""%1"""""; Tasks: asociar
Root: HKA; Subkey: "Software\Classes\EspanolCode.Programa\shell\edit"; ValueType: string; ValueName: ""; ValueData: "Editar"; Tasks: asociar
Root: HKA; Subkey: "Software\Classes\EspanolCode.Programa\shell\edit\command"; ValueType: string; ValueName: ""; ValueData: """{win}\notepad.exe"" ""%1"""; Tasks: asociar

[UninstallRun]
Filename: "{cmd}"; Parameters: "/s /c """"{code:RutaCode}"" --uninstall-extension espanolcode.espanolcode-vscode"""; Flags: runhidden; Tasks: vscode; RunOnceId: "QuitarExtensionVSCode"

[Code]
const
  ClaveEntornoSistema = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';
  ClaveEntornoUsuario = 'Environment';

function RaizEntorno: Integer;
begin
  if IsAdminInstallMode then
    Result := HKEY_LOCAL_MACHINE
  else
    Result := HKEY_CURRENT_USER;
end;

function ClaveEntorno: String;
begin
  if IsAdminInstallMode then
    Result := ClaveEntornoSistema
  else
    Result := ClaveEntornoUsuario;
end;

{ Devuelve el PATH sin la carpeta indicada (comparando sin mayúsculas ni barra final).
  Encontrada vale True si la carpeta estaba. }
function QuitarDelPath(const Path, Carpeta: String; var Encontrada: Boolean): String;
var
  Resto, Parte: String;
  P: Integer;
begin
  Result := '';
  Encontrada := False;
  Resto := Path;
  while Resto <> '' do
  begin
    P := Pos(';', Resto);
    if P = 0 then
    begin
      Parte := Resto;
      Resto := '';
    end
    else
    begin
      Parte := Copy(Resto, 1, P - 1);
      Resto := Copy(Resto, P + 1, Length(Resto));
    end;
    if CompareText(RemoveBackslashUnlessRoot(Trim(Parte)), RemoveBackslashUnlessRoot(Carpeta)) = 0 then
      Encontrada := True
    else if Parte <> '' then
    begin
      if Result <> '' then
        Result := Result + ';';
      Result := Result + Parte;
    end;
  end;
end;

procedure AnadirAlPath(const Carpeta: String);
var
  Path: String;
  Encontrada: Boolean;
begin
  if not RegQueryStringValue(RaizEntorno, ClaveEntorno, 'Path', Path) then
    Path := '';
  QuitarDelPath(Path, Carpeta, Encontrada);
  if Encontrada then
    Exit;
  if (Path <> '') and (Path[Length(Path)] <> ';') then
    Path := Path + ';';
  RegWriteExpandStringValue(RaizEntorno, ClaveEntorno, 'Path', Path + Carpeta);
end;

procedure QuitarCarpetaDelPath(const Carpeta: String);
var
  Path, Nuevo: String;
  Encontrada: Boolean;
begin
  if not RegQueryStringValue(RaizEntorno, ClaveEntorno, 'Path', Path) then
    Exit;
  Nuevo := QuitarDelPath(Path, Carpeta, Encontrada);
  if Encontrada then
    RegWriteExpandStringValue(RaizEntorno, ClaveEntorno, 'Path', Nuevo);
end;

{ Busca code.cmd de Visual Studio Code: primero en el PATH y luego en las carpetas habituales. }
function BuscarCode: String;
begin
  Result := FileSearch('code.cmd', GetEnv('PATH'));
  if Result = '' then
    Result := ExpandConstant('{localappdata}\Programs\Microsoft VS Code\bin\code.cmd');
  if not FileExists(Result) then
    Result := ExpandConstant('{commonpf64}\Microsoft VS Code\bin\code.cmd');
  if not FileExists(Result) then
    Result := '';
end;

function VSCodeEncontrado: Boolean;
begin
  Result := BuscarCode <> '';
end;

function RutaCode(Param: String): String;
begin
  Result := BuscarCode;
end;

procedure InstalarExtensionVSCode;
var
  Code: String;
  Resultado: Integer;
begin
  Code := BuscarCode;
  if (Code = '') or
     not Exec(ExpandConstant('{cmd}'),
              '/s /c ""' + Code + '" --install-extension "' + ExpandConstant('{tmp}\espanolcode.vsix') + '" --force"',
              '', SW_HIDE, ewWaitUntilTerminated, Resultado) or
     (Resultado <> 0) then
    SuppressibleMsgBox('No se pudo instalar la extensión de Visual Studio Code. EspañolCode sí se ha instalado; puedes instalar la extensión más tarde desde VS Code.',
                       mbInformation, MB_OK, IDOK);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    if WizardIsTaskSelected('path') then
      AnadirAlPath(ExpandConstant('{app}'));
    if WizardIsTaskSelected('vscode') then
      InstalarExtensionVSCode;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    QuitarCarpetaDelPath(ExpandConstant('{app}'));
end;
