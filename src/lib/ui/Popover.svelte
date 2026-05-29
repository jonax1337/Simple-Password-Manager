<script lang="ts">
  import type { Snippet } from "svelte";
  import { Popover as PopoverPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  type Props = {
    open?: boolean;
    trigger: Snippet;
    children: Snippet;
    align?: "start" | "center" | "end";
    side?: "top" | "right" | "bottom" | "left";
    class?: string;
  };

  let {
    open = $bindable(false),
    trigger,
    children,
    align = "center",
    side = "bottom",
    class: klass,
  }: Props = $props();
</script>

<PopoverPrimitive.Root bind:open>
  <PopoverPrimitive.Trigger>
    {@render trigger()}
  </PopoverPrimitive.Trigger>
  <PopoverPrimitive.Portal>
    <PopoverPrimitive.Content
      {align}
      {side}
      sideOffset={6}
      class={cn(
        "bg-popover text-popover-foreground z-50 w-72 rounded-md border p-4 shadow-md outline-none",
        klass,
      )}
    >
      {@render children()}
    </PopoverPrimitive.Content>
  </PopoverPrimitive.Portal>
</PopoverPrimitive.Root>
