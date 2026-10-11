@echo off
rem Ejecuta un programa .esp al hacer doble clic y deja la ventana abierta al terminar.
rem Lo usa la asociacion de archivos que crea el instalador de Windows.
rem Las rutas van SIEMPRE entre comillas: un nombre con & o parentesis no debe ejecutar nada mas.
title EspanolCode
cd /d "%~dp1"
"%~dp0espanolcode.exe" "%~1"
echo.
echo Pulsa una tecla para cerrar...
pause >nul
