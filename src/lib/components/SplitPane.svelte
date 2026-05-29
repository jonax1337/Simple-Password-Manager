<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    left: Snippet;
    right: Snippet;
    defaultWidth?: number;
    minWidth?: number;
    maxWidth?: number;
    storageKey?: string;
  };

  let {
    left,
    right,
    defaultWidth = 260,
    minWidth = 180,
    maxWidth = 500,
    storageKey,
  }: Props = $props();

  let width = $state(loadInitial());
  let dragging = $state(false);
  let container: HTMLDivElement | null = $state(null);

  function loadInitial(): number {
    if (!storageKey || typeof window === "undefined") return defaultWidth;
    const raw = localStorage.getItem(storageKey);
    if (!raw) return defaultWidth;
    const n = Number(raw);
    if (Number.isFinite(n) && n >= minWidth && n <= maxWidth) return n;
    return defaultWidth;
  }

  function persist(w: number) {
    if (!storageKey || typeof window === "undefined") return;
    localStorage.setItem(storageKey, String(w));
  }

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging || !container) return;
    const rect = container.getBoundingClientRect();
    const local = e.clientX - rect.left;
    width = Math.max(minWidth, Math.min(maxWidth, local));
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    persist(width);
  }
</script>

<div bind:this={container} class="flex h-full min-h-0 flex-1 overflow-hidden">
  <div class="shrink-0 h-full overflow-hidden" style="width: {width}px;">
    {@render left()}
  </div>
  <div
    class="relative w-px shrink-0 bg-border group/sep"
    role="separator"
    aria-orientation="vertical"
    tabindex="-1"
  >
    <div
      class="absolute inset-y-0 -left-1 -right-1 cursor-col-resize hover:bg-primary/20 transition-colors {dragging ? 'bg-primary/30' : ''}"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      role="presentation"
    ></div>
  </div>
  <div class="flex-1 min-w-0 h-full overflow-hidden">
    {@render right()}
  </div>
</div>
