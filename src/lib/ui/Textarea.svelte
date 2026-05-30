<script lang="ts">
  import type { HTMLTextareaAttributes } from "svelte/elements";
  import { cn } from "$lib/utils";

  type Props = HTMLTextareaAttributes & {
    class?: string;
    value?: string | null;
  };

  let { class: klass, value = $bindable(), ...rest }: Props = $props();
</script>

<textarea
  bind:value
  class={cn(
    "border-input placeholder:text-muted-foreground dark:bg-input/30 flex field-sizing-content min-h-16 w-full rounded-md border bg-transparent px-3 py-2 text-sm shadow-xs transition-[color,box-shadow] disabled:cursor-not-allowed disabled:opacity-50",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background focus-visible:border-ring",
    "aria-invalid:ring-2 aria-invalid:ring-destructive/50 aria-invalid:border-destructive",
    klass,
  )}
  {...rest}
></textarea>
