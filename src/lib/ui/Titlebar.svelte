<script lang="ts">
  import type { Snippet } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Minus, Square, Copy, X } from "@lucide/svelte";
  import { onMount } from "svelte";

  type Props = {
    title?: string;
    showAppMark?: boolean;
    children?: Snippet;
    actions?: Snippet;
  };
  let { title = "Simple Password Manager", showAppMark = true, children, actions }: Props = $props();

  const win = getCurrentWindow();
  let maximized = $state(false);

  onMount(() => {
    win.isMaximized().then((m) => (maximized = m));
    const un = win.onResized(async () => {
      maximized = await win.isMaximized();
    });
    return () => {
      un.then((u) => u());
    };
  });
</script>

<header
  data-tauri-drag-region
  class="titlebar select-none flex items-center h-9 shrink-0 bg-sidebar border-b border-border/60 text-foreground"
>
  {#if showAppMark}
    <div data-tauri-drag-region class="flex items-center pl-3 pr-3">
      <img src="/app-icon.png" alt="" aria-hidden="true" class="size-[18px] object-contain" />
    </div>
  {:else}
    <div data-tauri-drag-region class="w-3"></div>
  {/if}

  <div
    data-tauri-drag-region
    class="flex-1 flex items-center gap-3 text-[11.5px] font-medium text-muted-foreground tracking-tight min-w-0"
  >
    <span class="truncate">{title}</span>
    {@render children?.()}
  </div>

  {#if actions}
    <div class="flex items-center h-full pr-1" style="-webkit-app-region: no-drag">
      {@render actions()}
    </div>
  {/if}

  <div class="flex items-center h-full">
    <button type="button" class="titlebar-btn" aria-label="Minimize" onclick={() => win.minimize()}>
      <Minus class="size-3.5" />
    </button>
    <button
      type="button"
      class="titlebar-btn"
      aria-label={maximized ? "Restore" : "Maximize"}
      onclick={() => win.toggleMaximize()}
    >
      {#if maximized}
        <Copy class="size-3 -scale-x-100" />
      {:else}
        <Square class="size-3" />
      {/if}
    </button>
    <button type="button" class="titlebar-btn titlebar-btn-close" aria-label="Close" onclick={() => win.close()}>
      <X class="size-4" />
    </button>
  </div>
</header>

<style>
  .titlebar {
    -webkit-app-region: drag;
  }
  .titlebar :global(button) {
    -webkit-app-region: no-drag;
  }
  :global(.titlebar-btn) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 100%;
    color: var(--color-muted-foreground);
    background: transparent;
    transition: background-color 120ms ease, color 120ms ease;
    cursor: pointer;
  }
  :global(.titlebar-btn:hover) {
    background-color: color-mix(in oklch, var(--color-foreground) 8%, transparent);
    color: var(--color-foreground);
  }
  :global(.titlebar-btn-close:hover) {
    background-color: oklch(0.58 0.235 27);
    color: white;
  }
</style>
