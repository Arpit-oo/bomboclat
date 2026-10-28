@echo off
rem Starts Bomboclat in dev mode. First run installs dependencies and
rem downloads Live2D Cubism Core (needed for BongoCat models).
setlocal
cd /d "%~dp0"

where npm >nul 2>nul || (
  echo Node.js not found. Install Node 20+ from https://nodejs.org
  pause
  exit /b 1
)
where cargo >nul 2>nul || (
  echo Rust not found. Install it from https://rustup.rs
  pause
  exit /b 1
)

if not exist node_modules (
  echo Installing dependencies...
  call npm install || goto :fail
)
if not exist public\live2dcubismcore.min.js (
  call npm run setup:live2d
  if errorlevel 1 echo Live2D Cubism Core missing - BongoCat models disabled, pixel cat still works.
)

call npm run tauri dev || goto :fail
exit /b 0

:fail
echo.
echo Bomboclat failed to start. See the output above.
pause
exit /b 1
