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
    analyzeConflicts,
    resolveConflicts,
    parseLockHeldError,
    cloudStatus,
    cloudPush,
    type GroupData,
    type EntryData,
    type EntryConflict,
    type ConflictChoice,
  } from "$lib/tauri";
  import { getCloseToTray } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";
  import { undoStack } from "$lib/undo-stack.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import EntryList from "$lib/components/EntryList.svelte";
  import EntryEditor from "$lib/components/EntryEditor.svelte";
  import EmptyDetail from "$lib/components/EmptyDetail.svelte";
  import Dashboard from "$lib/components/Dashboard.svelte";
  import SplitPane from "$lib/components/SplitPane.svelte";
  import UnsavedChangesDialog from "$lib/components/UnsavedChangesDialog.svelte";
  import DatabaseConflictDialog from "$lib/components/DatabaseConflictDialog.svelte";
  import ConflictResolutionDialog from "$lib/components/ConflictResolutionDialog.svelte";
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
  let perEntryConflicts = $state<EntryConflict[]>([]);
  let showPerEntryConflict = $state(false);
  let paletteOpen = $state(false);
  let settingsOpen = $state(false);
  let aboutOpen = $state(false);

  // Bumped from the command palette / Ctrl+N to ask EntryList to open its
  // create dialog. A monotonic counter lets the child react on every press
  // without us having to clear an explicit flag.
  let createTrigger = $state(0);

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
    // Cloud-only vaults have no dbPath but still need their groups reloaded
    // after a switch or push/pull — gate on cloudVaultId OR dbPath.
    if (appState.dbPath || appState.cloudVaultId) {
      getGroups().then((g) => (rootGroup = g)).catch(() => undefined);
      if (isFavoritesView) {
        getFavoriteEntries().then((e) => (favoriteEntries = e)).catch(() => undefined);
      }
      if (isAllView && rootGroup) {
        collectEntries(rootGroup).then((e) => (allEntries = e)).catch(() => undefined);
      }
    }
  });

  // External DB-change handler. The Rust side runs a `notify` watcher and
  // emits `database-external-change` whenever the watched file or its parent
  // directory changes. We re-verify the change via check_database_changes()
  // (mtime diff) because some FS events fire for our own writes too.
  //
  // If a change is real and we're not dirty, silent-merge in the background.
  // KeePass merges by UUID; surfacing conflict UI is the auto-save path's job.
  $effect(() => {
    if (!appState.dbPath) return;
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    void listen<string>("database-external-change", async () => {
      if (cancelled) return;
      if (appState.isDirty) return; // auto-save flow handles merge during save
      try {
        const changed = await checkDatabaseChanges();
        if (!changed) return;
        appState.setSyncStatus("merging");
        await mergeDatabase();
        appState.markSynced();
        appState.refresh();
      } catch (e) {
        console.error("External change merge failed", e);
        appState.setSyncStatus("conflict");
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
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
        appState.setSyncStatus("merging");
        try {
          // Surface user-visible conflicts BEFORE a silent merge so the user
          // can decide which version to keep. If no per-entry conflict
          // (the common case — disjoint edits), proceed with newer-wins.
          const conflicts = await analyzeConflicts();
          if (conflicts.length > 0) {
            perEntryConflicts = conflicts;
            showPerEntryConflict = true;
            appState.setSyncStatus("conflict");
            return;
          }
          await mergeDatabase();
          appState.refresh();
        } catch {
          appState.setSyncStatus("conflict");
          showConflict = true;
          return;
        }
      }
      appState.setSyncStatus("saving");
      await saveDatabase();
      // If linked + active vault, the cloud push is the real persistence
      // step (cloud-only vaults have no local file to write). Awaiting it
      // here means `markClean` only fires once everything actually landed.
      const cs = await cloudStatus();
      if (cs.linked && cs.active_vault_id) {
        appState.setSyncStatus("cloud-sync");
        await cloudPush();
      }
      appState.markClean();
      appState.markSynced();
    } catch (e) {
      const held = parseLockHeldError(String(e));
      if (held) {
        appState.setSyncStatus("conflict");
        toast.error(
          "Database in use",
          `Lock held by ${held.host} (pid ${held.pid}). Auto-save will retry on next change.`,
        );
      } else {
        appState.setSyncStatus("conflict");
        toast.error("Auto-save failed", String(e));
      }
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
    if (appState.isReadOnly) {
      toast.error("Read-only", "You don't have edit permission on this vault.");
      return;
    }
    try {
      const changed = await checkDatabaseChanges();
      if (changed) {
        appState.setSyncStatus("conflict");
        showConflict = true;
        return;
      }
      appState.setSyncStatus("saving");
      await saveDatabase();
      appState.markClean();
      appState.markSynced();
      toast.success("Saved", "Database saved successfully");
    } catch (e) {
      const held = parseLockHeldError(String(e));
      appState.setSyncStatus("conflict");
      if (held) {
        toast.error("Database in use", `Lock held by ${held.host} (pid ${held.pid}).`);
      } else {
        toast.error("Save failed", String(e));
      }
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

  async function handleNewEntry() {
    if (appState.isReadOnly) {
      toast.error("Read-only", "You don't have edit permission on this vault.");
      return;
    }
    // From Home the list isn't visible — bounce to All Items first so the
    // user can see the result of the create flow.
    if (isDashboardView) {
      await selectGroup("_all");
      // Wait one tick so EntryList mounts before we bump the trigger.
      await new Promise((r) => setTimeout(r, 30));
    }
    createTrigger++;
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
      appState.setSyncStatus("merging");
      await mergeDatabase();
      appState.refresh();
      appState.setSyncStatus("saving");
      await saveDatabase();
      appState.markClean();
      appState.markSynced();
      showConflict = false;
      toast.success("Synchronized", "Database synchronized and saved");
    } catch (e) {
      appState.setSyncStatus("conflict");
      toast.error("Sync failed", String(e));
      showConflict = false;
    }
  }

  async function overwriteSave() {
    try {
      appState.setSyncStatus("saving");
      await saveDatabase();
      appState.markClean();
      appState.markSynced();
      showConflict = false;
      toast.success("Saved");
    } catch (e) {
      appState.setSyncStatus("conflict");
      toast.error("Save failed", String(e));
      showConflict = false;
    }
  }

  async function applyPerEntryDecisions(decisions: Record<string, ConflictChoice>) {
    try {
      appState.setSyncStatus("merging");
      await resolveConflicts(decisions);
      appState.refresh();
      appState.setSyncStatus("saving");
      await saveDatabase();
      appState.markClean();
      appState.markSynced();
      showPerEntryConflict = false;
      perEntryConflicts = [];
      toast.success("Conflicts resolved", "Database synchronized");
    } catch (e) {
      appState.setSyncStatus("conflict");
      toast.error("Resolve failed", String(e));
    }
  }

  function cancelPerEntryConflict() {
    showPerEntryConflict = false;
    perEntryConflicts = [];
    // Status stays "conflict" until next successful save attempt — the pill
    // keeps showing the user there's still unresolved divergence.
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
    } else if (mod && e.key.toLowerCase() === "n") {
      if (isEditable(e.target)) return;
      e.preventDefault();
      void handleNewEntry();
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
    { id: "act-new-entry", label: "New entry", section: "Actions", icon: KeyRound, keywords: "create add new entry password", hint: "Ctrl N", onSelect: handleNewEntry },
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
        onVaultSwitched={async () => {
          // The Rust state.database has already been swapped to the new
          // vault's in-memory KDBX; we just reload the UI off it.
          await load();
          selectedEntryUuid = "";
          appState.refresh();
        }}
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
              {createTrigger}
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

<ConflictResolutionDialog
  bind:open={showPerEntryConflict}
  conflicts={perEntryConflicts}
  onResolve={applyPerEntryDecisions}
  onCancel={cancelPerEntryConflict}
/>
