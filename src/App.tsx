import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  AlertCircle,
  ChevronDown,
  ChevronUp,
  Check,
  Download,
  ExternalLink,
  FolderOpen,
  HardDriveDownload,
  Info,
  RefreshCw,
  Search,
  Settings,
  Terminal,
  Trash2,
  X,
} from "lucide-react";
import {
  addCollectionToQueue,
  addToQueue,
  browseSteamcmdPath,
  browseZomboidModPath,
  cancelDownload,
  closeWorkshopBrowser,
  clearQueue,
  deleteLocalMod,
  loadInitialState,
  onWorkshopCollectionRequested,
  onDownloadProgress,
  onWorkshopQueueChanged,
  openWorkshopBrowser,
  openUrl,
  refreshLocalMods,
  removeFromQueue,
  resolveWorkshopEntry,
  revealPath,
  saveSettings,
  startDownload,
  validatePaths,
} from "./api";
import type {
  AppSettings,
  DownloadProgressEvent,
  LocalMod,
  QueuedMod,
  WorkshopCollectionPreview,
  WorkshopItem,
} from "./types";

const workshopHome = "https://steamcommunity.com/app/108600/workshop/";
const detailUrl = (id: string) => `https://steamcommunity.com/sharedfiles/filedetails/?id=${id}`;

const emptySettings: AppSettings = {
  steamcmdPath: "",
  modDownloadPath: "",
  steamUsername: "",
  useAnonymousLogin: true,
  autoClearQueue: true,
  windowWidth: 1280,
  windowHeight: 820,
};

function parseWorkshopInput(input: string): WorkshopItem | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  const match = trimmed.match(/[?&]id=(\d+)/) ?? trimmed.match(/^\d{5,}$/);
  const id = match ? (Array.isArray(match) ? match[1] ?? match[0] : "") : "";
  if (!id) return null;
  return {
    publishedfileid: id,
    title: `Workshop Item ${id}`,
    url: detailUrl(id),
  };
}

function formatSize(size: number) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = size;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value.toFixed(1)} ${units[index]}`;
}

export function App() {
  const [settings, setSettings] = useState<AppSettings>(emptySettings);
  const [queue, setQueue] = useState<QueuedMod[]>([]);
  const [localMods, setLocalMods] = useState<LocalMod[]>([]);
  const [installedIds, setInstalledIds] = useState<string[]>([]);
  const [selectedLocalPath, setSelectedLocalPath] = useState("");
  const [workshopInput, setWorkshopInput] = useState("");
  const [workshopItems, setWorkshopItems] = useState<WorkshopItem[]>([]);
  const [collectionPreview, setCollectionPreview] = useState<WorkshopCollectionPreview | null>(null);
  const [activeView, setActiveView] = useState<"workshop" | "local">("workshop");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [status, setStatus] = useState("Loading app data...");
  const [downloadEvents, setDownloadEvents] = useState<DownloadProgressEvent[]>([]);
  const [steamCmdLogVisible, setSteamCmdLogVisible] = useState(true);
  const [isDownloading, setIsDownloading] = useState(false);
  const [error, setError] = useState("");
  const openSettings = useCallback(() => {
    closeWorkshopBrowser().catch(() => undefined);
    window.setTimeout(() => closeWorkshopBrowser().catch(() => undefined), 0);
    window.setTimeout(() => closeWorkshopBrowser().catch(() => undefined), 250);
    setSettingsOpen(true);
  }, []);

  useEffect(() => {
    loadInitialState()
      .then((state) => {
        setSettings(state.settings);
        setQueue(state.queue);
        setLocalMods(state.localMods);
        setInstalledIds(state.installedWorkshopIds);
        setStatus(state.settings.steamcmdPath && state.settings.modDownloadPath ? "Ready" : "Settings required");
        if (!state.settings.steamcmdPath || !state.settings.modDownloadPath) {
          openSettings();
        }
      })
      .catch((err) => setError(String(err)));

    const unlisten = onDownloadProgress((event) => {
      setDownloadEvents((events) => [...events, event]);
      if (event.kind === "started") {
        setIsDownloading(true);
        setStatus("Downloading mods...");
      }
      if (event.kind === "finished") {
        setIsDownloading(false);
        setStatus(event.success ? "Download completed" : "Download failed");
        refreshLocalMods().then(setLocalMods).catch((err) => setError(String(err)));
        loadInitialState()
          .then((state) => {
            setQueue(state.queue);
            setInstalledIds(state.installedWorkshopIds);
          })
          .catch((err) => setError(String(err)));
      }
    });

    const unlistenQueue = onWorkshopQueueChanged(() => {
      loadInitialState()
        .then((state) => {
          setQueue(state.queue);
          setLocalMods(state.localMods);
          setInstalledIds(state.installedWorkshopIds);
          setStatus("Queue updated");
        })
        .catch((err) => setError(String(err)));
    });

    const unlistenCollection = onWorkshopCollectionRequested((event) => {
      setStatus("Resolving collection...");
      resolveWorkshopEntry(event.url)
        .then((entry) => {
          if (entry.collection) {
            setCollectionPreview(entry.collection);
            setStatus(`Collection found: ${entry.collection.title}`);
          } else {
            setError("This Workshop page did not expose collection mods.");
          }
        })
        .catch((err) => setError(String(err)));
    });

    return () => {
      unlisten.then((fn) => fn()).catch(() => undefined);
      unlistenQueue.then((fn) => fn()).catch(() => undefined);
      unlistenCollection.then((fn) => fn()).catch(() => undefined);
    };
  }, [openSettings]);

  const selectedLocalMod = useMemo(
    () => localMods.find((mod) => mod.path === selectedLocalPath) ?? localMods[0],
    [localMods, selectedLocalPath],
  );

  async function addWorkshopItem(item: WorkshopItem) {
    const updatedQueue = await addToQueue(item.publishedfileid, item.title);
    setQueue(updatedQueue);
    setStatus(`Queued ${item.title}`);
  }

  async function removeWorkshopItem(publishedfileid: string) {
    const updatedQueue = await removeFromQueue(publishedfileid);
    setQueue(updatedQueue);
    setStatus("Removed item from queue");
  }

  async function addManualWorkshopItem() {
    const fallbackItem = parseWorkshopInput(workshopInput);
    if (!fallbackItem) {
      setError("Paste a Steam Workshop URL or numeric Workshop ID.");
      return;
    }
    setError("");
    setStatus("Resolving Workshop link...");
    try {
      const resolved = await resolveWorkshopEntry(workshopInput);
      if (resolved.collection) {
        setCollectionPreview(resolved.collection);
        setStatus(`Collection found: ${resolved.collection.title}`);
        return;
      }
      const item = resolved.item ?? fallbackItem;
      await addWorkshopItem(item);
      setWorkshopItems((items) => (items.some((existing) => existing.publishedfileid === item.publishedfileid) ? items : [item, ...items]));
      setWorkshopInput("");
      setStatus(`Queued ${item.title}`);
    } catch (err) {
      setError(String(err));
      setStatus("Ready");
    }
  }

  async function confirmCollectionQueue(collection: WorkshopCollectionPreview) {
    const addableItems = collection.items.filter((item) => !item.queued && !item.installed);
    if (!addableItems.length) {
      setCollectionPreview(null);
      setStatus("No new mods to queue from this collection");
      return;
    }
    try {
      const updatedQueue = await addCollectionToQueue(addableItems);
      setQueue(updatedQueue);
      setWorkshopItems((items) => {
        const existingIds = new Set(items.map((item) => item.publishedfileid));
        const nextItems = addableItems
          .filter((item) => !existingIds.has(item.publishedfileid))
          .map((item) => ({
            publishedfileid: item.publishedfileid,
            title: item.title,
            url: item.url,
          }));
        return [...nextItems, ...items];
      });
      setCollectionPreview(null);
      setWorkshopInput("");
      setError("");
      setStatus(`Queued ${addableItems.length} mod${addableItems.length === 1 ? "" : "s"} from ${collection.title}`);
    } catch (err) {
      setError(String(err));
    }
  }

  async function importModList(file: File | null) {
    if (!file) return;
    const raw = await file.text();
    const data = JSON.parse(raw);
    const mods = Array.isArray(data.mods) ? data.mods : [];
    let updated = queue;
    for (const mod of mods) {
      if (!mod.workshop_id) continue;
      if (installedIds.includes(String(mod.workshop_id))) continue;
      updated = await addToQueue(String(mod.workshop_id), String(mod.name || `Workshop Item ${mod.workshop_id}`));
    }
    setQueue(updated);
    setStatus(`Imported ${mods.length} mod entries`);
  }

  function exportModList() {
    const mods = localMods
      .filter((mod) => mod.workshopId)
      .map((mod) => ({
        folder_name: mod.folderName,
        name: mod.displayName,
        workshop_id: mod.workshopId,
        workshop_url: mod.workshopUrl || detailUrl(mod.workshopId),
      }));
    const blob = new Blob(
      [
        JSON.stringify(
          {
            version: "2.0",
            export_date: new Date().toISOString(),
            mods,
          },
          null,
          2,
        ),
      ],
      { type: "application/json" },
    );
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "zomboid_mods.json";
    link.click();
    URL.revokeObjectURL(url);
  }

  async function runDownload() {
    setError("");
    setDownloadEvents([]);
    const validation = await validatePaths(settings);
    if (!validation.valid) {
      setError(validation.message);
      openSettings();
      return;
    }
    const result = await startDownload();
    if (!result.success) setError(result.message);
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <h1>Zomboid Mod Downloader</h1>
          <p>{status}</p>
        </div>
        <nav className="segmented" aria-label="Primary view">
          <button className={activeView === "workshop" ? "active" : ""} onClick={() => setActiveView("workshop")}>
            Workshop
          </button>
          <button className={activeView === "local" ? "active" : ""} onClick={() => setActiveView("local")}>
            Local Mods
          </button>
        </nav>
        <button className="icon-button" title="Settings" onClick={openSettings}>
          <Settings size={18} />
        </button>
      </header>

      {error ? (
        <div className="notice error">
          <AlertCircle size={18} />
          <span>{error}</span>
          <button onClick={() => setError("")} title="Dismiss">
            <X size={16} />
          </button>
        </div>
      ) : null}

      <section className="workspace">
        <section className="primary-pane">
          {activeView === "workshop" ? (
            <WorkshopPane
              settingsOpen={settingsOpen}
              collectionModalOpen={collectionPreview !== null}
              installedIds={installedIds}
              queue={queue}
              items={workshopItems}
              input={workshopInput}
              setInput={setWorkshopInput}
              onAddManual={addManualWorkshopItem}
              onAdd={addWorkshopItem}
              onRemove={removeWorkshopItem}
              onImport={importModList}
            />
          ) : (
            <LocalModsPane
              mods={localMods}
              selected={selectedLocalMod}
              onSelect={setSelectedLocalPath}
              onRefresh={async () => setLocalMods(await refreshLocalMods())}
              onDelete={async (path) => setLocalMods(await deleteLocalMod(path))}
              onOpenFolder={revealPath}
              onOpenWorkshop={openUrl}
              onExport={exportModList}
            />
          )}
        </section>

        <aside className="queue-pane">
          <div className="pane-header">
            <div>
              <h2>Download Queue</h2>
              <p>{queue.length} mod{queue.length === 1 ? "" : "s"} queued</p>
            </div>
            <button className="icon-button" title="Clear queue" disabled={!queue.length || isDownloading} onClick={async () => setQueue(await clearQueue())}>
              <Trash2 size={17} />
            </button>
          </div>

          <div className="queue-list">
            {queue.length ? (
              queue.map((item) => (
                <article key={item.publishedfileid} className="queue-item">
                  <div>
                    <h3>{item.title}</h3>
                    <p>ID {item.publishedfileid}</p>
                  </div>
                  <button className="icon-button" title="Remove" disabled={isDownloading} onClick={() => removeWorkshopItem(item.publishedfileid)}>
                    <X size={16} />
                  </button>
                </article>
              ))
            ) : (
              <div className="empty-state">Paste Workshop links or import a mod list to build the queue.</div>
            )}
          </div>

          <button className="primary-action" disabled={!queue.length || isDownloading} onClick={runDownload}>
            <Download size={18} />
            Download Mods
          </button>
          <button
            className="secondary-action"
            disabled={!isDownloading}
            onClick={async () => {
              const result = await cancelDownload();
              if (!result.success) setError(result.message);
            }}
          >
            Cancel Download
          </button>

          <section className={`log-section ${steamCmdLogVisible ? "" : "collapsed"}`}>
            <button
              type="button"
              className="log-toggle"
              aria-expanded={steamCmdLogVisible}
              onClick={() => setSteamCmdLogVisible((visible) => !visible)}
            >
              <Terminal size={16} />
              <span>SteamCMD Log</span>
              <small>{downloadEvents.length ? `${downloadEvents.length} event${downloadEvents.length === 1 ? "" : "s"}` : "Idle"}</small>
              {steamCmdLogVisible ? <ChevronDown size={16} /> : <ChevronUp size={16} />}
            </button>
            {steamCmdLogVisible ? (
              <div className="log-panel" aria-live="polite">
                {downloadEvents.length ? (
                  downloadEvents.map((event, index) => <p key={`${event.kind}-${index}`}>{event.message}</p>)
                ) : (
                  <p>SteamCMD output will appear here.</p>
                )}
              </div>
            ) : null}
          </section>
        </aside>
      </section>

      {settingsOpen ? (
        <SettingsModal
          settings={settings}
          onClose={() => setSettingsOpen(false)}
          onSave={async (next) => {
            const state = await saveSettings(next);
            setSettings(state.settings);
            setLocalMods(state.localMods);
            setInstalledIds(state.installedWorkshopIds);
            setSettingsOpen(false);
            setStatus("Settings saved");
          }}
        />
      ) : null}
      {collectionPreview ? (
        <CollectionConfirmModal
          collection={collectionPreview}
          onCancel={() => setCollectionPreview(null)}
          onConfirm={() => confirmCollectionQueue(collectionPreview)}
        />
      ) : null}
    </main>
  );
}

function WorkshopPane(props: {
  settingsOpen: boolean;
  collectionModalOpen: boolean;
  installedIds: string[];
  queue: QueuedMod[];
  items: WorkshopItem[];
  input: string;
  setInput: (input: string) => void;
  onAddManual: () => void;
  onAdd: (item: WorkshopItem) => void;
  onRemove: (publishedfileid: string) => void;
  onImport: (file: File | null) => void;
}) {
  const browserHostRef = useRef<HTMLDivElement>(null);
  const settingsOpenRef = useRef(props.settingsOpen);
  const collectionModalOpenRef = useRef(props.collectionModalOpen);
  const [browserUrl, setBrowserUrl] = useState("");
  const previewItem = parseWorkshopInput(props.input);
  const rows = previewItem ? [previewItem, ...props.items.filter((item) => item.publishedfileid !== previewItem.publishedfileid)] : props.items;
  const getBrowserBounds = useCallback(() => {
    const rect = browserHostRef.current?.getBoundingClientRect();
    if (!rect) return null;
    return {
      x: Math.round(rect.left),
      y: Math.round(rect.top),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    };
  }, []);

  useEffect(() => {
    settingsOpenRef.current = props.settingsOpen;
  }, [props.settingsOpen]);

  useEffect(() => {
    collectionModalOpenRef.current = props.collectionModalOpen;
  }, [props.collectionModalOpen]);

  const syncBrowserBoundsForUrl = useCallback(
    (url: string) => {
      if (!url || settingsOpenRef.current || collectionModalOpenRef.current) return;
      const bounds = getBrowserBounds();
      if (bounds) openWorkshopBrowser(url, bounds).catch(() => undefined);
    },
    [getBrowserBounds],
  );

  const syncBrowserBounds = useCallback(() => {
    syncBrowserBoundsForUrl(browserUrl);
  }, [browserUrl, syncBrowserBoundsForUrl]);

  const openInPane = useCallback(
    async (url: string) => {
      if (settingsOpenRef.current || collectionModalOpenRef.current) return;
      await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
      if (settingsOpenRef.current || collectionModalOpenRef.current) return;
      const bounds = getBrowserBounds();
      if (!bounds) return;
      setBrowserUrl(url);
      await openWorkshopBrowser(url, bounds);
      window.setTimeout(() => syncBrowserBoundsForUrl(url), 0);
      window.setTimeout(() => syncBrowserBoundsForUrl(url), 250);
    },
    [getBrowserBounds, syncBrowserBoundsForUrl],
  );

  useEffect(() => {
    if (!browserUrl || props.settingsOpen || props.collectionModalOpen || !browserHostRef.current) return;
    const observer = new ResizeObserver(syncBrowserBounds);
    observer.observe(browserHostRef.current);
    window.addEventListener("resize", syncBrowserBounds);
    window.visualViewport?.addEventListener("resize", syncBrowserBounds);
    const animationFrame = window.requestAnimationFrame(syncBrowserBounds);
    const syncTimers = [100, 300, 700].map((delay) => window.setTimeout(syncBrowserBounds, delay));
    return () => {
      window.cancelAnimationFrame(animationFrame);
      syncTimers.forEach((timer) => window.clearTimeout(timer));
      observer.disconnect();
      window.removeEventListener("resize", syncBrowserBounds);
      window.visualViewport?.removeEventListener("resize", syncBrowserBounds);
    };
  }, [browserUrl, props.settingsOpen, props.collectionModalOpen, syncBrowserBounds]);

  useEffect(() => {
    if (!props.settingsOpen) return;
    setBrowserUrl("");
    closeWorkshopBrowser()
      .catch(() => undefined);
  }, [props.settingsOpen]);

  useEffect(() => {
    if (!browserUrl) return;
    if (props.collectionModalOpen) {
      closeWorkshopBrowser().catch(() => undefined);
      return;
    }
    syncBrowserBoundsForUrl(browserUrl);
    window.setTimeout(() => syncBrowserBoundsForUrl(browserUrl), 0);
    window.setTimeout(() => syncBrowserBoundsForUrl(browserUrl), 250);
  }, [browserUrl, props.collectionModalOpen, syncBrowserBoundsForUrl]);

  useEffect(() => {
    return () => {
      closeWorkshopBrowser().catch(() => undefined);
    };
  }, []);

  return (
    <div className="workshop-pane">
      <div className="tool-row">
        <div className="search-box">
          <Search size={18} />
          <input
            value={props.input}
            onChange={(event) => props.setInput(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") props.onAddManual();
            }}
            placeholder="Paste Workshop URL or item ID"
          />
        </div>
        <button onClick={props.onAddManual}>
          <HardDriveDownload size={17} />
          Queue
        </button>
        <label className="file-button">
          Import JSON
          <input type="file" accept="application/json,.json" onChange={(event) => props.onImport(event.target.files?.[0] ?? null)} />
        </label>
        <button onClick={() => openInPane(workshopHome)}>
          <ExternalLink size={17} />
          Steam Workshop
        </button>
        {browserUrl ? (
          <button onClick={() => closeWorkshopBrowser().then(() => setBrowserUrl(""))}>
            <X size={17} />
            Close Browser
          </button>
        ) : null}
      </div>

      <div className={`workshop-launcher ${browserUrl ? "browser-active" : ""}`} ref={browserHostRef}>
        <div>
          <div className="launcher-title">
            <Info size={18} />
            <h2>{browserUrl ? "Steam Workshop" : "Workshop Offline"}</h2>
          </div>
        </div>
        <div className="launcher-actions overlay-actions">
          <button onClick={() => openInPane(previewItem?.url ?? workshopHome)}>
            <ExternalLink size={17} />
            {previewItem ? "Open Item" : browserUrl ? "Home" : "Open Workshop"}
          </button>
          {previewItem ? (
            <button onClick={() => props.onAdd(previewItem)}>
              <Download size={17} />
              Queue Preview
            </button>
          ) : null}
        </div>
      </div>

      <div className="workshop-results">
        {rows.length ? (
          rows.map((item) => {
            const installed = props.installedIds.includes(item.publishedfileid);
            const queued = props.queue.some((queuedItem) => queuedItem.publishedfileid === item.publishedfileid);
            return (
              <article key={item.publishedfileid} className="mod-row">
                <div>
                  <h3>{item.title}</h3>
                  <p>{item.url}</p>
                </div>
                <div className="row-actions">
                  <button className="icon-button" title="Open Workshop page" onClick={() => openInPane(item.url)}>
                    <ExternalLink size={17} />
                  </button>
                  <button
                    className={queued ? "danger" : ""}
                    disabled={installed}
                    onClick={() => (queued ? props.onRemove(item.publishedfileid) : props.onAdd(item))}
                  >
                    {installed ? <Check size={17} /> : queued ? <X size={17} /> : <Download size={17} />}
                    {installed ? "Installed" : queued ? "Remove" : "Add"}
                  </button>
                </div>
              </article>
            );
          })
        ) : (
          <div className="empty-state large">Add mods by URL, numeric Workshop ID, or the embedded Steam Workshop page.</div>
        )}
      </div>
    </div>
  );
}

function LocalModsPane(props: {
  mods: LocalMod[];
  selected?: LocalMod;
  onSelect: (path: string) => void;
  onRefresh: () => void;
  onDelete: (path: string) => void;
  onOpenFolder: (path: string) => void;
  onOpenWorkshop: (url: string) => void;
  onExport: () => void;
}) {
  return (
    <div className="local-pane">
      <div className="tool-row">
        <strong>Active [{props.mods.length}]</strong>
        <span className="spacer" />
        <button onClick={props.onExport}>Export Mod List</button>
        <button onClick={props.onRefresh}>
          <RefreshCw size={17} />
          Refresh
        </button>
      </div>

      <div className="local-grid">
        <div className="mods-list">
          {props.mods.length ? (
            props.mods.map((mod) => (
              <button key={mod.path} className={props.selected?.path === mod.path ? "selected" : ""} onClick={() => props.onSelect(mod.path)}>
                {mod.displayName}
                <span>{mod.folderName}</span>
              </button>
            ))
          ) : (
            <div className="empty-state">No local mods found.</div>
          )}
        </div>

        <section className="detail-panel">
          {props.selected ? (
            <>
              <div className="poster-frame">
                {props.selected.posterPath ? (
                  <img src={convertFileSrc(props.selected.posterPath)} alt="" />
                ) : (
                  <span>No poster available</span>
                )}
              </div>
              <h2>{props.selected.displayName}</h2>
              <dl>
                <dt>Package ID</dt>
                <dd>{props.selected.packageId || "Unknown"}</dd>
                <dt>Authors</dt>
                <dd>{props.selected.authors || "Unknown"}</dd>
                <dt>Mod Version</dt>
                <dd>{props.selected.modVersion || "Not specified"}</dd>
                <dt>Supported Version</dt>
                <dd>{props.selected.pzVersion || "Unknown"}</dd>
                <dt>Folder Size</dt>
                <dd>{formatSize(props.selected.sizeBytes)}</dd>
                <dt>Path</dt>
                <dd>{props.selected.path}</dd>
              </dl>
              <div className="detail-actions">
                <button onClick={() => props.onOpenFolder(props.selected!.path)}>
                  <FolderOpen size={17} />
                  Open Folder
                </button>
                <button disabled={!props.selected.workshopUrl} onClick={() => props.onOpenWorkshop(props.selected!.workshopUrl)}>
                  <ExternalLink size={17} />
                  Steam Workshop
                </button>
                <button className="danger" onClick={() => props.onDelete(props.selected!.path)}>
                  <Trash2 size={17} />
                  Delete
                </button>
              </div>
            </>
          ) : (
            <div className="empty-state">Select a mod to view details.</div>
          )}
        </section>
      </div>
    </div>
  );
}

function CollectionConfirmModal(props: {
  collection: WorkshopCollectionPreview;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { collection } = props;
  const skippedCount = collection.queuedCount + collection.installedCount;

  return (
    <div className="modal-backdrop">
      <section className="collection-modal" role="dialog" aria-modal="true" aria-labelledby="collection-title">
        <header>
          <div>
            <h2 id="collection-title">{collection.title}</h2>
            <p>
              {collection.addableCount} new of {collection.totalCount} collection mod{collection.totalCount === 1 ? "" : "s"}
              {skippedCount ? `, ${skippedCount} skipped` : ""}
            </p>
          </div>
          <button type="button" className="icon-button" title="Close" onClick={props.onCancel}>
            <X size={18} />
          </button>
        </header>

        <div className="collection-summary">
          <span>{collection.addableCount} new</span>
          <span>{collection.queuedCount} already queued</span>
          <span>{collection.installedCount} installed</span>
        </div>

        <div className="collection-list">
          {collection.items.map((item) => {
            const skippedReason = item.installed ? "Installed" : item.queued ? "Queued" : "";
            return (
              <article key={item.publishedfileid} className={skippedReason ? "collection-item skipped" : "collection-item"}>
                <div>
                  <h3>{item.title}</h3>
                  <p>ID {item.publishedfileid}</p>
                </div>
                {skippedReason ? <span>{skippedReason}</span> : <span className="ready">New</span>}
              </article>
            );
          })}
        </div>

        <footer>
          <button type="button" className="secondary-action" onClick={props.onCancel}>
            Cancel
          </button>
          <button type="button" className="primary-action" disabled={!collection.addableCount} onClick={props.onConfirm}>
            <Download size={18} />
            Add Mods
          </button>
        </footer>
      </section>
    </div>
  );
}

function SettingsModal(props: {
  settings: AppSettings;
  onClose: () => void;
  onSave: (settings: AppSettings) => void;
}) {
  const [draft, setDraft] = useState(props.settings);

  return (
    <div className="modal-backdrop">
      <form
        className="settings-modal"
        onSubmit={(event) => {
          event.preventDefault();
          props.onSave(draft);
        }}
      >
        <header>
          <h2>Settings</h2>
          <button type="button" className="icon-button" title="Close" onClick={props.onClose}>
            <X size={18} />
          </button>
        </header>
        <label>
          SteamCMD Executable
          <div className="path-row">
            <input value={draft.steamcmdPath} onChange={(event) => setDraft({ ...draft, steamcmdPath: event.target.value })} placeholder="C:\\SteamCMD\\steamcmd.exe" />
            <button
              type="button"
              onClick={async () => {
                const path = await browseSteamcmdPath();
                if (path) setDraft((current) => ({ ...current, steamcmdPath: path }));
              }}
            >
              <FolderOpen size={17} />
              Browse
            </button>
          </div>
        </label>
        <label>
          Zomboid Mod Path
          <div className="path-row">
            <input value={draft.modDownloadPath} onChange={(event) => setDraft({ ...draft, modDownloadPath: event.target.value })} placeholder="C:\\Users\\You\\Zomboid\\mods" />
            <button
              type="button"
              onClick={async () => {
                const path = await browseZomboidModPath();
                if (path) setDraft((current) => ({ ...current, modDownloadPath: path }));
              }}
            >
              <FolderOpen size={17} />
              Browse
            </button>
          </div>
        </label>
        <label className="checkbox-row">
          <input
            type="checkbox"
            checked={draft.useAnonymousLogin}
            onChange={(event) => setDraft({ ...draft, useAnonymousLogin: event.target.checked })}
          />
          Use Anonymous Login
        </label>
        <label>
          Steam Username
          <input
            value={draft.steamUsername}
            disabled={draft.useAnonymousLogin}
            onChange={(event) => setDraft({ ...draft, steamUsername: event.target.value })}
          />
        </label>
        <label className="checkbox-row">
          <input
            type="checkbox"
            checked={draft.autoClearQueue}
            onChange={(event) => setDraft({ ...draft, autoClearQueue: event.target.checked })}
          />
          Automatically clear queue after a successful download
        </label>
        <footer>
          <button type="button" className="secondary-action" onClick={props.onClose}>
            Cancel
          </button>
          <button type="submit" className="primary-action">
            Save Settings
          </button>
        </footer>
      </form>
    </div>
  );
}
