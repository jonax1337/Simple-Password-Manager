<script lang="ts">
  import { onMount } from "svelte";
  import {
    getGroups,
    getFavoriteEntries,
    saveDatabase,
    closeDatabase,
    searchEntries,
    searchEntriesInGroup,
    checkDatabaseChanges,
    mergeDatabase,
    type GroupData,
    type EntryData,
  } from "$lib/tauri";
  import {
    getSearchScope,
    saveSearchScope,
    getLiveUpdates,
    getCloseToTray,
    type SearchScope,
  } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import GroupTree from "$lib/components/GroupTree.svelte";
  import EntryList from "$lib/components/EntryList.svelte";
  import Dashboard from "$lib/components/Dashboard.svelte";
  import SplitPane from "$lib/components/SplitPane.svelte";
  import UnsavedChangesDialog from "$lib/components/UnsavedChangesDialog.svelte";
  import DatabaseConflictDialog from "$lib/components/DatabaseConflictDialog.svelte";
  import { Button, Input, toast } from "$lib/ui";
  import { Save, LogOut, Search, X, Settings, Info } from "@lucide/svelte";
  import { loadGroupTreeState } from "$lib/group-state";
  import { getGroupPath } from "$lib/group-utils";
  import { push } from "svelte-spa-router";

  type Props = { onClose: (isManualLogout?: boolean) => void };
  let { onClose }: Props = $props();

  let rootGroup = $state<GroupData | null>(null);
  let selectedUuid = $state<string>("_dashboard");
  let favoriteEntries = $state<EntryData[]>([]);
  let isFavoritesView = $state(false);
  let isDashboardView = $state(true);

  let searchQuery = $state("");
  let searchResults = $state<EntryData[]>([]);
  let isSearching = $state(false);
  let isSearchVisible = $state(false);
  let searchScope = $state<SearchScope>("global");

  let showUnsaved = $state(false);
  let closeAction = $state<"logout" | "window" | null>(null);
  let showConflict = $state(false);
  let liveUpdatesEnabled = $state(false);

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
      searchScope = getSearchScope(appState.dbPath);
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

  // Live updates polling
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
    isSearching = false;
    searchQuery = "";
    searchResults = [];
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

  async function performClose(manual: boolean) {
    try {
      const win = getCurrentWindow();
      await win.setTitle("Simple Password Manager");
      selectedUuid = "";
      isDashboardView = true;
      isFavoritesView = false;
      rootGroup = null;
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

  async function handleSearch(q: string) {
    searchQuery = q;
    if (!q.trim()) {
      isSearching = false;
      searchResults = [];
      return;
    }
    isSearching = true;
    try {
      const canScopeToFolder = !isDashboardView && !isFavoritesView && selectedUuid !== "";
      let results: EntryData[];
      if (searchScope === "folder" && canScopeToFolder) {
        results = await searchEntriesInGroup(q, selectedUuid);
      } else {
        results = await searchEntries(q);
      }
      if (isFavoritesView && searchScope === "folder") {
        results = results.filter((e) => e.is_favorite);
      }
      searchResults = results;
    } catch (e) {
      toast.error("Search failed", String(e));
    }
  }

  function toggleSearch() {
    isSearchVisible = !isSearchVisible;
    if (!isSearchVisible) {
      searchQuery = "";
      searchResults = [];
      isSearching = false;
    }
  }

  // Conflict handling
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

  // Keyboard shortcuts
  function onKey(e: KeyboardEvent) {
    const isMac = /Mac|iPhone|iPad/i.test(navigator.platform);
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (mod && e.key === "s") {
      e.preventDefault();
      void handleSave();
    } else if (mod && e.key === "f") {
      e.preventDefault();
      toggleSearch();
    } else if (e.key === "Escape" && isSearchVisible) {
      toggleSearch();
    }
  }

  const groupName = $derived(
    isSearching
      ? undefined
      : isFavoritesView
        ? "Favorites"
        : rootGroup && selectedUuid && !selectedUuid.startsWith("_")
          ? getGroupPath(rootGroup, selectedUuid)
          : undefined,
  );

  function onRefresh() {
    appState.markDirty();
    appState.refresh();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-full w-full flex-col">
  <!-- Toolbar -->
  <div class="shrink-0 flex items-center justify-between border-b px-3 py-1.5 bg-card/40 gap-2">
    <div class="flex items-center gap-1">
      <Button variant="ghost" size="sm" onclick={handleSave} disabled={!appState.isDirty} title="Save (Ctrl+S)">
        <Save class="h-4 w-4" />
        <span class="hidden md:inline">Save</span>
        {#if appState.isDirty}<span class="text-warning">•</span>{/if}
      </Button>
      <Button variant="ghost" size="sm" onclick={toggleSearch} title="Search (Ctrl+F)">
        <Search class="h-4 w-4" />
      </Button>
    </div>

    {#if isSearchVisible}
      <div class="flex-1 max-w-md flex items-center gap-2">
        <!-- svelte-ignore a11y_autofocus -->
        <Input
          placeholder="Search entries…"
          value={searchQuery}
          oninput={(e) => handleSearch(e.currentTarget.value)}
          class="h-8"
          autofocus
        />
        <Button variant="ghost" size="icon" class="h-7 w-7" onclick={toggleSearch}>
          <X class="h-4 w-4" />
        </Button>
      </div>
    {/if}

    <div class="flex items-center gap-1">
      <Button variant="ghost" size="sm" onclick={() => push("/settings")} title="Settings">
        <Settings class="h-4 w-4" />
      </Button>
      <Button variant="ghost" size="sm" onclick={() => push("/about")} title="About">
        <Info class="h-4 w-4" />
      </Button>
      <Button variant="ghost" size="sm" onclick={handleLogout} title="Logout">
        <LogOut class="h-4 w-4" />
      </Button>
    </div>
  </div>

  <SplitPane defaultWidth={260} minWidth={200} maxWidth={500} storageKey="groupTreeWidth">
    {#snippet left()}
      {#if rootGroup}
        <GroupTree
          group={rootGroup}
          {selectedUuid}
          onSelectGroup={selectGroup}
          onRefresh={() => appState.refresh()}
          onGroupDeleted={(deleted) => {
            if (selectedUuid === deleted && rootGroup) selectGroup(rootGroup.uuid);
          }}
          dbPath={appState.dbPath}
          initialExpandedGroups={initialExpanded}
        />
      {/if}
    {/snippet}

    {#snippet right()}
      {#if isDashboardView && !isSearching}
        <Dashboard refreshTrigger={appState.refreshCounter} />
      {:else}
        <EntryList
          groupUuid={isSearching || isFavoritesView ? "" : selectedUuid}
          searchResults={isSearching ? searchResults : isFavoritesView ? favoriteEntries : []}
          onRefresh={onRefresh}
          isSearching={isSearching || isFavoritesView}
          {isFavoritesView}
          rootGroupUuid={rootGroup?.uuid}
          selectedGroupName={groupName}
        />
      {/if}
    {/snippet}
  </SplitPane>
</div>

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
