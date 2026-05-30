<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import { cn } from "$lib/utils";

  // Three-tier pattern (Carbon, Linear, Notion convention):
  //   tone="quiet"   — secondary panes idling. No CTA, monochrome icon.
  //   tone="prompt"  — first-time / empty state where action is expected.
  //                    Includes optional CTA + keyboard hint.
  //   tone="no-results" — search returned nothing. Text-only, no illustration
  //                       (illustrations grow stale when triggered often).
  type Props = {
    tone?: "quiet" | "prompt" | "no-results";
    icon?: Component<any, any, any>;
    title: string;
    description?: string;
    class?: string;
    children?: Snippet;
  };

  let { tone = "quiet", icon: Icon, title, description, class: klass, children }: Props = $props();
</script>

<div
  class={cn(
    "flex h-full flex-col items-center justify-center text-center select-none",
    tone === "no-results" ? "px-6 py-10 gap-2" : "px-10 py-8 gap-3",
    klass,
  )}
>
  {#if Icon && tone !== "no-results"}
    <div class={cn(
      "grid place-items-center rounded-2xl mb-1",
      tone === "quiet"
        ? "size-14 bg-muted text-muted-foreground"
        : "size-14 bg-primary/10 text-primary",
    )}>
      <Icon class="size-6" />
    </div>
  {/if}
  <p class={cn(
    "font-semibold tracking-tight",
    tone === "no-results" ? "text-sm" : "text-md",
  )}>
    {title}
  </p>
  {#if description}
    <p class="text-xs text-muted-foreground max-w-[280px] leading-relaxed">
      {description}
    </p>
  {/if}
  {#if children}
    <div class="mt-2">
      {@render children()}
    </div>
  {/if}
</div>
