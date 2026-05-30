<script lang="ts" module>
  export type DialogSize = "sm" | "md" | "lg" | "xl" | "fullscreen";
  export type DialogPlacement = "center" | "top";

  // Size token → max-width Tailwind class. xl + fullscreen also constrain
  // height so the content can scroll. Used by the wider settings/about
  // dialogs and by the command palette.
  const SIZE_CLASSES: Record<DialogSize, string> = {
    sm: "max-w-[440px]",
    md: "max-w-lg",
    lg: "max-w-[640px]",
    xl: "w-[min(900px,92vw)] h-[min(640px,82vh)]",
    fullscreen: "w-[min(1100px,96vw)] h-[min(720px,90vh)]",
  };
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import { Dialog as DialogPrimitive } from "bits-ui";
  import { X } from "@lucide/svelte";
  import { cn } from "$lib/utils";
  import { iconButton } from "./recipes";

  type Props = {
    open?: boolean;
    title?: string;
    description?: string;
    size?: DialogSize;
    placement?: DialogPlacement;
    showClose?: boolean;
    // When set, padding/gap are dropped so the dialog can host a fully
    // custom inner layout (e.g. SettingsDialog's split rail + content).
    bare?: boolean;
    class?: string;
    children?: Snippet;
    trigger?: Snippet;
    footer?: Snippet;
    onOpenChange?: (open: boolean) => void;
  };

  let {
    open = $bindable(false),
    title,
    description,
    size = "md",
    placement = "center",
    showClose = true,
    bare = false,
    class: klass,
    children,
    trigger,
    footer,
    onOpenChange,
  }: Props = $props();

  const showHeader = $derived(!!(title || description));
</script>

<DialogPrimitive.Root bind:open {onOpenChange}>
  {#if trigger}
    <DialogPrimitive.Trigger>
      {@render trigger()}
    </DialogPrimitive.Trigger>
  {/if}

  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay
      class="fixed inset-0 z-50 bg-black/50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
    />
    <DialogPrimitive.Content
      class={cn(
        "fixed left-[50%] z-50 w-full translate-x-[-50%] outline-none rounded-xl border bg-background shadow-2xl",
        "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
        placement === "top"
          ? "top-[18%]"
          : "top-[50%] translate-y-[-50%]",
        SIZE_CLASSES[size],
        bare ? "overflow-hidden flex" : "grid gap-4 p-6",
        klass,
      )}
    >
      {#if showHeader}
        <div class="flex flex-col gap-1.5 text-left">
          {#if title}
            <DialogPrimitive.Title class="text-lg leading-none font-semibold tracking-tight">
              {title}
            </DialogPrimitive.Title>
          {:else}
            <DialogPrimitive.Title class="sr-only">Dialog</DialogPrimitive.Title>
          {/if}
          {#if description}
            <DialogPrimitive.Description class="text-muted-foreground text-sm">
              {description}
            </DialogPrimitive.Description>
          {:else}
            <DialogPrimitive.Description class="sr-only">{title ?? "Dialog content"}</DialogPrimitive.Description>
          {/if}
        </div>
      {:else if !bare}
        <DialogPrimitive.Title class="sr-only">Dialog</DialogPrimitive.Title>
        <DialogPrimitive.Description class="sr-only">Dialog content</DialogPrimitive.Description>
      {/if}

      {@render children?.()}

      {#if footer}
        <div class="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          {@render footer()}
        </div>
      {/if}

      {#if showClose}
        <DialogPrimitive.Close
          class={cn(
            iconButton({ size: "sm" }),
            "absolute top-3 right-3",
          )}
        >
          <X class="size-4" />
          <span class="sr-only">Close</span>
        </DialogPrimitive.Close>
      {/if}
    </DialogPrimitive.Content>
  </DialogPrimitive.Portal>
</DialogPrimitive.Root>
