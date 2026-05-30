<script lang="ts">
  import type { Snippet } from "svelte";
  import { cn } from "$lib/utils";

  type Props = { class?: string; children?: Snippet };
  let { class: klass, children }: Props = $props();
</script>

<kbd
  class={cn(
    "inline-flex items-center justify-center rounded border border-border-strong bg-muted/40 px-1 min-w-[1.25em] h-[1.4em] text-2xs font-mono font-medium leading-none text-muted-foreground",
    klass,
  )}
>
  {@render children?.()}
</kbd>
