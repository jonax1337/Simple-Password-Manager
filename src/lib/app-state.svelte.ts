import type { GroupData } from "./tauri";

type Phase = "checking" | "unlock" | "quick-unlock" | "main";

/**
 * UI-visible sync state. Distinct from `isDirty` (which tracks in-memory edits
 * relative to the last save) — `syncStatus` tracks our relationship with the
 * on-disk file plus the optional cloud peer.
 *
 *   idle      — saved & no external activity
 *   saving    — local save in flight
 *   merging   — external change detected, pulling it in
 *   conflict  — merge required user attention (DatabaseConflictDialog open)
 *   cloud-sync — pushing to / pulling from a linked cloud account
 */
export type SyncStatus = "idle" | "saving" | "merging" | "conflict" | "cloud-sync";

class AppState {
  phase = $state<Phase>("checking");
  rootGroup = $state<GroupData | null>(null);
  dbPath = $state<string>("");
  initialFilePath = $state<string | null>(null);
  isDirty = $state<boolean>(false);
  refreshCounter = $state<number>(0);
  // Increments on every markDirty() so debounced auto-save can re-arm.
  dirtyVersion = $state<number>(0);

  syncStatus = $state<SyncStatus>("idle");
  // ms-epoch of the last successful sync (save or merge). Null until first save.
  lastSyncedAt = $state<number | null>(null);

  /** Name of the active cloud vault when running in cloud-only mode (no
   * local file). Drives the sidebar label and the window title. Null when
   * we're working with a local kdbx file (dbPath is the source then). */
  cloudVaultName = $state<string | null>(null);
  /** Server-side id of the active cloud vault. Used by the vault switcher
   * to highlight the current entry and skip no-op re-opens. */
  cloudVaultId = $state<string | null>(null);

  setPhase(p: Phase) {
    this.phase = p;
  }
  setRootGroup(g: GroupData | null) {
    this.rootGroup = g;
  }
  setDbPath(p: string) {
    this.dbPath = p;
  }
  markDirty() {
    this.isDirty = true;
    this.dirtyVersion += 1;
  }
  markClean() {
    this.isDirty = false;
  }
  refresh() {
    this.refreshCounter += 1;
  }
  setSyncStatus(s: SyncStatus) {
    this.syncStatus = s;
  }
  setCloudVaultName(name: string | null) {
    this.cloudVaultName = name;
  }
  setCloudVaultId(id: string | null) {
    this.cloudVaultId = id;
  }
  /** Successful save or merge: clear status, stamp the clock. */
  markSynced() {
    this.syncStatus = "idle";
    this.lastSyncedAt = Date.now();
  }
}

export const appState = new AppState();
