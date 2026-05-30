<script lang="ts">
  import type { Snippet } from "svelte";
  import { ContextMenu as Primitive } from "bits-ui";
  import { cn } from "$lib/utils";

  // Right-click menu wrapper. Uses the same `DropdownItem` /
  // `DropdownSeparator` children as the regular DropdownMenu — bits-ui's
  // menu items share a single internal primitive, so the existing items
  // render inside either menu root without modification.
  type Props = {
    // `trigger` receives the bits-ui props bag — the caller forwards it
    // onto a single DOM element so right-click in that area opens the menu.
    trigger: Snippet<[{ props: Record<string, unknown> }]>;
    children: Snippet;
    class?: string;
  };

  let { trigger, children, class: klass }: Props = $props();

  // bits-ui's trigger calls preventDefault on the contextmenu event but not
  // stopPropagation, so a ContextMenu nested inside another ContextMenu
  // would open BOTH menus. We wrap the trigger props to also stop bubbling
  // — innermost wins, exactly like a native right-click menu does.
  function wrapTriggerProps(props: Record<string, unknown>): Record<string, unknown> {
    const inner = props.oncontextmenu as ((e: MouseEvent) => void) | undefined;
    return {
      ...props,
      oncontextmenu: (e: MouseEvent) => {
        e.stopPropagation();
        inner?.(e);
      },
    };
  }
</script>

<Primitive.Root>
  <Primitive.Trigger>
    {#snippet child({ props })}
      {@render trigger({ props: wrapTriggerProps(props) })}
    {/snippet}
  </Primitive.Trigger>
  <Primitive.Portal>
    <Primitive.Content
      class={cn(
        "bg-popover text-popover-foreground z-50 min-w-[10rem] origin-[var(--bits-context-menu-content-transform-origin)] overflow-hidden rounded-md border p-1 shadow-md",
        "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
        klass,
      )}
    >
      {@render children()}
    </Primitive.Content>
  </Primitive.Portal>
</Primitive.Root>
