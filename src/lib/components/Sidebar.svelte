<script lang="ts">
  import GroupTree from "./GroupTree.svelte";
  import {
    Settings as SettingsIcon,
    Lock,
    Database,
    Home,
    Star,
    KeyRound,
  } from "@lucide/svelte";
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
    onLogout,
  }: Props = $props();

  const dbFileName = $derived(appState.dbPath ? appState.dbPath.split(/[\\/]/).pop() ?? "Vault" : "Vault");
  const dbDisplayName = $derived(dbFileName.replace(/\.kdbx$/i, ""));

  type NavItem = { id: string; label: string; icon: typeof Home };
  const navItems: NavItem[] = [
    { id: "_dashboard", label: "Home", icon: Home },
    { id: "_all", label: "All Items", icon: KeyRound },
    { id: "_favorites", label: "Favorites", icon: Star },
  ];
</script>

<aside class="flex flex-col h-full bg-sidebar min-h-0 border-r border-border/60">
  <!-- Vault header -->
  <div class="px-3 pt-3 pb-3 shrink-0">
    <div
      class="flex items-center gap-2.5 rounded-lg bg-background/70 border border-border/70 px-2.5 py-2"
      title={appState.dbPath}
    >
      <div class="grid place-items-center size-8 rounded-md bg-primary/15 text-primary shrink-0">
        <Database class="size-4" />
      </div>
      <div class="min-w-0 flex-1">
        <div class="text-[13px] font-semibold truncate leading-tight">{dbDisplayName}</div>
        <div class="text-[10.5px] text-muted-foreground flex items-center gap-1.5">
          {#if appState.isDirty}
            <span class="inline-block size-1.5 rounded-full bg-warning animate-pulse-soft"></span>
            <span>Saving…</span>
          {:else}
            <span class="inline-block size-1.5 rounded-full bg-success/70"></span>
            <span>All changes saved</span>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <!-- Primary nav -->
  <nav class="px-2 pb-2 space-y-0.5 shrink-0" aria-label="Primary">
    {#each navItems as item (item.id)}
      {@const active = selectedUuid === item.id}
      <button
        type="button"
        onclick={() => onSelectGroup(item.id)}
        class="w-full flex items-center gap-2.5 h-8 px-2.5 rounded-md text-[13px] font-medium transition-colors {active
          ? 'bg-selected text-selected-foreground'
          : 'text-foreground/80 hover:bg-accent/60 hover:text-foreground'}"
      >
        <item.icon class="size-4 shrink-0" />
        <span class="truncate">{item.label}</span>
      </button>
    {/each}
  </nav>

  <!-- Folders section -->
  <div class="px-2 pt-2 pb-1 shrink-0">
    <div class="px-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/70">
      Folders
    </div>
  </div>

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

  <!-- Bottom: Settings + Lock -->
  <div class="border-t border-border/60 px-2 py-2 flex items-center gap-1.5 shrink-0">
    <button
      type="button"
      onclick={onOpenSettings}
      class="flex-1 flex items-center gap-2 h-9 px-2.5 rounded-md text-[12.5px] font-medium text-foreground/80 hover:bg-accent/60 hover:text-foreground transition-colors"
      title="Settings (Ctrl ,)"
    >
      <SettingsIcon class="size-4" />
      <span>Settings</span>
    </button>

    <button
      type="button"
      onclick={() => void onLogout()}
      class="size-9 inline-flex items-center justify-center rounded-md text-muted-foreground hover:bg-accent/60 hover:text-foreground transition-colors"
      aria-label="Lock database"
      title="Lock database"
    >
      <Lock class="size-4" />
    </button>
  </div>
</aside>
