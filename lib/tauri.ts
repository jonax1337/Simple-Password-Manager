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

export async function createDatabase(path: string, password: string): Promise<GroupData> {
  return invoke("create_database", { path, password });
}

export async function openDatabase(path: string, password: string): Promise<[GroupData, string]> {
  return invoke("open_database", { path, password });
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
