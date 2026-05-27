<script lang="ts">
  import { Button, Dialog, Input, Label, DropdownMenu, DropdownItem, DropdownSeparator, toast } from "$lib/ui";
  import IconPicker from "./IconPicker.svelte";
  import DynamicIcon from "./DynamicIcon.svelte";
  import {
    Plus,
    Copy,
    Trash2,
    Star,
    MoreHorizontal,
    ExternalLink,
    Search,
    ArrowUpDown,
    ArrowDown,
    ArrowUp,
    CheckSquare,
    X,
  } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { open as openShell } from "@tauri-apps/plugin-shell";
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
    if (isSearching) {
      entries = searchResults;
    } else if (groupUuid) {
      void loadEntries();
    } else {
      entries = [];
    }
  });

  // Reset selection when leaving the current view
  $effect(() => {
    void groupUuid;
    void isFavoritesView;
    selection = new Set();
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
      `Delete ${count} ${count === 1 ? "entry" : "entries"}? This cannot be undone via the Tauri prompt, but Ctrl+Z will revert it.`,
      { kind: "warning", title: "Delete Entries" },
    );
    if (!ok) return;
    const snap = entries.filter((e) => selection.has(e.uuid)).map((e) => ({ ...e }));
    try {
      await Promise.all(snap.map((e) => deleteEntry(e.uuid)));
      undoStack.add(
        `Delete ${count} ${count === 1 ? "entry" : "entries"}`,
        async () => {
          await Promise.all(snap.map((e) => createEntry(e)));
        },
        async () => {
          await Promise.all(snap.map((e) => deleteEntry(e.uuid)));
        },
      );
      selection = new Set();
      toast.success("Deleted", `${count} ${count === 1 ? "entry" : "entries"} removed`);
      await onRefresh();
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  async function handleCreate() {
    const target = isFavoritesView ? rootGroupUuid : groupUuid;
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
        async () => {
          await deleteEntry(entryUuid);
        },
        async () => {
          await createEntry(newEntry);
        },
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

  async function copy(text: string, label: string) {
    if (!text) return;
    try {
      await writeText(text);
      toast.success("Copied", `${label} copied to clipboard`);
      setTimeout(() => void writeText(""), 30000);
    } catch {
      toast.error("Copy failed");
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
        async () => {
          await createEntry(snap);
        },
        async () => {
          await deleteEntry(snap.uuid);
        },
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
        async () => {
          await updateEntry({ ...entry, is_favorite: prev });
        },
        async () => {
          await updateEntry({ ...entry, is_favorite: next });
        },
      );
      await onRefresh();
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  async function openUrl(url: string) {
    if (!url) return;
    try {
      const full = url.match(/^https?:\/\//) ? url : `https://${url}`;
      await openShell(full);
    } catch {
      toast.error("Failed to open URL");
    }
  }

  const allChecked = $derived(filtered.length > 0 && selection.size === filtered.length);
</script>

<div class="flex h-full flex-col bg-list">
  <!-- Header -->
  <div class="shrink-0 border-b">
    <div class="flex items-center justify-between px-3 pt-3 pb-2 gap-2">
      <div class="min-w-0 flex-1">
        <h2 class="text-sm font-semibold truncate" title={selectedGroupName ?? ""}>
          {#if isFavoritesView}
            Favorites
          {:else if selectedGroupName}
            {selectedGroupName}
          {:else}
            Entries
          {/if}
        </h2>
        <p class="text-[11px] text-muted-foreground">
          {filtered.length}
          {filtered.length === 1 ? "item" : "items"}{filter ? ` · filtered from ${entries.length}` : ""}
        </p>
      </div>
      <div class="flex items-center gap-1 shrink-0">
        <DropdownMenu align="end">
          {#snippet trigger()}
            <button
              type="button"
              class="size-7 inline-flex items-center justify-center rounded hover:bg-accent text-muted-foreground"
              aria-label="Sort"
              title="Sort"
            >
              <ArrowUpDown class="h-3.5 w-3.5" />
            </button>
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

        {#if !isSearching && (groupUuid || isFavoritesView)}
          <Button variant="ghost" size="icon" class="h-7 w-7" onclick={() => (showCreate = true)} title="New entry">
            <Plus class="h-4 w-4" />
          </Button>
        {/if}
      </div>
    </div>

    <!-- Filter -->
    <div class="px-3 pb-3">
      <div class="relative">
        <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
        <input
          type="text"
          bind:value={filter}
          placeholder="Filter…"
          class="h-8 w-full rounded-md border border-input bg-background/60 pl-8 pr-7 text-xs outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]"
        />
        {#if filter}
          <button
            type="button"
            onclick={() => (filter = "")}
            aria-label="Clear filter"
            class="absolute right-1.5 top-1/2 -translate-y-1/2 size-5 inline-flex items-center justify-center rounded hover:bg-accent text-muted-foreground"
          >
            <X class="size-3" />
          </button>
        {/if}
      </div>
    </div>

    {#if selection.size > 0}
      <div class="flex items-center gap-2 border-t bg-primary/5 px-3 py-2">
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

  <!-- List -->
  <div class="flex-1 overflow-y-auto">
    {#if filtered.length === 0}
      <div class="flex h-32 items-center justify-center text-sm text-muted-foreground text-center px-6">
        {#if entries.length === 0}
          {isFavoritesView ? "No favorites yet" : groupUuid ? "No entries here yet" : "Select a folder to view entries"}
        {:else}
          No entries match "{filter}"
        {/if}
      </div>
    {:else}
      <ul class="py-1">
        {#each filtered as entry (entry.uuid)}
          {@const iconId = entry.icon_id ?? 0}
          {@const isSelected = selectedEntryUuid === entry.uuid}
          {@const isChecked = selection.has(entry.uuid)}
          <li>
            <div
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
              class="group/row w-full flex items-center gap-3 mx-2 my-0.5 px-2 py-2 rounded-md text-left transition-colors cursor-pointer {isSelected
                ? 'bg-primary text-primary-foreground'
                : isChecked
                  ? 'bg-accent'
                  : 'hover:bg-accent/60'}"
              onclick={() => onSelectEntry(entry.uuid)}
              onkeydown={(e: KeyboardEvent) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  onSelectEntry(entry.uuid);
                }
              }}
            >
              <span
                class="size-4 grid place-items-center shrink-0 {selection.size > 0 || isChecked
                  ? 'opacity-100'
                  : 'opacity-0 group-hover/row:opacity-100'} transition-opacity"
                onclick={(e: MouseEvent) => toggleSelect(entry.uuid, e)}
                role="presentation"
              >
                <span
                  class="size-3.5 rounded-[3px] border {isChecked
                    ? 'bg-primary border-primary'
                    : 'border-input bg-background'} flex items-center justify-center"
                >
                  {#if isChecked}
                    <svg viewBox="0 0 24 24" class="size-2.5 text-primary-foreground" fill="none" stroke="currentColor" stroke-width="3">
                      <path d="M5 12l5 5L20 7" />
                    </svg>
                  {/if}
                </span>
              </span>

              <span
                class="grid place-items-center size-9 rounded-lg shrink-0 {isSelected
                  ? 'bg-primary-foreground/20 text-primary-foreground'
                  : 'bg-muted text-muted-foreground'}"
              >
                <DynamicIcon iconId={iconId} class="size-4" />
              </span>

              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-1.5">
                  <span class="text-sm font-medium truncate">{entry.title || "(untitled)"}</span>
                  {#if entry.is_favorite}
                    <Star class="h-3 w-3 {isSelected ? 'text-primary-foreground' : 'text-warning fill-warning'} shrink-0" />
                  {/if}
                </div>
                {#if entry.username}
                  <div class="text-xs truncate {isSelected ? 'text-primary-foreground/80' : 'text-muted-foreground'}">{entry.username}</div>
                {/if}
              </div>

              <div
                class="flex items-center gap-0.5 opacity-0 group-hover/row:opacity-100 data-[state=open]:opacity-100 transition-opacity shrink-0"
              >
                <button
                  type="button"
                  class="size-7 inline-flex items-center justify-center rounded {isSelected ? 'hover:bg-primary-foreground/20 text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}"
                  title="Copy password"
                  disabled={!entry.password}
                  onclick={(e: MouseEvent) => {
                    e.stopPropagation();
                    copy(entry.password, "Password");
                  }}
                >
                  <Copy class="h-3.5 w-3.5" />
                </button>
                <DropdownMenu align="end">
                  {#snippet trigger()}
                    <button
                      type="button"
                      class="size-7 inline-flex items-center justify-center rounded {isSelected ? 'hover:bg-primary-foreground/20 text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}"
                      aria-label="Entry actions"
                      onclick={(e: MouseEvent) => e.stopPropagation()}
                    >
                      <MoreHorizontal class="h-3.5 w-3.5" />
                    </button>
                  {/snippet}
                  <DropdownItem onSelect={() => copy(entry.username, "Username")} disabled={!entry.username}>
                    <Copy class="h-4 w-4" />
                    <span>Copy username</span>
                  </DropdownItem>
                  <DropdownItem onSelect={() => copy(entry.password, "Password")} disabled={!entry.password}>
                    <Copy class="h-4 w-4" />
                    <span>Copy password</span>
                  </DropdownItem>
                  {#if entry.url}
                    <DropdownItem onSelect={() => openUrl(entry.url)}>
                      <ExternalLink class="h-4 w-4" />
                      <span>Open URL</span>
                    </DropdownItem>
                  {/if}
                  <DropdownItem onSelect={() => toggleFavorite(entry)}>
                    <Star class="h-4 w-4" />
                    <span>{entry.is_favorite ? "Unfavorite" : "Favorite"}</span>
                  </DropdownItem>
                  <DropdownSeparator />
                  <DropdownItem destructive onSelect={() => handleDelete(entry)}>
                    <Trash2 class="h-4 w-4" />
                    <span>Delete</span>
                  </DropdownItem>
                </DropdownMenu>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<Dialog bind:open={showCreate} title="Create New Entry" description="Add a new entry to this group">
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
