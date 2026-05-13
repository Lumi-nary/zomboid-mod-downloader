# Building Zomboid Mod Downloader

This guide explains how to build the Tauri version of Zomboid Mod Downloader.

## Prerequisites

- **Node.js 20 or higher**
- **Rust and Cargo**
- **Windows OS** for the primary release target
- **SteamCMD** remains a user-configured external dependency

## Quick Build

```cmd
npm install
npm run tauri build
```

The Windows installer and executable artifacts are written under:

```cmd
src-tauri\target\release\bundle\
```

## Development Run

```cmd
npm run tauri dev
```

## Frontend Build Only

```cmd
npm run build
```

## Antivirus False Positive Mitigation

- Do not add UPX or other binary packers to the new build.
- Keep release metadata stable in `src-tauri/tauri.conf.json`.
- For public distribution, code signing is still the strongest long-term mitigation.
- Test unsigned development releases against target antivirus products before publishing.

## App Data And Migration

The Tauri app stores settings and SQLite state in the OS app-data directory. On first run it copies root-level files if they exist:

- `settings.json`
- `zomboid_mods.db`

This keeps existing user configuration and downloaded-mod tracking without writing mutable state beside the executable.

## Build Artifacts

- `dist-tauri-ui/` - Vite frontend output.
- `src-tauri/target/` - Rust build output and installer bundles.

## GitHub Actions Example

```yaml
name: Build Tauri App

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - uses: dtolnay/rust-toolchain@stable
      - run: npm ci
      - run: npm run tauri build
      - uses: actions/upload-artifact@v3
        with:
          name: ZomboidModDownloader-Tauri
          path: src-tauri/target/release/bundle/
```
