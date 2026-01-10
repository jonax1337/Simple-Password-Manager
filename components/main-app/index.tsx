"use client";

import { useState, useEffect, useCallback } from "react";
import { motion } from "framer-motion";
import { saveDatabase, closeDatabase, getGroups, getFavoriteEntries, moveEntry, checkDatabaseChanges, mergeDatabase } from "@/lib/tauri";
import { GroupTree } from "@/components/group-tree";
import { EntryList } from "@/components/entry-list";
import { UnsavedChangesDialog } from "@/components/UnsavedChangesDialog";
import { DatabaseConflictDialog } from "@/components/DatabaseConflictDialog";
import { Dashboard } from "@/components/Dashboard";
import { openEntryWindow, requestCloseAllChildWindows, openAboutWindow } from "@/lib/window";
import { useToast } from "@/components/ui/use-toast";
import type { GroupData, EntryData } from "@/lib/tauri";
import { loadGroupTreeState } from "@/lib/group-state";
import { ResizablePanel } from "@/components/ResizablePanel";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getSearchScope, saveSearchScope, getLiveUpdates } from "@/lib/storage";
import {
  DndContext,
  DragOverlay,
  useSensor,
  useSensors,
  PointerSensor,
  DragStartEvent,
  DragOverEvent,
  DragEndEvent,
  pointerWithin,
  rectIntersection,
  CollisionDetection,
} from "@dnd-kit/core";
import { getIconComponent } from "@/components/IconPicker";
import { moveGroup } from "@/lib/tauri";
import { findGroupByUuid, findParentGroup, isDescendant } from "@/components/group-tree/utils";

import { CustomTitleBar } from "@/components/CustomTitleBar";
import { SearchHeader } from "@/components/SearchHeader";
import { CreateDatabaseDialog } from "@/components/CreateDatabaseDialog";
import { useAutoLock } from "./hooks/useAutoLock";
import { useWindowManagement } from "./hooks/useWindowManagement";
import { useSearch } from "./hooks/useSearch";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useEntryEvents } from "./hooks/useEntryEvents";
import { useUndoRedo } from "./hooks/useUndoRedo";
import { listen } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { addRecentDatabase } from "@/lib/storage";

interface MainAppProps {
  onClose: (isManualLogout?: boolean) => void;
}

type DragData = 
  | { type: 'entry'; entry: EntryData }
  | { type: 'folder' }
  | null;

// Type guard to safely validate drag data
function isDragData(data: unknown): data is DragData {
  if (data === null) return true;
  if (typeof data !== 'object' || data === null) return false;
  
  if (!('type' in data)) return false;
  
  const obj = data as { type: unknown };
  
  if (obj.type === 'entry') {
    return 'entry' in data && typeof (data as any).entry === 'object' && (data as any).entry !== null;
  }
  if (obj.type === 'folder') {
    return true;
  }
  
  return false;
}

export function MainApp({ onClose }: MainAppProps) {
  const [rootGroup, setRootGroup] = useState<GroupData | null>(null);
  const [selectedGroupUuid, setSelectedGroupUuid] = useState<string>("");
  const [favoriteEntries, setFavoriteEntries] = useState<EntryData[]>([]);
  const [isFavoritesView, setIsFavoritesView] = useState(false);
  const [isDashboardView, setIsDashboardView] = useState(true);
  const [refreshTrigger, setRefreshTrigger] = useState(0);
  const [isDirty, setIsDirty] = useState(false);
  const [dbPath, setDbPath] = useState<string>("");
  const [showUnsavedDialog, setShowUnsavedDialog] = useState(false);
  const [closeAction, setCloseAction] = useState<'logout' | 'window' | null>(null);
  const [initialExpandedGroups, setInitialExpandedGroups] = useState<Set<string> | undefined>(undefined);
  const [dndActiveId, setDndActiveId] = useState<string | null>(null);
  const [dndOverId, setDndOverId] = useState<string | null>(null);
  const [dndActiveType, setDndActiveType] = useState<'folder' | 'entry' | null>(null);
  const [dndActiveData, setDndActiveData] = useState<DragData>(null);
  const [showConflictDialog, setShowConflictDialog] = useState(false);
  const [liveUpdatesEnabled, setLiveUpdatesEnabled] = useState(false);
  const [isSearchVisible, setIsSearchVisible] = useState(false);
  const [selectedEntryForCopy, setSelectedEntryForCopy] = useState<EntryData | null>(null);
  const [passwordsVisible, setPasswordsVisible] = useState(false);
  const [showCreateDatabaseDialog, setShowCreateDatabaseDialog] = useState(false);
  const { toast } = useToast();
  const { addToHistory, undo, redo, canUndo, canRedo } = useUndoRedo();

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );

  // Custom collision detection: dynamically switches between pointerWithin (strict, for entries) and rectIntersection (lenient, for folders) based on what's being dragged
  const customCollisionDetection: CollisionDetection = useCallback((args) => {
    // For entry drags, use pointerWithin - only detect when pointer is directly over target
    if (dndActiveType === 'entry') {
      return pointerWithin(args);
    }
    // For folder drags, use rectIntersection for easier targeting
    return rectIntersection(args);
  }, [dndActiveType]);

  useEffect(() => {
    if (dndActiveId) {
      document.body.style.cursor = 'grabbing';
    } else {
      document.body.style.cursor = '';
    }
    return () => { document.body.style.cursor = ''; };
  }, [dndActiveId]);

  // Define handleRefresh early so it can be used in hooks
  const handleRefresh = useCallback(() => {
    setRefreshTrigger((prev) => prev + 1);
    setIsDirty(true);
    
    if (isFavoritesView) {
      getFavoriteEntries().then(setFavoriteEntries);
    }
  }, [isFavoritesView]);

  // Define performClose
  const performClose = useCallback(async (isManualLogout: boolean = false) => {
    try {
      // Request all child windows to close (they will prompt for unsaved changes)
      const allClosed = await requestCloseAllChildWindows();
      
      if (!allClosed) {
        // Some windows stayed open (user cancelled due to unsaved changes)
        // Abort the logout/close
        return;
      }
      
      const appWindow = getCurrentWindow();
      await appWindow.setTitle("Simple Password Manager");
      
      setSelectedGroupUuid("");
      setIsDashboardView(true);
      setIsFavoritesView(false);
      setRootGroup(null);
      
      await closeDatabase();
      onClose(isManualLogout);
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to close database"),
        variant: "destructive",
      });
    }
  }, [onClose, toast]);

  // Custom hooks
  const { searchQuery, searchResults, isSearching, searchScope, setSearchScope, handleSearch, clearSearch, refreshSearch, setIsSearching } = useSearch();
  
  // Set global addToHistory for components that need it (e.g., EntryListItem favorite toggle)
  useEffect(() => {
    (window as any).__addToHistory = addToHistory;
    return () => {
      delete (window as any).__addToHistory;
    };
  }, [addToHistory]);
  
  useAutoLock(performClose);
  
  const { windowTitle } = useWindowManagement({
    dbPath,
    isDirty,
    onCloseRequested: () => {
      setCloseAction('window');
      setShowUnsavedDialog(true);
    },
  });

  const handleSave = useCallback(async () => {
    try {
      const hasChanges = await checkDatabaseChanges();
      if (hasChanges) {
        setShowConflictDialog(true);
        return;
      }
      await saveDatabase();
      setIsDirty(false);
      toast({
        title: "Success",
        description: "Database saved successfully",
        variant: "success",
      });
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to save database"),
        variant: "destructive",
      });
    }
  }, [toast]);

  const handleSynchronize = useCallback(async () => {
    try {
      await mergeDatabase();
      await handleRefresh();
      setShowConflictDialog(false);
      // Automatically save after merge to mark as not dirty
      await saveDatabase();
      setIsDirty(false);
      toast({
        title: "Success",
        description: "Database synchronized and saved successfully.",
        variant: "success",
      });
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to synchronize database"),
        variant: "destructive",
      });
      setShowConflictDialog(false);
    }
  }, [toast, handleRefresh]);

  const handleOverwrite = useCallback(async () => {
    try {
      await saveDatabase();
      setIsDirty(false);
      setShowConflictDialog(false);
      toast({
        title: "Success",
        description: "Database saved successfully",
        variant: "success",
      });
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to save database"),
        variant: "destructive",
      });
      setShowConflictDialog(false);
    }
  }, [toast]);

  const handleConflictCancel = useCallback(() => {
    setShowConflictDialog(false);
  }, []);

  const handleCopyPassword = useCallback(async () => {
    if (!selectedEntryForCopy) {
      toast({
        title: "No Entry Selected",
        description: "Please select an entry first",
        variant: "destructive",
      });
      return;
    }

    try {
      await writeText(selectedEntryForCopy.password);
      toast({
        title: "Copied",
        description: "Password copied to clipboard",
        variant: "success",
      });
    } catch (error: any) {
      toast({
        title: "Error",
        description: "Failed to copy password",
        variant: "destructive",
      });
    }
  }, [selectedEntryForCopy, toast]);

  const handlePaste = useCallback(() => {
    toast({
      title: "Paste",
      description: "Paste functionality is context-dependent",
      variant: "default",
    });
  }, [toast]);


  const handleNewDatabase = useCallback(() => {
    if (isDirty) {
      toast({
        title: "Unsaved Changes",
        description: "Please save or discard changes before creating a new database",
        variant: "destructive",
      });
      return;
    }
    setShowCreateDatabaseDialog(true);
  }, [isDirty, toast]);

  const handleNewDatabaseSuccess = useCallback(async () => {
    await performClose(false);
    window.location.reload();
  }, [performClose]);

  const handleTogglePasswords = useCallback(() => {
    setPasswordsVisible(prev => !prev);
    toast({
      title: passwordsVisible ? "Passwords Hidden" : "Passwords Visible",
      description: passwordsVisible ? "Password columns are now hidden" : "Password columns are now visible",
      variant: "default",
    });
  }, [passwordsVisible, toast]);

  const handleUndo = useCallback(async () => {
    try {
      await undo();
      handleRefresh();
      toast({
        title: "Undo",
        description: "Action undone",
        variant: "success",
      });
    } catch (error: any) {
      toast({
        title: "Error",
        description: "Failed to undo action",
        variant: "destructive",
      });
    }
  }, [undo, handleRefresh, toast]);

  const handleRedo = useCallback(async () => {
    try {
      await redo();
      handleRefresh();
      toast({
        title: "Redo",
        description: "Action redone",
        variant: "success",
      });
    } catch (error: any) {
      toast({
        title: "Error",
        description: "Failed to redo action",
        variant: "destructive",
      });
    }
  }, [redo, handleRefresh, toast]);

  const handleAbout = useCallback(() => {
    openAboutWindow();
  }, []);

  // Listen for HIBP setting changes from Settings window
  useEffect(() => {
    const unlisten = listen('hibp-setting-changed', async () => {
      console.log('HIBP setting changed, reloading database...');
      await performClose(false);
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, [performClose]);

  // Load database path and search scope on mount
  useEffect(() => {
    const loadDbInfo = async () => {
      const lastPath = localStorage.getItem("lastDatabasePath");
      if (lastPath) {
        setDbPath(lastPath);
        const savedScope = getSearchScope(lastPath);
        setSearchScope(savedScope);
        // Load live updates setting for this database
        setLiveUpdatesEnabled(getLiveUpdates(lastPath));
      }
    };
    loadDbInfo();
  }, [setSearchScope]);

  // Listen for live updates setting changes from Settings window
  useEffect(() => {
    const handleLiveUpdatesChange = (event: Event) => {
      const customEvent = event as CustomEvent<{ enabled: boolean }>;
      setLiveUpdatesEnabled(customEvent.detail.enabled);
    };

    window.addEventListener('liveUpdatesChanged', handleLiveUpdatesChange);

    return () => {
      window.removeEventListener('liveUpdatesChanged', handleLiveUpdatesChange);
    };
  }, []);

  // Periodic polling for database changes when live updates is enabled
  useEffect(() => {
    if (!liveUpdatesEnabled || !dbPath || isDirty) {
      return;
    }

    const pollInterval = setInterval(async () => {
      try {
        const hasChanges = await checkDatabaseChanges();
        
        if (hasChanges) {
            await mergeDatabase();
            await handleRefresh();
            await saveDatabase();
            setIsDirty(false);
            toast({
              title: "Auto-sync",
              description: "Database synchronized automatically.",
              variant: "success",
            });
        }
      } catch (error) {
        console.error('Failed to check for database changes:', error);
      }
    }, 5000);

    return () => {
      clearInterval(pollInterval);
    };
  }, [liveUpdatesEnabled, dbPath, isDirty, handleRefresh, toast]);

  const loadGroups = useCallback(async () => {
    try {
      const groups = await getGroups();
      setRootGroup(groups);
      
      if (!selectedGroupUuid) {
        setSelectedGroupUuid("_dashboard");
        setIsDashboardView(true);
        if (dbPath) {
          const state = loadGroupTreeState(dbPath, groups.uuid, groups);
          setInitialExpandedGroups(new Set(state.expandedGroups));
        }
      }
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to load groups"),
        variant: "destructive",
      });
    }
  }, [dbPath, selectedGroupUuid, toast]);

  // Load groups after dbPath is available
  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    if (dbPath) {
      loadGroups();
    }
  }, [refreshTrigger, dbPath, loadGroups]);
  /* eslint-enable react-hooks/set-state-in-effect */

  const handleClose = async () => {
    if (isDirty) {
      setCloseAction('logout');
      setShowUnsavedDialog(true);
    } else {
      await performClose(true);
    }
  };

  // Keyboard shortcuts and entry events - must be after all handlers are defined
  useKeyboardShortcuts({ 
    onSave: handleSave,
    onClose: handleClose,
    onNewDatabase: handleNewDatabase,
    onToggleSearch: () => {
      if (isSearchVisible) {
        setIsSearchVisible(false);
        clearSearch();
      } else {
        setIsSearchVisible(true);
      }
    },
    onCloseSearch: () => {
      setIsSearchVisible(false);
      clearSearch();
    },
    onUndo: handleUndo,
    onRedo: handleRedo,
    isSearchVisible
  });
  useEntryEvents(handleRefresh);

  const handleUnsavedCancel = () => {
    setShowUnsavedDialog(false);
    setCloseAction(null);
  };

  const handleUnsavedDontSave = async () => {
    setShowUnsavedDialog(false);
    setIsDirty(false);
    
    if (closeAction === 'window') {
      const appWindow = getCurrentWindow();
      const closeToTray = localStorage.getItem("closeToTray") === "true";
      if (closeToTray) {
        await appWindow.hide();
      } else {
        await appWindow.close();
      }
    } else if (closeAction === 'logout') {
      await performClose(true);
    }
    
    setCloseAction(null);
  };

  const handleUnsavedSave = async () => {
    setShowUnsavedDialog(false);
    await handleSave();
    
    if (closeAction === 'window') {
      const appWindow = getCurrentWindow();
      const closeToTray = localStorage.getItem("closeToTray") === "true";
      if (closeToTray) {
        await appWindow.hide();
      } else {
        await appWindow.close();
      }
    } else if (closeAction === 'logout') {
      await performClose(true);
    }
    
    setCloseAction(null);
  };

  const handleGroupSelect = async (uuid: string) => {
    setSelectedGroupUuid(uuid);
    setIsSearching(false);
    clearSearch();
    
    if (uuid === "_dashboard") {
      setIsDashboardView(true);
      setIsFavoritesView(false);
      setFavoriteEntries([]);
      // Don't reset search scope - preserve user's preference
      return;
    }
    
    setIsDashboardView(false);
    
    if (uuid === "_favorites") {
      setIsFavoritesView(true);
      // Don't force global scope in Favorites - let user choose
      try {
        const favorites = await getFavoriteEntries();
        setFavoriteEntries(favorites);
      } catch (error: any) {
        toast({
          title: "Error",
          description: typeof error === 'string' ? error : (error?.message || "Failed to load favorites"),
          variant: "destructive",
        });
      }
    } else {
      setIsFavoritesView(false);
      setFavoriteEntries([]);
    }
  };

  const getGroupPath = (root: GroupData, targetUuid: string): string => {
    const findPath = (g: GroupData, uuid: string, path: string[]): string[] | null => {
      if (g.uuid === uuid) {
        return [...path, g.name];
      }
      for (const child of g.children) {
        const result = findPath(child, uuid, [...path, g.name]);
        if (result) return result;
      }
      return null;
    };
    
    const path = findPath(root, targetUuid, []);
    return path ? path.join(" / ") : "";
  };

  // DnD Handlers
  const handleDragStart = (event: DragStartEvent) => {
    const { active } = event;
    const data = active.data.current;
    
    if (!isDragData(data)) {
      console.warn('Invalid drag data structure:', data);
      return;
    }
    
    setDndActiveId(active.id as string);
    setDndActiveType(data?.type || null);
    setDndActiveData(data);
  };

  const handleDragOver = (event: DragOverEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) {
      if (dndOverId !== null) setDndOverId(null);
      return;
    }
    if (dndOverId !== over.id) setDndOverId(over.id as string);
  };

  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event;
    const data = active.data.current;
    
    setDndActiveId(null);
    setDndOverId(null);
    setDndActiveType(null);
    setDndActiveData(null);

    if (!over || active.id === over.id) return;
    
    if (!isDragData(data)) {
      console.warn('Invalid drag data structure:', data);
      return;
    }

    const draggedType = data?.type;
    const targetId = over.id as string;

    // Handle Entry drop onto Folder
    if (data && data.type === 'entry') {
      const entry = data.entry;
      const targetGroup = rootGroup ? findGroupByUuid(rootGroup, targetId) : null;
      
      if (!targetGroup) {
        toast({
          title: "Invalid Target",
          description: "Please drop the entry onto a valid folder",
          variant: "destructive",
        });
        return;
      }

      // Don't move if already in the same group
      if (entry.group_uuid === targetId) {
        return;
      }

      try {
        const oldGroupUuid = entry.group_uuid;
        await moveEntry(entry.uuid, targetId);
        
        // Track for undo/redo
        addToHistory(
          `Move entry "${entry.title}" to group`,
          async () => {
            await moveEntry(entry.uuid, oldGroupUuid);
            await handleRefresh();
          },
          async () => {
            await moveEntry(entry.uuid, targetId);
            await handleRefresh();
          }
        );
        
        setIsDirty(true);
        await handleRefresh();
        toast({
          title: "Success",
          description: `Moved "${entry.title}" to "${targetGroup.name}"`,
          variant: "success",
        });
      } catch (error: any) {
        toast({
          title: "Error",
          description: typeof error === 'string' ? error : (error?.message || "Failed to move entry"),
          variant: "destructive",
        });
      }
      return;
    }

    // Handle Folder drop onto Folder
    if (draggedType === 'folder' && rootGroup) {
      const draggedId = active.id as string;
      const draggedGroup = findGroupByUuid(rootGroup, draggedId);
      const targetGroup = findGroupByUuid(rootGroup, targetId);
      
      if (!draggedGroup || !targetGroup) return;

      try {
        if (isDescendant(draggedGroup, targetGroup)) {
          toast({
            title: "Invalid Move",
            description: "Cannot move a group into its own descendant",
            variant: "destructive",
          });
          return;
        }

        const oldParent = findParentGroup(rootGroup, draggedId);
        const oldParentUuid = oldParent?.uuid || rootGroup.uuid;
        
        await moveGroup(draggedId, targetId);
        
        // Track for undo/redo
        const movedGroup = findGroupByUuid(rootGroup, draggedId);
        if (movedGroup) {
          addToHistory(
            `Move group "${movedGroup.name}"`,
            async () => {
              await moveGroup(draggedId, oldParentUuid);
              await handleRefresh();
            },
            async () => {
              await moveGroup(draggedId, targetId);
              await handleRefresh();
            }
          );
        }
        
        setIsDirty(true);
        await handleRefresh();
        toast({
          title: "Success",
          description: `Moved "${draggedGroup.name}" into "${targetGroup.name}"`,
          variant: "success",
        });
      } catch (error: any) {
        toast({
          title: "Error",
          description: typeof error === 'string' ? error : (error?.message || "Failed to move group"),
          variant: "destructive",
        });
      }
    }
  };

  // Get active entry for DragOverlay
  const getActiveEntry = (): EntryData | null => {
    if (dndActiveData && dndActiveData.type === 'entry') {
      return dndActiveData.entry;
    }
    return null;
  };

  // Get active group for DragOverlay
  const getActiveGroup = (): GroupData | null => {
    if (dndActiveType === 'folder' && dndActiveId && rootGroup) {
      return findGroupByUuid(rootGroup, dndActiveId);
    }
    return null;
  };

  const activeEntry = getActiveEntry();
  const activeGroup = getActiveGroup();

  // Show dropdown in Dashboard (disabled), Favorites (enabled), and folders (enabled)
  const showSearchScopeDropdown = isDashboardView || isFavoritesView || selectedGroupUuid !== "";
  
  // Can actually use folder search only in real folders (not Dashboard, not Favorites)
  const canSearchInFolder = !isDashboardView && !isFavoritesView && selectedGroupUuid !== "";
  
  // Effective search scope for display: Dashboard always shows 'global', others show actual preference
  const effectiveSearchScope = isDashboardView ? "global" : searchScope;

  // Handle search with current scope
  const handleSearchWithScope = async (query: string) => {
    // In Favorites view with 'folder' scope, we need to search globally then filter to favorites
    if (isFavoritesView && searchScope === 'folder') {
      // Perform global search
      await handleSearch(query, 'global', undefined);
      // The results will be filtered in the render logic below
    } else {
      await handleSearch(query, searchScope, canSearchInFolder ? selectedGroupUuid : undefined);
    }
  };

  // Handle search scope change and save to localStorage
  const handleSearchScopeChange = (newScope: typeof searchScope) => {
    setSearchScope(newScope);
    if (dbPath) {
      saveSearchScope(dbPath, newScope);
    }
    // Re-run search if there's an active query
    if (searchQuery.trim()) {
      handleSearch(searchQuery, newScope, canSearchInFolder ? selectedGroupUuid : undefined);
    }
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={customCollisionDetection}
      onDragStart={handleDragStart}
      onDragOver={handleDragOver}
      onDragEnd={handleDragEnd}
    >
      <div className="flex h-full w-full flex-col">
        <CustomTitleBar 
          title={windowTitle}
          showMenu={true}
          isDirty={isDirty}
          onSave={handleSave}
          onLogout={handleClose}
          onToggleSearch={() => setIsSearchVisible(!isSearchVisible)}
          onUndo={handleUndo}
          onRedo={handleRedo}
          onCopy={handleCopyPassword}
          onPaste={handlePaste}
          onNewDatabase={handleNewDatabase}
          onTogglePasswords={handleTogglePasswords}
          onAbout={handleAbout}
          canUndo={canUndo}
          canRedo={canRedo}
          passwordsVisible={passwordsVisible}
        />
        
        <SearchHeader
          searchQuery={searchQuery}
          onSearchChange={handleSearchWithScope}
          searchScope={effectiveSearchScope}
          onSearchScopeChange={handleSearchScopeChange}
          showSearchScopeDropdown={showSearchScopeDropdown}
          isSearchScopeDisabled={isDashboardView}
          isVisible={isSearchVisible}
          onToggle={() => setIsSearchVisible(!isSearchVisible)}
        />

        <motion.div 
          className="flex flex-1 overflow-hidden"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.3 }}
        >
          <ResizablePanel
            defaultWidth={256}
            minWidth={200}
            maxWidth={500}
            storageKey="groupTreeWidth"
          >
            {rootGroup && (
              <GroupTree
                group={rootGroup}
                selectedUuid={selectedGroupUuid}
                onSelectGroup={handleGroupSelect}
                onRefresh={handleRefresh}
                onGroupDeleted={(deletedUuid) => {
                  if (selectedGroupUuid === deletedUuid) {
                    handleGroupSelect(rootGroup.uuid);
                  }
                }}
                dbPath={dbPath}
                initialExpandedGroups={initialExpandedGroups}
                activeId={dndActiveId}
                overId={dndOverId}
                activeType={dndActiveType}
                addToHistory={addToHistory}
              />
            )}
          </ResizablePanel>

          <div className="flex-1 overflow-hidden">
            <div className={isDashboardView && !isSearching ? "h-full" : "hidden"}>
              <Dashboard refreshTrigger={refreshTrigger} databasePath={dbPath} isDirty={isDirty} />
            </div>
            
            {(!isDashboardView || isSearching) && (
              <EntryList
                groupUuid={isSearching || isFavoritesView ? "" : selectedGroupUuid}
                searchResults={
                  isSearching 
                    ? (isFavoritesView && searchScope === 'folder' 
                        ? searchResults.filter(entry => entry.is_favorite) 
                        : searchResults)
                    : isFavoritesView 
                    ? favoriteEntries 
                    : []
                }
                selectedEntry={null}
                onSelectEntry={(entry) => openEntryWindow(entry, entry.group_uuid)}
                onRefresh={handleRefresh}
                onSearchRefresh={() => refreshSearch(searchScope, canSearchInFolder ? selectedGroupUuid : undefined)}
                isSearching={isSearching || isFavoritesView}
                hasActiveSearch={isSearching}
                isFavoritesView={isFavoritesView}
                rootGroupUuid={rootGroup?.uuid}
                selectedGroupName={
                  isSearching
                    ? undefined
                    : isFavoritesView 
                    ? "Favorites"
                    : rootGroup && selectedGroupUuid 
                    ? getGroupPath(rootGroup, selectedGroupUuid)
                    : undefined
                }
                databasePath={dbPath}
                addToHistory={addToHistory}
              />
            )}
          </div>
        </motion.div>

        <UnsavedChangesDialog
          open={showUnsavedDialog}
          onCancel={handleUnsavedCancel}
          onDontSave={handleUnsavedDontSave}
          onSave={handleUnsavedSave}
        />

        <DatabaseConflictDialog
          open={showConflictDialog}
          databasePath={dbPath}
          onSynchronize={handleSynchronize}
          onOverwrite={handleOverwrite}
          onCancel={handleConflictCancel}
        />

        <CreateDatabaseDialog
          isOpen={showCreateDatabaseDialog}
          onClose={() => setShowCreateDatabaseDialog(false)}
          onSuccess={handleNewDatabaseSuccess}
        />
      </div>

      <DragOverlay dropAnimation={null}>
        {activeEntry ? (
          <div className="flex items-center gap-2 px-4 py-2.5 bg-accent rounded shadow-lg opacity-80 border">
            {(() => {
              const EntryIcon = getIconComponent(activeEntry.icon_id ?? 0);
              return <EntryIcon className="h-4 w-4 text-muted-foreground" />;
            })()}
            <span className="text-sm font-medium">{activeEntry.title}</span>
          </div>
        ) : activeGroup ? (
          <div className="flex items-center gap-1 px-2 py-1.5 bg-accent/50 rounded shadow-lg opacity-60">
            {(() => {
              const FolderIcon = getIconComponent(activeGroup.icon_id ?? 48);
              return <FolderIcon className="h-4 w-4 text-muted-foreground" />;
            })()}
            <span className="text-sm font-medium">{activeGroup.name}</span>
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
  );
}
