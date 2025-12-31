@echo off
echo ============================================
echo Zomboid Mod Downloader - Build Script
echo ============================================
echo.

REM Check if Python is installed
python --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: Python is not installed or not in PATH
    echo Please install Python 3.8 or higher
    pause
    exit /b 1
)

echo [1/4] Checking/Installing dependencies...
pip install -r requirements.txt
if errorlevel 1 (
    echo ERROR: Failed to install requirements
    pause
    exit /b 1
)

echo.
echo [2/4] Installing PyInstaller...
pip install pyinstaller
if errorlevel 1 (
    echo ERROR: Failed to install PyInstaller
    pause
    exit /b 1
)

echo.
echo [3/4] Building executable...
echo This may take a few minutes...
pyinstaller --clean --noconfirm zomboid_mod_downloader.spec
if errorlevel 1 (
    echo ERROR: Build failed
    pause
    exit /b 1
)

echo.
echo [4/4] Build complete!
echo.
echo Application folder: dist\ZomboidModDownloader\
echo Main executable: dist\ZomboidModDownloader\ZomboidModDownloader.exe
echo.
echo ============================================
echo Build completed successfully!
echo ============================================
echo.
echo You can now:
echo   1. Run: dist\ZomboidModDownloader\ZomboidModDownloader.exe
echo   2. Distribute: Copy the entire "ZomboidModDownloader" folder
echo.

REM Ask if user wants to open the dist folder
set /p OPEN_FOLDER="Open dist folder? (Y/N): "
if /i "%OPEN_FOLDER%"=="Y" (
    explorer dist\ZomboidModDownloader
)

pause
