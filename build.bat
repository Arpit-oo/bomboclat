@echo off
rem Builds the Windows installer (NSIS). Output:
rem   src-tauri\target\release\bundle\nsis\Bomboclat_<version>_x64-setup.exe
setlocal
cd /d "%~dp0"

where npm >nul 2>nul || (echo Node.js not found. Install Node 20+ from https://nodejs.org & pause & exit /b 1)
where cargo >nul 2>nul || (echo Rust not found. Install it from https://rustup.rs & pause & exit /b 1)

if not exist node_modules (
  call npm install || goto :fail
)
if not exist public\live2dcubismcore.min.js (
  call npm run setup:live2d
  if errorlevel 1 echo Live2D Cubism Core missing - installer will only have the pixel cat.
)

call npm run tauri build || goto :fail
echo.
echo Installer:
dir /b src-tauri\target\release\bundle\nsis\*.exe
explorer src-tauri\target\release\bundle\nsis
exit /b 0

:fail
echo.
echo Build failed. See the output above.
pause
exit /b 1
