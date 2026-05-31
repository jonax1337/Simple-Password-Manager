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

export interface YubikeyConfig {
  serial_number: number;
  slot: string;
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

// Yubikey

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

// Windows Hello

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

// Cloud-side Hello bundle. Opaque JSON the frontend serializes (cloud
// password + default vault id + that vault's KDBX password) so a single
// Hello prompt drops the user straight into their default vault.
export async function helloCloudIsEnrolled(): Promise<boolean> {
  return invoke("hello_cloud_is_enrolled");
}
export async function helloCloudStore(bundleJson: string): Promise<void> {
  return invoke("hello_cloud_store", { bundleJson });
}
export async function helloCloudRetrieve(): Promise<string> {
  return invoke("hello_cloud_retrieve");
}
export async function helloCloudClear(): Promise<void> {
  return invoke("hello_cloud_clear");
}

export async function saveDatabase(): Promise<void> {
  return invoke("save_database");
}

export async function checkDatabaseChanges(): Promise<boolean> {
  return invoke("check_database_changes");
}

export async function mergeDatabase(): Promise<void> {
  return invoke("merge_database");
}

export type ConflictChoice = "keep_local" | "keep_remote";

export interface EntryConflict {
  uuid: string;
  local: EntryData;
  remote: EntryData;
}

export async function analyzeConflicts(): Promise<EntryConflict[]> {
  return invoke("analyze_conflicts");
}

export async function resolveConflicts(
  decisions: Record<string, ConflictChoice>,
): Promise<void> {
  return invoke("resolve_conflicts", { decisions });
}

export interface LockInfo {
  pid: number;
  host: string;
  acquired_at: number;
}

export async function peekLockStatus(): Promise<LockInfo | null> {
  return invoke("peek_lock_status");
}

/**
 * `save_database` returns the string "LOCK_HELD:<host>:<pid>" when a fresh
 * foreign lock blocked the save. Parses that into structured form, or returns
 * null for any other error.
 */
export function parseLockHeldError(message: string): LockInfo | null {
  if (!message.startsWith("LOCK_HELD:")) return null;
  const rest = message.slice("LOCK_HELD:".length);
  const lastColon = rest.lastIndexOf(":");
  if (lastColon < 0) return null;
  const host = rest.slice(0, lastColon);
  const pid = Number(rest.slice(lastColon + 1));
  if (!Number.isFinite(pid)) return null;
  return { host, pid, acquired_at: 0 };
}

export async function closeDatabase(): Promise<void> {
  return invoke("close_database");
}

export async function getGroups(): Promise<GroupData> {
  return invoke("get_groups");
}

export async function getEntries(groupUuid: string): Promise<EntryData[]> {
  return invoke("get_entries", { groupUuid });
}

export async function getFavoriteEntries(): Promise<EntryData[]> {
  return invoke("get_favorite_entries");
}

export async function getEntry(entryUuid: string): Promise<EntryData> {
  return invoke("get_entry", { entryUuid });
}

export async function createEntry(entry: EntryData): Promise<void> {
  return invoke("create_entry", { entry });
}

export async function updateEntry(entry: EntryData): Promise<void> {
  return invoke("update_entry", { entry });
}

export async function deleteEntry(entryUuid: string): Promise<void> {
  return invoke("delete_entry", { entryUuid });
}

export async function moveEntry(entryUuid: string, newGroupUuid: string): Promise<void> {
  return invoke("move_entry", { entryUuid, newGroupUuid });
}

export async function createGroup(
  name: string,
  parentUuid: string | null,
  iconId?: number,
): Promise<void> {
  return invoke("create_group", { name, parentUuid, iconId });
}

export async function renameGroup(
  groupUuid: string,
  newName: string,
  iconId?: number,
): Promise<void> {
  return invoke("rename_group", { groupUuid, newName, iconId });
}

export async function moveGroup(groupUuid: string, newParentUuid: string): Promise<void> {
  return invoke("move_group", { groupUuid, newParentUuid });
}

export async function reorderGroup(groupUuid: string, targetIndex: number): Promise<void> {
  return invoke("reorder_group", { groupUuid, targetIndex });
}

export async function deleteGroup(groupUuid: string): Promise<void> {
  return invoke("delete_group", { groupUuid });
}

export async function searchEntries(query: string): Promise<EntryData[]> {
  return invoke("search_entries", { query });
}

export async function searchEntriesInGroup(query: string, groupUuid: string): Promise<EntryData[]> {
  return invoke("search_entries_in_group", { query, groupUuid });
}

export async function generatePassword(
  length: number,
  useUppercase: boolean,
  useLowercase: boolean,
  useNumbers: boolean,
  useSymbols: boolean,
): Promise<string> {
  return invoke("generate_password", {
    length,
    useUppercase,
    useLowercase,
    useNumbers,
    useSymbols,
  });
}

export async function getDashboardStats(): Promise<DashboardStats> {
  return invoke("get_dashboard_stats");
}

export async function checkBreachedPasswords(): Promise<BreachedEntry[]> {
  return invoke("check_breached_passwords");
}

export async function openDatabaseInNewInstance(dbPath: string): Promise<void> {
  return invoke("open_database_in_new_instance", { dbPath });
}

export async function validateDatabaseFile(path: string): Promise<boolean> {
  return invoke("validate_database_file", { path });
}

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
  return invoke("detect_browsers");
}

export async function installNativeHost(extensionId: string): Promise<InstallReport> {
  return invoke("install_native_host", { extensionId });
}

export async function uninstallNativeHost(): Promise<string[]> {
  return invoke("uninstall_native_host");
}

export interface TotpPreview {
  code: string;
  period: number;
  remaining_seconds: number;
  algorithm: string;
  otpauth_uri: string;
}

export async function previewTotp(input: string): Promise<TotpPreview> {
  return invoke("preview_totp", { input });
}

export async function getInitialFilePath(): Promise<string | null> {
  return invoke("get_initial_file_path");
}

export async function clearInitialFilePath(): Promise<void> {
  return invoke("clear_initial_file_path");
}

// Cloud sync (multi-vault)

export interface CloudStatus {
  linked: boolean;
  server_url: string | null;
  email: string | null;
  user_id: string | null;
  active_vault_id: string | null;
  active_vault_name: string | null;
  active_vault_role: "owner" | "editor" | "reader" | null;
}

export interface CloudActionResp {
  email: string;
  server_url: string;
}

export interface CloudSignupResp extends CloudActionResp {
  /// Shown to the user once. Lost = no password recovery later.
  recovery_code: string;
  initial_vault_id: string;
}

export interface CloudVaultEntry {
  id: string;
  name: string;
  role: "owner" | "editor" | "reader";
  owner_user_id: string;
  etag: string;
  updated_at: number;
}

export async function cloudStatus(): Promise<CloudStatus> {
  return invoke("cloud_status");
}

export async function cloudSignup(
  serverUrl: string,
  email: string,
  masterPassword: string,
  initialVaultName?: string,
): Promise<CloudSignupResp> {
  return invoke("cloud_signup", {
    serverUrl,
    email,
    masterPassword,
    initialVaultName: initialVaultName ?? null,
  });
}

export async function cloudLogin(
  serverUrl: string,
  email: string,
  masterPassword: string,
): Promise<CloudActionResp> {
  return invoke("cloud_login", { serverUrl, email, masterPassword });
}

export async function cloudRecover(
  serverUrl: string,
  email: string,
  recoveryCode: string,
  newMasterPassword: string,
): Promise<CloudActionResp> {
  return invoke("cloud_recover", {
    serverUrl,
    email,
    recoveryCode,
    newMasterPassword,
  });
}

export async function cloudListVaults(): Promise<CloudVaultEntry[]> {
  return invoke("cloud_list_vaults");
}

/**
 * Open a cloud vault directly into the in-memory database — no file ever
 * lands on disk. `kdbxPassword` is the KeePass-layer master password
 * (separate from the cloud account password).
 */
export async function cloudOpenVault(
  vaultId: string,
  kdbxPassword: string,
): Promise<void> {
  return invoke("cloud_open_vault", { vaultId, kdbxPassword });
}

/**
 * Mint a brand-new empty vault on the server. KDBX is generated in memory
 * with the supplied `kdbxPassword`; no local file ever exists. After
 * success the new vault becomes active and the in-memory DB swaps to it.
 */
export async function cloudCreateVault(
  name: string,
  kdbxPassword: string,
): Promise<{ id: string }> {
  return invoke("cloud_create_vault", { name, kdbxPassword });
}

export async function cloudRenameVault(vaultId: string, name: string): Promise<void> {
  return invoke("cloud_rename_vault", { vaultId, name });
}

export async function cloudDeleteVault(vaultId: string): Promise<void> {
  return invoke("cloud_delete_vault", { vaultId });
}

export async function cloudPush(): Promise<void> {
  return invoke("cloud_push");
}

export async function cloudPull(): Promise<void> {
  return invoke("cloud_pull");
}

export async function cloudDisconnect(): Promise<void> {
  return invoke("cloud_disconnect");
}

export async function cloudLookupUser(
  email: string,
): Promise<{ user_id: string; account_pubkey_b64: string }> {
  return invoke("cloud_lookup_user", { email });
}

export async function cloudShareVault(
  vaultId: string,
  recipientEmail: string,
  role: "editor" | "reader",
): Promise<void> {
  return invoke("cloud_share_vault", { vaultId, recipientEmail, role });
}

export async function cloudUnshareVault(
  vaultId: string,
  userId: string,
): Promise<void> {
  return invoke("cloud_unshare_vault", { vaultId, userId });
}

export interface CloudMemberRow {
  user_id: string;
  email: string;
  role: "owner" | "editor" | "reader";
  invited_at: number;
  accepted_at: number | null;
}

export async function cloudListMembers(vaultId: string): Promise<CloudMemberRow[]> {
  return invoke("cloud_list_members", { vaultId });
}

export async function cloudUpdateMemberRole(
  vaultId: string,
  userId: string,
  role: "editor" | "reader" | "owner",
): Promise<void> {
  return invoke("cloud_update_member_role", { vaultId, userId, role });
}

// Persistence
export async function cloudPersistSession(): Promise<boolean> {
  return invoke("cloud_persist_session");
}
export interface CloudRehydrateResp {
  linked: boolean;
  server_url: string | null;
  email: string | null;
  last_vault_id: string | null;
}

export async function cloudRehydrateSession(): Promise<CloudRehydrateResp> {
  return invoke("cloud_rehydrate_session");
}
export async function cloudForgetSession(): Promise<void> {
  return invoke("cloud_forget_session");
}
