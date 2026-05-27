<script lang="ts">
  import type { Snippet } from "svelte";
  import { Tabs as TabsPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  type Props = {
    value: string;
    class?: string;
    children?: Snippet;
  };

  let { value, class: klass, children }: Props = $props();
</script>

<TabsPrimitive.Content {value} class={cn("focus-visible:outline-none", klass)}>
  {@render children?.()}
</TabsPrimitive.Content>
