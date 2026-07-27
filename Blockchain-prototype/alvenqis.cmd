@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0alvenqis.ps1" %*
exit /b %ERRORLEVEL%
