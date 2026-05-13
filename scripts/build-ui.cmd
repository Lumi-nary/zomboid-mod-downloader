@echo off
setlocal

set "ROOT=%~dp0.."
set "OUT=%ROOT%\dist-tauri-ui"
set "ESBUILD=%ROOT%\node_modules\@esbuild\win32-x64\esbuild.exe"

if not exist "%ESBUILD%" (
  echo ERROR: esbuild executable not found. Run npm install first.
  exit /b 1
)

if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%\assets"

"%ESBUILD%" "%ROOT%\src\main.tsx" ^
  --bundle ^
  --format=esm ^
  --target=es2020 ^
  --outfile="%OUT%\assets\main.js" ^
  --loader:.tsx=tsx ^
  --loader:.ts=ts ^
  --loader:.css=css

if errorlevel 1 exit /b 1

(
  echo ^<!doctype html^>
  echo ^<html lang="en"^>
  echo   ^<head^>
  echo     ^<meta charset="UTF-8" /^>
  echo     ^<meta name="viewport" content="width=device-width, initial-scale=1.0" /^>
  echo     ^<title^>Zomboid Mod Downloader^</title^>
  echo     ^<script type="module" crossorigin src="/assets/main.js"^>^</script^>
  echo     ^<link rel="stylesheet" crossorigin href="/assets/main.css"^>
  echo   ^</head^>
  echo   ^<body^>
  echo     ^<div id="root"^>^</div^>
  echo   ^</body^>
  echo ^</html^>
) > "%OUT%\index.html"

echo Built frontend to %OUT%
