====================================================
     ZOMBOID MOD DOWNLOADER - DISTRIBUTION
====================================================

WHAT TO DISTRIBUTE:
-------------------
After building, share the entire folder:
   dist\ZomboidModDownloader\

This folder contains:
   - ZomboidModDownloader.exe (main program)
   - All required DLL files
   - Qt libraries
   - Python runtime
   - Other dependencies

Total size: ~200-300 MB

====================================================

PACKAGING OPTIONS:
------------------

Option 1: ZIP Archive (Recommended)
   cd dist
   powershell Compress-Archive -Path ZomboidModDownloader -DestinationPath ZomboidModDownloader-v1.0.zip

   Then share: ZomboidModDownloader-v1.0.zip

Option 2: Direct Folder
   Copy the entire "ZomboidModDownloader" folder
   Share via USB drive, network, etc.

Option 3: Installer (Advanced)
   Use NSIS or Inno Setup to create an installer
   See BUILD.md for details

====================================================

USER INSTRUCTIONS:
------------------
Include these instructions for your users:

1. Extract/copy the "ZomboidModDownloader" folder
   to your computer

2. Open the folder

3. Double-click "ZomboidModDownloader.exe"

4. On first run:
   - Go to File > Settings
   - Set SteamCMD path
   - Set mod download folder

5. Start downloading mods!

IMPORTANT:
- Keep all files in the folder together
- Don't move just the .exe file
- Windows 10 or higher required

====================================================

REQUIREMENTS FOR END USERS:
---------------------------
- Windows 10 or higher
- ~300 MB free disk space
- Internet connection
- SteamCMD (configured in app)
- Project Zomboid (to use the mods)

No Python installation needed!
Everything is bundled.

====================================================

TROUBLESHOOTING:
----------------

"Missing DLL" error:
   → Make sure all files stayed together
   → Re-extract from ZIP if needed

Doesn't start:
   → Try running as Administrator
   → Check Windows Defender/Antivirus
   → Enable console mode for error messages

Browser not working:
   → Qt WebEngine requires Windows 10+
   → Try disabling antivirus temporarily

====================================================
