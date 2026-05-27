<script lang="ts">
  import { Separator as SeparatorPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  type Props = {
    orientation?: "horizontal" | "vertical";
    class?: string;
  };

  let { orientation = "horizontal", class: klass }: Props = $props();
</script>

<SeparatorPrimitive.Root
  {orientation}
  class={cn(
    "shrink-0 bg-border",
    orientation === "horizontal" ? "h-px w-full" : "h-full w-px",
    klass,
  )}
/>
