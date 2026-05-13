export interface AppSettings {
  steamcmdPath: string;
  modDownloadPath: string;
  steamUsername: string;
  useAnonymousLogin: boolean;
  autoClearQueue: boolean;
  windowWidth: number;
  windowHeight: number;
}

export interface QueuedMod {
  publishedfileid: string;
  title: string;
  addedDate: string;
}

export interface DownloadedMod {
  publishedfileid: string;
  title: string;
  downloadDate: string;
  fileSize: number;
  lastUpdated: string;
  workshopUrl: string;
}

export interface LocalMod {
  folderName: string;
  displayName: string;
  packageId: string;
  authors: string;
  modVersion: string;
  pzVersion: string;
  sizeBytes: number;
  path: string;
  modifiedAt: string;
  workshopUrl: string;
  workshopId: string;
  posterPath: string;
  posterUrl: string;
}

export interface AppState {
  settings: AppSettings;
  queue: QueuedMod[];
  downloadedMods: DownloadedMod[];
  localMods: LocalMod[];
  installedWorkshopIds: string[];
}

export interface DownloadProgressEvent {
  kind: "started" | "output" | "progress" | "processed" | "finished";
  message: string;
  success?: boolean;
  publishedfileid?: string;
  folders?: string[];
}

export interface DownloadResult {
  success: boolean;
  message: string;
}

export interface WorkshopItem {
  publishedfileid: string;
  title: string;
  url: string;
}

export interface WorkshopModPreview extends WorkshopItem {
  queued: boolean;
  installed: boolean;
}

export interface WorkshopCollectionPreview {
  collectionId: string;
  title: string;
  url: string;
  items: WorkshopModPreview[];
  totalCount: number;
  queuedCount: number;
  installedCount: number;
  addableCount: number;
}

export interface ResolvedWorkshopEntry {
  kind: "item" | "collection";
  item?: WorkshopModPreview | null;
  collection?: WorkshopCollectionPreview | null;
}

export interface WorkshopCollectionRequested {
  collectionId: string;
  title: string;
  url: string;
}
