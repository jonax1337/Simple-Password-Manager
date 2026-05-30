<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";
  import { Search, X } from "@lucide/svelte";
  import { cn } from "$lib/utils";

  type Props = HTMLInputAttributes & {
    class?: string;
    value?: string;
    showClear?: boolean;
  };

  let {
    class: klass,
    value = $bindable(""),
    showClear = true,
    placeholder = "Filter…",
    ...rest
  }: Props = $props();

  const hasValue = $derived(typeof value === "string" && value.length > 0);
</script>

<div class={cn("relative", klass)}>
  <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
  <input
    type="text"
    bind:value
    {placeholder}
    class="h-9 w-full rounded-md border border-input bg-background/70 pl-8 pr-7 text-sm transition-shadow focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background focus-visible:border-ring"
    {...rest}
  />
  {#if showClear && hasValue}
    <button
      type="button"
      onclick={() => (value = "")}
      aria-label="Clear"
      class="absolute right-1.5 top-1/2 -translate-y-1/2 size-5 inline-flex items-center justify-center rounded hover:bg-accent text-muted-foreground"
    >
      <X class="size-3" />
    </button>
  {/if}
</div>
