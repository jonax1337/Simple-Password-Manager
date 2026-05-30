<script lang="ts">
  import type { Snippet } from "svelte";
  import { Tooltip as TooltipPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  // Two-form trigger API:
  //   <Tooltip label="…"><Button>…</Button></Tooltip>
  //       — wraps the children in a focusable <span> with the trigger props
  //         attached. Use this for IconButton, Button, etc.
  //   <Tooltip label="…">
  //     {#snippet trigger({ props })}<MyEl {...props} />{/snippet}
  //   </Tooltip>
  //       — explicit form where the caller forwards the trigger props onto
  //         their own DOM node (no wrapper element).
  type Props = {
    children?: Snippet;
    trigger?: Snippet<[{ props: Record<string, unknown> }]>;
    label?: string;
    // Right-aligned keyboard hint shown as kbd badges inside the tooltip.
    // Space-separated, e.g. "Ctrl K" or "⌘ Z".
    shortcut?: string;
    content?: Snippet;
    side?: "top" | "right" | "bottom" | "left";
    align?: "start" | "center" | "end";
    delayMs?: number;
    disabled?: boolean;
    class?: string;
  };

  let {
    children,
    trigger,
    label,
    shortcut,
    content,
    side = "top",
    align = "center",
    delayMs = 350,
    disabled,
    class: klass,
  }: Props = $props();

  const shortcutKeys = $derived(shortcut ? shortcut.split(/\s+/) : []);
</script>

<TooltipPrimitive.Provider delayDuration={delayMs} disableHoverableContent>
  <TooltipPrimitive.Root {disabled}>
    <TooltipPrimitive.Trigger>
      {#snippet child({ props })}
        {#if trigger}
          {@render trigger({ props })}
        {:else}
          <span {...props} class="inline-flex">
            {@render children?.()}
          </span>
        {/if}
      {/snippet}
    </TooltipPrimitive.Trigger>
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        {side}
        {align}
        sideOffset={6}
        class={cn(
          "z-50 max-w-xs rounded-md bg-foreground text-background px-2 py-1 text-2xs shadow-md",
          "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
          "data-[side=bottom]:slide-in-from-top-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1",
          "flex items-center gap-2",
          klass,
        )}
      >
        {#if content}
          {@render content()}
        {:else if label}
          <span class="font-medium leading-tight">{label}</span>
        {/if}
        {#if shortcutKeys.length > 0}
          <span class="flex items-center gap-0.5 opacity-75">
            {#each shortcutKeys as k (k)}
              <kbd
                class="inline-flex items-center justify-center rounded bg-background/15 px-1 min-w-[1.1em] h-[1.25em] text-[10px] font-mono font-medium leading-none"
              >{k}</kbd>
            {/each}
          </span>
        {/if}
      </TooltipPrimitive.Content>
    </TooltipPrimitive.Portal>
  </TooltipPrimitive.Root>
</TooltipPrimitive.Provider>
