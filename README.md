# Zomboid Mod Downloader

A desktop application for browsing and downloading Project Zomboid mods from Steam Workshop using SteamCMD.

## Features

- **Steam Workshop Browser**: Browse Project Zomboid mods directly within the app
- **Easy Mod Selection**: Click "Add to Queue" buttons on mod thumbnails
- **Batch Downloads**: Download multiple mods at once with SteamCMD
- **Download Queue**: Manage your download queue with add/remove functionality
- **Progress Tracking**: Real-time SteamCMD output and download progress
- **Download History**: Tracks previously downloaded mods
- **Flexible Authentication**: Support for both anonymous and authenticated Steam login

## Prerequisites

1. **Python 3.8 or higher**
2. **SteamCMD**: Download from [https://developer.valvesoftware.com/wiki/SteamCMD](https://developer.valvesoftware.com/wiki/SteamCMD)
   - Windows: Download `steamcmd.zip` and extract it
   - Linux: Use package manager or download from Valve
   - macOS: Download from Valve

## Installation

1. Clone or download this repository

2. Install Python dependencies:
```bash
pip install -r requirements.txt
```

## Usage

1. **Run the application**:
```bash
python main.py
```

2. **Initial Setup**:
   - On first run, you'll be prompted to configure settings
   - Set the path to your `steamcmd.exe` (or `steamcmd.sh` on Linux/Mac)
   - Set the mod download directory (where mods will be saved)
   - Configure Steam login preferences

3. **Browse and Download Mods**:
   - The Steam Workshop will load in the left panel
   - Browse mods as you normally would on Steam
   - Click the "Add to Queue" button on any mod thumbnail to add it to your download queue
   - View your queue in the right panel
   - Click "Download Mods" to start downloading all queued mods

4. **Settings** (File → Settings):
   - **SteamCMD Path**: Location of steamcmd executable
   - **Mod Download Path**: Where to save downloaded mods
   - **Use Anonymous Login**: Check to download without Steam credentials
   - **Steam Username**: Your Steam username (if not using anonymous)
   - **Auto-clear Queue**: Automatically clear queue after successful download

## Project Structure

```
Zomboid Mod Downloader/
├── main.py                      # Application entry point
├── requirements.txt             # Python dependencies
├── settings.json                # User settings (created on first run)
├── zomboid_mods.db              # SQLite database (created on first run)
├── core/
│   ├── __init__.py
│   ├── database.py              # Database management
│   ├── settings.py              # Settings management
│   └── steamcmd.py              # SteamCMD wrapper
└── ui/
    ├── __init__.py
    ├── main_window.py           # Main application window
    ├── browser_widget.py        # Steam Workshop browser
    └── progress_dialog.py       # Download progress dialog
```

## How It Works

1. **Browser Integration**: The app uses PySide6's QWebEngineView to embed a Chromium-based browser
2. **JavaScript Injection**: Custom JavaScript is injected into Steam Workshop pages to add "Add to Queue" buttons
3. **SteamCMD Integration**: Uses QProcess to run SteamCMD commands asynchronously
4. **Database Tracking**: SQLite database tracks download queue and download history

## Troubleshooting

### SteamCMD Issues

- **"Failed to start SteamCMD"**: Check that the SteamCMD path is correct
- **Download fails**: Ensure you have internet connection and Steam Workshop is accessible
- **Authentication errors**: If using Steam login, verify your credentials

### Browser Issues

- **Workshop not loading**: Check your internet connection
- **Buttons not appearing**: Try reloading the page (View → Reload Browser)

### Download Location and Processing

The application automatically processes downloaded mods:

1. **SteamCMD downloads to**: `<Mod Download Path>/steamapps/workshop/content/108600/<mod_id>/mods/`
2. **Application extracts to**: `<Mod Download Path>/` (configured location)
3. **Temporary files cleaned up**: The steamapps folder is removed after processing

**Important**: Set your "Mod Download Path" in settings to your Project Zomboid mods folder for direct installation:
- Windows: `C:\Users\<username>\Zomboid\mods\`
- Linux: `~/.local/share/Zomboid/mods/`
- Mac: `~/Library/Application Support/Zomboid/mods/`

Mods are automatically extracted from the workshop structure and placed directly in your configured mods folder!

## Tips

- Use anonymous login for Workshop mods (most don't require authentication)
- **Set the mod download path to your Project Zomboid mods directory** - mods will be ready to use immediately!
- The download queue persists between sessions
- You can remove individual mods from the queue before downloading
- Watch the progress dialog for detailed processing information

## Known Limitations

- Requires SteamCMD to be manually installed
- Some Workshop features (ratings, comments) use Steam's interface
- Download progress is shown as SteamCMD console output (not a progress bar percentage)

## License

This project is for educational purposes. Steam, SteamCMD, and Project Zomboid are properties of their respective owners.

## Credits

Inspired by [RimSort](https://github.com/RimSort/RimSort/), a similar tool for RimWorld mod management.
