<script lang="ts">
  import GroupTree from "./GroupTree.svelte";
  import {
    Settings as SettingsIcon, Lock, Database, Home, Star, KeyRound,
  } from "@lucide/svelte";
  import type { GroupData } from "$lib/tauri";
  import { appState } from "$lib/app-state.svelte";
  import { IconButton, Eyebrow, Tooltip } from "$lib/ui";
  import { navRow } from "$lib/ui/recipes";
  import { onMount } from "svelte";

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

  // Cheap ticker so the relative "X seconds ago" label refreshes without
  // forcing every appState mutation to bump a counter.
  let now = $state(Date.now());
  onMount(() => {
    const iv = setInterval(() => (now = Date.now()), 5000);
    return () => clearInterval(iv);
  });

  function relativeTime(ms: number, ref: number): string {
    const s = Math.max(0, Math.round((ref - ms) / 1000));
    if (s < 5) return "just now";
    if (s < 60) return `${s}s ago`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    if (h < 24) return `${h}h ago`;
    return `${Math.round(h / 24)}d ago`;
  }

  type Pill = { dotClass: string; label: string; title?: string };

  const syncPill: Pill = $derived.by(() => {
    // Priority: dirty > active transitions > idle. `isDirty` is the strongest
    // signal of "work in progress" — even if status briefly settles to idle
    // between debounced saves, we want the pill to keep showing pending work.
    if (appState.isDirty) {
      return {
        dotClass: "bg-warning animate-pulse-soft",
        label: "Unsaved changes",
      };
    }
    switch (appState.syncStatus) {
      case "saving":
        return { dotClass: "bg-warning animate-pulse-soft", label: "Saving…" };
      case "merging":
        return { dotClass: "bg-primary animate-pulse-soft", label: "Merging…" };
      case "cloud-sync":
        return { dotClass: "bg-primary animate-pulse-soft", label: "Syncing cloud…" };
      case "conflict":
        return {
          dotClass: "bg-destructive",
          label: "Sync needs attention",
        };
      case "idle":
      default: {
        if (appState.lastSyncedAt === null) {
          return { dotClass: "bg-success/70", label: "Ready" };
        }
        return {
          dotClass: "bg-success/70",
          label: `Synced ${relativeTime(appState.lastSyncedAt, now)}`,
          title: new Date(appState.lastSyncedAt).toLocaleString(),
        };
      }
    }
  });

  type NavItem = { id: string; label: string; icon: typeof Home };
  const navItems: NavItem[] = [
    { id: "_dashboard", label: "Home", icon: Home },
    { id: "_all", label: "All Items", icon: KeyRound },
    { id: "_favorites", label: "Favorites", icon: Star },
  ];
</script>

<aside class="flex flex-col h-full bg-sidebar min-h-0 border-r border-border-subtle">
  <!-- Vault header -->
  <div class="px-3 pt-3 pb-3 shrink-0">
    <div
      class="flex items-center gap-2.5 rounded-lg bg-background/70 border border-border px-2.5 py-2"
      title={appState.dbPath}
    >
      <div class="grid place-items-center size-8 rounded-md bg-primary/15 text-primary shrink-0">
        <Database class="size-4" />
      </div>
      <div class="min-w-0 flex-1">
        <div class="text-sm font-semibold truncate leading-tight">{dbDisplayName}</div>
        <div
          class="text-2xs text-muted-foreground flex items-center gap-1.5"
          title={syncPill.title}
        >
          <span class="inline-block size-1.5 rounded-full {syncPill.dotClass}"></span>
          <span class="truncate">{syncPill.label}</span>
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
        class={navRow({ active, size: "sm" })}
      >
        <item.icon class="size-4 shrink-0" />
        <span class="truncate">{item.label}</span>
      </button>
    {/each}
  </nav>

  <!-- Folders section -->
  <div class="px-2 pt-2 pb-1 shrink-0">
    <Eyebrow class="px-2">Folders</Eyebrow>
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
  <div class="border-t border-border-subtle px-2 py-2 flex items-center gap-1.5 shrink-0">
    <Tooltip label="Settings" shortcut="Ctrl ,">
      <button
        type="button"
        onclick={onOpenSettings}
        class="w-full flex items-center gap-2 h-9 px-2.5 rounded-md text-sm font-medium text-foreground/80 hover:bg-accent/60 hover:text-foreground transition-colors"
      >
        <SettingsIcon class="size-4" />
        <span>Settings</span>
      </button>
    </Tooltip>

    <Tooltip label="Lock database">
      <IconButton onclick={() => void onLogout()} aria-label="Lock database">
        <Lock />
      </IconButton>
    </Tooltip>
  </div>
</aside>
