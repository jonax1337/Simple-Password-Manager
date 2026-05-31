<script lang="ts">
  import type { Snippet } from "svelte";
  import { DropdownMenu as Menu } from "bits-ui";
  import { cn } from "$lib/utils";

  type Props = {
    trigger: Snippet;
    children: Snippet;
    align?: "start" | "center" | "end";
    side?: "top" | "right" | "bottom" | "left";
    class?: string;
    /** Lifted open state so callers can react when the menu actually opens
     * (e.g. lazy-load list contents). Optional — when omitted, the menu
     * manages its own open state internally. */
    open?: boolean;
    onOpenChange?: (open: boolean) => void;
  };

  let {
    trigger,
    children,
    align = "end",
    side = "bottom",
    class: klass,
    open = $bindable(false),
    onOpenChange,
  }: Props = $props();
</script>

<Menu.Root bind:open {onOpenChange}>
  <Menu.Trigger>
    {@render trigger()}
  </Menu.Trigger>
  <Menu.Portal>
    <Menu.Content
      {align}
      {side}
      sideOffset={6}
      class={cn(
        "bg-popover text-popover-foreground z-50 min-w-[10rem] origin-[var(--bits-dropdown-menu-content-transform-origin)] overflow-hidden rounded-md border p-1 shadow-md",
        klass,
      )}
    >
      {@render children()}
    </Menu.Content>
  </Menu.Portal>
</Menu.Root>
