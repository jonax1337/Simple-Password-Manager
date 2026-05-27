<script lang="ts" module>
  import { tv, type VariantProps } from "tailwind-variants";

  export const badgeVariants = tv({
    base: "inline-flex items-center justify-center rounded-md border px-2 py-0.5 text-xs font-medium w-fit whitespace-nowrap shrink-0 [&>svg]:size-3 gap-1 [&>svg]:pointer-events-none transition-[color,box-shadow] overflow-hidden",
    variants: {
      variant: {
        default: "border-transparent bg-primary text-primary-foreground",
        secondary: "border-transparent bg-secondary text-secondary-foreground",
        destructive: "border-transparent bg-destructive text-white dark:bg-destructive/60",
        success: "border-transparent bg-success/15 text-success dark:bg-success/25",
        warning: "border-transparent bg-warning/15 text-warning dark:bg-warning/25",
        outline: "text-foreground",
      },
    },
    defaultVariants: { variant: "default" },
  });

  export type BadgeVariant = VariantProps<typeof badgeVariants>["variant"];
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import { cn } from "$lib/utils";

  type Props = {
    variant?: BadgeVariant;
    class?: string;
    children?: Snippet;
  };

  let { variant = "default", class: klass, children }: Props = $props();
</script>

<span class={cn(badgeVariants({ variant }), klass)}>
  {@render children?.()}
</span>
