<script lang="ts">
  import { Button, DropdownMenu, DropdownItem, toast } from "$lib/ui";
  import GroupTree from "./GroupTree.svelte";
  import {
    Save,
    Search,
    Settings as SettingsIcon,
    Info,
    LogOut,
    Sun,
    Moon,
    Monitor,
    Lock,
    Undo2,
    Redo2,
  } from "@lucide/svelte";
  import type { GroupData } from "$lib/tauri";
  import { theme, type ThemeMode } from "$lib/theme.svelte";
  import { appState } from "$lib/app-state.svelte";
  import { push } from "svelte-spa-router";
  import { undoStack } from "$lib/undo-stack.svelte";

  type Props = {
    rootGroup: GroupData | null;
    selectedUuid: string;
    onSelectGroup: (uuid: string) => void;
    onRefresh: () => void | Promise<void>;
    onGroupDeleted?: (uuid: string) => void;
    initialExpandedGroups?: Set<string>;
    onOpenPalette: () => void;
    onSave: () => void | Promise<void>;
    onLogout: () => void | Promise<void>;
    onUndo: () => void | Promise<void>;
    onRedo: () => void | Promise<void>;
  };

  let {
    rootGroup,
    selectedUuid,
    onSelectGroup,
    onRefresh,
    onGroupDeleted,
    initialExpandedGroups,
    onOpenPalette,
    onSave,
    onLogout,
    onUndo,
    onRedo,
  }: Props = $props();

  const themeIcons = { system: Monitor, light: Sun, dark: Moon } as const;
  const themeLabel: Record<ThemeMode, string> = { system: "System", light: "Light", dark: "Dark" };
  const ThemeIcon = $derived(themeIcons[theme.mode]);

  const dbName = $derived(appState.dbPath ? appState.dbPath.split(/[\\/]/).pop() : "");
</script>

<aside class="flex flex-col h-full bg-sidebar min-h-0">
  <!-- Folders -->
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

  <!-- Bottom: search trigger -->
  <div class="px-2 pt-2 border-t">
    <button
      type="button"
      onclick={onOpenPalette}
      class="w-full flex items-center gap-2 h-8 px-2 rounded-md border bg-background/40 hover:bg-accent transition-colors text-xs text-muted-foreground"
      title="Search & commands (Ctrl+K)"
    >
      <Search class="size-3.5" />
      <span class="flex-1 text-left">Search…</span>
      <kbd class="text-[10px] border rounded px-1 py-0.5 bg-muted/60">Ctrl K</kbd>
    </button>
  </div>

  <!-- Bottom: actions -->
  <div class="p-2 flex items-center gap-1">
    <Button
      variant="ghost"
      size="icon"
      class="h-8 w-8"
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
    </Button>

    <Button variant="ghost" size="icon" class="h-8 w-8" title="Undo (Ctrl+Z)" disabled={!undoStack.canUndo} onclick={() => void onUndo()}>
      <Undo2 class="h-4 w-4" />
    </Button>
    <Button variant="ghost" size="icon" class="h-8 w-8" title="Redo (Ctrl+Y)" disabled={!undoStack.canRedo} onclick={() => void onRedo()}>
      <Redo2 class="h-4 w-4" />
    </Button>

    <div class="flex-1"></div>

    <DropdownMenu align="end" side="top">
      {#snippet trigger()}
        <button
          type="button"
          class="size-8 inline-flex items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
          aria-label="Theme: {themeLabel[theme.mode]}"
          title="Theme: {themeLabel[theme.mode]}"
        >
          <ThemeIcon class="size-4" />
        </button>
      {/snippet}
      <DropdownItem onSelect={() => theme.set("system")}>
        <Monitor class="size-4" />
        <span>System</span>
        {#if theme.mode === "system"}<span class="ml-auto text-xs">✓</span>{/if}
      </DropdownItem>
      <DropdownItem onSelect={() => theme.set("light")}>
        <Sun class="size-4" />
        <span>Light</span>
        {#if theme.mode === "light"}<span class="ml-auto text-xs">✓</span>{/if}
      </DropdownItem>
      <DropdownItem onSelect={() => theme.set("dark")}>
        <Moon class="size-4" />
        <span>Dark</span>
        {#if theme.mode === "dark"}<span class="ml-auto text-xs">✓</span>{/if}
      </DropdownItem>
    </DropdownMenu>

    <Button variant="ghost" size="icon" class="h-8 w-8" title="Settings" onclick={() => push("/settings")}>
      <SettingsIcon class="h-4 w-4" />
    </Button>
    <Button variant="ghost" size="icon" class="h-8 w-8" title="About" onclick={() => push("/about")}>
      <Info class="h-4 w-4" />
    </Button>
    <Button variant="ghost" size="icon" class="h-8 w-8" title="Lock database" onclick={() => void onLogout()}>
      <Lock class="h-4 w-4" />
    </Button>
  </div>

  {#if dbName}
    <div class="px-3 pb-2 text-[10px] text-muted-foreground/70 truncate" title={appState.dbPath}>
      {dbName}
    </div>
  {/if}
</aside>
