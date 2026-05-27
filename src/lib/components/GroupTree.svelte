<script lang="ts">
  import { Button, Dialog, Input, Label, DropdownMenu, DropdownItem, DropdownSeparator, toast } from "$lib/ui";
  import IconPicker from "./IconPicker.svelte";
  import DynamicIcon from "./DynamicIcon.svelte";
  import { ChevronRight, Plus, Edit2, Trash2, Star, LayoutPanelLeft, MoreHorizontal } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { createGroup, renameGroup, deleteGroup, type GroupData } from "$lib/tauri";
  import { saveGroupTreeState } from "$lib/group-state";
  import { findGroupByUuid } from "$lib/group-utils";

  type Props = {
    group: GroupData;
    selectedUuid: string;
    onSelectGroup: (uuid: string) => void;
    onRefresh: () => void | Promise<void>;
    onGroupDeleted?: (uuid: string) => void;
    dbPath: string;
    initialExpandedGroups?: Set<string>;
  };

  let {
    group,
    selectedUuid,
    onSelectGroup,
    onRefresh,
    onGroupDeleted,
    dbPath,
    initialExpandedGroups,
  }: Props = $props();

  // svelte-ignore state_referenced_locally
  let expanded = $state<Set<string>>(initialExpandedGroups ?? new Set([group.uuid]));

  let showCreate = $state(false);
  let showRename = $state(false);
  let newName = $state("");
  let newIconId = $state(48);
  let parentUuid = $state<string | null>(null);
  let renameUuid = $state("");
  let renameName = $state("");
  let renameIconId = $state(48);

  $effect(() => {
    saveGroupTreeState(dbPath, expanded, selectedUuid);
  });

  function toggle(uuid: string) {
    const next = new Set(expanded);
    if (next.has(uuid)) next.delete(uuid);
    else next.add(uuid);
    expanded = next;
  }

  function openCreate(p: string | null) {
    parentUuid = p;
    newName = "";
    newIconId = 48;
    showCreate = true;
  }

  function openRename(g: GroupData) {
    renameUuid = g.uuid;
    renameName = g.name;
    renameIconId = g.icon_id ?? 48;
    showRename = true;
  }

  async function handleCreate() {
    if (!newName.trim()) return;
    try {
      await createGroup(newName, parentUuid, newIconId);
      toast.success("Group created");
      showCreate = false;
      await onRefresh();
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }

  async function handleRename() {
    if (!renameName.trim()) return;
    try {
      await renameGroup(renameUuid, renameName, renameIconId);
      toast.success("Group renamed");
      showRename = false;
      await onRefresh();
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }

  async function handleDelete(uuid: string) {
    const g = findGroupByUuid(group, uuid);
    const name = g?.name ?? "this group";
    const ok = await ask(
      `Are you sure you want to delete "${name}" and all its contents?\n\nThis cannot be undone.`,
      { kind: "warning", title: "Delete Group" },
    );
    if (!ok) return;
    try {
      await deleteGroup(uuid);
      toast.success("Group deleted");
      onGroupDeleted?.(uuid);
      await onRefresh();
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }
</script>

{#snippet folder(g: GroupData, depth: number)}
  {@const hasChildren = g.children && g.children.length > 0}
  {@const isExpanded = expanded.has(g.uuid)}
  {@const isSelected = g.uuid === selectedUuid}
  {@const iconId = g.icon_id ?? 48}
  <div
    class="group/row flex items-center gap-1 pr-1 py-1 rounded transition-colors hover:bg-accent/50 {isSelected
      ? 'bg-accent font-medium'
      : ''}"
    style="padding-left: {depth * 12 + 4}px;"
  >
    <button
      type="button"
      class="size-5 inline-flex items-center justify-center shrink-0 {hasChildren
        ? 'hover:bg-muted rounded'
        : 'invisible'}"
      onclick={(e) => {
        e.stopPropagation();
        if (hasChildren) toggle(g.uuid);
      }}
      tabindex={hasChildren ? 0 : -1}
    >
      <ChevronRight class="h-3 w-3 transition-transform duration-200 {isExpanded ? 'rotate-90' : ''}" />
    </button>

    <DynamicIcon iconId={iconId} class="h-4 w-4 text-muted-foreground shrink-0" />

    <button
      type="button"
      class="flex-1 text-left text-sm truncate"
      onclick={() => onSelectGroup(g.uuid)}
      ondblclick={(e) => {
        e.stopPropagation();
        if (hasChildren) toggle(g.uuid);
      }}
    >
      {g.name}
    </button>

    <DropdownMenu align="end">
      {#snippet trigger()}
        <button
          type="button"
          aria-label="Folder actions"
          class="opacity-0 group-hover/row:opacity-100 data-[state=open]:opacity-100 size-6 inline-flex items-center justify-center rounded hover:bg-accent"
        >
          <MoreHorizontal class="h-3.5 w-3.5" />
        </button>
      {/snippet}
      <DropdownItem onSelect={() => openCreate(g.uuid)}>
        <Plus class="h-4 w-4" />
        <span>New Subgroup</span>
      </DropdownItem>
      <DropdownItem onSelect={() => openRename(g)}>
        <Edit2 class="h-4 w-4" />
        <span>Rename</span>
      </DropdownItem>
      {#if depth > 0}
        <DropdownSeparator />
        <DropdownItem destructive onSelect={() => handleDelete(g.uuid)}>
          <Trash2 class="h-4 w-4" />
          <span>Delete</span>
        </DropdownItem>
      {/if}
    </DropdownMenu>
  </div>

  {#if isExpanded && hasChildren}
    {#each g.children as child (child.uuid)}
      {@render folder(child, depth + 1)}
    {/each}
  {/if}
{/snippet}

<div class="h-full flex flex-col">
  <div class="flex items-center justify-between border-b px-3 py-2">
    <h2 class="text-sm font-semibold">Folders</h2>
    <Button variant="ghost" size="icon" class="h-7 w-7" onclick={() => openCreate(null)} title="New top-level group">
      <Plus class="h-4 w-4" />
    </Button>
  </div>

  <div class="flex-1 overflow-y-auto p-1">
    <button
      type="button"
      class="w-full flex items-center gap-2 px-2 py-1.5 cursor-pointer rounded transition-colors text-sm font-medium {selectedUuid === '_dashboard'
        ? 'bg-accent'
        : 'hover:bg-accent/50'}"
      onclick={() => onSelectGroup("_dashboard")}
    >
      <LayoutPanelLeft class="h-4 w-4 text-muted-foreground shrink-0" />
      <span class="truncate">Dashboard</span>
    </button>
    <button
      type="button"
      class="w-full flex items-center gap-2 px-2 py-1.5 cursor-pointer rounded transition-colors text-sm font-medium {selectedUuid === '_favorites'
        ? 'bg-accent'
        : 'hover:bg-accent/50'}"
      onclick={() => onSelectGroup("_favorites")}
    >
      <Star class="h-4 w-4 text-muted-foreground shrink-0" />
      <span class="truncate">Favorites</span>
    </button>

    {@render folder(group, 0)}
  </div>
</div>

<Dialog bind:open={showCreate} title="Create New Group" description="Enter a name for the new group">
  <div class="space-y-2">
    <Label for="groupName">Group Name</Label>
    <div class="flex gap-2">
      <IconPicker value={newIconId} onChange={(id) => (newIconId = id)} />
      <!-- svelte-ignore a11y_autofocus -->
      <Input
        id="groupName"
        bind:value={newName}
        onkeydown={(e) => e.key === "Enter" && handleCreate()}
        placeholder="Enter group name"
        class="flex-1"
        autofocus
      />
    </div>
  </div>
  {#snippet footer()}
    <Button variant="outline" onclick={() => (showCreate = false)}>Cancel</Button>
    <Button onclick={handleCreate}>Create</Button>
  {/snippet}
</Dialog>

<Dialog bind:open={showRename} title="Rename Group" description="Enter a new name for the group">
  <div class="space-y-2">
    <Label for="renameGroupName">Group Name</Label>
    <div class="flex gap-2">
      <IconPicker value={renameIconId} onChange={(id) => (renameIconId = id)} />
      <!-- svelte-ignore a11y_autofocus -->
      <Input
        id="renameGroupName"
        bind:value={renameName}
        onkeydown={(e) => e.key === "Enter" && handleRename()}
        placeholder="Enter group name"
        class="flex-1"
        autofocus
      />
    </div>
  </div>
  {#snippet footer()}
    <Button variant="outline" onclick={() => (showRename = false)}>Cancel</Button>
    <Button onclick={handleRename}>Rename</Button>
  {/snippet}
</Dialog>
