<script lang="ts">
  import { Button, Dialog, Input, Label, DropdownMenu, DropdownItem, DropdownSeparator, toast } from "$lib/ui";
  import IconPicker from "./IconPicker.svelte";
  import DynamicIcon from "./DynamicIcon.svelte";
  import { Plus, Copy, Edit2, Trash2, Star, MoreHorizontal, ExternalLink, Globe } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { open as openShell } from "@tauri-apps/plugin-shell";
  import { getEntries, createEntry, deleteEntry, updateEntry, type EntryData } from "$lib/tauri";
  import { formatTimestamp } from "$lib/entry-utils";
  import { push } from "svelte-spa-router";

  type Props = {
    groupUuid: string;
    searchResults: EntryData[];
    onRefresh: () => void | Promise<void>;
    isSearching?: boolean;
    isFavoritesView?: boolean;
    rootGroupUuid?: string;
    selectedGroupName?: string;
  };

  let {
    groupUuid,
    searchResults,
    onRefresh,
    isSearching = false,
    isFavoritesView = false,
    rootGroupUuid,
    selectedGroupName,
  }: Props = $props();

  let entries = $state<EntryData[]>([]);
  let showCreate = $state(false);
  let newTitle = $state("");
  let newIconId = $state(0);

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

  function openEditor(entry: EntryData) {
    push(`/entry/${entry.uuid}`);
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
      toast.success("Entry created");
      showCreate = false;
      newTitle = "";
      newIconId = 0;
      await onRefresh();
      await new Promise((r) => setTimeout(r, 50));
      push(`/entry/${entryUuid}`);
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
    try {
      await deleteEntry(entry.uuid);
      toast.success("Entry deleted");
      await onRefresh();
    } catch (e) {
      toast.error("Error", String(e));
    }
  }

  async function toggleFavorite(entry: EntryData) {
    try {
      await updateEntry({ ...entry, is_favorite: !entry.is_favorite });
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
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b px-4 py-2">
    <div class="min-w-0 flex-1">
      <h2 class="text-sm font-semibold truncate">
        {#if isSearching}
          Search Results
        {:else if isFavoritesView}
          Favorites
        {:else if selectedGroupName}
          {selectedGroupName}
        {:else}
          Entries
        {/if}
      </h2>
      <p class="text-xs text-muted-foreground">{entries.length} entries</p>
    </div>
    {#if !isSearching && (groupUuid || isFavoritesView)}
      <Button variant="ghost" size="icon" class="h-7 w-7" onclick={() => (showCreate = true)} title="New Entry">
        <Plus class="h-4 w-4" />
      </Button>
    {/if}
  </div>

  <div class="flex-1 overflow-y-auto">
    {#if entries.length === 0}
      <div class="flex h-32 items-center justify-center text-sm text-muted-foreground">
        {#if isSearching}
          No results found
        {:else if groupUuid || isFavoritesView}
          No entries here yet
        {:else}
          Select a group to view entries
        {/if}
      </div>
    {:else}
      <table class="w-full text-sm">
        <thead class="sticky top-0 bg-card border-b text-xs text-muted-foreground">
          <tr>
            <th class="text-left font-medium px-3 py-2 w-8"></th>
            <th class="text-left font-medium px-3 py-2">Title</th>
            <th class="text-left font-medium px-3 py-2 hidden md:table-cell">Username</th>
            <th class="text-left font-medium px-3 py-2 hidden lg:table-cell">URL</th>
            <th class="text-left font-medium px-3 py-2 hidden xl:table-cell">Modified</th>
            <th class="w-28"></th>
          </tr>
        </thead>
        <tbody>
          {#each entries as entry (entry.uuid)}
            {@const iconId = entry.icon_id ?? 0}
            <tr
              class="border-b last:border-b-0 hover:bg-accent/40 cursor-pointer group/row"
              ondblclick={() => openEditor(entry)}
            >
              <td class="px-3 py-2 align-middle">
                <DynamicIcon iconId={iconId} class="h-4 w-4 text-muted-foreground" />
              </td>
              <td class="px-3 py-2 align-middle">
                <div class="flex items-center gap-2 min-w-0">
                  <button type="button" class="truncate text-left" onclick={() => openEditor(entry)}>
                    {entry.title || "(untitled)"}
                  </button>
                  {#if entry.is_favorite}
                    <Star class="h-3 w-3 text-warning fill-warning shrink-0" />
                  {/if}
                </div>
              </td>
              <td class="px-3 py-2 align-middle hidden md:table-cell truncate max-w-[200px]">
                {entry.username || "—"}
              </td>
              <td class="px-3 py-2 align-middle hidden lg:table-cell truncate max-w-[240px] text-muted-foreground">
                {entry.url || "—"}
              </td>
              <td class="px-3 py-2 align-middle hidden xl:table-cell text-xs text-muted-foreground whitespace-nowrap">
                {formatTimestamp(entry.modified)}
              </td>
              <td class="px-3 py-2 align-middle">
                <div class="flex items-center justify-end gap-0.5 opacity-0 group-hover/row:opacity-100 transition-opacity">
                  <Button
                    variant="ghost"
                    size="icon"
                    class="h-7 w-7"
                    title="Copy username"
                    disabled={!entry.username}
                    onclick={(e: MouseEvent) => {
                      e.stopPropagation();
                      copy(entry.username, "Username");
                    }}
                  >
                    <Copy class="h-3.5 w-3.5" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    class="h-7 w-7"
                    title="Copy password"
                    disabled={!entry.password}
                    onclick={(e: MouseEvent) => {
                      e.stopPropagation();
                      copy(entry.password, "Password");
                    }}
                  >
                    <Globe class="h-3.5 w-3.5" />
                  </Button>
                  <DropdownMenu align="end">
                    {#snippet trigger()}
                      <button
                        type="button"
                        class="size-7 inline-flex items-center justify-center rounded hover:bg-accent"
                        aria-label="Entry actions"
                      >
                        <MoreHorizontal class="h-3.5 w-3.5" />
                      </button>
                    {/snippet}
                    <DropdownItem onSelect={() => openEditor(entry)}>
                      <Edit2 class="h-4 w-4" />
                      <span>Edit</span>
                    </DropdownItem>
                    <DropdownItem onSelect={() => toggleFavorite(entry)}>
                      <Star class="h-4 w-4" />
                      <span>{entry.is_favorite ? "Unfavorite" : "Favorite"}</span>
                    </DropdownItem>
                    {#if entry.url}
                      <DropdownItem onSelect={() => openUrl(entry.url)}>
                        <ExternalLink class="h-4 w-4" />
                        <span>Open URL</span>
                      </DropdownItem>
                    {/if}
                    <DropdownSeparator />
                    <DropdownItem destructive onSelect={() => handleDelete(entry)}>
                      <Trash2 class="h-4 w-4" />
                      <span>Delete</span>
                    </DropdownItem>
                  </DropdownMenu>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
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
