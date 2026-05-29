<script lang="ts">
  import { Button, Dialog, Input, Label, DropdownMenu, DropdownItem, DropdownSeparator, toast } from "$lib/ui";
  import IconPicker from "./IconPicker.svelte";
  import DynamicIcon from "./DynamicIcon.svelte";
  import { ChevronRight, Plus, Edit2, Trash2, MoreHorizontal, FolderPlus } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { createGroup, renameGroup, deleteGroup, moveGroup, moveEntry, type GroupData } from "$lib/tauri";
  import { saveGroupTreeState } from "$lib/group-state";
  import { findGroupByUuid, findParentGroup, isDescendant } from "$lib/group-utils";
  import { undoStack } from "$lib/undo-stack.svelte";
  import { appState } from "$lib/app-state.svelte";

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

  let dropTarget = $state<string | null>(null);

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
    const before = findGroupByUuid(group, renameUuid);
    const oldName = before?.name ?? "";
    const oldIcon = before?.icon_id ?? 48;
    const id = renameUuid;
    const nName = renameName;
    const nIcon = renameIconId;
    try {
      await renameGroup(id, nName, nIcon);
      undoStack.add(
        `Rename "${oldName}" → "${nName}"`,
        async () => {
          await renameGroup(id, oldName, oldIcon);
        },
        async () => {
          await renameGroup(id, nName, nIcon);
        },
      );
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

  const DT_FOLDER = "application/x-pw-folder";
  const DT_ENTRY = "application/x-pw-entry";

  function onFolderDragStart(e: DragEvent, uuid: string) {
    if (!e.dataTransfer) return;
    e.dataTransfer.setData(DT_FOLDER, uuid);
    e.dataTransfer.effectAllowed = "move";
  }

  function isAcceptedDrag(dt: DataTransfer): boolean {
    const types = Array.from(dt.types);
    return types.includes(DT_FOLDER) || types.includes(DT_ENTRY);
  }

  // Both dragenter and dragover need preventDefault — without dragenter
  // some Chromium builds skip the dragover entirely.
  function onFolderDragEnter(e: DragEvent, uuid: string) {
    const dt = e.dataTransfer;
    if (!dt || !isAcceptedDrag(dt)) return;
    e.preventDefault();
    if (dropTarget !== uuid) dropTarget = uuid;
  }

  function onFolderDragOver(e: DragEvent, uuid: string) {
    const dt = e.dataTransfer;
    if (!dt || !isAcceptedDrag(dt)) return;
    e.preventDefault();
    dt.dropEffect = "move";
    if (dropTarget !== uuid) dropTarget = uuid;
  }

  function onFolderDragLeave(uuid: string) {
    if (dropTarget === uuid) dropTarget = null;
  }

  async function onFolderDrop(e: DragEvent, targetUuid: string) {
    e.preventDefault();
    dropTarget = null;
    const dt = e.dataTransfer;
    if (!dt) return;

    const folderUuid = dt.getData(DT_FOLDER);
    const entryUuid = dt.getData(DT_ENTRY);

    if (folderUuid) {
      await handleFolderDrop(folderUuid, targetUuid);
    } else if (entryUuid) {
      await handleEntryDrop(entryUuid, targetUuid);
    }
  }

  async function handleFolderDrop(draggedUuid: string, targetUuid: string) {
    if (draggedUuid === targetUuid) return;
    const dragged = findGroupByUuid(group, draggedUuid);
    const target = findGroupByUuid(group, targetUuid);
    if (!dragged || !target) return;
    if (isDescendant(dragged, target)) {
      toast.error("Invalid move", "Cannot move a group into its own descendant");
      return;
    }
    const oldParent = findParentGroup(group, draggedUuid);
    const oldParentUuid = oldParent?.uuid ?? group.uuid;
    try {
      await moveGroup(draggedUuid, targetUuid);
      undoStack.add(
        `Move "${dragged.name}" into "${target.name}"`,
        async () => {
          await moveGroup(draggedUuid, oldParentUuid);
        },
        async () => {
          await moveGroup(draggedUuid, targetUuid);
        },
      );
      appState.markDirty();
      toast.success("Moved", `"${dragged.name}" → "${target.name}"`);
      await onRefresh();
    } catch (e) {
      toast.error("Move failed", String(e));
    }
  }

  async function handleEntryDrop(entryUuid: string, targetUuid: string) {
    const oldGroupUuid = window.__pwLastDraggedEntryGroup ?? null;
    if (oldGroupUuid && oldGroupUuid === targetUuid) return;
    try {
      await moveEntry(entryUuid, targetUuid);
      if (oldGroupUuid) {
        undoStack.add(
          `Move entry`,
          async () => {
            await moveEntry(entryUuid, oldGroupUuid);
          },
          async () => {
            await moveEntry(entryUuid, targetUuid);
          },
        );
      }
      appState.markDirty();
      toast.success("Entry moved");
      await onRefresh();
    } catch (e) {
      toast.error("Move failed", String(e));
    }
  }
</script>

{#snippet folder(g: GroupData, depth: number)}
  {@const hasChildren = g.children && g.children.length > 0}
  {@const isExpanded = expanded.has(g.uuid)}
  {@const isSelected = g.uuid === selectedUuid}
  {@const iconId = g.icon_id ?? 48}
  {@const isDropTarget = dropTarget === g.uuid}
  <div
    role="treeitem"
    tabindex="-1"
    aria-selected={isSelected}
    draggable={depth > 0}
    ondragstart={(e) => onFolderDragStart(e, g.uuid)}
    ondragenter={(e) => onFolderDragEnter(e, g.uuid)}
    ondragover={(e) => onFolderDragOver(e, g.uuid)}
    ondragleave={() => onFolderDragLeave(g.uuid)}
    ondrop={(e) => onFolderDrop(e, g.uuid)}
    onclick={() => onSelectGroup(g.uuid)}
    ondblclick={() => { if (hasChildren) toggle(g.uuid); }}
    onkeydown={(e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onSelectGroup(g.uuid);
      }
    }}
    class="group/row flex items-center gap-1 pr-1.5 h-7 rounded-md transition-colors cursor-pointer active:cursor-grabbing {isSelected
      ? 'bg-selected text-selected-foreground'
      : 'text-foreground/80 hover:bg-accent/60 hover:text-foreground'} {isDropTarget
      ? 'ring-2 ring-primary ring-inset bg-primary/10'
      : ''}"
    style="padding-left: {depth * 14 + 4}px;"
  >
    <button
      type="button"
      class="size-5 inline-flex items-center justify-center shrink-0 rounded {hasChildren
        ? 'hover:bg-foreground/8'
        : 'invisible'}"
      onclick={(e) => {
        e.stopPropagation();
        if (hasChildren) toggle(g.uuid);
      }}
      tabindex={hasChildren ? 0 : -1}
      aria-label={isExpanded ? "Collapse" : "Expand"}
    >
      <ChevronRight class="size-3 transition-transform duration-150 {isExpanded ? 'rotate-90' : ''}" />
    </button>

    <DynamicIcon
      iconId={iconId}
      class="size-3.5 shrink-0 {isSelected ? 'text-current' : 'text-muted-foreground'}"
    />

    <span class="flex-1 text-left text-[12.5px] font-medium truncate min-w-0 select-none">
      {g.name}
    </span>

    <DropdownMenu align="end">
      {#snippet trigger()}
        <button
          type="button"
          aria-label="Folder actions"
          onclick={(e: MouseEvent) => e.stopPropagation()}
          class="opacity-0 group-hover/row:opacity-100 data-[state=open]:opacity-100 size-5 inline-flex items-center justify-center rounded hover:bg-foreground/10"
        >
          <MoreHorizontal class="size-3.5" />
        </button>
      {/snippet}
      <DropdownItem onSelect={() => openCreate(g.uuid)}>
        <Plus class="size-4" />
        <span>New Subfolder</span>
      </DropdownItem>
      <DropdownItem onSelect={() => openRename(g)}>
        <Edit2 class="size-4" />
        <span>Rename</span>
      </DropdownItem>
      {#if depth > 0}
        <DropdownSeparator />
        <DropdownItem destructive onSelect={() => handleDelete(g.uuid)}>
          <Trash2 class="size-4" />
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
  <div class="flex-1 overflow-y-auto px-2 pb-2">
    {@render folder(group, 0)}
  </div>
  <div class="px-2 pb-2 shrink-0">
    <button
      type="button"
      onclick={() => openCreate(null)}
      class="w-full flex items-center gap-2 h-7 px-2 rounded-md text-[12px] text-muted-foreground hover:text-foreground hover:bg-accent/60 transition-colors"
      title="New top-level folder"
    >
      <FolderPlus class="size-3.5" />
      <span>New folder</span>
    </button>
  </div>
</div>

<Dialog bind:open={showCreate} title="Create New Folder" description="Enter a name for the new folder">
  <div class="space-y-2">
    <Label for="groupName">Folder Name</Label>
    <div class="flex gap-2">
      <IconPicker value={newIconId} onChange={(id) => (newIconId = id)} />
      <!-- svelte-ignore a11y_autofocus -->
      <Input
        id="groupName"
        bind:value={newName}
        onkeydown={(e) => e.key === "Enter" && handleCreate()}
        placeholder="Enter folder name"
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

<Dialog bind:open={showRename} title="Rename Folder" description="Enter a new name for the folder">
  <div class="space-y-2">
    <Label for="renameGroupName">Folder Name</Label>
    <div class="flex gap-2">
      <IconPicker value={renameIconId} onChange={(id) => (renameIconId = id)} />
      <!-- svelte-ignore a11y_autofocus -->
      <Input
        id="renameGroupName"
        bind:value={renameName}
        onkeydown={(e) => e.key === "Enter" && handleRename()}
        placeholder="Enter folder name"
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
