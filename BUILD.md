# Building Zomboid Mod Downloader

This guide explains how to build the Zomboid Mod Downloader into a standalone executable (.exe) file.

## Prerequisites

- **Python 3.8 or higher** installed on your system
- **pip** (Python package manager, comes with Python)
- **Windows OS** (for building Windows executables)

## Quick Build (Recommended)

The easiest way to build the application is using the provided build script:

1. Open Command Prompt or PowerShell
2. Navigate to the project directory:
   ```cmd
   cd "E:\Zomboid Mod Downloader"
   ```

3. Run the build script:
   ```cmd
   build.bat
   ```

4. Wait for the build process to complete (this may take 3-5 minutes)

5. The application folder will be created in the `dist` folder:
   ```
   dist\ZomboidModDownloader\
       ├── ZomboidModDownloader.exe  (main executable)
       ├── Python DLLs and libraries
       └── Qt framework files
   ```

## Manual Build Process

If you prefer to build manually or need more control:

### Step 1: Install Dependencies

```cmd
pip install -r requirements.txt
```

This will install:
- PySide6 (GUI framework)
- PyInstaller (executable builder)

### Step 2: Build the Executable

```cmd
pyinstaller --clean --noconfirm zomboid_mod_downloader.spec
```

### Step 3: Locate the Output

The application folder will be in:
```
dist\ZomboidModDownloader\
```

Run the executable:
```
dist\ZomboidModDownloader\ZomboidModDownloader.exe
```

## Build Options

### Console vs Windowed Mode

By default, the application is built in **windowed mode** (no console window).

To enable console mode (useful for debugging):
1. Open `zomboid_mod_downloader.spec`
2. Change `console=False` to `console=True`
3. Rebuild

### Directory-Based Build (Current)

The current spec creates a **directory-based build** with multiple files:
- **Pros:** Faster startup, smaller main .exe, easier to update individual components
- **Cons:** Multiple files to distribute (must keep all files together)

**Structure:**
```
ZomboidModDownloader/
├── ZomboidModDownloader.exe    ~5-10 MB (main executable)
├── python3XX.dll                ~5 MB (Python runtime)
├── Qt6Core.dll, Qt6Gui.dll      ~20-30 MB (Qt libraries)
├── Qt6WebEngineCore.dll         ~100 MB (Browser engine)
├── _internal/                   (Additional dependencies)
└── ... (other DLLs)
Total size: ~200-300 MB
```

### Single File Build (Optional)

For a single .exe file (slower startup, very large file):
1. Edit `zomboid_mod_downloader.spec`
2. Remove `exclude_binaries=True` and the `COLLECT()` section
3. Add all binaries/datas to the `EXE()` call
4. Rebuild

**Not recommended** - creates 150-250 MB single file with slower startup

## Adding an Icon

To add a custom icon to your executable:

1. Create or obtain a `.ico` file (Windows icon format)
2. Place it in the project directory (e.g., `icon.ico`)
3. Edit `zomboid_mod_downloader.spec`:
   ```python
   icon='icon.ico'
   ```
4. Rebuild

## Troubleshooting

### Build Fails with "Module Not Found"

If PyInstaller can't find a module:
1. Add it to `hiddenimports` in the spec file:
   ```python
   hiddenimports=[
       'PySide6.QtCore',
       'PySide6.QtGui',
       'missing_module_name',  # Add here
   ],
   ```

### Executable Crashes on Startup

1. Build with console mode enabled to see error messages
2. Check that all dependencies are in requirements.txt
3. Try running: `pyinstaller --clean` to clear cache
4. Make sure all files in the folder stay together (don't move just the .exe)

### Large Folder Size

The application folder includes Python runtime and Qt libraries (~200-300 MB). This is normal.

To reduce size:
- UPX compression is already enabled
- Exclude unnecessary Qt modules
- Clean up temporary build files after successful build

### Missing DLL Errors

If you get "DLL not found" errors:
- Make sure you're running the .exe from its folder (don't move it elsewhere)
- All files in the `ZomboidModDownloader` folder must stay together
- Don't delete any DLLs from the folder

### WebEngine Not Working

If the embedded browser doesn't work:
- Ensure Qt WebEngine files are included
- Check that PySide6-WebEngine is installed
- May need to manually copy Qt WebEngine process files

## Distribution

Once built, you can distribute the entire `ZomboidModDownloader` folder to users.

### What to Include:
- **The entire `dist\ZomboidModDownloader\` folder** - All files are required
- `README.md` - Usage instructions (optional, place outside the folder)
- `QUICKSTART.md` - Quick start guide (optional, place outside the folder)

### Distribution Package Structure:
```
YourDistribution/
├── ZomboidModDownloader/        (The built application folder)
│   ├── ZomboidModDownloader.exe
│   ├── *.dll files
│   ├── _internal/
│   └── ... (all other files)
├── README.md                     (optional)
└── QUICKSTART.md                 (optional)
```

**IMPORTANT:** Users must keep ALL files in the `ZomboidModDownloader` folder together. The .exe won't work if moved by itself.

### What Users Need:
- Windows 10 or higher
- SteamCMD (downloaded separately, configured in app settings)
- Project Zomboid installation (for mod testing)

### First Run for Users:
1. Extract/copy the entire `ZomboidModDownloader` folder to their computer
2. Open the folder and run `ZomboidModDownloader.exe`
3. Configure SteamCMD path in Settings
4. Configure mod download path in Settings
5. Start browsing and downloading mods!

### Packaging for Distribution:
You can create a ZIP file for easy distribution:
```cmd
cd dist
powershell Compress-Archive -Path ZomboidModDownloader -DestinationPath ZomboidModDownloader-v1.0.zip
```

Or use 7-Zip, WinRAR, etc. to create an archive.

## Build Artifacts

After building, these folders will be created:
- `build/` - Temporary build files (~50-100 MB, can be deleted)
- `dist/ZomboidModDownloader/` - **The final application folder** (~200-300 MB)
- `__pycache__/` - Python cache files (can be deleted)

**To clean up after building:**
```cmd
rmdir /s /q build
rmdir /s /q __pycache__
```

Keep only the `dist/ZomboidModDownloader/` folder for distribution.

## Advanced: GitHub Actions CI/CD

To automate builds using GitHub Actions:

1. Create `.github/workflows/build.yml`:
```yaml
name: Build Executable

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.11'
      - run: pip install -r requirements.txt
      - run: pyinstaller --clean zomboid_mod_downloader.spec
      - run: cd dist && powershell Compress-Archive -Path ZomboidModDownloader -DestinationPath ZomboidModDownloader.zip
      - uses: actions/upload-artifact@v3
        with:
          name: ZomboidModDownloader
          path: dist/ZomboidModDownloader.zip
```

2. Create a git tag and push to trigger the build:
```bash
git tag v1.0.0
git push origin v1.0.0
```

## Version Management

To update the version number shown in the app:

1. Edit `main.py` and update the version string (if present)
2. Rebuild the executable
3. Consider adding version info to the spec file:
   ```python
   version='1.0.0',
   description='Zomboid Mod Downloader',
   ```

## Support

For build issues:
- Check the PyInstaller documentation: https://pyinstaller.org/
- Review PySide6 deployment guide: https://doc.qt.io/qtforpython/deployment.html
- Open an issue on the project repository

---

**Happy Building!** 🚀
