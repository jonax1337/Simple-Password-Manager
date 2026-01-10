import type { GroupData } from "@/lib/tauri";

export interface GroupTreeProps {
  group: GroupData;
  selectedUuid: string;
  onSelectGroup: (uuid: string) => void;
  onRefresh: () => void;
  onGroupDeleted?: (deletedUuid: string) => void;
  dbPath: string;
  initialExpandedGroups?: Set<string>;
  // DnD props from parent DndContext
  activeId?: string | null;
  overId?: string | null;
  activeType?: 'folder' | 'entry' | null;
  addToHistory?: (action: string, undo: () => Promise<void>, redo: () => Promise<void>) => void;
}

export interface DraggableFolderProps {
  group: GroupData;
  depth: number;
  selectedUuid: string;
  expandedGroups: Set<string>;
  overId: string | null;
  activeType?: 'folder' | 'entry' | null;
  onToggleExpand: (uuid: string) => void;
  onSelectGroup: (uuid: string) => void;
  onOpenCreateDialog: (parentUuid: string) => void;
  onOpenRenameDialog: (group: GroupData) => void;
  onDeleteGroup: (uuid: string) => void;
}

export type { GroupData };
