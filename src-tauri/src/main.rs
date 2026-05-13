#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use regex::Regex;
use rusqlite::{params, Connection, Error as SqlError};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tauri::{
    webview::PageLoadEvent, AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State,
    WebviewBuilder, WebviewUrl, WebviewWindowBuilder,
};
use walkdir::WalkDir;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const PROJECT_ZOMBOID_APP_ID: &str = "108600";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    #[serde(default, alias = "steamcmd_path")]
    steamcmd_path: String,
    #[serde(default, alias = "mod_download_path")]
    mod_download_path: String,
    #[serde(default, alias = "steam_username")]
    steam_username: String,
    #[serde(default = "default_true", alias = "use_anonymous_login")]
    use_anonymous_login: bool,
    #[serde(default = "default_true", alias = "auto_clear_queue")]
    auto_clear_queue: bool,
    #[serde(default = "default_window_width", alias = "window_width")]
    window_width: u32,
    #[serde(default = "default_window_height", alias = "window_height")]
    window_height: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            steamcmd_path: String::new(),
            mod_download_path: String::new(),
            steam_username: String::new(),
            use_anonymous_login: true,
            auto_clear_queue: true,
            window_width: 1280,
            window_height: 820,
        }
    }
}

fn default_window_width() -> u32 {
    1280
}

fn default_window_height() -> u32 {
    820
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatePayload {
    settings: AppSettings,
    queue: Vec<QueuedMod>,
    downloaded_mods: Vec<DownloadedMod>,
    local_mods: Vec<LocalMod>,
    installed_workshop_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueuedMod {
    publishedfileid: String,
    title: String,
    added_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadedMod {
    publishedfileid: String,
    title: String,
    download_date: String,
    file_size: i64,
    last_updated: String,
    workshop_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalMod {
    folder_name: String,
    display_name: String,
    package_id: String,
    authors: String,
    mod_version: String,
    pz_version: String,
    size_bytes: u64,
    path: String,
    modified_at: String,
    workshop_url: String,
    workshop_id: String,
    poster_path: String,
    poster_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgressEvent {
    kind: String,
    message: String,
    success: Option<bool>,
    publishedfileid: Option<String>,
    folders: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
struct CommandResult {
    success: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationResult {
    valid: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkshopItemState {
    queued: bool,
    installed: bool,
    title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkshopModPreview {
    publishedfileid: String,
    title: String,
    url: String,
    queued: bool,
    installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkshopCollectionPreview {
    collection_id: String,
    title: String,
    url: String,
    items: Vec<WorkshopModPreview>,
    total_count: usize,
    queued_count: usize,
    installed_count: usize,
    addable_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedWorkshopEntry {
    kind: String,
    item: Option<WorkshopModPreview>,
    collection: Option<WorkshopCollectionPreview>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueueCollectionItem {
    publishedfileid: String,
    title: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkshopCollectionRequested {
    collection_id: String,
    title: String,
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone)]
struct AppPaths {
    data_dir: PathBuf,
    settings_path: PathBuf,
    db_path: PathBuf,
    root_settings_path: PathBuf,
    root_db_path: PathBuf,
}

struct AppContext {
    paths: AppPaths,
    active_child: Arc<Mutex<Option<std::process::Child>>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_app(app).map_err(|message| {
                let _ = write_startup_log(&message);
                std::io::Error::new(std::io::ErrorKind::Other, message).into()
            })
        })
        .invoke_handler(tauri::generate_handler![
            load_initial_state,
            save_settings,
            validate_paths,
            browse_steamcmd_path,
            browse_zomboid_mod_path,
            add_to_queue,
            add_collection_to_queue,
            resolve_workshop_entry,
            remove_from_queue,
            get_workshop_item_state,
            queue_workshop_item,
            unqueue_workshop_item,
            clear_queue,
            start_download,
            cancel_download,
            list_local_mods,
            delete_local_mod,
            reveal_path,
            open_url,
            open_workshop_browser,
            close_workshop_browser
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zomboid Mod Downloader");
}

fn setup_app(app: &mut tauri::App) -> Result<(), String> {
    let mut paths = resolve_paths(app.handle()).map_err(|err| format!("resolve paths: {err}"))?;
    if let Err(primary_err) = fs::create_dir_all(&paths.data_dir) {
        let fallback = portable_data_dir().map_err(|err| {
            format!(
                "create app data dir {} failed: {primary_err}; resolve portable data dir failed: {err}",
                paths.data_dir.display()
            )
        })?;
        fs::create_dir_all(&fallback).map_err(|fallback_err| {
            format!(
                "create app data dir {} failed: {primary_err}; create portable data dir {} failed: {fallback_err}",
                paths.data_dir.display(),
                fallback.display()
            )
        })?;
        paths = paths.with_data_dir(fallback);
    }
    let webview_data_dir = paths.data_dir.join("webview-data");
    fs::create_dir_all(&webview_data_dir).map_err(|err| {
        format!(
            "create webview data dir {}: {err}",
            webview_data_dir.display()
        )
    })?;
    migrate_root_files(&paths).map_err(|err| format!("migrate root files: {err}"))?;
    initialize_database(&paths.db_path)
        .map_err(|err| format!("initialize database {}: {err}", paths.db_path.display()))?;
    app.manage(AppContext {
        paths,
        active_child: Arc::new(Mutex::new(None)),
    });
    WebviewWindowBuilder::new(app.handle(), "main", WebviewUrl::App("index.html".into()))
        .title("Zomboid Mod Downloader")
        .inner_size(1280.0, 820.0)
        .min_inner_size(960.0, 720.0)
        .data_directory(webview_data_dir)
        .build()
        .map_err(|err| format!("create main window: {err}"))?;
    Ok(())
}

fn write_startup_log(message: &str) -> std::io::Result<()> {
    let log_path = std::env::current_exe()?
        .parent()
        .map(|path| path.join("startup-error.log"))
        .unwrap_or_else(|| PathBuf::from("startup-error.log"));
    fs::write(log_path, message)
}

fn resolve_paths(_app: &AppHandle) -> Result<AppPaths, Box<dyn std::error::Error>> {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ZomboidModDownloader");
    let current_dir = std::env::current_dir()?;
    Ok(AppPaths {
        settings_path: data_dir.join("settings.json"),
        db_path: data_dir.join("zomboid_mods.db"),
        data_dir,
        root_settings_path: current_dir.join("settings.json"),
        root_db_path: current_dir.join("zomboid_mods.db"),
    })
}

fn portable_data_dir() -> std::io::Result<PathBuf> {
    Ok(std::env::current_exe()?
        .parent()
        .map(|path| path.join("data"))
        .unwrap_or_else(|| PathBuf::from("data")))
}

impl AppPaths {
    fn with_data_dir(&self, data_dir: PathBuf) -> Self {
        Self {
            settings_path: data_dir.join("settings.json"),
            db_path: data_dir.join("zomboid_mods.db"),
            data_dir,
            root_settings_path: self.root_settings_path.clone(),
            root_db_path: self.root_db_path.clone(),
        }
    }
}

fn migrate_root_files(paths: &AppPaths) -> Result<(), Box<dyn std::error::Error>> {
    if !paths.settings_path.exists() && paths.root_settings_path.exists() {
        fs::copy(&paths.root_settings_path, &paths.settings_path)?;
    }
    if !paths.db_path.exists() && paths.root_db_path.exists() {
        fs::copy(&paths.root_db_path, &paths.db_path)?;
    }
    Ok(())
}

fn initialize_database(db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|err| err.to_string())?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS downloaded_mods (
            publishedfileid TEXT PRIMARY KEY,
            title TEXT,
            download_date TEXT,
            file_size INTEGER,
            last_updated TEXT,
            workshop_url TEXT
        );
        CREATE TABLE IF NOT EXISTS download_queue (
            publishedfileid TEXT PRIMARY KEY,
            title TEXT,
            added_date TEXT
        );
        CREATE TABLE IF NOT EXISTS workshop_metadata_cache (
            publishedfileid TEXT PRIMARY KEY,
            title TEXT,
            author TEXT,
            mod_version TEXT,
            supported_version TEXT,
            poster_url TEXT,
            fetched_at TEXT
        );
        CREATE TABLE IF NOT EXISTS local_mod_metadata_cache (
            folder_name TEXT PRIMARY KEY,
            folder_modified_at TEXT,
            display_name TEXT,
            package_id TEXT,
            authors TEXT,
            mod_version TEXT,
            pz_version TEXT,
            size_bytes INTEGER,
            poster_path TEXT,
            poster_url TEXT,
            workshop_id TEXT,
            workshop_url TEXT,
            cached_at TEXT
        );
        ",
    )
    .map_err(|err| err.to_string())?;
    let _ = conn.execute(
        "ALTER TABLE downloaded_mods ADD COLUMN workshop_url TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE local_mod_metadata_cache ADD COLUMN size_bytes INTEGER",
        [],
    );
    Ok(())
}

fn open_database(paths: &AppPaths) -> Result<Connection, String> {
    initialize_database(&paths.db_path)?;
    Connection::open(&paths.db_path).map_err(|err| err.to_string())
}

fn load_settings(paths: &AppPaths) -> AppSettings {
    fs::read_to_string(&paths.settings_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<AppSettings>(&raw).ok())
        .unwrap_or_default()
}

fn write_settings(paths: &AppPaths, settings: &AppSettings) -> Result<(), String> {
    fs::create_dir_all(&paths.data_dir).map_err(|err| err.to_string())?;
    let json = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    fs::write(&paths.settings_path, json).map_err(|err| err.to_string())
}

fn load_queue_from_db(conn: &Connection) -> Result<Vec<QueuedMod>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT publishedfileid, title, added_date FROM download_queue ORDER BY added_date",
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(QueuedMod {
                publishedfileid: row.get(0)?,
                title: row.get(1)?,
                added_date: row.get(2)?,
            })
        })
        .map_err(|err| err.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())
}

fn load_downloaded_from_db(conn: &Connection) -> Result<Vec<DownloadedMod>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT publishedfileid, title, download_date, file_size, last_updated, COALESCE(workshop_url, '')
             FROM downloaded_mods ORDER BY download_date DESC",
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(DownloadedMod {
                publishedfileid: row.get(0)?,
                title: row.get(1)?,
                download_date: row.get(2)?,
                file_size: row.get(3)?,
                last_updated: row.get(4)?,
                workshop_url: row.get(5)?,
            })
        })
        .map_err(|err| err.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())
}

fn app_state(paths: &AppPaths) -> Result<AppStatePayload, String> {
    let settings = load_settings(paths);
    let conn = open_database(paths)?;
    let local_mods = scan_local_mods(&settings, &conn, paths)?;
    let installed_workshop_ids = installed_ids_from_mods(&local_mods).into_iter().collect();
    Ok(AppStatePayload {
        settings,
        queue: load_queue_from_db(&conn)?,
        downloaded_mods: load_downloaded_from_db(&conn)?,
        local_mods,
        installed_workshop_ids,
    })
}

#[tauri::command]
fn load_initial_state(ctx: State<AppContext>) -> Result<AppStatePayload, String> {
    app_state(&ctx.paths)
}

#[tauri::command]
fn save_settings(settings: AppSettings, ctx: State<AppContext>) -> Result<AppStatePayload, String> {
    write_settings(&ctx.paths, &settings)?;
    app_state(&ctx.paths)
}

#[tauri::command]
fn validate_paths(settings: AppSettings) -> ValidationResult {
    validate_settings_paths(&settings)
}

#[tauri::command]
fn browse_steamcmd_path() -> Result<Option<String>, String> {
    Ok(pick_file("Select steamcmd.exe", "steamcmd.exe"))
}

#[tauri::command]
fn browse_zomboid_mod_path() -> Result<Option<String>, String> {
    Ok(pick_folder("Select Zomboid mod folder"))
}

fn pick_folder(title: &str) -> Option<String> {
    pick_folder_native(title)
}

fn pick_file(title: &str, file_name: &str) -> Option<String> {
    pick_file_native(title, file_name)
}

#[cfg(windows)]
fn pick_folder_native(title: &str) -> Option<String> {
    use windows::{
        core::HSTRING,
        Win32::{
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
            },
            UI::Shell::{
                FileOpenDialog, IFileDialog, FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST,
                FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
            },
        },
    };

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().ok()?;
        let result = (|| {
            let dialog: IFileDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
            let options = dialog.GetOptions().ok()?;
            dialog
                .SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST)
                .ok()?;
            let title = HSTRING::from(title);
            dialog.SetTitle(&title).ok()?;
            dialog.Show(None).ok()?;
            let item = dialog.GetResult().ok()?;
            let path = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
            let path_string = path.to_string().ok();
            CoTaskMemFree(Some(path.as_ptr().cast()));
            path_string
        })();
        CoUninitialize();
        result
    }
}

#[cfg(not(windows))]
fn pick_folder_native(_title: &str) -> Option<String> {
    None
}

#[cfg(windows)]
fn pick_file_native(title: &str, file_name: &str) -> Option<String> {
    use windows::{
        core::HSTRING,
        Win32::{
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
            },
            UI::Shell::{
                FileOpenDialog, IFileDialog, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM,
                FOS_PATHMUSTEXIST, SIGDN_FILESYSPATH,
            },
        },
    };

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().ok()?;
        let result = (|| {
            let dialog: IFileDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
            let options = dialog.GetOptions().ok()?;
            dialog
                .SetOptions(options | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST)
                .ok()?;
            let title = HSTRING::from(title);
            dialog.SetTitle(&title).ok()?;
            let file_name = HSTRING::from(file_name);
            dialog.SetFileName(&file_name).ok()?;
            dialog.Show(None).ok()?;
            let item = dialog.GetResult().ok()?;
            let path = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
            let path_string = path.to_string().ok();
            CoTaskMemFree(Some(path.as_ptr().cast()));
            path_string
        })();
        CoUninitialize();
        result
    }
}

#[cfg(not(windows))]
fn pick_file_native(_title: &str, _file_name: &str) -> Option<String> {
    None
}

fn resolve_steamcmd_executable(path: &str) -> PathBuf {
    let steamcmd = PathBuf::from(path.trim());
    if steamcmd.is_dir() {
        steamcmd.join("steamcmd.exe")
    } else {
        steamcmd
    }
}

fn validate_settings_paths(settings: &AppSettings) -> ValidationResult {
    if settings.steamcmd_path.trim().is_empty() {
        return ValidationResult {
            valid: false,
            message: "SteamCMD executable path is required.".into(),
        };
    }
    let steamcmd = resolve_steamcmd_executable(&settings.steamcmd_path);
    if !steamcmd.exists() || !steamcmd.is_file() {
        return ValidationResult {
            valid: false,
            message: format!("SteamCMD was not found at: {}", steamcmd.display()),
        };
    }
    if settings.mod_download_path.trim().is_empty() {
        return ValidationResult {
            valid: false,
            message: "Mod download path is required.".into(),
        };
    }
    ValidationResult {
        valid: true,
        message: "Paths are valid.".into(),
    }
}

#[tauri::command]
fn add_to_queue(
    app: AppHandle,
    publishedfileid: String,
    title: String,
    ctx: State<AppContext>,
) -> Result<Vec<QueuedMod>, String> {
    let conn = open_database(&ctx.paths)?;
    let settings = load_settings(&ctx.paths);
    let local_mods = scan_local_mods(&settings, &conn, &ctx.paths)?;
    let installed_ids = installed_ids_from_mods(&local_mods);
    queue_item_with_required_items(&conn, &installed_ids, &publishedfileid, &title)?;
    let queue = load_queue_from_db(&conn)?;
    emit_queue_changed(&app);
    push_workshop_known_states_to_browser(&app);
    Ok(queue)
}

#[tauri::command]
fn add_collection_to_queue(
    app: AppHandle,
    items: Vec<QueueCollectionItem>,
    ctx: State<AppContext>,
) -> Result<Vec<QueuedMod>, String> {
    let conn = open_database(&ctx.paths)?;
    let settings = load_settings(&ctx.paths);
    let local_mods = scan_local_mods(&settings, &conn, &ctx.paths)?;
    let installed_ids = installed_ids_from_mods(&local_mods);
    let mut seen = HashSet::new();

    for item in items {
        let id = item.publishedfileid.trim();
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if installed_ids.contains(id) || !seen.insert(id.to_string()) {
            continue;
        }
        queue_item_with_required_items(&conn, &installed_ids, id, &item.title)?;
    }

    let queue = load_queue_from_db(&conn)?;
    emit_queue_changed(&app);
    push_workshop_known_states_to_browser(&app);
    Ok(queue)
}

#[tauri::command]
fn resolve_workshop_entry(
    input: String,
    ctx: State<AppContext>,
) -> Result<ResolvedWorkshopEntry, String> {
    resolve_workshop_entry_impl(&input, &ctx.paths)
}

#[tauri::command]
fn remove_from_queue(
    app: AppHandle,
    publishedfileid: String,
    ctx: State<AppContext>,
) -> Result<Vec<QueuedMod>, String> {
    let conn = open_database(&ctx.paths)?;
    delete_queue_item(&conn, &publishedfileid)?;
    let queue = load_queue_from_db(&conn)?;
    emit_queue_changed(&app);
    push_workshop_known_states_to_browser(&app);
    Ok(queue)
}

#[tauri::command]
fn get_workshop_item_state(
    publishedfileid: String,
    ctx: State<AppContext>,
) -> Result<WorkshopItemState, String> {
    workshop_item_state(&ctx.paths, &publishedfileid)
}

#[tauri::command]
fn queue_workshop_item(
    app: AppHandle,
    publishedfileid: String,
    title: String,
    ctx: State<AppContext>,
) -> Result<WorkshopItemState, String> {
    let conn = open_database(&ctx.paths)?;
    let settings = load_settings(&ctx.paths);
    let local_mods = scan_local_mods(&settings, &conn, &ctx.paths)?;
    let installed = installed_ids_from_mods(&local_mods).contains(&publishedfileid);
    if !installed {
        let title = normalized_workshop_title(&publishedfileid, &title);
        let installed_ids = installed_ids_from_mods(&local_mods);
        queue_item_with_required_items(&conn, &installed_ids, &publishedfileid, &title)?;
    }
    emit_queue_changed(&app);
    workshop_item_state(&ctx.paths, &publishedfileid)
}

#[tauri::command]
fn unqueue_workshop_item(
    app: AppHandle,
    publishedfileid: String,
    ctx: State<AppContext>,
) -> Result<WorkshopItemState, String> {
    let conn = open_database(&ctx.paths)?;
    delete_queue_item(&conn, &publishedfileid)?;
    emit_queue_changed(&app);
    workshop_item_state(&ctx.paths, &publishedfileid)
}

fn insert_queue_item(conn: &Connection, publishedfileid: &str, title: &str) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let title = normalized_workshop_title(publishedfileid, title);
    conn.execute(
        "INSERT OR REPLACE INTO download_queue (publishedfileid, title, added_date) VALUES (?, ?, ?)",
        params![publishedfileid, title, now],
    )
    .map(|_| ())
    .map_err(|err| err.to_string())
}

fn queue_item_with_required_items(
    conn: &Connection,
    installed_ids: &HashSet<String>,
    publishedfileid: &str,
    title: &str,
) -> Result<(), String> {
    let id = publishedfileid.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) || installed_ids.contains(id) {
        return Ok(());
    }

    insert_queue_item(conn, id, title)?;

    for (required_id, required_title) in required_items_for_workshop_item(id) {
        if required_id == id || installed_ids.contains(&required_id) {
            continue;
        }
        insert_queue_item(conn, &required_id, &required_title)?;
    }

    Ok(())
}

fn required_items_for_workshop_item(publishedfileid: &str) -> Vec<(String, String)> {
    fetch_workshop_page(&workshop_detail_url(publishedfileid))
        .map(|html| required_items_from_html(&html, publishedfileid))
        .unwrap_or_default()
}

fn delete_queue_item(conn: &Connection, publishedfileid: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM download_queue WHERE publishedfileid = ?",
        params![publishedfileid],
    )
    .map(|_| ())
    .map_err(|err| err.to_string())
}

fn queued_title(conn: &Connection, publishedfileid: &str) -> Result<Option<String>, String> {
    match conn.query_row(
        "SELECT title FROM download_queue WHERE publishedfileid = ?",
        params![publishedfileid],
        |row| row.get(0),
    ) {
        Ok(title) => Ok(Some(title)),
        Err(SqlError::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn workshop_item_state(
    paths: &AppPaths,
    publishedfileid: &str,
) -> Result<WorkshopItemState, String> {
    let conn = open_database(paths)?;
    let queued_title = queued_title(&conn, publishedfileid)?;
    let settings = load_settings(paths);
    let local_mods = scan_local_mods(&settings, &conn, paths)?;
    let installed = installed_ids_from_mods(&local_mods).contains(publishedfileid);
    Ok(WorkshopItemState {
        queued: queued_title.is_some(),
        installed,
        title: queued_title,
    })
}

fn normalized_workshop_title(publishedfileid: &str, title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        format!("Workshop Item {publishedfileid}")
    } else {
        trimmed.to_string()
    }
}

fn resolve_workshop_entry_impl(
    input: &str,
    paths: &AppPaths,
) -> Result<ResolvedWorkshopEntry, String> {
    let trimmed = input.trim();
    let (id, url, should_fetch) = parse_workshop_input(trimmed)
        .ok_or_else(|| "Paste a Steam Workshop URL or numeric Workshop ID.".to_string())?;

    if should_fetch {
        let html = fetch_workshop_page(&url)?;
        let title = page_title(&html).unwrap_or_else(|| format!("Workshop Item {id}"));
        let child_items = if looks_like_collection_page(&html) {
            collection_items_from_html(&html, &id)
        } else {
            Vec::new()
        };
        if !child_items.is_empty() {
            let collection = build_collection_preview(paths, &id, &url, &title, child_items)?;
            return Ok(ResolvedWorkshopEntry {
                kind: "collection".into(),
                item: None,
                collection: Some(collection),
            });
        }
        let item = build_mod_preview(paths, &id, &title)?;
        return Ok(ResolvedWorkshopEntry {
            kind: "item".into(),
            item: Some(item),
            collection: None,
        });
    }

    let item = build_mod_preview(paths, &id, &format!("Workshop Item {id}"))?;
    Ok(ResolvedWorkshopEntry {
        kind: "item".into(),
        item: Some(item),
        collection: None,
    })
}

fn parse_workshop_input(input: &str) -> Option<(String, String, bool)> {
    if input.chars().all(|c| c.is_ascii_digit()) && input.len() >= 5 {
        let id = input.to_string();
        return Some((id.clone(), workshop_detail_url(&id), false));
    }

    let url = input.parse::<tauri::Url>().ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    if host != "steamcommunity.com" && !host.ends_with(".steamcommunity.com") {
        return None;
    }
    let id = url.query_pairs().find_map(|(key, value)| {
        (key == "id" || key == "publishedfileid")
            .then(|| value.into_owned())
            .filter(|candidate| candidate.chars().all(|c| c.is_ascii_digit()))
    })?;
    Some((id.clone(), workshop_detail_url(&id), true))
}

fn fetch_workshop_page(url: &str) -> Result<String, String> {
    let script = "& { param([string]$WorkshopUrl) $ProgressPreference='SilentlyContinue'; [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; (Invoke-WebRequest -UseBasicParsing -Uri $WorkshopUrl).Content }";
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
        url,
    ]);
    apply_no_window(&mut command);
    let output = command
        .output()
        .map_err(|err| format!("Could not start PowerShell to load Workshop page: {err}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            "Could not load Workshop page.".into()
        } else {
            format!("Could not load Workshop page: {message}")
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn apply_no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

fn build_collection_preview(
    paths: &AppPaths,
    collection_id: &str,
    url: &str,
    title: &str,
    child_items: Vec<(String, String)>,
) -> Result<WorkshopCollectionPreview, String> {
    let conn = open_database(paths)?;
    let queued_ids: HashSet<String> = load_queue_from_db(&conn)?
        .into_iter()
        .map(|item| item.publishedfileid)
        .collect();
    let settings = load_settings(paths);
    let installed_ids = installed_ids_from_mods(&scan_local_mods(&settings, &conn, paths)?);
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let mut queued_count = 0;
    let mut installed_count = 0;

    for (id, title) in child_items {
        if id == collection_id || !seen.insert(id.clone()) {
            continue;
        }
        let queued = queued_ids.contains(&id);
        let installed = installed_ids.contains(&id);
        if queued {
            queued_count += 1;
        }
        if installed {
            installed_count += 1;
        }
        items.push(WorkshopModPreview {
            url: workshop_detail_url(&id),
            title: normalized_workshop_title(&id, &title),
            publishedfileid: id,
            queued,
            installed,
        });
    }

    let total_count = items.len();
    let addable_count = items
        .iter()
        .filter(|item| !item.queued && !item.installed)
        .count();

    Ok(WorkshopCollectionPreview {
        collection_id: collection_id.to_string(),
        title: normalized_workshop_title(collection_id, title),
        url: url.to_string(),
        items,
        total_count,
        queued_count,
        installed_count,
        addable_count,
    })
}

fn build_mod_preview(
    paths: &AppPaths,
    publishedfileid: &str,
    title: &str,
) -> Result<WorkshopModPreview, String> {
    let state = workshop_item_state(paths, publishedfileid)?;
    Ok(WorkshopModPreview {
        publishedfileid: publishedfileid.to_string(),
        title: normalized_workshop_title(publishedfileid, state.title.as_deref().unwrap_or(title)),
        url: workshop_detail_url(publishedfileid),
        queued: state.queued,
        installed: state.installed,
    })
}

fn page_title(html: &str) -> Option<String> {
    let selectors = [
        r#"(?is)<div[^>]+class=["'][^"']*workshopItemTitle[^"']*["'][^>]*>(.*?)</div>"#,
        r#"(?is)<title[^>]*>(.*?)</title>"#,
    ];
    selectors.iter().find_map(|pattern| {
        Regex::new(pattern)
            .ok()?
            .captures(html)?
            .get(1)
            .map(|value| clean_html_text(value.as_str()))
            .filter(|value| !value.is_empty())
            .map(|value| value.replace(" - Steam Workshop", ""))
    })
}

fn looks_like_collection_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("collectionchildren")
        || lower.contains("workshopcollection")
        || lower.contains("collectionitem")
        || lower.contains("items in this collection")
}

fn collection_items_from_html(html: &str, collection_id: &str) -> Vec<(String, String)> {
    let Ok(id_regex) = Regex::new(
        r#"(?is)<a[^>]+href=["'][^"']*(?:sharedfiles|workshop)/filedetails/[^"']*(?:\?|&amp;|&)id=(\d+)[^"']*["'][^>]*>(.*?)</a>"#,
    ) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for block in collection_item_blocks(html) {
        let Some(captures) = id_regex.captures(block) else {
            continue;
        };
        let Some(id) = captures.get(1).map(|value| value.as_str().to_string()) else {
            continue;
        };
        if id == collection_id || !seen.insert(id.clone()) {
            continue;
        }
        let anchor_text = captures
            .get(2)
            .map(|value| clean_html_text(value.as_str()))
            .unwrap_or_default();
        let title = collection_item_title(block)
            .filter(|value| !value.is_empty())
            .or_else(|| valid_workshop_link_text(&anchor_text).then_some(anchor_text))
            .unwrap_or_else(|| format!("Workshop Item {id}"));
        items.push((id, title));
    }

    items
}

fn collection_item_blocks(html: &str) -> Vec<&str> {
    let Ok(open_div_regex) = Regex::new(
        r#"(?is)<div\b[^>]*class=["'][^"']*(?:collectionItem|workshopItemCollection)[^"']*["'][^>]*>"#,
    ) else {
        return Vec::new();
    };
    let mut blocks = Vec::new();
    let mut last_end = 0;

    for open_div in open_div_regex.find_iter(html) {
        if open_div.start() < last_end {
            continue;
        }
        let Some(end) = matching_div_end(html, open_div.start()) else {
            continue;
        };
        last_end = end;
        blocks.push(&html[open_div.start()..end]);
    }

    blocks
}

fn matching_div_end(html: &str, start: usize) -> Option<usize> {
    let Ok(div_regex) = Regex::new(r#"(?is)</?div\b[^>]*>"#) else {
        return None;
    };
    let mut depth = 0;

    for tag in div_regex.find_iter(&html[start..]) {
        let tag_text = tag.as_str();
        if tag_text.starts_with("</") {
            depth -= 1;
        } else {
            depth += 1;
        }
        if depth == 0 {
            return Some(start + tag.end());
        }
    }

    None
}

fn collection_item_title(block: &str) -> Option<String> {
    let selectors = [
        r#"(?is)<div[^>]+class=["'][^"']*workshopItemTitle[^"']*["'][^>]*>(.*?)</div>"#,
        r#"(?is)<div[^>]+class=["'][^"']*title[^"']*["'][^>]*>(.*?)</div>"#,
    ];
    selectors.iter().find_map(|pattern| {
        Regex::new(pattern)
            .ok()?
            .captures(block)?
            .get(1)
            .map(|value| clean_html_text(value.as_str()))
            .filter(|value| !value.is_empty())
    })
}

fn required_items_from_html(html: &str, publishedfileid: &str) -> Vec<(String, String)> {
    let Some(section) = required_items_section(html) else {
        return Vec::new();
    };
    workshop_links_from_html(section, publishedfileid)
}

fn required_items_section(html: &str) -> Option<&str> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("required items")?;
    let after_start = &lower[start..];
    let end = [
        "created by",
        "subscribe to download",
        "description",
        "workshop id:",
        "popular discussions",
    ]
    .into_iter()
    .filter_map(|marker| after_start.find(marker))
    .filter(|position| *position > "required items".len())
    .min()
    .map(|position| start + position)
    .unwrap_or(html.len());

    Some(&html[start..end])
}

fn workshop_links_from_html(html: &str, excluded_id: &str) -> Vec<(String, String)> {
    let Ok(link_regex) = Regex::new(
        r#"(?is)<a[^>]+href=["'][^"']*(?:sharedfiles|workshop)/filedetails/[^"']*(?:\?|&amp;|&)id=(\d+)[^"']*["'][^>]*>(.*?)</a>"#,
    ) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for captures in link_regex.captures_iter(html) {
        let Some(id) = captures.get(1).map(|value| value.as_str().to_string()) else {
            continue;
        };
        if id == excluded_id || !seen.insert(id.clone()) {
            continue;
        }

        let link_text = captures
            .get(2)
            .map(|value| clean_html_text(value.as_str()))
            .unwrap_or_default();
        let title =
            collection_item_title(captures.get(0).map(|value| value.as_str()).unwrap_or(""))
                .filter(|value| !value.is_empty())
                .or_else(|| valid_workshop_link_text(&link_text).then_some(link_text))
                .unwrap_or_else(|| format!("Workshop Item {id}"));
        items.push((id, title));
    }

    items
}

fn valid_workshop_link_text(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with("http://")
        && !trimmed.starts_with("https://")
        && !trimmed.eq_ignore_ascii_case("instructions page")
}

fn clean_html_text(value: &str) -> String {
    let without_tags = Regex::new(r"(?is)<[^>]+>")
        .map(|regex| regex.replace_all(value, " ").into_owned())
        .unwrap_or_else(|_| value.to_string());
    decode_html_entities(&without_tags)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

fn workshop_detail_url(id: &str) -> String {
    format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}")
}

#[tauri::command]
fn clear_queue(app: AppHandle, ctx: State<AppContext>) -> Result<Vec<QueuedMod>, String> {
    let conn = open_database(&ctx.paths)?;
    conn.execute("DELETE FROM download_queue", [])
        .map_err(|err| err.to_string())?;
    let queue = load_queue_from_db(&conn)?;
    emit_queue_changed(&app);
    push_workshop_known_states_to_browser(&app);
    Ok(queue)
}

#[tauri::command]
fn start_download(app: AppHandle, ctx: State<AppContext>) -> Result<CommandResult, String> {
    if ctx
        .active_child
        .lock()
        .map_err(|err| err.to_string())?
        .is_some()
    {
        return Ok(CommandResult {
            success: false,
            message: "Download already in progress.".into(),
        });
    }

    let settings = load_settings(&ctx.paths);
    let validation = validate_settings_paths(&settings);
    if !validation.valid {
        return Ok(CommandResult {
            success: false,
            message: validation.message,
        });
    }

    let conn = open_database(&ctx.paths)?;
    let queue = load_queue_from_db(&conn)?;
    if queue.is_empty() {
        return Ok(CommandResult {
            success: false,
            message: "No mods are queued.".into(),
        });
    }

    let paths = ctx.paths.clone();
    let active_child = ctx.active_child.clone();
    thread::spawn(move || {
        let result = run_download_worker(
            app.clone(),
            paths.clone(),
            settings,
            queue,
            active_child.clone(),
        );
        if let Err(message) = result {
            emit_download_event(&app, "finished", message, Some(false), None, None);
            if let Ok(mut guard) = active_child.lock() {
                *guard = None;
            }
        }
    });

    Ok(CommandResult {
        success: true,
        message: "Download started.".into(),
    })
}

fn run_download_worker(
    app: AppHandle,
    paths: AppPaths,
    settings: AppSettings,
    queue: Vec<QueuedMod>,
    active_child: Arc<Mutex<Option<std::process::Child>>>,
) -> Result<(), String> {
    emit_download_event(
        &app,
        "started",
        "Starting SteamCMD...".into(),
        None,
        None,
        None,
    );

    fs::create_dir_all(&settings.mod_download_path).map_err(|err| err.to_string())?;
    let mut args = Vec::new();
    if settings.use_anonymous_login {
        args.extend(["+login".to_string(), "anonymous".to_string()]);
    } else {
        if settings.steam_username.trim().is_empty() {
            return Err("Steam username is required when anonymous login is disabled.".into());
        }
        args.extend(["+login".to_string(), settings.steam_username.clone()]);
    }
    args.extend([
        "+force_install_dir".to_string(),
        settings.mod_download_path.clone(),
    ]);
    for item in &queue {
        args.extend([
            "+workshop_download_item".to_string(),
            PROJECT_ZOMBOID_APP_ID.to_string(),
            item.publishedfileid.clone(),
        ]);
    }
    args.push("+quit".to_string());

    emit_download_event(
        &app,
        "output",
        format!("Executing SteamCMD with {} queued item(s).\n", queue.len()),
        None,
        None,
        None,
    );

    let steamcmd_executable = resolve_steamcmd_executable(&settings.steamcmd_path);
    let mut command = Command::new(&steamcmd_executable);
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_no_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to start SteamCMD: {err}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    {
        let mut guard = active_child.lock().map_err(|err| err.to_string())?;
        *guard = Some(child);
    }

    if let Some(stdout) = stdout {
        let app_clone = app.clone();
        thread::spawn(move || stream_reader(app_clone, stdout, false));
    }
    if let Some(stderr) = stderr {
        let app_clone = app.clone();
        thread::spawn(move || stream_reader(app_clone, stderr, true));
    }

    let exit_code = loop {
        let mut guard = active_child.lock().map_err(|err| err.to_string())?;
        if let Some(child) = guard.as_mut() {
            match child.try_wait().map_err(|err| err.to_string())? {
                Some(status) => {
                    let code = status.code().unwrap_or(-1);
                    *guard = None;
                    break code;
                }
                None => {}
            }
        } else {
            return Ok(());
        }
        drop(guard);
        thread::sleep(Duration::from_millis(250));
    };

    if exit_code != 0 {
        emit_download_event(
            &app,
            "finished",
            format!("SteamCMD exited with code {exit_code}."),
            Some(false),
            None,
            None,
        );
        return Ok(());
    }

    emit_download_event(
        &app,
        "output",
        "\nProcessing downloaded mods...\n".into(),
        None,
        None,
        None,
    );
    process_downloaded_mods(&app, &paths, &settings, &queue)?;

    if settings.auto_clear_queue {
        let conn = open_database(&paths)?;
        conn.execute("DELETE FROM download_queue", [])
            .map_err(|err| err.to_string())?;
        emit_queue_changed(&app);
    }
    push_workshop_known_states_to_browser(&app);

    emit_download_event(
        &app,
        "finished",
        "Download completed and mods processed successfully.".into(),
        Some(true),
        None,
        None,
    );
    Ok(())
}

fn stream_reader<R: std::io::Read + Send + 'static>(app: AppHandle, reader: R, is_error: bool) {
    let reader = BufReader::new(reader);
    for line in reader.lines().flatten() {
        let message = if is_error {
            format!("ERROR: {line}\n")
        } else {
            format!("{line}\n")
        };
        emit_download_event(&app, "output", message.clone(), None, None, None);
        if line.contains("Downloading") {
            emit_download_event(&app, "progress", "Downloading...".into(), None, None, None);
        } else if line.contains("Update state") {
            emit_download_event(&app, "progress", "Updating...".into(), None, None, None);
        } else if line.contains("Success") || line.contains("fully installed") {
            emit_download_event(
                &app,
                "progress",
                "Download successful.".into(),
                None,
                None,
                None,
            );
        }
    }
}

fn process_downloaded_mods(
    app: &AppHandle,
    paths: &AppPaths,
    settings: &AppSettings,
    queue: &[QueuedMod],
) -> Result<(), String> {
    let mod_download_path = Path::new(&settings.mod_download_path);
    let workshop_base = mod_download_path
        .join("steamapps")
        .join("workshop")
        .join("content")
        .join(PROJECT_ZOMBOID_APP_ID);
    if !workshop_base.exists() {
        return Err(format!(
            "Workshop folder not found: {}",
            workshop_base.display()
        ));
    }

    let conn = open_database(paths)?;
    let mut processed = 0;
    for item in queue {
        let workshop_mod_folder = workshop_base.join(&item.publishedfileid);
        if !workshop_mod_folder.exists() {
            emit_download_event(
                app,
                "output",
                format!(
                    "Warning: Mod {} not found in workshop folder.\n",
                    item.publishedfileid
                ),
                None,
                None,
                None,
            );
            continue;
        }

        let mut created_folders = Vec::new();
        let mods_subfolder = workshop_mod_folder.join("mods");
        if mods_subfolder.exists() && mods_subfolder.is_dir() {
            emit_download_event(
                app,
                "output",
                format!("Processing mod {}...\n", item.publishedfileid),
                None,
                None,
                None,
            );
            for entry in fs::read_dir(&mods_subfolder).map_err(|err| err.to_string())? {
                let source = entry.map_err(|err| err.to_string())?.path();
                let Some(file_name) = source.file_name() else {
                    continue;
                };
                let destination = mod_download_path.join(file_name);
                replace_path(&source, &destination)?;
                created_folders.push(file_name.to_string_lossy().to_string());
                emit_download_event(
                    app,
                    "output",
                    format!(
                        "Moved {} to {}\n",
                        file_name.to_string_lossy(),
                        mod_download_path.display()
                    ),
                    None,
                    None,
                    None,
                );
            }
        } else {
            emit_download_event(
                app,
                "output",
                format!(
                    "Processing mod {} without a mods subfolder...\n",
                    item.publishedfileid
                ),
                None,
                None,
                None,
            );
            let destination = mod_download_path.join(&item.publishedfileid);
            replace_path(&workshop_mod_folder, &destination)?;
            created_folders.push(item.publishedfileid.clone());
        }

        for folder in &created_folders {
            let now = Utc::now().to_rfc3339();
            let workshop_url = format!(
                "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
                item.publishedfileid
            );
            conn.execute(
                "INSERT OR REPLACE INTO downloaded_mods
                 (publishedfileid, title, download_date, file_size, last_updated, workshop_url)
                 VALUES (?, ?, ?, ?, ?, ?)",
                params![folder, &item.title, now, 0, now, workshop_url],
            )
            .map_err(|err| err.to_string())?;
        }

        emit_download_event(
            app,
            "processed",
            format!("Processed {}.", item.title),
            None,
            Some(item.publishedfileid.clone()),
            Some(created_folders),
        );
        processed += 1;
    }

    let steamapps_folder = mod_download_path.join("steamapps");
    if steamapps_folder.exists() {
        if let Err(err) = fs::remove_dir_all(&steamapps_folder) {
            emit_download_event(
                app,
                "output",
                format!("Warning: Could not clean up steamapps folder: {err}\n"),
                None,
                None,
                None,
            );
        }
    }

    emit_download_event(
        app,
        "output",
        format!("\nSuccessfully processed {processed} mod(s).\n"),
        None,
        None,
        None,
    );
    Ok(())
}

fn replace_path(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        if destination.is_dir() {
            fs::remove_dir_all(destination).map_err(|err| err.to_string())?;
        } else {
            fs::remove_file(destination).map_err(|err| err.to_string())?;
        }
    }
    fs::rename(source, destination)
        .or_else(|_| {
            copy_recursively(source, destination)?;
            if source.is_dir() {
                fs::remove_dir_all(source)
            } else {
                fs::remove_file(source)
            }
        })
        .map_err(|err| err.to_string())
}

fn copy_recursively(source: &Path, destination: &Path) -> std::io::Result<()> {
    if source.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
        return Ok(());
    }
    for entry in WalkDir::new(source) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source).unwrap();
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn cancel_download(app: AppHandle, ctx: State<AppContext>) -> Result<CommandResult, String> {
    let mut guard = ctx.active_child.lock().map_err(|err| err.to_string())?;
    if let Some(child) = guard.as_mut() {
        child.kill().map_err(|err| err.to_string())?;
        *guard = None;
        emit_download_event(
            &app,
            "finished",
            "Download cancelled by user.".into(),
            Some(false),
            None,
            None,
        );
        Ok(CommandResult {
            success: true,
            message: "Download cancelled.".into(),
        })
    } else {
        Ok(CommandResult {
            success: false,
            message: "No download is running.".into(),
        })
    }
}

#[tauri::command]
fn list_local_mods(ctx: State<AppContext>) -> Result<Vec<LocalMod>, String> {
    let settings = load_settings(&ctx.paths);
    let conn = open_database(&ctx.paths)?;
    scan_local_mods(&settings, &conn, &ctx.paths)
}

#[tauri::command]
fn delete_local_mod(path: String, ctx: State<AppContext>) -> Result<Vec<LocalMod>, String> {
    let target = PathBuf::from(path);
    if target.exists() && target.is_dir() {
        fs::remove_dir_all(&target).map_err(|err| err.to_string())?;
    }
    list_local_mods(ctx)
}

#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    open::that(path).map_err(|err| err.to_string())
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    open::that(url).map_err(|err| err.to_string())
}

#[tauri::command]
async fn open_workshop_browser(
    app: AppHandle,
    url: String,
    bounds: BrowserBounds,
) -> Result<(), String> {
    let parsed_url = url
        .parse()
        .map_err(|err| format!("invalid Workshop URL: {err}"))?;
    let position = LogicalPosition::new(bounds.x, bounds.y);
    let size = LogicalSize::new(bounds.width.max(1.0), bounds.height.max(1.0));

    if let Some(existing) = app.get_webview("workshop-browser") {
        existing
            .navigate(parsed_url)
            .map_err(|err| format!("navigate Workshop browser: {err}"))?;
        existing
            .set_position(position)
            .map_err(|err| format!("position Workshop browser: {err}"))?;
        existing
            .set_size(size)
            .map_err(|err| format!("resize Workshop browser: {err}"))?;
        return Ok(());
    }

    let window = app
        .get_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;
    let app_for_navigation = app.clone();
    let app_for_load = app.clone();
    let webview = WebviewBuilder::new("workshop-browser", WebviewUrl::External(parsed_url))
        .initialization_script(workshop_browser_button_script())
        .on_navigation(move |navigation_url| {
            handle_workshop_bridge_navigation(&app_for_navigation, navigation_url)
        })
        .on_page_load(move |_webview, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            if let Some(publishedfileid) = workshop_id_from_url(payload.url()) {
                push_workshop_item_state_to_browser(&app_for_load, &publishedfileid);
            }
            push_workshop_known_states_to_browser(&app_for_load);
        });
    window
        .add_child(webview, position, size)
        .map_err(|err| format!("create Workshop browser: {err}"))?;

    Ok(())
}

#[tauri::command]
fn close_workshop_browser(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview("workshop-browser") {
        existing
            .close()
            .map_err(|err| format!("close Workshop browser: {err}"))?;
    }
    if let Some(window) = app.get_webview_window("workshop") {
        let _ = window.close();
    }
    Ok(())
}

fn handle_workshop_bridge_navigation(app: &AppHandle, navigation_url: &tauri::Url) -> bool {
    if navigation_url.scheme() != "zmd-workshop" {
        return true;
    }

    let action = navigation_url.host_str().unwrap_or_default();
    let mut publishedfileid = String::new();
    let mut title = String::new();
    for (key, value) in navigation_url.query_pairs() {
        match key.as_ref() {
            "publishedfileid" => publishedfileid = value.into_owned(),
            "title" => title = value.into_owned(),
            _ => {}
        }
    }

    if publishedfileid.chars().all(|c| c.is_ascii_digit()) {
        if let Some(ctx) = app.try_state::<AppContext>() {
            let result = (|| {
                let conn = open_database(&ctx.paths)?;
                match action {
                    "queue" => {
                        let settings = load_settings(&ctx.paths);
                        let local_mods = scan_local_mods(&settings, &conn, &ctx.paths)?;
                        let installed_ids = installed_ids_from_mods(&local_mods);
                        queue_item_with_required_items(
                            &conn,
                            &installed_ids,
                            &publishedfileid,
                            &title,
                        )?;
                    }
                    "unqueue" => {
                        delete_queue_item(&conn, &publishedfileid)?;
                    }
                    "collection" => {
                        let payload = WorkshopCollectionRequested {
                            collection_id: publishedfileid.clone(),
                            title: normalized_workshop_title(&publishedfileid, &title),
                            url: workshop_detail_url(&publishedfileid),
                        };
                        let _ = app.emit("workshop-collection-requested", payload);
                    }
                    _ => {}
                }
                workshop_item_state(&ctx.paths, &publishedfileid)
            })();

            match result {
                Ok(state) => {
                    if action != "collection" {
                        emit_queue_changed(app);
                        push_workshop_state_to_browser(app, &publishedfileid, &state);
                        push_workshop_known_states_to_browser(app);
                    }
                }
                Err(err) => {
                    let escaped =
                        serde_json::to_string(&err).unwrap_or_else(|_| "\"Unknown error\"".into());
                    if let Some(webview) = app.get_webview("workshop-browser") {
                        let _ = webview.eval(format!(
                            "window.__zmdWorkshopButtonError && window.__zmdWorkshopButtonError({escaped});"
                        ));
                    }
                }
            }
        }
    }

    false
}

fn workshop_id_from_url(url: &tauri::Url) -> Option<String> {
    let host = url.host_str()?;
    if host != "steamcommunity.com" && !host.ends_with(".steamcommunity.com") {
        return None;
    }
    let path = url.path();
    if !path.contains("/sharedfiles/filedetails") && !path.contains("/workshop/filedetails") {
        return None;
    }
    url.query_pairs().find_map(|(key, value)| {
        (key == "id" || key == "publishedfileid")
            .then(|| value.into_owned())
            .filter(|id| id.chars().all(|c| c.is_ascii_digit()))
    })
}

fn push_workshop_item_state_to_browser(app: &AppHandle, publishedfileid: &str) {
    if let Some(ctx) = app.try_state::<AppContext>() {
        if let Ok(state) = workshop_item_state(&ctx.paths, publishedfileid) {
            push_workshop_state_to_browser(app, publishedfileid, &state);
        }
    }
}

fn push_workshop_state_to_browser(
    app: &AppHandle,
    publishedfileid: &str,
    state: &WorkshopItemState,
) {
    let Ok(id_json) = serde_json::to_string(publishedfileid) else {
        return;
    };
    let Ok(state_json) = serde_json::to_string(state) else {
        return;
    };
    if let Some(webview) = app.get_webview("workshop-browser") {
        let _ = webview.eval(format!(
            "window.__zmdSetWorkshopButtonState && window.__zmdSetWorkshopButtonState({id_json}, {state_json});"
        ));
    }
}

fn push_workshop_known_states_to_browser(app: &AppHandle) {
    let Some(ctx) = app.try_state::<AppContext>() else {
        return;
    };
    let Ok(conn) = open_database(&ctx.paths) else {
        return;
    };
    let queued_ids: Vec<String> = load_queue_from_db(&conn)
        .map(|queue| queue.into_iter().map(|item| item.publishedfileid).collect())
        .unwrap_or_default();
    let settings = load_settings(&ctx.paths);
    let installed_ids: Vec<String> = scan_local_mods(&settings, &conn, &ctx.paths)
        .map(|mods| installed_ids_from_mods(&mods).into_iter().collect())
        .unwrap_or_default();
    let Ok(queued_json) = serde_json::to_string(&queued_ids) else {
        return;
    };
    let Ok(installed_json) = serde_json::to_string(&installed_ids) else {
        return;
    };
    if let Some(webview) = app.get_webview("workshop-browser") {
        let _ = webview.eval(format!(
            "window.__zmdSetWorkshopKnownStates && window.__zmdSetWorkshopKnownStates({queued_json}, {installed_json});"
        ));
    }
}

fn scan_local_mods(
    settings: &AppSettings,
    conn: &Connection,
    paths: &AppPaths,
) -> Result<Vec<LocalMod>, String> {
    let mod_path = Path::new(&settings.mod_download_path);
    if settings.mod_download_path.trim().is_empty() || !mod_path.exists() {
        return Ok(Vec::new());
    }

    let mut mods = Vec::new();
    let mut workshop_fallbacks = HashMap::new();
    for entry in fs::read_dir(mod_path).map_err(|err| err.to_string())? {
        let path = entry.map_err(|err| err.to_string())?.path();
        if !path.is_dir() {
            continue;
        }
        let folder_name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        if folder_name.starts_with('.') || folder_name == "steamapps" {
            continue;
        }
        let modified_at = path
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .map(|time| chrono::DateTime::<Utc>::from(time).to_rfc3339())
            .unwrap_or_default();
        if let Some(cached) = cached_local_mod(conn, paths, &folder_name, &path, &modified_at)? {
            mods.push(cached);
            continue;
        }

        let info = read_mod_info(&path);
        let workshop_url = get_workshop_url(conn, &folder_name).unwrap_or_default();
        let workshop_id = extract_workshop_id(&workshop_url)
            .or_else(|| {
                folder_name
                    .chars()
                    .all(|c| c.is_ascii_digit())
                    .then(|| folder_name.clone())
            })
            .unwrap_or_default();
        let fallback = if workshop_id.is_empty() {
            WorkshopLocalFallback::default()
        } else {
            workshop_fallbacks
                .entry(workshop_id.clone())
                .or_insert_with(|| workshop_local_fallback(conn, &workshop_id))
                .clone()
        };
        let cached_poster_path = if workshop_id.is_empty() {
            String::new()
        } else {
            cached_workshop_poster_path(paths, &workshop_id, fallback.poster_url.as_deref())
                .unwrap_or_default()
        };
        let size_bytes = folder_size(&path);

        let local_mod = LocalMod {
            display_name: info
                .get("name")
                .cloned()
                .or_else(|| fallback.title.clone())
                .unwrap_or_else(|| folder_name.clone()),
            package_id: info.get("id").cloned().unwrap_or_default(),
            authors: first_present(&info, &["authors", "author"])
                .cloned()
                .or_else(|| fallback.author.clone())
                .unwrap_or_default(),
            mod_version: first_present(&info, &["modversion", "version"])
                .cloned()
                .or_else(|| fallback.mod_version.clone())
                .unwrap_or_default(),
            pz_version: first_present(&info, &["pzversion", "pz_version", "build"])
                .cloned()
                .or_else(|| fallback.supported_version.clone())
                .unwrap_or_default(),
            size_bytes,
            path: path.to_string_lossy().to_string(),
            modified_at: modified_at.clone(),
            workshop_url: if workshop_url.is_empty() && !workshop_id.is_empty() {
                format!("https://steamcommunity.com/sharedfiles/filedetails/?id={workshop_id}")
            } else {
                workshop_url
            },
            workshop_id,
            poster_path: cached_poster_path,
            poster_url: fallback.poster_url.unwrap_or_default(),
            folder_name,
        };
        let _ = save_local_mod_cache(conn, &local_mod, &modified_at);
        mods.push(local_mod);
    }
    mods.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    Ok(mods)
}

fn cached_local_mod(
    conn: &Connection,
    paths: &AppPaths,
    folder_name: &str,
    path: &Path,
    modified_at: &str,
) -> Result<Option<LocalMod>, String> {
    match conn.query_row(
        "SELECT display_name, package_id, authors, mod_version, pz_version, COALESCE(size_bytes, 0),
                poster_path, poster_url, workshop_id, workshop_url
         FROM local_mod_metadata_cache
         WHERE folder_name = ? AND folder_modified_at = ?",
        params![folder_name, modified_at],
        |row| {
            let _poster_path: String = row.get(6)?;
            let poster_url: String = row.get(7)?;
            let workshop_id: String = row.get(8)?;
            let poster_url_for_cache = optional_nonempty(poster_url.clone())
                .or_else(|| workshop_local_fallback(conn, &workshop_id).poster_url);
            let poster_path = cached_workshop_poster_path(
                paths,
                &workshop_id,
                poster_url_for_cache.as_deref(),
            )
            .unwrap_or_default();
            Ok(LocalMod {
                folder_name: folder_name.to_string(),
                display_name: row.get(0)?,
                package_id: row.get(1)?,
                authors: row.get(2)?,
                mod_version: row.get(3)?,
                pz_version: row.get(4)?,
                size_bytes: row.get::<_, i64>(5)?.max(0) as u64,
                path: path.to_string_lossy().to_string(),
                modified_at: modified_at.to_string(),
                poster_path,
                poster_url,
                workshop_id,
                workshop_url: row.get(9)?,
            })
        },
    ) {
        Ok(local_mod) => Ok(Some(local_mod)),
        Err(SqlError::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn save_local_mod_cache(
    conn: &Connection,
    local_mod: &LocalMod,
    folder_modified_at: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO local_mod_metadata_cache
         (folder_name, folder_modified_at, display_name, package_id, authors, mod_version,
          pz_version, size_bytes, poster_path, poster_url, workshop_id, workshop_url, cached_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            &local_mod.folder_name,
            folder_modified_at,
            &local_mod.display_name,
            &local_mod.package_id,
            &local_mod.authors,
            &local_mod.mod_version,
            &local_mod.pz_version,
            local_mod.size_bytes as i64,
            &local_mod.poster_path,
            &local_mod.poster_url,
            &local_mod.workshop_id,
            &local_mod.workshop_url,
            Utc::now().to_rfc3339(),
        ],
    )
    .map(|_| ())
    .map_err(|err| err.to_string())
}

fn cached_workshop_poster_path(
    paths: &AppPaths,
    publishedfileid: &str,
    poster_url: Option<&str>,
) -> Option<String> {
    if publishedfileid.is_empty() {
        return None;
    }

    let cache_dir = paths.data_dir.join("poster-cache");
    if let Ok(entries) = fs::read_dir(&cache_dir) {
        let prefix = format!("{publishedfileid}.");
        if let Some(existing) = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .map(|name| name.to_string_lossy().starts_with(&prefix))
                        .unwrap_or(false)
            })
        {
            return Some(frontend_path(&existing));
        }
    }

    let poster_url = poster_url?.trim();
    if poster_url.is_empty() {
        return None;
    }

    fs::create_dir_all(&cache_dir).ok()?;
    let extension = poster_extension_from_url(poster_url);
    let target = cache_dir.join(format!("{publishedfileid}.{extension}"));
    if download_workshop_poster(poster_url, &target).is_ok() && target.exists() {
        Some(frontend_path(&target))
    } else {
        let _ = fs::remove_file(&target);
        None
    }
}

fn poster_extension_from_url(url: &str) -> &'static str {
    let lower = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if lower.ends_with(".png") {
        "png"
    } else if lower.ends_with(".webp") {
        "webp"
    } else if lower.ends_with(".gif") {
        "gif"
    } else {
        "jpg"
    }
}

fn download_workshop_poster(url: &str, target: &Path) -> Result<(), String> {
    let script = "& { param([string]$ImageUrl, [string]$OutFile) $ProgressPreference='SilentlyContinue'; [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -UseBasicParsing -Uri $ImageUrl -OutFile $OutFile }";
    let target_path = target.to_string_lossy().to_string();
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
        url,
        &target_path,
    ]);
    apply_no_window(&mut command);
    let output = command
        .output()
        .map_err(|err| format!("Could not start PowerShell to cache Workshop poster: {err}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            "Could not cache Workshop poster.".into()
        } else {
            format!("Could not cache Workshop poster: {message}")
        });
    }
    Ok(())
}

fn frontend_path(path: &Path) -> String {
    normalize_windows_extended_path(&path.to_string_lossy())
}

fn normalize_windows_extended_path(path: &str) -> String {
    #[cfg(windows)]
    {
        if let Some(stripped) = path.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{stripped}");
        }
        if let Some(stripped) = path.strip_prefix(r"\\?\") {
            return stripped.to_string();
        }
    }
    path.to_string()
}

fn read_mod_info(path: &Path) -> std::collections::HashMap<String, String> {
    let mut info = std::collections::HashMap::new();
    let Some(mod_info) = find_mod_info_path(path) else {
        return info;
    };
    let Ok(raw) = fs::read_to_string(mod_info) else {
        return info;
    };
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.contains('=') {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        let key = parts.next().unwrap_or_default().trim().to_lowercase();
        let value = parts.next().unwrap_or_default().trim().to_string();
        info.entry(key)
            .and_modify(|existing| {
                if !value.is_empty() {
                    existing.push('\n');
                    existing.push_str(&value);
                }
            })
            .or_insert(value);
    }
    info
}

fn find_mod_info_path(path: &Path) -> Option<PathBuf> {
    let root_info = path.join("mod.info");
    if root_info.exists() {
        return Some(root_info);
    }

    WalkDir::new(path)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_type().is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("mod.info")
        })
        .map(|entry| entry.into_path())
}

fn first_present<'a>(values: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a String> {
    keys.iter()
        .filter_map(|key| values.get(*key))
        .find(|value| !value.trim().is_empty())
}

#[derive(Default, Clone)]
struct WorkshopLocalFallback {
    title: Option<String>,
    author: Option<String>,
    mod_version: Option<String>,
    supported_version: Option<String>,
    poster_url: Option<String>,
}

fn workshop_local_fallback(conn: &Connection, publishedfileid: &str) -> WorkshopLocalFallback {
    if let Ok(Some(cached)) = cached_workshop_local_fallback(conn, publishedfileid) {
        if cached.poster_url.is_some() {
            return cached;
        }
    }

    let fallback = fetch_workshop_page(&workshop_detail_url(publishedfileid))
        .map(|html| workshop_local_fallback_from_html(&html))
        .unwrap_or_default();
    let _ = save_workshop_local_fallback(conn, publishedfileid, &fallback);
    fallback
}

fn cached_workshop_local_fallback(
    conn: &Connection,
    publishedfileid: &str,
) -> Result<Option<WorkshopLocalFallback>, String> {
    match conn.query_row(
        "SELECT title, author, mod_version, supported_version, poster_url
         FROM workshop_metadata_cache
         WHERE publishedfileid = ?",
        params![publishedfileid],
        |row| {
            Ok(WorkshopLocalFallback {
                title: optional_nonempty(row.get::<_, Option<String>>(0)?.unwrap_or_default()),
                author: optional_nonempty(row.get::<_, Option<String>>(1)?.unwrap_or_default()),
                mod_version: optional_nonempty(
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                ),
                supported_version: optional_nonempty(
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                ),
                poster_url: optional_nonempty(row.get::<_, Option<String>>(4)?.unwrap_or_default()),
            })
        },
    ) {
        Ok(fallback) => Ok(Some(fallback)),
        Err(SqlError::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn save_workshop_local_fallback(
    conn: &Connection,
    publishedfileid: &str,
    fallback: &WorkshopLocalFallback,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO workshop_metadata_cache
         (publishedfileid, title, author, mod_version, supported_version, poster_url, fetched_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            publishedfileid,
            fallback.title.as_deref().unwrap_or_default(),
            fallback.author.as_deref().unwrap_or_default(),
            fallback.mod_version.as_deref().unwrap_or_default(),
            fallback.supported_version.as_deref().unwrap_or_default(),
            fallback.poster_url.as_deref().unwrap_or_default(),
            Utc::now().to_rfc3339(),
        ],
    )
    .map(|_| ())
    .map_err(|err| err.to_string())
}

fn optional_nonempty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn workshop_local_fallback_from_html(html: &str) -> WorkshopLocalFallback {
    WorkshopLocalFallback {
        title: page_title(html),
        author: workshop_author_from_html(html),
        mod_version: workshop_mod_version_from_html(html),
        supported_version: workshop_supported_version_from_html(html),
        poster_url: workshop_poster_from_html(html),
    }
}

fn workshop_author_from_html(html: &str) -> Option<String> {
    let patterns = [
        r#"(?is)<div[^>]+class=["'][^"']*friendBlockContent[^"']*["'][^>]*>\s*<a[^>]*>(.*?)</a>"#,
        r#"(?is)<div[^>]+class=["'][^"']*workshopItemAuthorName[^"']*["'][^>]*>\s*<a[^>]*>(.*?)</a>"#,
        r#"(?is)Created by\s*</?[^>]*>\s*<a[^>]*>(.*?)</a>"#,
    ];
    patterns.iter().find_map(|pattern| {
        Regex::new(pattern)
            .ok()?
            .captures(html)?
            .get(1)
            .map(|value| clean_html_text(value.as_str()))
            .filter(|value| !value.is_empty())
    })
}

fn workshop_mod_version_from_html(html: &str) -> Option<String> {
    let text = clean_html_text(html);
    let patterns = [
        r#"(?i)\bmod\s+version\s*[:=-]\s*([A-Za-z0-9._ -]{1,32})"#,
        r#"(?i)\bversion\s*[:=-]\s*([A-Za-z0-9._ -]{1,32})"#,
    ];
    patterns.iter().find_map(|pattern| {
        Regex::new(pattern)
            .ok()?
            .captures(&text)?
            .get(1)
            .map(|value| value.as_str().trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn workshop_supported_version_from_html(html: &str) -> Option<String> {
    let text = clean_html_text(html);
    Regex::new(r#"(?i)\bBuild\s*([0-9]+(?:\.[0-9]+)?)\b"#)
        .ok()?
        .captures(&text)?
        .get(1)
        .map(|value| format!("Build {}", value.as_str()))
}

fn workshop_poster_from_html(html: &str) -> Option<String> {
    let patterns = [
        r#"(?is)<img[^>]+id=["']previewImageMain["'][^>]+src=["']([^"']+)["']"#,
        r#"(?is)<img[^>]+id=["']previewImageMain["'][^>]+data-src=["']([^"']+)["']"#,
        r#"(?is)<img[^>]+src=["']([^"']+)["'][^>]+id=["']previewImageMain["']"#,
        r#"(?is)<img[^>]+data-src=["']([^"']+)["'][^>]+id=["']previewImageMain["']"#,
        r#"(?is)<meta[^>]+property=["']og:image["'][^>]+content=["']([^"']+)["']"#,
        r#"(?is)<meta[^>]+content=["']([^"']+)["'][^>]+property=["']og:image["']"#,
    ];
    patterns
        .iter()
        .find_map(|pattern| {
            Regex::new(pattern)
                .ok()?
                .captures(html)?
                .get(1)
                .map(|value| normalize_workshop_image_url(value.as_str()))
                .filter(|value| is_workshop_image_url(value))
        })
        .or_else(|| steamusercontent_image_from_html(html))
}

fn steamusercontent_image_from_html(html: &str) -> Option<String> {
    let normalized_html = html.replace("\\/", "/");
    let Ok(regex) = Regex::new(r#"(?is)https?://images\.steamusercontent\.com/ugc/[^"' <>)\\]+"#)
    else {
        return None;
    };
    let image_url = regex
        .find_iter(&normalized_html)
        .map(|value| normalize_workshop_image_url(value.as_str()))
        .find(|value| is_workshop_image_url(value));
    image_url
}

fn normalize_workshop_image_url(value: &str) -> String {
    decode_html_entities(value)
        .replace("\\/", "/")
        .trim()
        .to_string()
}

fn is_workshop_image_url(value: &str) -> bool {
    (value.starts_with("http://") || value.starts_with("https://"))
        && (value.contains("images.steamusercontent.com/ugc/")
            || value.contains("steamuserimages-a.akamaihd.net/ugc/"))
}

fn get_workshop_url(conn: &Connection, folder_name: &str) -> Result<String, String> {
    match conn.query_row(
        "SELECT COALESCE(workshop_url, '') FROM downloaded_mods WHERE publishedfileid = ?",
        params![folder_name],
        |row| row.get(0),
    ) {
        Ok(url) => Ok(url),
        Err(SqlError::QueryReturnedNoRows) => Ok(String::new()),
        Err(err) => Err(err.to_string()),
    }
}

fn extract_workshop_id(url: &str) -> Option<String> {
    Regex::new(r"[?&]id=(\d+)")
        .ok()?
        .captures(url)?
        .get(1)
        .map(|value| value.as_str().to_string())
}

fn installed_ids_from_mods(mods: &[LocalMod]) -> HashSet<String> {
    mods.iter()
        .filter_map(|local_mod| {
            (!local_mod.workshop_id.is_empty()).then(|| local_mod.workshop_id.clone())
        })
        .collect()
}

fn workshop_browser_button_script() -> &'static str {
    r###"
(() => {
  if (window.__zmdWorkshopButtonInstalled) return;
  window.__zmdWorkshopButtonInstalled = true;

  const APP_ID = "108600";
  const STYLE_ID = "zmd-workshop-button-style";
  const ROOT_ID = "zmd-workshop-button-root";

  let activeId = "";
  let activeTitle = "";
  let busy = false;
  let currentState = { queued: false, installed: false };

  function workshopId() {
    try {
      const url = new URL(window.location.href);
      if (!/steamcommunity\.com$/i.test(url.hostname) && !/\.steamcommunity\.com$/i.test(url.hostname)) return "";
      if (!url.pathname.includes("/sharedfiles/filedetails") && !url.pathname.includes("/workshop/filedetails")) return "";
      const appId = url.searchParams.get("appid") || url.searchParams.get("appids");
      if (appId && appId !== APP_ID) return "";
      return url.searchParams.get("id") || url.searchParams.get("publishedfileid") || "";
    } catch (_) {
      return "";
    }
  }

  function itemTitle(id) {
    const selectors = [
      ".workshopItemTitle",
      ".workshopItemTitleContainer .title",
      ".apphub_AppName",
      "h1"
    ];
    for (const selector of selectors) {
      const text = (document.querySelector(selector)?.textContent || "").trim();
      if (text) return text.replace(/\s+/g, " ");
    }
    return `Workshop Item ${id}`;
  }

  function subscribeTarget() {
    const selectors = [
      "#SubscribeItemBtn",
      "#SubscribeItemOptionAdd",
      "#SubscribeItemOptionSubscribed",
      "#SubscribeItemOptionControls",
      ".subscribeOptionAdd",
      ".subscribeOptionSubscribed",
      ".subscribeOption",
      ".game_area_purchase_game",
      ".workshopItemControls",
      ".rightDetailsBlock",
      ".rightcol",
      "[id*='Subscribe'][class*='btn']",
      "[class*='subscribe'][class*='Button']"
    ];
    for (const selector of selectors) {
      const found = document.querySelector(selector);
      if (found && !found.closest(`#${ROOT_ID}`)) return found;
    }
    return null;
  }

  function fallbackTarget() {
    const selectors = [
      ".workshopItemControls",
      ".rightDetailsBlock",
      ".rightcol",
      ".workshopItemDetailsHeader",
      ".workshopItemTitle",
      "#ig_bottom"
    ];
    for (const selector of selectors) {
      const found = document.querySelector(selector);
      if (found && !found.closest(`#${ROOT_ID}`)) return found;
    }
    return document.body;
  }

  function insertAfter(target, node) {
    if (target.parentElement) {
      target.parentElement.insertBefore(node, target.nextSibling);
    } else {
      document.body.appendChild(node);
    }
  }

  function ensureStyle() {
    if (document.getElementById(STYLE_ID)) return;
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = `
      #${ROOT_ID} {
        margin: 10px 0;
        display: flex;
        align-items: stretch;
      }
      #${ROOT_ID} .zmd-workshop-button {
        border: 0;
        border-radius: 2px;
        color: #fff;
        cursor: pointer;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        min-height: 42px;
        min-width: 188px;
        padding: 0 22px;
        font: 600 15px/1 Arial, Helvetica, sans-serif;
        text-shadow: 0 1px 1px rgba(0, 0, 0, .65);
        box-shadow: inset 0 1px 0 rgba(255,255,255,.18), 0 1px 2px rgba(0,0,0,.35);
      }
      #${ROOT_ID} .zmd-workshop-button.add {
        background: linear-gradient(#8bc53f, #5c8f22);
      }
      #${ROOT_ID} .zmd-workshop-button.remove {
        background: linear-gradient(#d84b3e, #8e241e);
      }
      #${ROOT_ID} .zmd-workshop-button.installed {
        background: linear-gradient(#24572d, #183d20);
        cursor: not-allowed;
        opacity: .72;
      }
      #${ROOT_ID} .zmd-workshop-button:disabled {
        cursor: not-allowed;
      }
      .zmd-steam-subscribe-hidden {
        display: none !important;
      }
      .zmd-workshop-card-button {
        position: absolute;
        z-index: 20;
        top: 6px;
        right: 6px;
        border: 0;
        border-radius: 2px;
        color: #fff;
        cursor: pointer;
        min-height: 30px;
        padding: 0 10px;
        font: 600 13px/1 Arial, Helvetica, sans-serif;
        text-shadow: 0 1px 1px rgba(0, 0, 0, .65);
        box-shadow: inset 0 1px 0 rgba(255,255,255,.18), 0 1px 2px rgba(0,0,0,.45);
        background: linear-gradient(#38d277, #1f9a51);
      }
      .zmd-workshop-card-button.remove {
        background: linear-gradient(#d84b3e, #8e241e);
      }
    `;
    document.head.appendChild(style);
  }

  function ensureRoot(target) {
    let root = document.getElementById(ROOT_ID);
    if (!root) {
      root = document.createElement("div");
      root.id = ROOT_ID;
      insertAfter(target, root);
    } else if (target && !root.parentElement) {
      insertAfter(target, root);
    }
    if (target && target !== document.body && /Subscribe|subscribe/.test(target.id + " " + target.className) && !target.classList.contains("zmd-steam-subscribe-hidden")) {
      target.classList.add("zmd-steam-subscribe-hidden");
    }
    return root;
  }

  function render(state) {
    currentState = state;
    const target = subscribeTarget() || fallbackTarget();
    if (!target) return;
    ensureStyle();
    const root = ensureRoot(target);
    const disabled = state.installed || busy;
    const label = state.installed ? "Installed" : state.queued ? "Remove" : "Add Mod";
    const mode = state.installed ? "installed" : state.queued ? "remove" : "add";
    root.innerHTML = "";
    const button = document.createElement("button");
    button.type = "button";
    button.className = `zmd-workshop-button ${mode}`;
    button.textContent = label;
    button.disabled = disabled;
    button.addEventListener("click", async () => {
      if (busy || state.installed) return;
      busy = true;
      render(state);
      const next = { ...state, queued: !state.queued };
      render(next);
      sendBridgeNavigation(state.queued ? "unqueue" : "queue", activeId, activeTitle || itemTitle(activeId));
    });
    root.appendChild(button);
  }

  function collectionChildIds() {
    const collectionRoot = document.querySelector(".collectionChildren, .workshopCollectionItems, #CollectionItems, .collectionItem, .workshopItemCollection");
    if (!collectionRoot) return [];
    const ids = new Set();
    for (const link of Array.from(collectionRoot.querySelectorAll("a[href*='filedetails'][href*='id=']"))) {
      const id = idFromHref(link.href);
      if (id && id !== activeId && /^\d+$/.test(id)) ids.add(id);
    }
    return Array.from(ids);
  }

  function isCollectionPage() {
    return collectionChildIds().length > 0;
  }

  function renderCollectionButton() {
    const target = fallbackTarget();
    if (!target) return;
    ensureStyle();
    const root = ensureRoot(target);
    root.innerHTML = "";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "zmd-workshop-button add";
    button.textContent = busy ? "Resolving Collection..." : "Queue Collection";
    button.disabled = busy;
    button.addEventListener("click", () => {
      if (busy) return;
      busy = true;
      renderCollectionButton();
      sendBridgeNavigation("collection", activeId, activeTitle || itemTitle(activeId), true);
      window.setTimeout(() => {
        busy = false;
        renderCollectionButton();
      }, 1000);
    });
    root.appendChild(button);
  }

  function sendBridgeNavigation(action, id, title, skipDetailRender) {
    const url = new URL(`zmd-workshop://${action}/`);
    url.searchParams.set("publishedfileid", id);
    if (title) url.searchParams.set("title", title);
    window.location.href = url.toString();
    window.setTimeout(() => {
      busy = false;
      if (!skipDetailRender && activeId) render(currentState);
    }, 750);
  }

  function sendCardBridge(button, action, id, title) {
    button.dataset.zmdQueued = action === "queue" ? "true" : "false";
    button.textContent = action === "queue" ? "Remove" : "Add Mod";
    button.classList.toggle("remove", action === "queue");
    sendBridgeNavigation(action, id, title, true);
  }

  function idFromHref(href) {
    try {
      const url = new URL(href, window.location.href);
      if (!url.pathname.includes("/sharedfiles/filedetails") && !url.pathname.includes("/workshop/filedetails")) return "";
      return url.searchParams.get("id") || url.searchParams.get("publishedfileid") || "";
    } catch (_) {
      return "";
    }
  }

  function decorateBrowseItems() {
    ensureStyle();
    const links = Array.from(document.querySelectorAll("a[href*='filedetails'][href*='id=']"));
    for (const link of links) {
      const id = idFromHref(link.href);
      if (!id || !/^\d+$/.test(id)) continue;
      const card = link.closest(".workshopItem, .workshopItemPreviewHolder, .workshopBrowseItem, .workshopItemCollection, .item") || link.parentElement;
      if (!card || card.querySelector(`.zmd-workshop-card-button[data-publishedfileid="${id}"]`)) continue;
      const style = window.getComputedStyle(card);
      if (style.position === "static") card.style.position = "relative";
      const title =
        (card.querySelector(".workshopItemTitle, .workshopItemTitleContainer, .title")?.textContent || link.textContent || `Workshop Item ${id}`)
          .trim()
          .replace(/\s+/g, " ");
      const button = document.createElement("button");
      button.type = "button";
      button.className = "zmd-workshop-card-button";
      button.dataset.publishedfileid = id;
      button.dataset.zmdQueued = "false";
      button.textContent = "Add Mod";
      button.addEventListener("click", (event) => {
        event.preventDefault();
        event.stopPropagation();
        const queued = button.dataset.zmdQueued === "true";
        sendCardBridge(button, queued ? "unqueue" : "queue", id, title);
      });
      card.appendChild(button);
    }
  }

  window.__zmdSetWorkshopButtonState = (id, state) => {
    const stringId = String(id);
    for (const button of document.querySelectorAll(`.zmd-workshop-card-button[data-publishedfileid="${stringId}"]`)) {
      button.dataset.zmdQueued = state.queued ? "true" : "false";
      button.textContent = state.installed ? "Installed" : state.queued ? "Remove" : "Add Mod";
      button.disabled = !!state.installed;
      button.classList.toggle("remove", !!state.queued && !state.installed);
    }
    if (stringId !== String(activeId)) return;
    busy = false;
    render(state);
  };

  window.__zmdSetWorkshopKnownStates = (queuedIds, installedIds) => {
    const queued = new Set((queuedIds || []).map(String));
    const installed = new Set((installedIds || []).map(String));
    for (const button of document.querySelectorAll(".zmd-workshop-card-button[data-publishedfileid]")) {
      const id = String(button.dataset.publishedfileid || "");
      const isInstalled = installed.has(id);
      const isQueued = queued.has(id);
      button.dataset.zmdQueued = isQueued ? "true" : "false";
      button.textContent = isInstalled ? "Installed" : isQueued ? "Remove" : "Add Mod";
      button.disabled = isInstalled;
      button.classList.toggle("remove", isQueued && !isInstalled);
    }
    if (activeId) {
      busy = false;
      render({
        queued: queued.has(String(activeId)),
        installed: installed.has(String(activeId)),
        title: currentState && currentState.title ? currentState.title : null
      });
    }
  };

  window.__zmdWorkshopButtonError = (message) => {
    console.error("Zomboid Mod Downloader Workshop button failed", message);
    busy = false;
    render(currentState);
  };

  async function refresh() {
    const id = workshopId();
    if (!id) {
      document.getElementById(ROOT_ID)?.remove();
      decorateBrowseItems();
      return;
    }
    activeId = id;
    activeTitle = itemTitle(id);
    const target = subscribeTarget() || fallbackTarget();
    if (!target) return;
    if (isCollectionPage()) {
      renderCollectionButton();
      decorateBrowseItems();
      return;
    }
    render(currentState);
  }

  let scheduled = false;
  function scheduleRefresh() {
    if (scheduled) return;
    scheduled = true;
    window.setTimeout(() => {
      scheduled = false;
      refresh();
    }, 100);
  }

  document.addEventListener("DOMContentLoaded", scheduleRefresh);
  window.addEventListener("load", scheduleRefresh);
  window.addEventListener("popstate", scheduleRefresh);
  window.setInterval(scheduleRefresh, 1500);
  const observeDocument = () => {
    const root = document.documentElement || document.body;
    if (!root) {
      window.setTimeout(observeDocument, 50);
      return;
    }
    new MutationObserver(scheduleRefresh).observe(root, {
      childList: true,
      subtree: true
    });
  };
  observeDocument();
  scheduleRefresh();
})();
"###
}

fn folder_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok().map(|meta| meta.len()))
        .sum()
}

fn emit_download_event(
    app: &AppHandle,
    kind: &str,
    message: String,
    success: Option<bool>,
    publishedfileid: Option<String>,
    folders: Option<Vec<String>>,
) {
    let _ = app.emit(
        "download-progress",
        DownloadProgressEvent {
            kind: kind.into(),
            message,
            success,
            publishedfileid,
            folders,
        },
    );
}

fn emit_queue_changed(app: &AppHandle) {
    let _ = app.emit("workshop-queue-changed", ());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_workshop_input_defaults_to_single_item_without_fetch() {
        let (id, url, should_fetch) = parse_workshop_input("3567084868").unwrap();

        assert_eq!(id, "3567084868");
        assert_eq!(
            url,
            "https://steamcommunity.com/sharedfiles/filedetails/?id=3567084868"
        );
        assert!(!should_fetch);
    }

    #[test]
    fn workshop_url_requires_fetch_for_collection_detection() {
        let (id, url, should_fetch) = parse_workshop_input(
            "https://steamcommunity.com/sharedfiles/filedetails/?id=3724576677",
        )
        .unwrap();

        assert_eq!(id, "3724576677");
        assert_eq!(
            url,
            "https://steamcommunity.com/sharedfiles/filedetails/?id=3724576677"
        );
        assert!(should_fetch);
    }

    #[test]
    fn collection_html_exposes_child_mods() {
        let html = r#"
            <html>
                <body>
                    <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=2872282653">
                        instructions page
                    </a>
                    <div class="collectionChildren">
                        <div class="collectionItem">
                            <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=3567084868">
                                <div class="workshopItemTitle">Single Mod Title</div>
                            </a>
                        </div>
                    </div>
                </body>
            </html>
        "#;

        assert!(looks_like_collection_page(html));
        assert_eq!(
            collection_items_from_html(html, "3724576677"),
            vec![("3567084868".to_string(), "Single Mod Title".to_string())]
        );
    }

    #[test]
    fn collection_child_title_prefers_card_title_over_link_text() {
        let html = r#"
            <div class="collectionChildren">
                <div class="collectionItem">
                    <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=2627877543">
                        https://steamcommunity.com/sharedfiles/filedetails/?id=2627877543
                    </a>
                    <div class="workshopItemTitle">Become Desensitized</div>
                </div>
            </div>
        "#;

        assert_eq!(
            collection_items_from_html(html, "3724576677"),
            vec![("2627877543".to_string(), "Become Desensitized".to_string())]
        );
    }

    #[test]
    fn required_items_extracts_dependencies_only_from_required_section() {
        let html = r#"
            <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=2872282653">
                instructions page
            </a>
            <div>Required items</div>
            <div>This item requires all of the following other items</div>
            <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=2627877543">
                Become Desensitized
            </a>
            <div>Created by</div>
            <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=1234567890">
                Unrelated Link
            </a>
        "#;

        assert_eq!(
            required_items_from_html(html, "3567084868"),
            vec![("2627877543".to_string(), "Become Desensitized".to_string())]
        );
    }

    #[test]
    fn required_items_ignores_self_and_url_link_text() {
        let html = r#"
            <div>Required items</div>
            <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=3567084868">
                Same Item
            </a>
            <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=1111111111">
                https://steamcommunity.com/sharedfiles/filedetails/?id=1111111111
            </a>
            <div>Created by</div>
        "#;

        assert_eq!(
            required_items_from_html(html, "3567084868"),
            vec![(
                "1111111111".to_string(),
                "Workshop Item 1111111111".to_string()
            )]
        );
    }

    #[test]
    fn reads_nested_mod_info() {
        let root = std::env::temp_dir().join(format!(
            "zmd-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let mod_dir = root.join("mods").join("NestedMod");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(
            mod_dir.join("mod.info"),
            "name=Nested Mod\nauthor=Local Author\nversion=1.2.3\nposter=poster.png\n",
        )
        .unwrap();

        let info = read_mod_info(&root);
        assert_eq!(info.get("name").map(String::as_str), Some("Nested Mod"));
        assert_eq!(info.get("author").map(String::as_str), Some("Local Author"));
        assert_eq!(info.get("version").map(String::as_str), Some("1.2.3"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn workshop_fallback_extracts_author_build_and_poster() {
        let html = r#"
            <html>
                <head>
                    <title>[B42] Example Mod - Steam Workshop</title>
                    <meta property="og:image" content="https://images.steamusercontent.com/ugc/24307035126317491/0578D28193C22EBCECD677A4253ACE6426BD842D/">
                </head>
                <body>
                    <a>Build 42</a>
                    <div>Created by</div>
                    <div class="friendBlockContent"><a>Workshop Author</a></div>
                    <div>Description Version: 2.5</div>
                </body>
            </html>
        "#;

        let fallback = workshop_local_fallback_from_html(html);
        assert_eq!(fallback.title.as_deref(), Some("[B42] Example Mod"));
        assert_eq!(fallback.author.as_deref(), Some("Workshop Author"));
        assert_eq!(fallback.mod_version.as_deref(), Some("2.5"));
        assert_eq!(fallback.supported_version.as_deref(), Some("Build 42"));
        assert_eq!(
            fallback.poster_url.as_deref(),
            Some("https://images.steamusercontent.com/ugc/24307035126317491/0578D28193C22EBCECD677A4253ACE6426BD842D/")
        );
    }

    #[test]
    fn workshop_poster_extracts_steamusercontent_preview() {
        let html = r#"
            <div class="highlight_player_area">
                <img src="https://images.steamusercontent.com/ugc/24307035126317491/0578D28193C22EBCECD677A4253ACE6426BD842D/?imw=268&amp;imh=268&amp;ima=fit&amp;impolicy=Letterbox&amp;imcolor=%23000000&amp;letterbox=true">
            </div>
        "#;

        assert_eq!(
            workshop_poster_from_html(html).as_deref(),
            Some("https://images.steamusercontent.com/ugc/24307035126317491/0578D28193C22EBCECD677A4253ACE6426BD842D/?imw=268&imh=268&ima=fit&impolicy=Letterbox&imcolor=%23000000&letterbox=true")
        );
    }

    #[test]
    fn workshop_poster_extracts_escaped_steamusercontent_preview() {
        let html = r#"
            {"preview_url":"https:\/\/images.steamusercontent.com\/ugc\/24307035126317491\/0578D28193C22EBCECD677A4253ACE6426BD842D\/?imw=268&imh=268"}
        "#;

        assert_eq!(
            workshop_poster_from_html(html).as_deref(),
            Some("https://images.steamusercontent.com/ugc/24307035126317491/0578D28193C22EBCECD677A4253ACE6426BD842D/?imw=268&imh=268")
        );
    }

    #[test]
    fn normalizes_windows_extended_paths_for_frontend() {
        assert_eq!(
            normalize_windows_extended_path(r"\\?\C:\Mods\Example\poster.png"),
            r"C:\Mods\Example\poster.png"
        );
        assert_eq!(
            normalize_windows_extended_path(r"\\?\UNC\server\share\poster.png"),
            r"\\server\share\poster.png"
        );
    }

    #[test]
    fn single_item_html_does_not_look_like_collection() {
        let html = r#"
            <html>
                <head><title>Single Mod Title - Steam Workshop</title></head>
                <body><div class="workshopItemTitle">Single Mod Title</div></body>
            </html>
        "#;

        assert!(!looks_like_collection_page(html));
        assert!(collection_items_from_html(html, "3567084868").is_empty());
    }
}
