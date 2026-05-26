import type { EntryData } from "@/lib/tauri";

export type ColumnId = 'title' | 'username' | 'password' | 'url' | 'notes' | 'totp' | 'created' | 'modified';
export type SortDirection = 'asc' | 'desc';

export interface ColumnConfig {
  id: ColumnId;
  label: string;
  visible: boolean;
  width: number;
}

export interface SortConfig {
  column: ColumnId;
  direction: SortDirection;
}

export const DEFAULT_COLUMNS: ColumnConfig[] = [
  { id: 'title', label: 'Title', visible: true, width: 200 },
  { id: 'username', label: 'Username', visible: true, width: 150 },
  { id: 'password', label: 'Password', visible: true, width: 120 },
  { id: 'totp', label: '2FA', visible: true, width: 130 },
  { id: 'url', label: 'URL', visible: true, width: 180 },
  { id: 'notes', label: 'Notes', visible: true, width: 200 },
  { id: 'created', label: 'Created', visible: false, width: 160 },
  { id: 'modified', label: 'Modified', visible: false, width: 160 },
];

export function entryHasTotp(entry: { custom_fields?: { name: string }[] }): boolean {
  return (entry.custom_fields ?? []).some(
    (f) => f.name.toLowerCase() === "otp",
  );
}

export function getTotpFieldValue(entry: {
  custom_fields?: { name: string; value: string }[];
}): string | null {
  const f = (entry.custom_fields ?? []).find(
    (f) => f.name.toLowerCase() === "otp",
  );
  return f?.value ?? null;
}

export interface EntryListProps {
  groupUuid: string;
  searchResults: EntryData[];
  selectedEntry: EntryData | null;
  onSelectEntry: (entry: EntryData) => void;
  onRefresh: () => void;
  onSearchRefresh?: () => void;
  isSearching?: boolean;
  hasActiveSearch?: boolean;
  isFavoritesView?: boolean;
  rootGroupUuid?: string;
  selectedGroupName?: string;
  databasePath?: string;
  addToHistory?: (action: string, undo: () => Promise<void>, redo: () => Promise<void>) => void;
}

export type { EntryData };
