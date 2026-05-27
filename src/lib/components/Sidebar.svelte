<script lang="ts">
  import { Button } from "$lib/ui";
  import GroupTree from "./GroupTree.svelte";
  import { Save, Settings as SettingsIcon, Lock, Database } from "@lucide/svelte";
  import type { GroupData } from "$lib/tauri";
  import { appState } from "$lib/app-state.svelte";

  type Props = {
    rootGroup: GroupData | null;
    selectedUuid: string;
    onSelectGroup: (uuid: string) => void;
    onRefresh: () => void | Promise<void>;
    onGroupDeleted?: (uuid: string) => void;
    initialExpandedGroups?: Set<string>;
    onOpenSettings: () => void;
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
    onSave,
    onLogout,
  }: Props = $props();

  const dbName = $derived(appState.dbPath ? appState.dbPath.split(/[\\/]/).pop() : "");
</script>

<aside class="flex flex-col h-full bg-sidebar min-h-0">
  <!-- Top: database header -->
  {#if dbName}
    <div class="px-3 pt-3 pb-2.5 shrink-0">
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

  <!-- Bottom: minimal action bar -->
  <div class="border-t p-2 flex items-center gap-1">
    <Button
      variant="ghost"
      size="sm"
      class="flex-1 justify-start gap-2 h-8"
      title={appState.isDirty ? "Save (Ctrl+S)" : "No changes to save"}
      disabled={!appState.isDirty}
      onclick={() => void onSave()}
    >
      <div class="relative">
        <Save class="h-4 w-4" />
        {#if appState.isDirty}
          <span class="absolute -top-1 -right-1 size-1.5 rounded-full bg-warning"></span>
        {/if}
      </div>
      <span class="text-xs">Save</span>
    </Button>

    <Button
      variant="ghost"
      size="icon"
      class="h-8 w-8"
      title="Settings"
      onclick={onOpenSettings}
    >
      <SettingsIcon class="h-4 w-4" />
    </Button>
    <Button
      variant="ghost"
      size="icon"
      class="h-8 w-8"
      title="Lock database"
      onclick={() => void onLogout()}
    >
      <Lock class="h-4 w-4" />
    </Button>
  </div>
</aside>
