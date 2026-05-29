<script lang="ts">
  import type { Snippet } from "svelte";
  import { Tabs as TabsPrimitive } from "bits-ui";
  import { cn } from "$lib/utils";

  type Tab = { value: string; label: string };

  type Props = {
    tabs: Tab[];
    value?: string;
    class?: string;
    onValueChange?: (value: string) => void;
    children?: Snippet<[string]>;
  };

  let { tabs, value = $bindable(tabs[0]?.value ?? ""), class: klass, onValueChange, children }: Props = $props();
</script>

<TabsPrimitive.Root bind:value {onValueChange} class={cn("flex flex-col gap-4", klass)}>
  <TabsPrimitive.List
    class="inline-flex h-9 items-center justify-start rounded-md bg-muted p-1 text-muted-foreground w-fit"
  >
    {#each tabs as tab (tab.value)}
      <TabsPrimitive.Trigger
        value={tab.value}
        class="inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm"
      >
        {tab.label}
      </TabsPrimitive.Trigger>
    {/each}
  </TabsPrimitive.List>

  {@render children?.(value)}
</TabsPrimitive.Root>
