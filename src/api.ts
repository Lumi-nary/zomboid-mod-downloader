import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings,
  AppState,
  DownloadProgressEvent,
  DownloadResult,
  LocalMod,
  QueuedMod,
  ResolvedWorkshopEntry,
  WorkshopCollectionRequested,
  WorkshopItem,
} from "./types";

export function loadInitialState() {
  return invoke<AppState>("load_initial_state");
}

export function saveSettings(settings: AppSettings) {
  return invoke<AppState>("save_settings", { settings });
}

export function validatePaths(settings: AppSettings) {
  return invoke<{ valid: boolean; message: string }>("validate_paths", { settings });
}

export function browseSteamcmdPath() {
  return invoke<string | null>("browse_steamcmd_path");
}

export function browseZomboidModPath() {
  return invoke<string | null>("browse_zomboid_mod_path");
}

export function addToQueue(publishedfileid: string, title: string) {
  return invoke<QueuedMod[]>("add_to_queue", { publishedfileid, title });
}

export function addCollectionToQueue(items: WorkshopItem[]) {
  return invoke<QueuedMod[]>("add_collection_to_queue", { items });
}

export function resolveWorkshopEntry(input: string) {
  return invoke<ResolvedWorkshopEntry>("resolve_workshop_entry", { input });
}

export function removeFromQueue(publishedfileid: string) {
  return invoke<QueuedMod[]>("remove_from_queue", { publishedfileid });
}

export function clearQueue() {
  return invoke<QueuedMod[]>("clear_queue");
}

export function startDownload() {
  return invoke<DownloadResult>("start_download");
}

export function cancelDownload() {
  return invoke<DownloadResult>("cancel_download");
}

export function refreshLocalMods() {
  return invoke<LocalMod[]>("list_local_mods");
}

export function deleteLocalMod(path: string) {
  return invoke<LocalMod[]>("delete_local_mod", { path });
}

export function revealPath(path: string) {
  return invoke<void>("reveal_path", { path });
}

export function openUrl(url: string) {
  return invoke<void>("open_url", { url });
}

export interface BrowserBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function openWorkshopBrowser(url: string, bounds: BrowserBounds) {
  return invoke<void>("open_workshop_browser", { url, bounds });
}

export function closeWorkshopBrowser() {
  return invoke<void>("close_workshop_browser");
}

export function onDownloadProgress(handler: (event: DownloadProgressEvent) => void) {
  return listen<DownloadProgressEvent>("download-progress", (event) => handler(event.payload));
}

export function onWorkshopQueueChanged(handler: () => void) {
  return listen("workshop-queue-changed", () => handler());
}

export function onWorkshopCollectionRequested(handler: (event: WorkshopCollectionRequested) => void) {
  return listen<WorkshopCollectionRequested>("workshop-collection-requested", (event) => handler(event.payload));
}
