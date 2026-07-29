@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
set "REPO_PATH=%SCRIPT_DIR%"

if exist "%SCRIPT_DIR%..\..\.git" set "REPO_PATH=%SCRIPT_DIR%..\.."

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%alvenqis-release.ps1" -RepoPath "%REPO_PATH%"
set "EXIT_CODE=%ERRORLEVEL%"

echo.
if not "%EXIT_CODE%"=="0" echo The script stopped with error code %EXIT_CODE%.
pause
exit /b %EXIT_CODE%
