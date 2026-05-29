<script lang="ts">
  // Layout sketch (1Password 8):
  //   ┌──── sidebar (≈260) ────┬──── item list (≈340) ────┬──── detail (fills) ────┐
  //   │ vault picker           │ sticky header + search   │ hero (72px icon tile)  │
  //   │ Home / All / Favorites │ rounded-square tile rows │ LOGIN DETAILS card     │
  //   │ FOLDERS · tree         │ soft-tinted selection    │ Notes / TOTP / Tags    │
  //   │ Save · avatar menu     │                          │ Edit button → form     │
  //   └────────────────────────┴──────────────────────────┴────────────────────────┘
  // Sidebar virtual ids: _dashboard (Home), _all (All Items), _favorites (Favorites).
  // The middle column is hidden when on the Home/Watchtower dashboard view.

  import { onMount } from "svelte";
  import {
    getGroups,
    getEntry,
    getEntries,
    getFavoriteEntries,
    saveDatabase,
    closeDatabase,
    searchEntries,
    checkDatabaseChanges,
    mergeDatabase,
    type GroupData,
    type EntryData,
  } from "$lib/tauri";
  import { getCloseToTray } from "$lib/storage";
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
    Home,
    KeyRound,
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
  let allEntries = $state<EntryData[]>([]);
  let favoriteEntries = $state<EntryData[]>([]);

  let showUnsaved = $state(false);
  let closeAction = $state<"logout" | "window" | null>(null);
  let showConflict = $state(false);
  let paletteOpen = $state(false);
  let settingsOpen = $state(false);
  let aboutOpen = $state(false);

  let initialExpanded: Set<string> | undefined = $state(undefined);

  const isDashboardView = $derived(selectedUuid === "_dashboard");
  const isFavoritesView = $derived(selectedUuid === "_favorites");
  const isAllView = $derived(selectedUuid === "_all");

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
    }
    selectedUuid = "_dashboard";
  }

  async function collectEntries(g: GroupData): Promise<EntryData[]> {
    const acc: EntryData[] = [];
    const walk = async (node: GroupData) => {
      const here = await getEntries(node.uuid);
      acc.push(...here);
      for (const c of node.children) await walk(c);
    };
    await walk(g);
    return acc;
  }

  $effect(() => {
    void appState.refreshCounter;
    if (appState.dbPath) {
      getGroups().then((g) => (rootGroup = g)).catch(() => undefined);
      if (isFavoritesView) {
        getFavoriteEntries().then((e) => (favoriteEntries = e)).catch(() => undefined);
      }
      if (isAllView && rootGroup) {
        collectEntries(rootGroup).then((e) => (allEntries = e)).catch(() => undefined);
      }
    }
  });

  // Background poll for *external* DB changes. Always on (no opt-in setting).
  // If the file on disk diverged, merge it in silently — KeePass merges by UUID
  // and is conflict-free in practice. We don't even toast; the UI just refreshes.
  $effect(() => {
    if (!appState.dbPath) return;
    const iv = setInterval(async () => {
      if (appState.isDirty) return; // let the auto-save path handle it instead
      try {
        const changed = await checkDatabaseChanges();
        if (changed) {
          await mergeDatabase();
          appState.refresh();
        }
      } catch (e) {
        console.error("Remote check failed", e);
      }
    }, 3000);
    return () => clearInterval(iv);
  });

  // Instant auto-save: whenever something mutated (dirtyVersion ticked), flush
  // to disk on the next microtask. A tiny coalescing window keeps bursts of
  // edits from issuing a save per keystroke, but it never blocks: 60ms is
  // imperceptible to humans.
  //
  // On disk-changed: merge first, then save. Only surface the conflict dialog
  // if the merge itself throws — that's the only real conflict KeePass exposes.
  let savePending = false;
  let saveQueued = false;
  $effect(() => {
    void appState.dirtyVersion;
    if (!appState.isDirty || !appState.dbPath) return;
    const timer = window.setTimeout(() => void runSave(), 60);
    return () => clearTimeout(timer);
  });

  async function runSave() {
    if (savePending) {
      saveQueued = true;
      return;
    }
    savePending = true;
    try {
      if (await checkDatabaseChanges()) {
        try {
          await mergeDatabase();
          appState.refresh();
        } catch {
          showConflict = true;
          return;
        }
      }
      await saveDatabase();
      appState.markClean();
    } catch (e) {
      toast.error("Auto-save failed", String(e));
    } finally {
      savePending = false;
      if (saveQueued) {
        saveQueued = false;
        if (appState.isDirty) void runSave();
      }
    }
  }

  async function selectGroup(uuid: string) {
    selectedUuid = uuid;
    selectedEntryUuid = "";
    if (uuid === "_favorites") {
      try {
        favoriteEntries = await getFavoriteEntries();
      } catch (e) {
        toast.error("Error", String(e));
      }
    } else if (uuid === "_all" && rootGroup) {
      try {
        allEntries = await collectEntries(rootGroup);
      } catch (e) {
        toast.error("Error", String(e));
      }
    }
  }

  async function jumpToEntry(uuid: string) {
    try {
      const e = await getEntry(uuid);
      await selectGroup(e.group_uuid);
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
    if (mod && e.key.toLowerCase() === "k") {
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
      : isAllView
        ? "All Items"
        : rootGroup && selectedUuid && !selectedUuid.startsWith("_")
          ? getGroupPath(rootGroup, selectedUuid)
          : undefined,
  );

  function onRefresh() {
    appState.markDirty();
    appState.refresh();
  }

  const actionCommands: PaletteCommand[] = $derived([
    { id: "go-home", label: "Go to Home", section: "Navigate", icon: Home, keywords: "home dashboard", onSelect: () => selectGroup("_dashboard") },
    { id: "go-all", label: "Go to All Items", section: "Navigate", icon: KeyRound, onSelect: () => selectGroup("_all") },
    { id: "go-favorites", label: "Go to Favorites", section: "Navigate", icon: Star, onSelect: () => selectGroup("_favorites") },
    { id: "act-undo", label: "Undo last action", section: "Actions", icon: Undo2, hint: "Ctrl Z", onSelect: handleUndo },
    { id: "act-redo", label: "Redo last action", section: "Actions", icon: Redo2, hint: "Ctrl Y", onSelect: handleRedo },
    { id: "act-settings", label: "Open Settings", section: "Actions", icon: SettingsIcon, onSelect: () => (settingsOpen = true) },
    { id: "act-about", label: "Open About", section: "Actions", icon: Info, onSelect: () => (aboutOpen = true) },
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
        icon: KeyRound,
        hint: e.username || e.url || "",
        onSelect: () => jumpToEntry(e.uuid),
      }));
    } catch {
      return [];
    }
  }

  const middleSearchResults = $derived(
    isFavoritesView ? favoriteEntries : isAllView ? allEntries : [],
  );
  const isVirtualList = $derived(isFavoritesView || isAllView);
</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-full w-full">
  <SplitPane defaultWidth={260} minWidth={220} maxWidth={420} storageKey="sidebarWidth">
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
        onLogout={handleLogout}
      />
    {/snippet}

    {#snippet right()}
      {#if isDashboardView}
        <Dashboard refreshTrigger={appState.refreshCounter} onJumpToEntry={jumpToEntry} />
      {:else}
        <SplitPane defaultWidth={340} minWidth={280} maxWidth={520} storageKey="entryListWidth">
          {#snippet left()}
            <EntryList
              groupUuid={isVirtualList ? "" : selectedUuid}
              searchResults={middleSearchResults}
              onRefresh={onRefresh}
              isSearching={isVirtualList}
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
  placeholder="Search items, jump to folders, run commands…"
/>

<SettingsDialog bind:open={settingsOpen} />
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
