<script lang="ts">
  import {
    Button, Dialog, Input, Label, DropdownMenu, DropdownItem, DropdownSeparator,
    ContextMenu, IconButton, SearchInput, Tooltip, EmptyState, toast,
  } from "$lib/ui";
  import IconPicker from "./IconPicker.svelte";
  import DynamicIcon from "./DynamicIcon.svelte";
  import {
    Plus, Copy, Trash2, Star, MoreHorizontal, ExternalLink, ArrowUpDown,
    CheckSquare, X, Check, GripVertical, KeyRound,
  } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { copyWithFeedback } from "$lib/clipboard";
  import { openUrl } from "$lib/url";
  import { getEntries, createEntry, deleteEntry, updateEntry, type EntryData } from "$lib/tauri";
  import { undoStack } from "$lib/undo-stack.svelte";
  import { appState } from "$lib/app-state.svelte";

  type SortKey = "title" | "username" | "modified" | "created";
  type SortDir = "asc" | "desc";

  type Props = {
    groupUuid: string;
    searchResults: EntryData[];
    onRefresh: () => void | Promise<void>;
    isSearching?: boolean;
    isFavoritesView?: boolean;
    rootGroupUuid?: string;
    selectedGroupName?: string;
    selectedEntryUuid?: string | null;
    // Monotonic counter bumped from MainApp (palette / Ctrl+N) to ask us
    // to open the create dialog. Each bump triggers a single open.
    createTrigger?: number;
    onSelectEntry: (uuid: string) => void;
    onEntryCreated?: (uuid: string) => void;
  };

  let {
    groupUuid,
    searchResults,
    onRefresh,
    isSearching = false,
    isFavoritesView = false,
    rootGroupUuid,
    selectedGroupName,
    selectedEntryUuid = null,
    createTrigger = 0,
    onSelectEntry,
    onEntryCreated,
  }: Props = $props();

  let entries = $state<EntryData[]>([]);
  let showCreate = $state(false);
  let newTitle = $state("");
  let newIconId = $state(0);

  let filter = $state("");
  let sortKey = $state<SortKey>("title");
  let sortDir = $state<SortDir>("asc");

  let selection = $state<Set<string>>(new Set());

  async function loadEntries() {
    if (!groupUuid) {
      entries = [];
      return;
    }
    try {
      entries = await getEntries(groupUuid);
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  $effect(() => {
    // Re-fetch whenever anything triggered a refresh (move, edit, remote merge).
    void appState.refreshCounter;
    if (isSearching) {
      entries = searchResults;
    } else if (groupUuid) {
      void loadEntries();
    } else {
      entries = [];
    }
  });

  $effect(() => {
    void groupUuid;
    void isFavoritesView;
    selection = new Set();
  });

  // React to parent-initiated "open create dialog" requests. The first
  // value (0) is the initial mount — only respond to subsequent bumps.
  let lastCreateTrigger = -1;
  $effect(() => {
    if (createTrigger === lastCreateTrigger) return;
    const seen = lastCreateTrigger;
    lastCreateTrigger = createTrigger;
    if (seen === -1) return; // skip initial mount
    if (canCreate) showCreate = true;
  });

  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    let rows = entries;
    if (q) {
      rows = rows.filter((e) =>
        [e.title, e.username, e.url, e.tags].some((v) => (v ?? "").toLowerCase().includes(q)),
      );
    }
    const dir = sortDir === "asc" ? 1 : -1;
    const sorted = [...rows].sort((a, b) => {
      let av: string | number = "";
      let bv: string | number = "";
      switch (sortKey) {
        case "title":
          av = (a.title || "").toLowerCase();
          bv = (b.title || "").toLowerCase();
          break;
        case "username":
          av = (a.username || "").toLowerCase();
          bv = (b.username || "").toLowerCase();
          break;
        case "modified":
          av = a.modified ?? "";
          bv = b.modified ?? "";
          break;
        case "created":
          av = a.created ?? "";
          bv = b.created ?? "";
          break;
      }
      if (av < bv) return -1 * dir;
      if (av > bv) return 1 * dir;
      return 0;
    });
    return sorted;
  });

  function toggleSort(key: SortKey) {
    if (sortKey === key) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortKey = key;
      sortDir = "asc";
    }
  }

  function toggleSelect(uuid: string, e?: MouseEvent) {
    e?.stopPropagation();
    const next = new Set(selection);
    if (next.has(uuid)) next.delete(uuid);
    else next.add(uuid);
    selection = next;
  }

  function toggleSelectAll() {
    if (selection.size === filtered.length) selection = new Set();
    else selection = new Set(filtered.map((e) => e.uuid));
  }

  async function bulkDelete() {
    const count = selection.size;
    if (count === 0) return;
    const ok = await ask(
      `Delete ${count} ${count === 1 ? "entry" : "entries"}? Ctrl+Z will revert.`,
      { kind: "warning", title: "Delete Entries" },
    );
    if (!ok) return;
    const snap = entries.filter((e) => selection.has(e.uuid)).map((e) => ({ ...e }));
    try {
      await Promise.all(snap.map((e) => deleteEntry(e.uuid)));
      undoStack.add(
        `Delete ${count} ${count === 1 ? "entry" : "entries"}`,
        async () => { await Promise.all(snap.map((e) => createEntry(e))); },
        async () => { await Promise.all(snap.map((e) => deleteEntry(e.uuid))); },
      );
      selection = new Set();
      toast.success("Deleted", `${count} ${count === 1 ? "entry" : "entries"} removed`);
      await onRefresh();
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  // Resolve where to drop a newly-created entry:
  //   folder view  → the selected folder
  //   Favorites/All → root group (favorites are virtual; All Items has no
  //                   single owning folder, so root is the sensible default)
  const createTarget = $derived(groupUuid || rootGroupUuid);
  const canCreate = $derived(Boolean(createTarget));

  async function handleCreate() {
    const target = createTarget;
    if (!newTitle.trim() || !target) return;
    try {
      const entryUuid = crypto.randomUUID();
      const newEntry: EntryData = {
        uuid: entryUuid,
        title: newTitle,
        username: "",
        password: "",
        url: "",
        notes: "",
        tags: "",
        group_uuid: target,
        icon_id: newIconId,
        is_favorite: isFavoritesView,
        expires: false,
        usage_count: 0,
        custom_fields: [],
        history: [],
      };
      await createEntry(newEntry);
      undoStack.add(
        `Create entry "${newEntry.title}"`,
        async () => { await deleteEntry(entryUuid); },
        async () => { await createEntry(newEntry); },
      );
      toast.success("Entry created");
      showCreate = false;
      newTitle = "";
      newIconId = 0;
      await onRefresh();
      await new Promise((r) => setTimeout(r, 30));
      onEntryCreated?.(entryUuid);
      onSelectEntry(entryUuid);
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  async function handleDelete(entry: EntryData) {
    const ok = await ask(`Are you sure you want to delete "${entry.title}"?`, {
      kind: "warning",
      title: "Delete Entry",
    });
    if (!ok) return;
    const snap = { ...entry };
    try {
      await deleteEntry(entry.uuid);
      undoStack.add(
        `Delete entry "${snap.title}"`,
        async () => { await createEntry(snap); },
        async () => { await deleteEntry(snap.uuid); },
      );
      toast.success("Entry deleted");
      if (selectedEntryUuid === entry.uuid) onSelectEntry("");
      await onRefresh();
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  async function toggleFavorite(entry: EntryData) {
    const prev = entry.is_favorite;
    const next = !prev;
    try {
      await updateEntry({ ...entry, is_favorite: next });
      undoStack.add(
        `${next ? "Favorite" : "Unfavorite"} "${entry.title}"`,
        async () => { await updateEntry({ ...entry, is_favorite: prev }); },
        async () => { await updateEntry({ ...entry, is_favorite: next }); },
      );
      await onRefresh();
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  const allChecked = $derived(filtered.length > 0 && selection.size === filtered.length);

  // Deterministic tile tint per entry — keeps the list colorful like 1P,
  // but stays soft and tinted (no garish full-saturation colors).
  const tints = [
    "bg-blue-500/12 text-blue-600 dark:text-blue-300",
    "bg-violet-500/12 text-violet-600 dark:text-violet-300",
    "bg-emerald-500/12 text-emerald-600 dark:text-emerald-300",
    "bg-amber-500/12 text-amber-600 dark:text-amber-300",
    "bg-rose-500/12 text-rose-600 dark:text-rose-300",
    "bg-sky-500/12 text-sky-600 dark:text-sky-300",
    "bg-teal-500/12 text-teal-600 dark:text-teal-300",
    "bg-indigo-500/12 text-indigo-600 dark:text-indigo-300",
  ];
  function tileTint(seed: string): string {
    let h = 0;
    for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) >>> 0;
    return tints[h % tints.length];
  }
</script>

{#snippet entryMenuItems(entry: EntryData)}
  <DropdownItem onSelect={() => copyWithFeedback(entry.username, "Username")} disabled={!entry.username}>
    <Copy class="size-4" />
    <span>Copy username</span>
  </DropdownItem>
  <DropdownItem onSelect={() => copyWithFeedback(entry.password, "Password")} disabled={!entry.password}>
    <Copy class="size-4" />
    <span>Copy password</span>
  </DropdownItem>
  {#if entry.url}
    <DropdownItem onSelect={() => openUrl(entry.url)}>
      <ExternalLink class="size-4" />
      <span>Open URL</span>
    </DropdownItem>
  {/if}
  <DropdownItem onSelect={() => toggleFavorite(entry)}>
    <Star class="size-4" />
    <span>{entry.is_favorite ? "Unfavorite" : "Favorite"}</span>
  </DropdownItem>
  <DropdownSeparator />
  <DropdownItem destructive onSelect={() => handleDelete(entry)}>
    <Trash2 class="size-4" />
    <span>Delete</span>
  </DropdownItem>
{/snippet}

<div class="flex h-full flex-col bg-list">
  <div class="shrink-0">
    <div class="flex items-center justify-between px-4 pt-4 pb-2 gap-2">
      <div class="min-w-0 flex-1">
        <h2 class="text-md font-semibold tracking-tight truncate" title={selectedGroupName ?? ""}>
          {#if isFavoritesView}
            Favorites
          {:else if selectedGroupName}
            {selectedGroupName}
          {:else}
            All Items
          {/if}
        </h2>
        <p class="text-2xs text-muted-foreground mt-0.5">
          {filtered.length}
          {filtered.length === 1 ? "item" : "items"}{filter ? ` · filtered from ${entries.length}` : ""}
        </p>
      </div>
      <div class="flex items-center gap-0.5 shrink-0">
        <DropdownMenu align="end">
          {#snippet trigger()}
            <Tooltip label="Sort">
              <IconButton aria-label="Sort">
                <ArrowUpDown class="size-3.5" />
              </IconButton>
            </Tooltip>
          {/snippet}
          <DropdownItem onSelect={() => toggleSort("title")}>
            {#if sortKey === "title"}{sortDir === "asc" ? "↑" : "↓"}{/if}
            <span>Title</span>
          </DropdownItem>
          <DropdownItem onSelect={() => toggleSort("username")}>
            {#if sortKey === "username"}{sortDir === "asc" ? "↑" : "↓"}{/if}
            <span>Username</span>
          </DropdownItem>
          <DropdownItem onSelect={() => toggleSort("modified")}>
            {#if sortKey === "modified"}{sortDir === "asc" ? "↑" : "↓"}{/if}
            <span>Modified</span>
          </DropdownItem>
          <DropdownItem onSelect={() => toggleSort("created")}>
            {#if sortKey === "created"}{sortDir === "asc" ? "↑" : "↓"}{/if}
            <span>Created</span>
          </DropdownItem>
        </DropdownMenu>

        {#if canCreate}
          <Tooltip label="New item" shortcut="Ctrl N">
            <IconButton
              tone="primary"
              onclick={() => (showCreate = true)}
              aria-label="New item"
            >
              <Plus />
            </IconButton>
          </Tooltip>
        {/if}
      </div>
    </div>

    <div class="px-4 pb-3">
      <SearchInput bind:value={filter} placeholder="Filter…" />
    </div>

    {#if selection.size > 0}
      <div class="flex items-center gap-2 border-y border-primary/15 bg-primary/5 px-4 py-2">
        <CheckSquare class="size-4 text-primary" />
        <span class="text-xs font-medium">{selection.size} selected</span>
        <button
          type="button"
          class="text-xs text-muted-foreground hover:text-foreground"
          onclick={toggleSelectAll}
        >
          {allChecked ? "Deselect all" : "Select all"}
        </button>
        <div class="flex-1"></div>
        <Button variant="destructive" size="sm" onclick={bulkDelete}>
          <Trash2 class="size-3.5" />
          Delete
        </Button>
        <Button variant="ghost" size="sm" onclick={() => (selection = new Set())}>
          <X class="size-3.5" />
        </Button>
      </div>
    {/if}
  </div>

  <ContextMenu>
    {#snippet trigger({ props })}
      <div {...props} class="flex-1 overflow-y-auto">
    {#if filtered.length === 0}
      {#if entries.length === 0}
        <!-- Empty folder / view — encourage action with a CTA. -->
        <EmptyState
          tone="prompt"
          icon={isFavoritesView ? Star : KeyRound}
          title={isFavoritesView ? "No favorites yet" : groupUuid ? "Nothing here yet" : "Pick a folder"}
          description={isFavoritesView
            ? "Star an entry from any folder to pin it here for quick access."
            : groupUuid
              ? "Add your first entry to start organising credentials in this folder."
              : "Choose a folder from the sidebar to see its items."}
        >
          {#if canCreate}
            <Button size="sm" onclick={() => (showCreate = true)}>
              <Plus class="size-3.5" />
              New item
            </Button>
          {/if}
        </EmptyState>
      {:else}
        <!-- Text-only no-results state — illustrations grow stale here. -->
        <EmptyState
          tone="no-results"
          title="No matches"
          description={`Nothing matches "${filter}".`}
        >
          <button
            type="button"
            class="text-2xs text-primary hover:underline"
            onclick={() => (filter = "")}
          >
            Clear filter
          </button>
        </EmptyState>
      {/if}
    {:else}
      <ul class="px-2 py-1.5">
        {#each filtered as entry (entry.uuid)}
          {@const iconId = entry.icon_id ?? 0}
          {@const isSelected = selectedEntryUuid === entry.uuid}
          {@const isChecked = selection.has(entry.uuid)}
          {@const tint = tileTint(entry.uuid)}
          {@const selectionActive = selection.size > 0}
          <li>
            <ContextMenu>
            {#snippet trigger({ props })}
            <div
              {...props}
              role="button"
              tabindex="0"
              draggable={true}
              ondragstart={(e: DragEvent) => {
                if (!e.dataTransfer) return;
                e.dataTransfer.setData("application/x-pw-entry", entry.uuid);
                e.dataTransfer.effectAllowed = "move";
                window.__pwLastDraggedEntryGroup = entry.group_uuid;
              }}
              ondragend={() => {
                window.__pwLastDraggedEntryGroup = null;
              }}
              class="group/row w-full flex items-center gap-3 px-2 py-2 my-0.5 rounded-lg text-left transition-colors cursor-pointer active:cursor-grabbing {isSelected
                ? 'bg-selected text-selected-foreground'
                : isChecked
                  ? 'bg-accent/70'
                  : 'hover:bg-accent/50'}"
              onclick={() => onSelectEntry(entry.uuid)}
              onkeydown={(e: KeyboardEvent) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  onSelectEntry(entry.uuid);
                }
              }}
            >
              <!--
                Leftmost icon/checkbox tile — Gmail-style.
                Default shows the entry's icon; hover or selection-mode
                swaps to a checkbox; click toggles selection (doesn't open).
              -->
              <button
                type="button"
                onclick={(e: MouseEvent) => toggleSelect(entry.uuid, e)}
                aria-label={isChecked ? "Deselect" : "Select"}
                title={isChecked ? "Deselect" : "Select"}
                class="relative grid place-items-center size-9 rounded-lg shrink-0 transition-colors {isChecked
                  ? 'bg-primary text-primary-foreground'
                  : selectionActive
                    ? 'bg-muted text-muted-foreground hover:bg-primary/15 hover:text-primary'
                    : tint + ' hover:bg-primary/15 hover:text-primary'}"
              >
                {#if isChecked}
                  <Check class="size-4" />
                {:else if selectionActive}
                  <span class="size-4 rounded-[4px] border-2 border-current"></span>
                {:else}
                  <!-- Default: icon visible, swaps to empty checkbox on row-hover -->
                  <DynamicIcon iconId={iconId} class="size-[18px] group-hover/row:opacity-0 transition-opacity" />
                  <span class="absolute inset-0 grid place-items-center opacity-0 group-hover/row:opacity-100 transition-opacity">
                    <span class="size-4 rounded-[4px] border-2 border-current"></span>
                  </span>
                {/if}
              </button>

              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-1.5">
                  <span class="text-sm font-semibold truncate">{entry.title || "(untitled)"}</span>
                  {#if entry.is_favorite}
                    <Star class="size-3 text-warning fill-warning shrink-0" />
                  {/if}
                </div>
                {#if entry.username}
                  <div class="text-xs truncate text-muted-foreground mt-0.5">{entry.username}</div>
                {:else if entry.url}
                  <div class="text-xs truncate text-muted-foreground mt-0.5">{entry.url}</div>
                {/if}
              </div>

              <div class="flex items-center gap-0.5 opacity-60 group-hover/row:opacity-100 data-[state=open]:opacity-100 transition-opacity shrink-0">
                <!-- Drag handle indicator (visual cue only; the whole row is the drag source). -->
                <span class="text-muted-foreground/50 px-0.5" title="Drag to move">
                  <GripVertical class="size-4" />
                </span>
                <IconButton
                  size="sm"
                  tone="soft"
                  title="Copy password"
                  disabled={!entry.password}
                  onclick={(e: MouseEvent) => {
                    e.stopPropagation();
                    copyWithFeedback(entry.password, "Password");
                  }}
                >
                  <Copy />
                </IconButton>
                <DropdownMenu align="end">
                  {#snippet trigger()}
                    <IconButton
                      size="sm"
                      tone="soft"
                      aria-label="Entry actions"
                      onclick={(e: MouseEvent) => e.stopPropagation()}
                    >
                      <MoreHorizontal />
                    </IconButton>
                  {/snippet}
                  {@render entryMenuItems(entry)}
                </DropdownMenu>
              </div>
            </div>
            {/snippet}
            {@render entryMenuItems(entry)}
            </ContextMenu>
          </li>
        {/each}
      </ul>
    {/if}
      </div>
    {/snippet}
    {#if canCreate}
      <DropdownItem onSelect={() => (showCreate = true)}>
        <Plus class="size-4" />
        <span>New entry</span>
      </DropdownItem>
    {/if}
  </ContextMenu>
</div>

<Dialog bind:open={showCreate} title="Create New Item" description="Add a new entry to this folder">
  <div class="space-y-2">
    <Label for="entryTitle">Title</Label>
    <div class="flex gap-2">
      <IconPicker value={newIconId} onChange={(id) => (newIconId = id)} />
      <!-- svelte-ignore a11y_autofocus -->
      <Input
        id="entryTitle"
        bind:value={newTitle}
        placeholder="Entry title"
        class="flex-1"
        onkeydown={(e) => e.key === "Enter" && handleCreate()}
        autofocus
      />
    </div>
  </div>
  {#snippet footer()}
    <Button variant="outline" onclick={() => (showCreate = false)}>Cancel</Button>
    <Button onclick={handleCreate}>Create</Button>
  {/snippet}
</Dialog>
