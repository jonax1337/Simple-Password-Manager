<script lang="ts">
  import { DropdownMenu, DropdownItem, DropdownSeparator } from "$lib/ui";
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
    ChevronsUpDown,
    Home,
    Star,
    KeyRound,
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

  const dbFileName = $derived(appState.dbPath ? appState.dbPath.split(/[\\/]/).pop() ?? "Vault" : "Vault");
  const dbDisplayName = $derived(dbFileName.replace(/\.kdbx$/i, ""));
  const accountInitial = $derived((dbDisplayName.trim()[0] ?? "P").toUpperCase());

  type NavItem = { id: string; label: string; icon: typeof Home };
  const navItems: NavItem[] = [
    { id: "_dashboard", label: "Home", icon: Home },
    { id: "_all", label: "All Items", icon: KeyRound },
    { id: "_favorites", label: "Favorites", icon: Star },
  ];
</script>

<aside class="flex flex-col h-full bg-sidebar min-h-0 border-r border-border/60">
  <!-- Vault picker -->
  <div class="px-3 pt-3 pb-3 shrink-0">
    <button
      type="button"
      class="group/vault w-full flex items-center gap-2.5 rounded-lg bg-background/70 hover:bg-background border border-border/70 hover:border-border px-2.5 py-2 transition-colors shadow-xs text-left"
      title={appState.dbPath}
    >
      <div class="grid place-items-center size-8 rounded-md bg-gradient-to-br from-primary to-primary/70 text-primary-foreground shrink-0 shadow-xs">
        <Database class="size-4" />
      </div>
      <div class="min-w-0 flex-1">
        <div class="text-[13px] font-semibold truncate leading-tight">{dbDisplayName}</div>
        <div class="text-[10.5px] text-muted-foreground flex items-center gap-1.5">
          {#if appState.isDirty}
            <span class="inline-block size-1.5 rounded-full bg-warning animate-pulse-soft"></span>
            <span>Unsaved changes</span>
          {:else}
            <span class="inline-block size-1.5 rounded-full bg-success/70"></span>
            <span>All changes saved</span>
          {/if}
        </div>
      </div>
      <ChevronsUpDown class="size-3.5 text-muted-foreground/70 shrink-0 group-hover/vault:text-muted-foreground transition-colors" />
    </button>
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

  <!-- Bottom account / actions -->
  <div class="border-t border-border/60 px-2 py-2 flex items-center gap-1.5 shrink-0">
    <button
      type="button"
      onclick={() => void onSave()}
      disabled={!appState.isDirty}
      class="flex-1 flex items-center gap-2 h-9 px-2.5 rounded-md text-[12.5px] font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed {appState.isDirty
        ? 'bg-primary text-primary-foreground hover:bg-primary/90 shadow-xs'
        : 'text-muted-foreground hover:bg-accent/40'}"
      title={appState.isDirty ? 'Save changes (Ctrl+S)' : 'Database is saved'}
    >
      <Save class="size-3.5" />
      <span>{appState.isDirty ? "Save changes" : "Saved"}</span>
    </button>

    <DropdownMenu align="end" side="top">
      {#snippet trigger()}
        <button
          type="button"
          class="size-9 inline-flex items-center justify-center rounded-md font-semibold text-[11px] bg-background/60 border border-border/70 hover:bg-accent/60 transition-colors text-foreground/85"
          aria-label="Account menu"
          title="Account"
        >
          {accountInitial}
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
