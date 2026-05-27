<script lang="ts">
  import { Button, DropdownMenu, DropdownItem, DropdownSeparator } from "$lib/ui";
  import GroupTree from "./GroupTree.svelte";
  import {
    Save,
    Settings as SettingsIcon,
    Lock,
    Database,
    Info,
    Sun,
    Moon,
    Monitor,
    ChevronUp,
  } from "@lucide/svelte";
  import type { GroupData } from "$lib/tauri";
  import { appState } from "$lib/app-state.svelte";
  import { theme } from "$lib/theme.svelte";

  type Props = {
    rootGroup: GroupData | null;
    selectedUuid: string;
    onSelectGroup: (uuid: string) => void;
    onRefresh: () => void | Promise<void>;
    onGroupDeleted?: (uuid: string) => void;
    initialExpandedGroups?: Set<string>;
    onOpenSettings: () => void;
    onOpenAbout: () => void;
    onSave: () => void | Promise<void>;
    onLogout: () => void | Promise<void>;
  };

  let {
    rootGroup,
    selectedUuid,
    onSelectGroup,
    onRefresh,
    onGroupDeleted,
    initialExpandedGroups,
    onOpenSettings,
    onOpenAbout,
    onSave,
    onLogout,
  }: Props = $props();

  const dbName = $derived(appState.dbPath ? appState.dbPath.split(/[\\/]/).pop() : "");
</script>

<aside class="flex flex-col h-full bg-sidebar min-h-0">
  <!-- Top: database header -->
  {#if dbName}
    <div class="px-3 pt-3 pb-2 shrink-0">
      <div class="flex items-center gap-2.5 rounded-lg bg-background/60 border px-2.5 py-2">
        <div class="grid place-items-center size-7 rounded-md bg-primary/10 text-primary shrink-0">
          <Database class="size-3.5" />
        </div>
        <div class="min-w-0 flex-1">
          <div class="text-xs font-medium truncate" title={appState.dbPath}>{dbName}</div>
          <div class="text-[10px] text-muted-foreground">
            {appState.isDirty ? "Unsaved changes" : "Saved"}
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Folders / nav -->
  <div class="flex-1 min-h-0 overflow-hidden">
    {#if rootGroup}
      <GroupTree
        group={rootGroup}
        {selectedUuid}
        {onSelectGroup}
        {onRefresh}
        {onGroupDeleted}
        dbPath={appState.dbPath}
        {initialExpandedGroups}
      />
    {/if}
  </div>

  <!-- Bottom: Save + Menu -->
  <div class="border-t p-2 flex items-center gap-1.5">
    <Button
      variant={appState.isDirty ? "default" : "ghost"}
      size="sm"
      class="flex-1 justify-start gap-2 h-9"
      title={appState.isDirty ? "Save (Ctrl+S)" : "No changes to save"}
      disabled={!appState.isDirty}
      onclick={() => void onSave()}
    >
      <Save class="h-4 w-4" />
      <span class="text-xs font-medium">
        {appState.isDirty ? "Save" : "Saved"}
      </span>
    </Button>

    <DropdownMenu align="end" side="top">
      {#snippet trigger()}
        <button
          type="button"
          class="h-9 w-9 inline-flex items-center justify-center rounded-md hover:bg-accent transition-colors text-foreground/80 border bg-background/60"
          aria-label="App menu"
          title="Menu"
        >
          <ChevronUp class="size-4" />
        </button>
      {/snippet}

      <DropdownItem onSelect={onOpenSettings}>
        <SettingsIcon class="size-4" />
        <span>Settings…</span>
        <span class="ml-auto text-[10px] text-muted-foreground">Ctrl ,</span>
      </DropdownItem>

      <DropdownItem onSelect={onOpenAbout}>
        <Info class="size-4" />
        <span>About</span>
      </DropdownItem>

      <DropdownSeparator />

      <DropdownItem onSelect={() => theme.set("system")}>
        <Monitor class="size-4" />
        <span>Theme: System</span>
        {#if theme.mode === "system"}<span class="ml-auto text-xs">✓</span>{/if}
      </DropdownItem>
      <DropdownItem onSelect={() => theme.set("light")}>
        <Sun class="size-4" />
        <span>Theme: Light</span>
        {#if theme.mode === "light"}<span class="ml-auto text-xs">✓</span>{/if}
      </DropdownItem>
      <DropdownItem onSelect={() => theme.set("dark")}>
        <Moon class="size-4" />
        <span>Theme: Dark</span>
        {#if theme.mode === "dark"}<span class="ml-auto text-xs">✓</span>{/if}
      </DropdownItem>

      <DropdownSeparator />

      <DropdownItem destructive onSelect={() => void onLogout()}>
        <Lock class="size-4" />
        <span>Lock database</span>
      </DropdownItem>
    </DropdownMenu>
  </div>
</aside>
