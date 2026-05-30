<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import { cn } from "$lib/utils";

  type Props = {
    title: string;
    description?: string;
    icon?: Component<any, any, any>;
    class?: string;
    children?: Snippet;
  };

  let { title, description, icon: Icon, class: klass, children }: Props = $props();
</script>

<div class={cn("space-y-1.5", klass)}>
  <h3 class="text-sm font-medium flex items-center gap-2">
    {#if Icon}<Icon class="h-4 w-4" />{/if}
    {title}
  </h3>
  {#if description}
    <p class="text-xs text-muted-foreground">{description}</p>
  {/if}
  {#if children}
    <div class="pt-2">
      {@render children()}
    </div>
  {/if}
</div>
