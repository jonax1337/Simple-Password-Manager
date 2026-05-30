<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import { cn } from "$lib/utils";
  import { tile } from "./recipes";

  type Props = {
    label: string;
    value: string | number;
    description?: string;
    icon?: Component<any, any, any>;
    valueClass?: string;
    class?: string;
    children?: Snippet;
  };

  let {
    label,
    value,
    description,
    icon: Icon,
    valueClass,
    class: klass,
    children,
  }: Props = $props();
</script>

<div class={cn(tile({ padding: "md" }), klass)}>
  <div class="flex items-center justify-between">
    <span class="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
      {label}
    </span>
    {#if Icon}
      <Icon class="size-3.5 text-muted-foreground" />
    {/if}
  </div>
  <div class={cn("text-2xl font-semibold mt-1.5", valueClass)}>{value}</div>
  {#if description}
    <p class="text-2xs text-muted-foreground mt-0.5">{description}</p>
  {/if}
  {@render children?.()}
</div>
