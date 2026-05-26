import { invoke } from "@tauri-apps/api/core";
import { validateDatabaseFile } from "./tauri";

const LAST_DATABASE_KEY = "lastDatabasePath";
const RECENT_DATABASES_KEY = "recentDatabases";
const COLUMN_CONFIG_PREFIX = "columnConfig_";
const COLUMN_WIDTHS_PREFIX = "columnWidths_";
const HIBP_ENABLED_KEY = "hibpEnabled";
const SEARCH_SCOPE_PREFIX = "searchScope_";
const LIVE_UPDATES_PREFIX = "liveUpdates_";
const CLOSE_TO_TRAY_KEY_LEGACY = "closeToTray";
const CLOSE_TO_TRAY_KEY = "closeToTray_v2";

export function saveLastDatabasePath(path: string): void {
  if (typeof window !== "undefined") {
    localStorage.setItem(LAST_DATABASE_KEY, path);
  }
}

export function getLastDatabasePath(): string | null {
  if (typeof window !== "undefined") {
    return localStorage.getItem(LAST_DATABASE_KEY);
  }
  return null;
}

export function clearLastDatabasePath(): void {
  if (typeof window !== "undefined") {
    localStorage.removeItem(LAST_DATABASE_KEY);
  }
}

export function addRecentDatabase(path: string): void {
  if (typeof window !== "undefined") {
    const recent = getRecentDatabases();
    const filtered = recent.filter(p => p !== path);
    const updated = [path, ...filtered].slice(0, 10);
    localStorage.setItem(RECENT_DATABASES_KEY, JSON.stringify(updated));
  }
}

export function getRecentDatabases(): string[] {
  if (typeof window !== "undefined") {
    const stored = localStorage.getItem(RECENT_DATABASES_KEY);
    if (stored) {
      try {
        return JSON.parse(stored);
      } catch {
        return [];
      }
    }
  }
  return [];
}

/**
 * Validates all recent database paths and returns only valid ones.
 * This function checks if each path exists and is a valid KDBX file.
 * Invalid paths are automatically removed from localStorage.
 * 
 * Note: Validates paths concurrently for better performance. Since the recent 
 * databases list is limited to 10 items (see addRecentDatabase), this should 
 * not cause file system issues.
 */
export async function getValidatedRecentDatabases(): Promise<string[]> {
  const recent = getRecentDatabases();
  if (recent.length === 0) {
    return [];
  }

  // Validate each path concurrently (limited to 10 max by addRecentDatabase)
  // Note: File system operations are fast (checking existence + reading 8 bytes),
  // and Tauri invoke calls have built-in timeouts, so explicit timeout handling
  // is not necessary here.
  const validationResults = await Promise.all(
    recent.map(async (path) => {
      try {
        const isValid = await validateDatabaseFile(path);
        return { path, isValid };
      } catch (error) {
        // If validation fails (e.g., file system error), consider it invalid
        console.warn(`Failed to validate database path: ${path}`, error);
        return { path, isValid: false };
      }
    })
  );

  // Filter to only valid paths
  const validPaths = validationResults
    .filter(result => result.isValid)
    .map(result => result.path);

  // If some paths were invalid, update localStorage to remove them
  if (validPaths.length !== recent.length && typeof window !== "undefined") {
    localStorage.setItem(RECENT_DATABASES_KEY, JSON.stringify(validPaths));
  }

  return validPaths;
}

export function clearRecentDatabase(path: string): void {
  if (typeof window !== "undefined") {
    const recent = getRecentDatabases();
    const filtered = recent.filter(p => p !== path);
    localStorage.setItem(RECENT_DATABASES_KEY, JSON.stringify(filtered));
  }
}

// Column configuration per database
export interface ColumnVisibility {
  title: boolean;
  username: boolean;
  password: boolean;
  url: boolean;
  notes: boolean;
  totp?: boolean;
  created: boolean;
  modified: boolean;
}

export interface ColumnWidths {
  title: number;
  username: number;
  password: number;
  url: number;
  notes: number;
  totp?: number;
  created: number;
  modified: number;
}

function getDatabaseKey(dbPath: string): string {
  // Create a simple hash from the path for the storage key
  return COLUMN_CONFIG_PREFIX + btoa(dbPath).replace(/[^a-zA-Z0-9]/g, '').slice(0, 32);
}

export function saveColumnConfig(dbPath: string, config: ColumnVisibility): void {
  if (typeof window !== "undefined" && dbPath) {
    localStorage.setItem(getDatabaseKey(dbPath), JSON.stringify(config));
  }
}

export function getColumnConfig(dbPath: string): ColumnVisibility | null {
  if (typeof window !== "undefined" && dbPath) {
    const stored = localStorage.getItem(getDatabaseKey(dbPath));
    if (stored) {
      try {
        return JSON.parse(stored) as ColumnVisibility;
      } catch {
        return null;
      }
    }
  }
  return null;
}

function getColumnWidthsKey(dbPath: string): string {
  return COLUMN_WIDTHS_PREFIX + btoa(dbPath).replace(/[^a-zA-Z0-9]/g, '').slice(0, 32);
}

export function saveColumnWidths(dbPath: string, widths: ColumnWidths): void {
  if (typeof window !== "undefined" && dbPath) {
    localStorage.setItem(getColumnWidthsKey(dbPath), JSON.stringify(widths));
  }
}

export function getColumnWidths(dbPath: string): ColumnWidths | null {
  if (typeof window !== "undefined" && dbPath) {
    const stored = localStorage.getItem(getColumnWidthsKey(dbPath));
    if (stored) {
      try {
        return JSON.parse(stored) as ColumnWidths;
      } catch {
        return null;
      }
    }
  }
  return null;
}

// Dismissed breach management using secure Tauri backend storage
export async function saveDismissedBreach(dbPath: string, entryUuid: string): Promise<void> {
  if (!dbPath) {
    throw new Error("Database path is required to save dismissed breach");
  }
  if (!entryUuid) {
    throw new Error("Entry UUID is required to save dismissed breach");
  }
  
  try {
    await invoke("save_dismissed_breach", { dbPath, entryUuid });
  } catch (error) {
    // Log detailed error for debugging (only visible in dev console, not exposed to user)
    console.error("[Storage] Failed to save dismissed breach", { error });
    // Throw generic error without sensitive details
    throw new Error("Failed to save dismissed breach");
  }
}

export async function getDismissedBreaches(dbPath: string): Promise<string[]> {
  if (!dbPath) {
    console.warn("[Storage] Database path missing, returning empty dismissed breaches array");
    return [];
  }
  
  try {
    return await invoke<string[]>("get_dismissed_breaches", { dbPath });
  } catch (error) {
    // Log detailed error for debugging without exposing sensitive data
    console.error("[Storage] Failed to get dismissed breaches", { error });
    // Return empty array as fallback to prevent UI breakage
    return [];
  }
}

export async function clearDismissedBreach(dbPath: string, entryUuid: string): Promise<void> {
  if (!dbPath) {
    throw new Error("Database path is required to clear dismissed breach");
  }
  if (!entryUuid) {
    throw new Error("Entry UUID is required to clear dismissed breach");
  }
  
  try {
    await invoke("clear_dismissed_breach", { dbPath, entryUuid });
  } catch (error) {
    // Log detailed error for debugging (only visible in dev console, not exposed to user)
    console.error("[Storage] Failed to clear dismissed breach", { error });
    // Throw generic error without sensitive details
    throw new Error("Failed to clear dismissed breach");
  }
}

// Close-to-Tray setting.
// Default is `true` for new users. Existing users (who used the legacy
// `closeToTray` key) keep their previous choice — read once, migrate to v2,
// and the legacy key is left intact for forensic clarity.
export function getCloseToTray(): boolean {
  if (typeof window === "undefined") {
    return true;
  }
  const v2 = localStorage.getItem(CLOSE_TO_TRAY_KEY);
  if (v2 !== null) {
    return v2 === "true";
  }
  const legacy = localStorage.getItem(CLOSE_TO_TRAY_KEY_LEGACY);
  if (legacy !== null) {
    const migrated = legacy === "true";
    localStorage.setItem(CLOSE_TO_TRAY_KEY, migrated.toString());
    return migrated;
  }
  localStorage.setItem(CLOSE_TO_TRAY_KEY, "true");
  return true;
}

export function setCloseToTray(enabled: boolean): void {
  if (typeof window !== "undefined") {
    localStorage.setItem(CLOSE_TO_TRAY_KEY, enabled.toString());
  }
}

// HIBP (Have I Been Pwned) breach checking - disabled by default for privacy
export function setHibpEnabled(enabled: boolean): void {
  if (typeof window !== "undefined") {
    localStorage.setItem(HIBP_ENABLED_KEY, enabled.toString());
  }
}

export function getHibpEnabled(): boolean {
  if (typeof window !== "undefined") {
    return localStorage.getItem(HIBP_ENABLED_KEY) === "true";
  }
  return false;
}

// Search scope management (global or folder-specific)
export type SearchScope = 'global' | 'folder';

function getSearchScopeKey(dbPath: string): string {
  return SEARCH_SCOPE_PREFIX + btoa(dbPath).replace(/[^a-zA-Z0-9]/g, '').slice(0, 32);
}

export function saveSearchScope(dbPath: string, scope: SearchScope): void {
  if (typeof window !== "undefined" && dbPath) {
    localStorage.setItem(getSearchScopeKey(dbPath), scope);
  }
}

export function getSearchScope(dbPath: string): SearchScope {
  if (typeof window !== "undefined" && dbPath) {
    const stored = localStorage.getItem(getSearchScopeKey(dbPath));
    if (stored === 'folder') {
      return 'folder';
    }
    if (stored === 'global') {
      return 'global';
    }
    if (stored !== null) {
      // Invalid value in localStorage; reset to safe default
      // Avoid logging sensitive data such as full paths or database contents
      console.warn("Invalid search scope value in localStorage, resetting to 'global'.");
    }
  }
  return 'global';
}

// Yubikey-per-database state. Stored alongside the user's other per-DB
// preferences (search scope, live updates, column config etc.) so the
// unlock screen can decide whether to prompt for the key without first
// trying to open the file. We deliberately don't persist the secret —
// just the serial and slot, so the user can plug in the right key.
const YUBIKEY_PREFIX = "yubikey_";

export interface YubikeyHint {
  serial_number: number;
  slot: string;
}

function getYubikeyKey(dbPath: string): string {
  return YUBIKEY_PREFIX + btoa(dbPath).replace(/[^a-zA-Z0-9]/g, "").slice(0, 32);
}

export function setYubikeyHint(dbPath: string, hint: YubikeyHint | null): void {
  if (typeof window === "undefined" || !dbPath) return;
  const key = getYubikeyKey(dbPath);
  if (hint === null) {
    localStorage.removeItem(key);
  } else {
    localStorage.setItem(key, JSON.stringify(hint));
  }
}

export function getYubikeyHint(dbPath: string): YubikeyHint | null {
  if (typeof window === "undefined" || !dbPath) return null;
  const stored = localStorage.getItem(getYubikeyKey(dbPath));
  if (!stored) return null;
  try {
    const parsed = JSON.parse(stored) as YubikeyHint;
    if (typeof parsed.serial_number === "number" && typeof parsed.slot === "string") {
      return parsed;
    }
    return null;
  } catch {
    return null;
  }
}

// Live Updates management (automatic merging per database)
function getLiveUpdatesKey(dbPath: string): string {
  return LIVE_UPDATES_PREFIX + btoa(dbPath).replace(/[^a-zA-Z0-9]/g, '').slice(0, 32);
}

export function setLiveUpdates(dbPath: string, enabled: boolean): void {
  if (typeof window !== "undefined" && dbPath) {
    localStorage.setItem(getLiveUpdatesKey(dbPath), enabled.toString());
  }
}

export function getLiveUpdates(dbPath: string): boolean {
  if (typeof window !== "undefined" && dbPath) {
    const stored = localStorage.getItem(getLiveUpdatesKey(dbPath));
    return stored === "true";
  }
  return false;
}
