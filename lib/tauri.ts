import { invoke } from "@tauri-apps/api/core";

export interface CustomField {
  name: string;
  value: string;
  protected: boolean;
}

export interface HistoryEntry {
  timestamp: string;
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
}

export interface EntryData {
  uuid: string;
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
  tags: string;
  group_uuid: string;
  icon_id?: number;
  is_favorite: boolean;
  created?: string;
  modified?: string;
  last_accessed?: string;
  expiry_time?: string;
  expires: boolean;
  usage_count: number;
  custom_fields: CustomField[];
  history: HistoryEntry[];
}

export interface GroupData {
  uuid: string;
  name: string;
  parent_uuid: string | null;
  children: GroupData[];
  icon_id?: number;
}

export interface DashboardStats {
  total_entries: number;
  total_groups: number;
  weak_passwords: number;
  reused_passwords: number;
  old_passwords: number;
  expired_entries: number;
  favorite_entries: number;
  average_password_strength: number;
}

export interface BreachedEntry {
  uuid: string;
  title: string;
  username: string;
  breach_count: number;
}

/** A Yubikey HMAC-SHA1 challenge-response factor attached to a database. */
export interface YubikeyConfig {
  serial_number: number;
  slot: string; // "1" or "2"
}

export interface YubikeyInfo {
  serial_number: number;
  name: string | null;
}

export async function createDatabase(
  path: string,
  password: string,
  yubikey: YubikeyConfig | null = null,
): Promise<GroupData> {
  return invoke("create_database", { path, password, yubikey });
}

export async function openDatabase(
  path: string,
  password: string,
  yubikey: YubikeyConfig | null = null,
): Promise<[GroupData, string]> {
  return invoke("open_database", { path, password, yubikey });
}

// --- Yubikey-specific commands ---

export async function listYubikeys(): Promise<YubikeyInfo[]> {
  return invoke("list_yubikeys");
}

export async function enableYubikey(serialNumber: number, slot: string): Promise<void> {
  return invoke("enable_yubikey", { serialNumber, slot });
}

export async function disableYubikey(): Promise<void> {
  return invoke("disable_yubikey");
}

export async function yubikeyEnabledForOpenDb(): Promise<boolean> {
  return invoke("yubikey_enabled_for_open_db");
}

// --- Windows Hello commands ---

export async function helloAvailable(): Promise<boolean> {
  return invoke("hello_available");
}

export async function helloIsEnrolled(dbPath: string): Promise<boolean> {
  return invoke("hello_is_enrolled", { dbPath });
}

export async function helloStore(dbPath: string, password: string): Promise<void> {
  return invoke("hello_store", { dbPath, password });
}

export async function helloRetrieve(dbPath: string): Promise<string> {
  return invoke("hello_retrieve", { dbPath });
}

export async function helloClear(dbPath: string): Promise<void> {
  return invoke("hello_clear", { dbPath });
}

export async function saveDatabase(): Promise<void> {
  return await invoke<void>("save_database");
}

export async function checkDatabaseChanges(): Promise<boolean> {
  return await invoke<boolean>("check_database_changes");
}

export async function mergeDatabase(): Promise<void> {
  return await invoke<void>("merge_database");
}

export async function closeDatabase(): Promise<void> {
  return await invoke<void>("close_database");
}

export async function getGroups(): Promise<GroupData> {
  return await invoke<GroupData>("get_groups");
}

export async function getEntries(groupUuid: string): Promise<EntryData[]> {
  return await invoke<EntryData[]>("get_entries", { groupUuid });
}

export async function getFavoriteEntries(): Promise<EntryData[]> {
  return await invoke<EntryData[]>("get_favorite_entries");
}

export async function getEntry(entryUuid: string): Promise<EntryData> {
  return await invoke<EntryData>("get_entry", { entryUuid });
}

export async function createEntry(entry: EntryData): Promise<void> {
  return await invoke<void>("create_entry", { entry });
}

export async function updateEntry(entry: EntryData): Promise<void> {
  return await invoke<void>("update_entry", { entry });
}

export async function deleteEntry(entryUuid: string): Promise<void> {
  return await invoke<void>("delete_entry", { entryUuid });
}

export async function moveEntry(entryUuid: string, newGroupUuid: string): Promise<void> {
  return await invoke<void>("move_entry", { entryUuid, newGroupUuid });
}


export async function createGroup(
  name: string,
  parentUuid: string | null,
  iconId?: number
): Promise<void> {
  return await invoke<void>("create_group", { name, parentUuid, iconId });
}

export async function renameGroup(
  groupUuid: string,
  newName: string,
  iconId?: number
): Promise<void> {
  return await invoke<void>("rename_group", { groupUuid, newName, iconId });
}

export async function moveGroup(
  groupUuid: string,
  newParentUuid: string
): Promise<void> {
  return await invoke<void>("move_group", { groupUuid, newParentUuid });
}

export async function reorderGroup(
  groupUuid: string,
  targetIndex: number
): Promise<void> {
  return await invoke<void>("reorder_group", { groupUuid, targetIndex });
}

export async function deleteGroup(groupUuid: string): Promise<void> {
  return await invoke<void>("delete_group", { groupUuid });
}

export async function searchEntries(query: string): Promise<EntryData[]> {
  return await invoke<EntryData[]>("search_entries", { query });
}

export async function searchEntriesInGroup(query: string, groupUuid: string): Promise<EntryData[]> {
  return await invoke<EntryData[]>("search_entries_in_group", { query, groupUuid });
}

export async function generatePassword(
  length: number,
  useUppercase: boolean,
  useLowercase: boolean,
  useNumbers: boolean,
  useSymbols: boolean
): Promise<string> {
  return await invoke<string>("generate_password", {
    length,
    useUppercase,
    useLowercase,
    useNumbers,
    useSymbols,
  });
}

export async function getDashboardStats(): Promise<DashboardStats> {
  return await invoke<DashboardStats>("get_dashboard_stats");
}

export async function checkBreachedPasswords(): Promise<BreachedEntry[]> {
  return await invoke<BreachedEntry[]>("check_breached_passwords");
}

export async function openDatabaseInNewInstance(dbPath: string): Promise<void> {
  return await invoke<void>("open_database_in_new_instance", { dbPath });
}

export async function validateDatabaseFile(path: string): Promise<boolean> {
  return await invoke<boolean>("validate_database_file", { path });
}

// Browser extension setup

export interface BrowserInfo {
  id: string;
  label: string;
  kind: "chromium" | "firefox";
}

export interface InstallReport {
  registered: string[];
  failed: { label: string; reason: string }[];
}

export async function detectBrowsers(): Promise<BrowserInfo[]> {
  return await invoke<BrowserInfo[]>("detect_browsers");
}

export async function installNativeHost(extensionId: string): Promise<InstallReport> {
  return await invoke<InstallReport>("install_native_host", { extensionId });
}

export async function uninstallNativeHost(): Promise<string[]> {
  return await invoke<string[]>("uninstall_native_host");
}

// TOTP helpers

export interface TotpPreview {
  code: string;
  period: number;
  remaining_seconds: number;
  algorithm: string;
  otpauth_uri: string;
}

export async function previewTotp(input: string): Promise<TotpPreview> {
  return await invoke<TotpPreview>("preview_totp", { input });
}
