@echo off
setlocal
cd /d "%~dp0"
echo Starting ShareReady - local image editor
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-ShareReady.ps1"
if errorlevel 1 (
  echo.
  echo ShareReady could not start. Check that this package contains all its files.
  pause
)
