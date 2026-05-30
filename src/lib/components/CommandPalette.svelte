<script lang="ts" module>
  import type { Component } from "svelte";
  export type PaletteCommand = {
    id: string;
    label: string;
    section: string;
    icon?: Component<any, any, any>;
    hint?: string;
    keywords?: string;
    onSelect: () => void | Promise<void>;
  };
</script>

<script lang="ts">
  import { Dialog, Kbd } from "$lib/ui";
  import { Search } from "@lucide/svelte";

  type Props = {
    open?: boolean;
    placeholder?: string;
    onSearch?: (query: string) => Promise<PaletteCommand[]> | PaletteCommand[];
    commands: PaletteCommand[];
    emptyMessage?: string;
  };

  let {
    open = $bindable(false),
    placeholder = "Type a command or search…",
    onSearch,
    commands,
    emptyMessage = "No matches.",
  }: Props = $props();

  let query = $state("");
  let active = $state(0);
  let extra = $state<PaletteCommand[]>([]);

  function lower(s: string): string {
    return s.toLowerCase();
  }

  const baseFiltered = $derived.by(() => {
    const q = lower(query.trim());
    if (!q) return commands;
    return commands.filter((c) => {
      const hay = `${lower(c.label)} ${lower(c.keywords ?? "")} ${lower(c.section)}`;
      return hay.includes(q);
    });
  });

  const all = $derived([...baseFiltered, ...extra]);

  const sections = $derived.by(() => {
    const buckets = new Map<string, PaletteCommand[]>();
    for (const c of all) {
      if (!buckets.has(c.section)) buckets.set(c.section, []);
      buckets.get(c.section)!.push(c);
    }
    return Array.from(buckets.entries()).map(([name, items]) => ({ name, items }));
  });

  const flat = $derived(sections.flatMap((s) => s.items));

  // run async search when input changes
  let searchToken = 0;
  $effect(() => {
    if (!onSearch) {
      extra = [];
      return;
    }
    const q = query.trim();
    if (!q) {
      extra = [];
      return;
    }
    const token = ++searchToken;
    const t = setTimeout(async () => {
      const r = await onSearch(q);
      if (token === searchToken) extra = r ?? [];
    }, 150);
    return () => clearTimeout(t);
  });

  $effect(() => {
    if (open) {
      query = "";
      active = 0;
      extra = [];
    }
  });

  $effect(() => {
    // clamp active when results shrink
    if (active >= flat.length) active = Math.max(0, flat.length - 1);
  });

  function indexOfFlat(c: PaletteCommand): number {
    return flat.findIndex((x) => x === c);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (flat.length === 0) return;
      active = (active + 1) % flat.length;
      scrollIntoView();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (flat.length === 0) return;
      active = (active - 1 + flat.length) % flat.length;
      scrollIntoView();
    } else if (e.key === "Enter") {
      e.preventDefault();
      const item = flat[active];
      if (item) void run(item);
    }
  }

  async function run(c: PaletteCommand) {
    open = false;
    await c.onSelect();
  }

  let listEl: HTMLDivElement | null = $state(null);
  function scrollIntoView() {
    queueMicrotask(() => {
      const el = listEl?.querySelector<HTMLElement>(`[data-active="true"]`);
      el?.scrollIntoView({ block: "nearest" });
    });
  }
</script>

<Dialog bind:open bare size="lg" placement="top" showClose={false} class="bg-popover text-popover-foreground">
  <div class="flex flex-col w-full">
    <div class="flex items-center gap-2 border-b border-border-subtle px-3 h-12">
      <Search class="size-4 text-muted-foreground shrink-0" />
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        bind:value={query}
        {placeholder}
        autofocus
        class="flex-1 bg-transparent outline-none text-sm placeholder:text-muted-foreground"
        onkeydown={onKey}
        data-command-palette-input
      />
      <Kbd>Esc</Kbd>
    </div>

    <div bind:this={listEl} class="max-h-[60vh] overflow-y-auto p-1">
      {#if flat.length === 0}
        <div class="text-sm text-muted-foreground text-center py-8">{emptyMessage}</div>
      {:else}
        {#each sections as section (section.name)}
          <div class="px-2 pt-2 pb-1 text-2xs font-semibold uppercase tracking-wider text-muted-foreground/70">
            {section.name}
          </div>
          {#each section.items as item (item.id)}
            {@const idx = indexOfFlat(item)}
            {@const isActive = idx === active}
            <button
              type="button"
              data-active={isActive}
              onpointermove={() => (active = idx)}
              onclick={() => run(item)}
              class="w-full flex items-center gap-3 rounded-md px-2 py-2 text-sm text-left transition-colors {isActive
                ? 'bg-accent text-accent-foreground'
                : 'hover:bg-accent/50'}"
            >
              {#if item.icon}
                <item.icon class="size-4 shrink-0 text-muted-foreground" />
              {/if}
              <span class="flex-1 truncate">{item.label}</span>
              {#if item.hint}
                <span class="text-xs text-muted-foreground truncate">{item.hint}</span>
              {/if}
            </button>
          {/each}
        {/each}
      {/if}
    </div>

    <div class="border-t border-border-subtle px-3 py-1.5 flex items-center gap-3 text-2xs text-muted-foreground">
      <span class="flex items-center gap-1"><Kbd>↑↓</Kbd> Navigate</span>
      <span class="flex items-center gap-1"><Kbd>↵</Kbd> Select</span>
      <span class="flex items-center gap-1"><Kbd>Esc</Kbd> Close</span>
    </div>
  </div>
</Dialog>
