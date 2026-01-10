"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Plus } from "lucide-react";
import { getEntries, createEntry, deleteEntry } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";
import type { EntryData } from "@/lib/tauri";
import { ask } from "@tauri-apps/plugin-dialog";
import { openEntryWindow } from "@/lib/window";
import { open } from "@tauri-apps/plugin-shell";

import { useColumnConfig, useSortConfig } from "./hooks/useColumnConfig";
import { useEntrySelection } from "./hooks/useEntrySelection";
import { useClipboard } from "./hooks/useClipboard";
import { SelectionToolbar } from "./SelectionToolbar";
import { EntryListHeader } from "./EntryListHeader";
import { ColumnHeader } from "./ColumnHeader";
import { EntryListItem } from "./EntryListItem";
import { CreateEntryDialog } from "./CreateEntryDialog";
import { TimestampBar } from "./TimestampBar";
import type { EntryListProps, ColumnId } from "./types";

export function EntryList({
  groupUuid,
  searchResults,
  selectedEntry,
  onSelectEntry,
  onRefresh,
  onSearchRefresh,
  isSearching = false,
  hasActiveSearch = false,
  isFavoritesView = false,
  rootGroupUuid,
  selectedGroupName,
  databasePath,
  addToHistory,
}: EntryListProps) {
  const [entries, setEntries] = useState<EntryData[]>([]);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newEntryTitle, setNewEntryTitle] = useState("");
  const [newEntryIconId, setNewEntryIconId] = useState(0);
  const [contextMenuEntryUuid, setContextMenuEntryUuid] = useState<string | null>(null);
  const [hoveredEntry, setHoveredEntry] = useState<EntryData | null>(null);
  const { toast } = useToast();
  const [scrollContainer, setScrollContainer] = useState<HTMLDivElement | null>(null);
  const scrollRef = useCallback((node: HTMLDivElement | null) => {
    setScrollContainer(node);
  }, []);

  useEffect(() => {
    if (!scrollContainer) return;

    const handleScroll = (e: Event) => {
      const target = e.target as HTMLElement | null;
      if (!target) return;

      const scrollLeft = target.scrollLeft;
      const allScrollables = scrollContainer.querySelectorAll(".row-scroll");
      allScrollables.forEach((el) => {
        if (el !== target) {
          (el as HTMLElement).scrollLeft = scrollLeft;
        }
      });
    };

    const scrollables = scrollContainer.querySelectorAll(".row-scroll");
    scrollables.forEach((el) => {
      el.addEventListener("scroll", handleScroll);
    });

    return () => {
      scrollables.forEach((el) => {
        el.removeEventListener("scroll", handleScroll);
      });
    };
  }, [scrollContainer, entries.length]);

  // Custom hooks
  const { columns, visibleColumns, toggleColumn, updateColumnWidth } = useColumnConfig(databasePath);
  const { currentSort, handleSort } = useSortConfig(groupUuid);
  const { 
    selectedEntries, 
    toggleSelectEntry, 
    toggleSelectAll, 
    clearSelection, 
    isSelected, 
    selectionCount, 
    isAllSelected 
  } = useEntrySelection(groupUuid, isSearching, entries);
  const { handleCopyField } = useClipboard();

  // Sort entries based on current sort config
  const sortedEntries = [...entries].sort((a, b) => {
    const { column, direction } = currentSort;
    let aVal: string | number = '';
    let bVal: string | number = '';

    switch (column) {
      case 'title':
        aVal = a.title.toLowerCase();
        bVal = b.title.toLowerCase();
        break;
      case 'username':
        aVal = a.username.toLowerCase();
        bVal = b.username.toLowerCase();
        break;
      case 'password':
        aVal = a.password ? 1 : 0;
        bVal = b.password ? 1 : 0;
        break;
      case 'url':
        aVal = a.url.toLowerCase();
        bVal = b.url.toLowerCase();
        break;
      case 'notes':
        aVal = a.notes.toLowerCase();
        bVal = b.notes.toLowerCase();
        break;
      case 'created':
        aVal = a.created || '';
        bVal = b.created || '';
        break;
      case 'modified':
        aVal = a.modified || '';
        bVal = b.modified || '';
        break;
    }

    if (aVal < bVal) return direction === 'asc' ? -1 : 1;
    if (aVal > bVal) return direction === 'asc' ? 1 : -1;
    return 0;
  });

  const loadEntries = useCallback(async () => {
    if (!groupUuid) return;

    try {
      const fetchedEntries = await getEntries(groupUuid);
      setEntries(fetchedEntries);
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to load entries"),
        variant: "destructive",
      });
    }
  }, [groupUuid, toast]);

  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    if (groupUuid && !isSearching) {
      loadEntries();
    }
  }, [groupUuid, isSearching, loadEntries]);

  useEffect(() => {
    if (isSearching) {
      setEntries(searchResults);
    } else if (groupUuid) {
      loadEntries();
    }
  }, [searchResults, isSearching, groupUuid, loadEntries]);
  /* eslint-enable react-hooks/set-state-in-effect */

  const handleCreateEntry = async () => {
    // In Favorites view, use rootGroupUuid; otherwise use groupUuid
    const targetGroupUuid = isFavoritesView ? rootGroupUuid : groupUuid;
    
    if (!newEntryTitle.trim() || !targetGroupUuid) return;

    try {
      // Generate UUID for the new entry
      const entryUuid = crypto.randomUUID();
      
      const newEntry: EntryData = {
        uuid: entryUuid,
        title: newEntryTitle,
        username: "",
        password: "",
        url: "",
        notes: "",
        tags: "",
        group_uuid: targetGroupUuid,
        icon_id: newEntryIconId,
        is_favorite: isFavoritesView, // Auto-favorite if created in Favorites view
        expires: false,
        usage_count: 0,
        custom_fields: [],
        history: [],
      };

      await createEntry(newEntry);
      
      // Track for undo/redo
      if (addToHistory) {
        const entryDataCapture = { ...newEntry };
        addToHistory(
          `Create entry "${newEntry.title}"`,
          async () => {
            await deleteEntry(entryDataCapture.uuid);
            onRefresh();
          },
          async () => {
            await createEntry(entryDataCapture);
            onRefresh();
          }
        );
      }
      
      toast({
        title: "Success",
        description: isFavoritesView 
          ? "Favorite entry created successfully"
          : "Entry created successfully",
        variant: "success",
      });
      setNewEntryTitle("");
      setNewEntryIconId(0);
      setShowCreateDialog(false);
      onRefresh();
      
      // Wait a moment for database write to complete before opening window
      await new Promise(resolve => setTimeout(resolve, 100));
      
      // Open the entry editor window for the newly created entry
      await openEntryWindow(newEntry, targetGroupUuid);
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to create entry"),
        variant: "destructive",
      });
    }
  };

  const handleDuplicateEntry = async (entry: EntryData) => {
    try {
      const duplicatedEntry: EntryData = {
        ...entry,
        uuid: crypto.randomUUID(),
        title: `${entry.title} (Copy)`,
        created: undefined,
        modified: undefined,
        last_accessed: undefined,
        usage_count: 0,
        history: [],
      };

      await createEntry(duplicatedEntry);
      
      toast({
        title: "Success",
        description: "Entry duplicated successfully",
        variant: "success",
      });
      
      onRefresh();
      
      // Refresh search results if we're in search mode
      if (isSearching && onSearchRefresh) {
        onSearchRefresh();
      }
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to duplicate entry"),
        variant: "destructive",
      });
    }
  };

  const handleDeleteEntry = async (entry: EntryData) => {
    const shouldDelete = await ask(
      `Are you sure you want to delete "${entry.title}" and all its data?`,
      { kind: "warning", title: "Delete Entry" }
    );
    
    if (!shouldDelete) return;

    try {
      // Capture full entry data before deletion
      const entryDataCapture = { ...entry };
      
      await deleteEntry(entry.uuid);
      
      // Track for undo/redo
      if (addToHistory) {
        addToHistory(
          `Delete entry "${entryDataCapture.title}"`,
          async () => {
            await createEntry(entryDataCapture);
            onRefresh();
          },
          async () => {
            await deleteEntry(entryDataCapture.uuid);
            onRefresh();
          }
        );
      }
      
      toast({
        title: "Success",
        description: "Entry deleted successfully",
        variant: "success",
      });
      onRefresh();
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to delete entry"),
        variant: "destructive",
      });
    }
  };

  const handleBulkDelete = async () => {
    const count = selectionCount;
    const shouldDelete = await ask(
      `Are you sure you want to delete ${count} ${count === 1 ? 'entry' : 'entries'}?`,
      { kind: "warning", title: "Delete Entries" }
    );
    
    if (!shouldDelete) return;

    try {
      // Delete all selected entries
      await Promise.all(
        Array.from(selectedEntries).map(uuid => deleteEntry(uuid))
      );
      
      toast({
        title: "Success",
        description: `${count} ${count === 1 ? 'entry' : 'entries'} deleted successfully`,
        variant: "success",
      });
      
      clearSelection();
      onRefresh();
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to delete entries"),
        variant: "destructive",
      });
    }
  };

  const handleOpenUrl = async (url: string) => {
    if (!url) return;
    
    try {
      // Add https:// if no protocol specified
      const fullUrl = url.match(/^https?:\/\//) ? url : `https://${url}`;
      await open(fullUrl);
    } catch (error: any) {
      toast({
        title: "Error",
        description: "Failed to open URL",
        variant: "destructive",
      });
    }
  };

  const formatTimestamp = (timestamp?: string) => {
    if (!timestamp) return "—";
    try {
      const date = new Date(timestamp);
      return new Intl.DateTimeFormat('de-DE', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      }).format(date);
    } catch {
      return "—";
    }
  };

  // Determine whether to show context menu
  const shouldShowContextMenu = useMemo(() => {
    return !hasActiveSearch && (isFavoritesView || groupUuid);
  }, [hasActiveSearch, isFavoritesView, groupUuid]);

  // Common ScrollArea content
  const scrollAreaContent = useMemo(() => (
    <ScrollArea className="flex-1">
      {entries.length > 0 ? (
        <div ref={scrollRef} className="min-h-full">
          {/* Column Header */}
          <ColumnHeader
            columns={columns}
            visibleColumns={visibleColumns}
            currentSort={currentSort}
            isAllSelected={isAllSelected}
            entries={sortedEntries}
            onToggleSelectAll={toggleSelectAll}
            onSort={handleSort}
            onToggleColumn={toggleColumn}
            onColumnResize={updateColumnWidth}
          />
          
          {/* Entries */}
          {sortedEntries.map((entry) => (
            <div
              key={entry.uuid}
              onMouseEnter={() => setHoveredEntry(entry)}
              onMouseLeave={() => setHoveredEntry(null)}
            >
              <EntryListItem
                entry={entry}
                visibleColumns={visibleColumns}
                isSelected={selectedEntry?.uuid === entry.uuid}
                isContextMenuOpen={contextMenuEntryUuid === entry.uuid}
                isChecked={isSelected(entry.uuid)}
                onSelect={() => onSelectEntry(entry)}
                onToggleCheck={() => toggleSelectEntry(entry.uuid)}
                onContextMenuChange={(open) => setContextMenuEntryUuid(open ? entry.uuid : null)}
                onCopyField={handleCopyField}
                onOpenUrl={handleOpenUrl}
                onDuplicate={() => handleDuplicateEntry(entry)}
                onDelete={() => handleDeleteEntry(entry)}
                onRefresh={onRefresh}
                formatTimestamp={formatTimestamp}
              />
            </div>
          ))}
        </div>
      ) : (
        <div className="flex h-32 items-center justify-center text-sm text-muted-foreground">
          {isSearching
            ? "No results found"
            : groupUuid
            ? "No entries in this group"
            : "Select a group to view entries"}
        </div>
      )}
    </ScrollArea>
  ), [
    entries.length,
    scrollRef,
    columns,
    visibleColumns,
    currentSort,
    isAllSelected,
    sortedEntries,
    toggleSelectAll,
    handleSort,
    toggleColumn,
    updateColumnWidth,
    selectedEntry?.uuid,
    contextMenuEntryUuid,
    isSelected,
    onSelectEntry,
    toggleSelectEntry,
    handleCopyField,
    handleOpenUrl,
    handleDuplicateEntry,
    handleDeleteEntry,
    onRefresh,
    formatTimestamp,
    isSearching,
    groupUuid,
    setHoveredEntry,
  ]);

  return (
    <>
      <div className="flex h-full flex-col">
        {/* Selection Toolbar */}
        {selectionCount > 0 && (
          <SelectionToolbar
            selectionCount={selectionCount}
            onClearSelection={clearSelection}
            onBulkDelete={handleBulkDelete}
          />
        )}

        {/* Normal Header */}
        {selectionCount === 0 && (
          <EntryListHeader
            isSearching={isSearching}
            selectedGroupName={selectedGroupName}
            entryCount={entries.length}
            groupUuid={groupUuid}
            onCreateClick={() => setShowCreateDialog(true)}
          />
        )}

        {shouldShowContextMenu ? (
          <ContextMenu>
            <ContextMenuTrigger asChild>
              {scrollAreaContent}
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem onClick={() => setShowCreateDialog(true)}>
                <Plus className="mr-2 h-4 w-4" />
                {isFavoritesView ? "New Favorite Entry" : "New Entry"}
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        ) : scrollAreaContent}

        {/* Timestamp Bar */}
        <TimestampBar 
          hoveredEntry={hoveredEntry} 
          formatTimestamp={formatTimestamp} 
        />
      </div>

      <CreateEntryDialog
        open={showCreateDialog}
        onOpenChange={setShowCreateDialog}
        title={newEntryTitle}
        iconId={newEntryIconId}
        onTitleChange={setNewEntryTitle}
        onIconChange={setNewEntryIconId}
        onSubmit={handleCreateEntry}
      />
    </>
  );
}
