<script lang="ts">
  import type { Snippet } from "svelte";
  import { cn } from "$lib/utils";
  type Props = { class?: string; children?: Snippet };
  let { class: klass, children }: Props = $props();
</script>

<div class={cn("grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 px-6", klass)}>
  {@render children?.()}
</div>
