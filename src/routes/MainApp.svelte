<script lang="ts">
  import { onMount } from "svelte";
  import {
    getGroups,
    getEntry,
    getFavoriteEntries,
    saveDatabase,
    closeDatabase,
    searchEntries,
    checkDatabaseChanges,
    mergeDatabase,
    type GroupData,
    type EntryData,
  } from "$lib/tauri";
  import { getLiveUpdates, getCloseToTray } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";
  import { undoStack } from "$lib/undo-stack.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import EntryList from "$lib/components/EntryList.svelte";
  import EntryEditor from "$lib/components/EntryEditor.svelte";
  import EmptyDetail from "$lib/components/EmptyDetail.svelte";
  import Dashboard from "$lib/components/Dashboard.svelte";
  import SplitPane from "$lib/components/SplitPane.svelte";
  import UnsavedChangesDialog from "$lib/components/UnsavedChangesDialog.svelte";
  import DatabaseConflictDialog from "$lib/components/DatabaseConflictDialog.svelte";
  import CommandPalette, { type PaletteCommand } from "$lib/components/CommandPalette.svelte";
  import SettingsDialog from "$lib/components/SettingsDialog.svelte";
  import AboutDialog from "$lib/components/AboutDialog.svelte";
  import { toast } from "$lib/ui";
  import {
    Save,
    LogOut,
    Settings as SettingsIcon,
    Info,
    Sun,
    Moon,
    Monitor,
    Star,
    LayoutPanelLeft,
    Key,
    Folder,
    Undo2,
    Redo2,
  } from "@lucide/svelte";
  import { loadGroupTreeState } from "$lib/group-state";
  import { getGroupPath } from "$lib/group-utils";
  import { theme } from "$lib/theme.svelte";

  type Props = { onClose: (isManualLogout?: boolean) => void };
  let { onClose }: Props = $props();

  let rootGroup = $state<GroupData | null>(null);
  let selectedUuid = $state<string>("_dashboard");
  let selectedEntryUuid = $state<string>("");
  let favoriteEntries = $state<EntryData[]>([]);
  let isFavoritesView = $state(false);
  let isDashboardView = $state(true);

  let showUnsaved = $state(false);
  let closeAction = $state<"logout" | "window" | null>(null);
  let showConflict = $state(false);
  let liveUpdatesEnabled = $state(false);
  let paletteOpen = $state(false);
  let settingsOpen = $state(false);
  let aboutOpen = $state(false);

  let initialExpanded: Set<string> | undefined = $state(undefined);

  onMount(() => {
    void load();
    const win = getCurrentWindow();
    const u = win.onCloseRequested(async (event) => {
      event.preventDefault();
      if (appState.isDirty) {
        closeAction = "window";
        showUnsaved = true;
      } else if (getCloseToTray()) {
        await win.hide();
      } else {
        await win.destroy();
      }
    });
    return () => u.then((fn) => fn());
  });

  async function load() {
    const groups = await getGroups();
    rootGroup = groups;
    if (appState.dbPath) {
      const state = loadGroupTreeState(appState.dbPath, groups.uuid, groups);
      initialExpanded = new Set(state.expandedGroups);
      liveUpdatesEnabled = getLiveUpdates(appState.dbPath);
    }
    selectedUuid = "_dashboard";
    isDashboardView = true;
  }

  $effect(() => {
    void appState.refreshCounter;
    if (appState.dbPath) {
      getGroups().then((g) => (rootGroup = g)).catch(() => undefined);
      if (isFavoritesView) {
        getFavoriteEntries().then((e) => (favoriteEntries = e)).catch(() => undefined);
      }
    }
  });

  $effect(() => {
    if (!liveUpdatesEnabled || !appState.dbPath || appState.isDirty) return;
    const iv = setInterval(async () => {
      try {
        const changed = await checkDatabaseChanges();
        if (changed) {
          await mergeDatabase();
          appState.refresh();
          await saveDatabase();
          appState.markClean();
          toast.success("Auto-sync", "Database synchronized automatically");
        }
      } catch (e) {
        console.error("Live update check failed", e);
      }
    }, 5000);
    return () => clearInterval(iv);
  });

  async function selectGroup(uuid: string) {
    selectedUuid = uuid;
    selectedEntryUuid = "";
    if (uuid === "_dashboard") {
      isDashboardView = true;
      isFavoritesView = false;
      favoriteEntries = [];
      return;
    }
    isDashboardView = false;
    if (uuid === "_favorites") {
      isFavoritesView = true;
      try {
        favoriteEntries = await getFavoriteEntries();
      } catch (e) {
        toast.error("Error", String(e));
      }
    } else {
      isFavoritesView = false;
      favoriteEntries = [];
    }
  }

  // Navigate to an entry, switching to its folder first if needed
  async function jumpToEntry(uuid: string) {
    try {
      const e = await getEntry(uuid);
      await selectGroup(e.group_uuid);
      // give the list a tick to load
      await new Promise((r) => setTimeout(r, 30));
      selectedEntryUuid = uuid;
    } catch (err) {
      toast.error("Could not open entry", String(err));
    }
  }

  async function performClose(manual: boolean) {
    try {
      const win = getCurrentWindow();
      await win.setTitle("Simple Password Manager");
      selectedUuid = "";
      selectedEntryUuid = "";
      isDashboardView = true;
      isFavoritesView = false;
      rootGroup = null;
      undoStack.clear();
      await closeDatabase();
      onClose(manual);
    } catch (e) {
      toast.error("Error", String(e) || "Failed to close database");
    }
  }

  async function handleSave() {
    try {
      const changed = await checkDatabaseChanges();
      if (changed) {
        showConflict = true;
        return;
      }
      await saveDatabase();
      appState.markClean();
      toast.success("Saved", "Database saved successfully");
    } catch (e) {
      toast.error("Save failed", String(e));
    }
  }

  async function handleLogout() {
    if (appState.isDirty) {
      closeAction = "logout";
      showUnsaved = true;
    } else {
      await performClose(true);
    }
  }

  async function handleUndo() {
    const desc = await undoStack.undo();
    if (desc) {
      appState.markDirty();
      appState.refresh();
      toast.success("Undone", desc);
    }
  }

  async function handleRedo() {
    const desc = await undoStack.redo();
    if (desc) {
      appState.markDirty();
      appState.refresh();
      toast.success("Redone", desc);
    }
  }

  async function syncAndSave() {
    try {
      await mergeDatabase();
      appState.refresh();
      await saveDatabase();
      appState.markClean();
      showConflict = false;
      toast.success("Synchronized", "Database synchronized and saved");
    } catch (e) {
      toast.error("Sync failed", String(e));
      showConflict = false;
    }
  }

  async function overwriteSave() {
    try {
      await saveDatabase();
      appState.markClean();
      showConflict = false;
      toast.success("Saved");
    } catch (e) {
      toast.error("Save failed", String(e));
      showConflict = false;
    }
  }

  function isEditable(el: EventTarget | null): boolean {
    if (!(el instanceof HTMLElement)) return false;
    if (el.dataset.commandPaletteInput !== undefined) return false;
    const tag = el.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
    if (el.isContentEditable) return true;
    return false;
  }

  function onKey(e: KeyboardEvent) {
    const isMac = /Mac|iPhone|iPad/i.test(navigator.platform);
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (mod && e.key.toLowerCase() === "s") {
      e.preventDefault();
      void handleSave();
    } else if (mod && e.key.toLowerCase() === "k") {
      if (paletteOpen) return;
      if (isEditable(e.target)) return;
      e.preventDefault();
      paletteOpen = true;
    } else if (mod && e.shiftKey && e.key.toLowerCase() === "z") {
      if (isEditable(e.target)) return;
      e.preventDefault();
      void handleRedo();
    } else if (mod && e.key.toLowerCase() === "z") {
      if (isEditable(e.target)) return;
      e.preventDefault();
      void handleUndo();
    } else if (mod && e.key.toLowerCase() === "y") {
      if (isEditable(e.target)) return;
      e.preventDefault();
      void handleRedo();
    } else if (mod && e.key === ",") {
      e.preventDefault();
      settingsOpen = true;
    }
  }

  const groupName = $derived(
    isFavoritesView
      ? "Favorites"
      : rootGroup && selectedUuid && !selectedUuid.startsWith("_")
        ? getGroupPath(rootGroup, selectedUuid)
        : undefined,
  );

  function onRefresh() {
    appState.markDirty();
    appState.refresh();
  }

  const actionCommands: PaletteCommand[] = $derived([
    {
      id: "go-dashboard",
      label: "Go to Dashboard",
      section: "Navigate",
      icon: LayoutPanelLeft,
      keywords: "home stats",
      onSelect: () => selectGroup("_dashboard"),
    },
    {
      id: "go-favorites",
      label: "Go to Favorites",
      section: "Navigate",
      icon: Star,
      onSelect: () => selectGroup("_favorites"),
    },
    { id: "act-save", label: "Save database", section: "Actions", icon: Save, hint: "Ctrl S", onSelect: handleSave },
    { id: "act-undo", label: "Undo last action", section: "Actions", icon: Undo2, hint: "Ctrl Z", onSelect: handleUndo },
    { id: "act-redo", label: "Redo last action", section: "Actions", icon: Redo2, hint: "Ctrl Y", onSelect: handleRedo },
    {
      id: "act-settings",
      label: "Open Settings",
      section: "Actions",
      icon: SettingsIcon,
      onSelect: () => (settingsOpen = true),
    },
    {
      id: "act-about",
      label: "Open About",
      section: "Actions",
      icon: Info,
      onSelect: () => (aboutOpen = true),
    },
    { id: "act-lock", label: "Lock database", section: "Actions", icon: LogOut, onSelect: handleLogout },
    { id: "theme-system", label: "Theme: System", section: "Theme", icon: Monitor, onSelect: () => theme.set("system") },
    { id: "theme-light", label: "Theme: Light", section: "Theme", icon: Sun, onSelect: () => theme.set("light") },
    { id: "theme-dark", label: "Theme: Dark", section: "Theme", icon: Moon, onSelect: () => theme.set("dark") },
  ]);

  function folderCommands(g: GroupData, path: string[] = []): PaletteCommand[] {
    if (rootGroup && g.uuid === rootGroup.uuid && path.length === 0) {
      return g.children.flatMap((c) => folderCommands(c, [g.name]));
    }
    const here: PaletteCommand[] = [
      {
        id: `folder-${g.uuid}`,
        label: g.name,
        hint: path.join(" / "),
        section: "Folders",
        icon: Folder,
        keywords: path.join(" "),
        onSelect: () => selectGroup(g.uuid),
      },
    ];
    for (const c of g.children) {
      here.push(...folderCommands(c, [...path, g.name]));
    }
    return here;
  }

  const commandsAll = $derived([
    ...actionCommands,
    ...(rootGroup ? folderCommands(rootGroup) : []),
  ]);

  async function paletteSearch(q: string): Promise<PaletteCommand[]> {
    try {
      const results = await searchEntries(q);
      return results.slice(0, 25).map((e) => ({
        id: `entry-${e.uuid}`,
        label: e.title || "(untitled)",
        section: "Entries",
        icon: Key,
        hint: e.username || e.url || "",
        onSelect: () => jumpToEntry(e.uuid),
      }));
    } catch {
      return [];
    }
  }

</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-full w-full">
  <SplitPane defaultWidth={240} minWidth={200} maxWidth={420} storageKey="sidebarWidth">
    {#snippet left()}
      <Sidebar
        {rootGroup}
        {selectedUuid}
        onSelectGroup={selectGroup}
        onRefresh={() => appState.refresh()}
        onGroupDeleted={(deleted) => {
          if (selectedUuid === deleted && rootGroup) selectGroup(rootGroup.uuid);
        }}
        initialExpandedGroups={initialExpanded}
        onOpenSettings={() => (settingsOpen = true)}
        onSave={handleSave}
        onLogout={handleLogout}
      />
    {/snippet}

    {#snippet right()}
      {#if isDashboardView}
        <Dashboard refreshTrigger={appState.refreshCounter} onJumpToEntry={jumpToEntry} />
      {:else}
        <SplitPane defaultWidth={340} minWidth={260} maxWidth={520} storageKey="entryListWidth">
          {#snippet left()}
            <EntryList
              groupUuid={isFavoritesView ? "" : selectedUuid}
              searchResults={isFavoritesView ? favoriteEntries : []}
              onRefresh={onRefresh}
              isSearching={isFavoritesView}
              {isFavoritesView}
              rootGroupUuid={rootGroup?.uuid}
              selectedGroupName={groupName}
              {selectedEntryUuid}
              onSelectEntry={(uuid) => (selectedEntryUuid = uuid)}
              onEntryCreated={(uuid) => (selectedEntryUuid = uuid)}
            />
          {/snippet}
          {#snippet right()}
            {#if selectedEntryUuid}
              {#key selectedEntryUuid}
                <EntryEditor
                  uuid={selectedEntryUuid}
                  onClose={() => (selectedEntryUuid = "")}
                  onChange={onRefresh}
                />
              {/key}
            {:else}
              <EmptyDetail />
            {/if}
          {/snippet}
        </SplitPane>
      {/if}
    {/snippet}
  </SplitPane>
</div>

<CommandPalette
  bind:open={paletteOpen}
  commands={commandsAll}
  onSearch={paletteSearch}
  placeholder="Search entries, jump to folders, run commands…"
/>

<SettingsDialog bind:open={settingsOpen} onOpenAbout={() => { settingsOpen = false; aboutOpen = true; }} />
<AboutDialog bind:open={aboutOpen} />

<UnsavedChangesDialog
  bind:open={showUnsaved}
  onCancel={() => {
    showUnsaved = false;
    closeAction = null;
  }}
  onDontSave={async () => {
    showUnsaved = false;
    appState.markClean();
    if (closeAction === "window") {
      const w = getCurrentWindow();
      if (getCloseToTray()) await w.hide();
      else await w.destroy();
    } else if (closeAction === "logout") {
      await performClose(true);
    }
    closeAction = null;
  }}
  onSave={async () => {
    showUnsaved = false;
    await handleSave();
    if (closeAction === "window") {
      const w = getCurrentWindow();
      if (getCloseToTray()) await w.hide();
      else await w.destroy();
    } else if (closeAction === "logout") {
      await performClose(true);
    }
    closeAction = null;
  }}
/>

<DatabaseConflictDialog
  bind:open={showConflict}
  databasePath={appState.dbPath}
  onSynchronize={syncAndSave}
  onOverwrite={overwriteSave}
  onCancel={() => (showConflict = false)}
/>
