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
    if (!dragging) return;
    const next = Math.max(minWidth, Math.min(maxWidth, e.clientX));
    width = next;
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    persist(width);
  }
</script>

<div class="flex h-full min-h-0 flex-1 overflow-hidden">
  <div class="shrink-0 h-full overflow-hidden border-r bg-card/30" style="width: {width}px;">
    {@render left()}
  </div>
  <div
    class="w-1 cursor-col-resize bg-transparent hover:bg-primary/30 transition-colors {dragging ? 'bg-primary/40' : ''}"
    role="separator"
    aria-orientation="vertical"
    tabindex="-1"
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
  ></div>
  <div class="flex-1 min-w-0 h-full overflow-hidden">
    {@render right()}
  </div>
</div>
