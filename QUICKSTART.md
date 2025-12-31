# Quick Start Guide

## 1. Install Dependencies

```bash
pip install -r requirements.txt
```

This will install PySide6, which includes:
- Qt GUI framework
- QtWebEngine (Chromium-based browser)
- All necessary Qt components

## 2. Download and Setup SteamCMD

### Windows:
1. Download SteamCMD: https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip
2. Extract to a folder (e.g., `C:\steamcmd\`)
3. Note the path to `steamcmd.exe`

### Linux:
```bash
# Ubuntu/Debian
sudo apt-get install steamcmd

# Or download manually
wget https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz
tar -xvzf steamcmd_linux.tar.gz
```

### macOS:
```bash
# Download and extract
curl -sqL "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_osx.tar.gz" | tar zxvf -
```

## 3. Run the Application

```bash
python main.py
```

## 4. First-Time Setup

When you first run the app, you'll be prompted to configure:

1. **SteamCMD Path**:
   - Windows: `C:\steamcmd\steamcmd.exe`
   - Linux/Mac: `/path/to/steamcmd.sh`

2. **Mod Download Path**:
   - Choose where you want mods downloaded
   - Example: `C:\Users\YourName\Documents\ZomboidMods`
   - Mods will be saved in: `<path>/steamapps/workshop/content/108600/`

3. **Login Settings**:
   - Check "Use Anonymous Login" (recommended for most Workshop mods)
   - Or enter your Steam username for authenticated downloads

## 5. Download Your First Mod

1. The Steam Workshop will load in the left panel
2. Browse or search for mods
3. Click the purple "Add to Queue" button on any mod
4. The mod appears in the right panel queue
5. Click the green "Download Mods" button
6. Watch the progress in the download dialog
7. Mods are saved to your configured location!

## 6. Mods Are Ready to Use!

**The application automatically extracts and processes mods for you!**

After download completes, mods are automatically:
1. Extracted from the workshop structure
2. Moved to your configured "Mod Download Path"
3. Ready to use in Project Zomboid!

**Best Practice**: Set your "Mod Download Path" to your Project Zomboid mods folder:
- Windows: `C:\Users\<username>\Zomboid\mods\`
- Linux: `~/.local/share/Zomboid/mods/`
- Mac: `~/Library/Application Support/Zomboid/mods/`

This way, downloaded mods are **instantly available** in Project Zomboid - no manual copying needed!

## Common Issues

### "Failed to start SteamCMD"
- Check the SteamCMD path in Settings
- Ensure you have execute permissions (Linux/Mac: `chmod +x steamcmd.sh`)

### "Workshop not loading"
- Check internet connection
- Try View → Reload Browser

### No "Add to Queue" buttons appearing
- Wait for page to fully load
- Try View → Reload Browser
- Check browser console (if developer tools enabled)

## Tips

- **Set your Mod Download Path to your Project Zomboid mods folder** for instant mod availability
- Use anonymous login unless a mod specifically requires authentication
- The queue persists between app restarts
- You can select and remove mods from the queue before downloading
- Watch the progress dialog to see mods being extracted and processed

## Next Steps

- Read the full [README.md](README.md) for detailed documentation
- Check File → Settings to customize behavior
- Start building your mod collection!

---

**Need Help?** Check the README.md or create an issue on the project repository.
