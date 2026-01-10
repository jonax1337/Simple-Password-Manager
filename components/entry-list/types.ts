import type { EntryData } from "@/lib/tauri";

export type ColumnId = 'title' | 'username' | 'password' | 'url' | 'notes' | 'created' | 'modified';
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
  { id: 'url', label: 'URL', visible: true, width: 180 },
  { id: 'notes', label: 'Notes', visible: true, width: 200 },
  { id: 'created', label: 'Created', visible: false, width: 160 },
  { id: 'modified', label: 'Modified', visible: false, width: 160 },
];

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
