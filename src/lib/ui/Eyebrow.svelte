<script lang="ts">
  import type { Snippet } from "svelte";
  import { cn } from "$lib/utils";
  import { eyebrow } from "./recipes";

  type Props = { class?: string; children?: Snippet };
  let { class: klass, children }: Props = $props();
</script>

<div class={cn(eyebrow, klass)}>
  {@render children?.()}
</div>
