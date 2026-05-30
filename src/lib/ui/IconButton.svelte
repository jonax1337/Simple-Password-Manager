<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import { cn } from "$lib/utils";
  import { iconButton, type IconButtonVariants } from "./recipes";

  type Props = HTMLButtonAttributes &
    IconButtonVariants & {
      class?: string;
      children?: Snippet;
    };

  let {
    class: klass,
    size,
    tone,
    softReveal,
    children,
    type = "button",
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class={cn(iconButton({ size, tone, softReveal }), klass)}
  {...rest}
>
  {@render children?.()}
</button>
