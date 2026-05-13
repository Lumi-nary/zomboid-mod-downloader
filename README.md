# Zomboid Mod Downloader

> A desktop application for browsing and downloading Project Zomboid mods from Steam Workshop using SteamCMD.

[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-18-blue.svg)](https://react.dev/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Features

- **Tauri desktop shell** - Small native desktop package powered by Rust and WebView2.
- **Workshop workspace** - Browse/open Project Zomboid Workshop pages and queue mods by Workshop URL or item ID.
- **Batch downloads** - Download multiple queued Workshop items with SteamCMD.
- **Real-time progress** - Live SteamCMD output and processing status in the app.
- **Local Mods Browser** - View, manage, export, and delete downloaded mods.
- **Workshop URL Tracking** - Links downloaded folders back to Steam Workshop when possible.
- **Flexible Authentication** - Anonymous or Steam account username login.
- **App-data persistence** - Settings and SQLite state live in the OS app-data directory.

## Prerequisites

1. **Node.js 20 or higher** for frontend tooling.
2. **Rust and Cargo** for the Tauri backend.
3. **SteamCMD** from [Valve's SteamCMD page](https://developer.valvesoftware.com/wiki/SteamCMD).

## Installation

```bash
npm install
```

## Usage

Run the Tauri app in development mode:

```bash
npm run tauri dev
```

On first launch, configure:

- **SteamCMD Path**: Location of `steamcmd.exe`.
- **Mod Download Path**: Where downloaded mods should be placed.
- **Use Anonymous Login**: Recommended for public Workshop mods.
- **Steam Username**: Used only when anonymous login is disabled.
- **Auto-clear Queue**: Clears the queue after a successful download.

## Download Flow

1. Browse or open the Project Zomboid Steam Workshop from the Workshop view.
2. Paste a Steam Workshop URL or numeric Workshop ID.
3. Add it to the persistent queue.
4. Import or export JSON mod lists when needed.
5. Start the download and watch SteamCMD output in the progress log.

SteamCMD downloads to:

```text
<Mod Download Path>/steamapps/workshop/content/108600/<workshop_id>/mods/
```

The app moves mod folders into:

```text
<Mod Download Path>/
```

The temporary `steamapps` folder is removed after processing.

## Project Structure

```text
Zomboid Mod Downloader/
├── src/                         # React frontend
├── src-tauri/                   # Tauri/Rust backend
├── scripts/                     # Frontend build helpers
├── release/                     # Local release/debug executables
└── dist-tauri-ui/               # Built frontend output
```

## How It Works

1. **Tauri Frontend**: React renders the downloader workspace, settings, local mod browser, and progress log.
2. **Tauri Backend**: Rust owns SteamCMD execution, file processing, settings, and SQLite state.
3. **Event Streaming**: SteamCMD output is emitted to the frontend as `download-progress` events.
4. **State Migration**: Existing `settings.json` and `zomboid_mods.db` in the repo root are copied into app data on first run.

## Building

```bash
npm run tauri build
```

Artifacts are written under:

```text
src-tauri/target/release/bundle/
```

Release builds should be created through Tauri.
